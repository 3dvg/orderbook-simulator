use anyhow::Error;
use chrono::{DateTime, Utc};
use indicatif::ProgressBar;
use log::*;
use rand::{rngs::ThreadRng, thread_rng, Rng};
use rand_distr::{Distribution, Normal};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, ops::Range};
use tokio::sync::broadcast::{self, Receiver, Sender};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct CancelOrder {
    id: Uuid,
    order_id: Uuid,
    time: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateOrder {
    id: Uuid,
    order_id: Uuid,
    price: Option<f64>,
    qty: Option<f64>,
    time: i64,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub enum EventType {
    Cancel,
    #[default]
    New,
    Update,
}

#[derive(Debug, Default, Clone, Serialize, PartialEq, Eq, Deserialize)]
pub enum OrderKind {
    Market,
    #[default]
    Limit,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub enum OrderSide {
    #[default]
    Buy,
    Sell,
}
/// A single order event
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: Uuid,
    pub order_id: Uuid,
    pub trader: u64,
    pub event: EventType,
    pub kind: OrderKind,
    pub side: OrderSide,
    pub price: f64,
    pub qty: f64,
    pub instrument: String,
    pub sequence: u64,
    pub time: DateTime<Utc>,
}

impl Order {
    fn new(
        trader: u64,
        kind: OrderKind,
        side: OrderSide,
        price: f64,
        qty: f64,
        instrument: String,
        sequence: u64,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            order_id: Uuid::new_v4(),
            trader,
            kind,
            event: EventType::New,
            side,
            price,
            qty,
            instrument,
            sequence,
            time: chrono::offset::Utc::now(),
        }
    }
}

#[derive(Debug, Clone)]
struct OrderGenerator {
    max_orders: u64,
    n_tasks: u64,
    traders: Vec<Trader>,
    price: f64,
    price_dev: f64,
    price_decimals: u32,
    latency_min: u64,
    latency_max: u64,
    qty_max: f64,
    qty_decimals: u32,
    pct_limit_orders: f64,
    instrument: String,
}

impl OrderGenerator {
    fn get_kind(&self, rng: &mut ThreadRng) -> OrderKind {
        // match rng.gen_range(0..=1) {
        //     0 => OrderKind::Market,
        //     1 => OrderKind::Limit,
        //     _ => OrderKind::default(),
        // }
        //70% provide liquidity
        let n = rng.gen_range(0.00..0.99);
        if n < self.pct_limit_orders {
            OrderKind::Limit
        } else {
            OrderKind::Market
        }
    }

    fn get_side(&self, rng: &mut ThreadRng) -> OrderSide {
        match rng.gen_range(0..=1) {
            0 => OrderSide::Buy,
            1 => OrderSide::Sell,
            _ => OrderSide::default(),
        }
    }

    fn get_price(&self, rng: &mut ThreadRng, kind: &OrderKind, side: &OrderSide) -> f64 {
        let mut price: f64 = self.price;
        let dist = Normal::new(self.price, self.price_dev).unwrap();
        // let dist = Normal::from_mean_cv(self.price, 0.01).unwrap();
        if *kind == OrderKind::Limit {
            match side {
                // OrderSide::Buy => price = self.price * (1.0 + rng.gen_range(-0.1..0.0)),
                // OrderSide::Sell => price = self.price * (1.0 + rng.gen_range(0.0..0.1)),
                OrderSide::Buy => price = dist.sample(rng),
                OrderSide::Sell => price = dist.sample(rng),
            }
        }

        price = f64::trunc(price * 10_u64.pow(self.price_decimals) as f64)
            / 10_u64.pow(self.price_decimals) as f64;
        price
    }

    fn get_qty(&self, rng: &mut ThreadRng, kind: &OrderKind) -> f64 {
        // If is market, the qty will be no more than half the qty for limits, with a max of 1/4th the size
        // of limits. This will make the OB liquidity to increase
        let mut qty = match kind {
            OrderKind::Limit => rng.gen_range(1.0..self.qty_max),
            OrderKind::Market => rng.gen_range(0.0..0.25) * rng.gen_range(1.0..self.qty_max),
        };
        qty = f64::trunc(qty * 10_u64.pow(self.qty_decimals) as f64)
            / 10_u64.pow(self.qty_decimals) as f64;
        qty
    }

    fn cancel_order(
        &mut self,
        rng: &mut ThreadRng,
        limit_orders: &mut HashMap<Uuid, Order>,
        trader_id: usize,
        sequence: u64,
    ) -> Order {
        let key_id = rng.gen_range(0..limit_orders.keys().len());
        let (_key, order) = limit_orders.iter_mut().nth(key_id).unwrap();
        let orders = &mut self.traders[trader_id as usize].orders;
        orders.remove(&order.id);
        order.id = Uuid::new_v4();
        order.event = EventType::Cancel;
        order.sequence = sequence;
        order.time = chrono::offset::Utc::now();
        order.to_owned()
    }

    fn new_order(
        &mut self,
        trader_id: u64,
        kind: OrderKind,
        side: OrderSide,
        price: f64,
        qty: f64,
        sequence: u64,
    ) -> Order {
        let order: Order = Order::new(
            trader_id,
            kind,
            side,
            price,
            qty,
            self.instrument.clone(),
            sequence,
        );
        let orders = &mut self.traders[trader_id as usize].orders;
        orders.insert(order.id, order.clone());
        order
    }

    fn update_order(
        &mut self,
        rng: &mut ThreadRng,
        limit_orders: &mut HashMap<Uuid, Order>,
        trader_id: usize,
        sequence: u64,
        mut price: f64,
        mut qty: f64,
    ) -> Order {
        let key_id = rng.gen_range(0..limit_orders.keys().len());
        let (key, order) = limit_orders.iter_mut().nth(key_id).unwrap();
        let orders = &mut self.traders[trader_id as usize].orders;
        // let mut updated_price = price;
        // let mut updated_qty = qty;
        let (update_price, update_qty) = match rng.gen_range(0..=1) {
            0 => {
                // updated_price = self.price * (1.0 + rng.gen_range(-self.price_dev..self.price_dev));
                price = f64::trunc(price * 10_u64.pow(self.price_decimals) as f64)
                    / 10_u64.pow(self.price_decimals) as f64;
                order.price = price;
                (Some(price), None)
            }
            1 => {
                // updated_qty = rng.gen_range(0.0..self.qty_max);
                qty = f64::trunc(qty * 10_u64.pow(self.qty_decimals) as f64)
                    / 10_u64.pow(self.qty_decimals) as f64;
                order.qty = qty;
                (None, Some(qty))
            }
            _ => (None, None),
        };
        orders.insert(*key, order.clone());

        order.id = Uuid::new_v4();
        match update_price {
            Some(price) => order.price = price,
            None => {}
        };
        match update_qty {
            Some(qty) => order.qty = qty,
            None => {}
        };
        order.event = EventType::Update;
        order.sequence = sequence;
        order.time = chrono::offset::Utc::now();
        order.to_owned()
    }

    fn gen_order(&mut self, sequence: u64) -> (Order, u64) {
        let mut rng = thread_rng();
        let trader_id: u64 = rng.gen_range(0..self.traders.len() as u64);
        let trader: Trader = self.traders[trader_id as usize].clone();
        let kind = self.get_kind(&mut rng);
        let side = self.get_side(&mut rng);
        let price: f64 = self.get_price(&mut rng, &kind, &side);
        let qty: f64 = self.get_qty(&mut rng, &kind);
        let latency: u64 = rng.gen_range(self.latency_min..self.latency_max);

        let has_limit_orders: bool = trader
            .orders
            .clone()
            .into_values()
            .any(|order| order.kind == OrderKind::Limit);

        if trader.orders.len() > 0 && has_limit_orders {
            let mut limit_orders: HashMap<Uuid, Order> = trader
                .orders
                .into_iter()
                .filter(|(_k, v)| v.kind == OrderKind::Limit)
                .collect();
            let event = match rng.gen_range(0..=2) {
                0 => self.cancel_order(&mut rng, &mut limit_orders, trader_id as usize, sequence),
                1 => self.new_order(trader_id, kind, side, price, qty, sequence),
                2 => self.update_order(
                    &mut rng,
                    &mut limit_orders,
                    trader_id as usize,
                    sequence,
                    price,
                    qty,
                ),
                _ => Order::default(),
            };

            (event, latency)
        } else {
            let order = self.new_order(trader_id, kind, side, price, qty, sequence);
            (order, latency)
        }
    }
}

#[derive(Debug, Clone)]
pub struct Trader {
    id: u64,
    orders: HashMap<Uuid, Order>,
}

#[derive(Clone)]
pub struct OrderSimulation {
    generator: OrderGenerator,
    sender: Sender<Order>,
}

impl OrderSimulation {
    pub fn new(
        max_orders: u64,
        n_traders: u64,
        n_tasks: u64,
        price: f64,
        price_dev: f64,
        price_decimals: u32,
        latency_min: u64,
        latency_max: u64,
        qty_max: f64,
        qty_decimals: u32,
        pct_limit_orders: f64,
        instrument: String,
    ) -> Self {
        if latency_max < latency_min {
            panic!("Max latency has to be greater than latency_min")
        }
        if price < price_dev {
            panic!("Price has to be greater than price_dev")
        }
        let (sender, _receiver) = broadcast::channel(10_000_000);
        let traders: Vec<Trader> = generate_traders(n_traders);
        let generator = OrderGenerator {
            max_orders,
            n_tasks,
            traders,
            price,
            price_dev,
            price_decimals,
            latency_min,
            latency_max,
            qty_max,
            qty_decimals,
            pct_limit_orders,
            instrument,
        };
        Self { generator, sender }
    }

    pub async fn run(&self) -> Receiver<Order> {
        let simulation = self.clone();
        let n_chunks = self.generator.n_tasks;
        let max_orders = simulation.generator.max_orders;
        let step = max_orders / n_chunks;
        let order_chunks = (0..=max_orders - step)
            .step_by(step as usize)
            .map(|current| (current..current + step));

        let mut tasks = vec![];

        let receiver = self.sender.subscribe();
        let bar = ProgressBar::new(n_chunks);

        info!("Spawning {n_chunks} tasks to generate {max_orders} orders");
        for chunk in order_chunks {
            let simulation = self.clone();
            // println!("running chunk {:?}...", chunk);
            let task = tokio::spawn(run_orders(chunk, simulation));
            tasks.push(task);
            bar.inc(1);
        }
        bar.finish();
        let _join_result = futures::future::join_all(tasks).await;
        receiver
    }

    pub fn get_receiver(&self) -> Receiver<Order> {
        self.sender.subscribe()
    }
}

async fn run_orders(chunk: Range<u64>, mut simulation: OrderSimulation) -> Result<(), Error> {
    for i in chunk {
        let (event, _) = simulation.generator.gen_order(i);
        let _ = simulation
            .sender
            .send(event)
            .map(|_| {})
            .map_err(|err| println!("Error: {}", err));
        // tokio::time::sleep(time::Duration::from_nanos(latency)).await;
    }
    Ok(())
}

fn generate_traders(n_traders: u64) -> Vec<Trader> {
    let mut traders: Vec<Trader> = Vec::with_capacity(n_traders as usize);
    for i in 0..traders.capacity() {
        traders.push(Trader {
            id: i as u64,
            orders: HashMap::new(),
        });
    }
    traders
}

impl Default for OrderSimulation {
    fn default() -> Self {
        // gen
        // Self::new(1_000_000, 100_000, 1000, 142.45, 0.5, 2, 0, 1, 10_000.0, 0, "AAPL".to_string())
        // Self::new(
        //     1_000_000,
        //     100_000,
        //     1000,
        //     100.0,
        //     0.02,
        //     2,
        //     0,
        //     1,
        //     10_000.0,
        //     0,
        //     0.7,
        //     "AAPL".to_string(),
        // )
        // Self::new(
        //     10_000_000,
        //     1_000_000,
        //     1000,
        //     100.0,
        //     0.02,
        //     2,
        //     0,
        //     1,
        //     10_000.0,
        //     0,
        //     "AAPL".to_string(),
        // )
        // dev
        Self::new(
            1000,
            10,
            10,
            100.0,
            2.0,
            2,
            0,
            1,
            10_000.0,
            0,
            0.75,
            "AAPL".to_string(),
        )
    }
}

#[cfg(test)]
mod tests {
    // use crate::*;

    #[test]
    fn new_order() {
        // assert!(Order::new(1,1,1.0,1.0));
    }
}
