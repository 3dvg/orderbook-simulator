use chrono::{DateTime, Utc};
use rand::{thread_rng, Rng};
use serde::Serialize;
use std::{collections::HashMap, time};
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

#[derive(Debug, Default, Clone, Serialize)]
pub enum EventType {
    Cancel,
    #[default]
    New,
    Update,
}

#[derive(Debug, Default, Clone, Serialize, PartialEq, Eq)]
enum OrderKind {
    #[default]
    Market,
    Limit,
}

#[derive(Debug, Default, Clone, Serialize)]
enum OrderSide {
    #[default]
    Buy,
    Sell,
}
/// A single order event
#[derive(Debug, Default, Clone, Serialize)]
pub struct Order {
    /// event id
    id: Uuid,
    /// order id
    order_id: Uuid,
    /// positive integer to identify a trader
    trader: u64,
    /// type of event, delete, new, update
    event: EventType,
    /// type of order, 0 for market, 1 for limit
    kind: OrderKind,
    /// side of the order, 0 for sell, 1 for buy
    side: OrderSide,
    /// price of the order
    price: f64,
    /// quantity of the order
    qty: f64,
    /// the instrument to trade,
    instrument: String,
    /// sequence number of the order
    sequence: u64,
    /// time the order was generated in ns
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
    traders: Vec<Trader>,
    price: f64,
    price_dev: f64,
    price_decimals: u32,
    latency_min: u64,
    latency_max: u64,
    qty_max: f64,
    qty_decimals: u32,
    instrument: String,
}

impl OrderGenerator {
    fn gen_order(&mut self, sequence: u64) -> (Order, u64) {
        let mut rng = thread_rng();
        let trader_id: u64 = rng.gen_range(0..self.traders.len() as u64);
        let trader: Trader = self.traders[trader_id as usize].clone();

        let kind: OrderKind = match rng.gen_range(0..=1) {
            0 => OrderKind::Market,
            1 => OrderKind::Limit,
            _ => OrderKind::default(),
        };

        let side: OrderSide = match rng.gen_range(0..=1) {
            0 => OrderSide::Buy,
            1 => OrderSide::Sell,
            _ => OrderSide::default(),
        };
        let mut price: f64 = self.price;
        if kind == OrderKind::Limit {
            match side {
                OrderSide::Buy => price = self.price * (1.0 + rng.gen_range(-self.price_dev..0.0)),
                OrderSide::Sell => price = self.price * (1.0 + rng.gen_range(0.0..self.price_dev)),
            }
        }

        price = f64::trunc(price * 10_u64.pow(self.price_decimals) as f64)
            / 10_u64.pow(self.price_decimals) as f64;

        let mut qty: f64 = rng.gen_range(0.0..1.0) * rng.gen_range(0.0..self.qty_max);
        qty = f64::trunc(qty * 10_u64.pow(self.qty_decimals) as f64)
            / 10_u64.pow(self.qty_decimals) as f64;

        let latency: u64 = rng.gen_range(self.latency_min..self.latency_max);

        let has_limit_orders: bool = trader
            .orders
            .clone()
            .into_values()
            .any(|order| order.kind == OrderKind::Limit);
        let limit_orders: HashMap<Uuid, Order> = trader
            .orders
            .clone()
            .into_iter()
            .filter(|(_k, v)| v.kind == OrderKind::Limit)
            .collect();

        if trader.orders.len() > 0 && has_limit_orders {
            let event = match rng.gen_range(0..=2) {
                0 => {
                    let key_id = rng.gen_range(0..limit_orders.keys().len());
                    let (_key, order) = limit_orders.into_iter().nth(key_id).unwrap();
                    let orders = &mut self.traders[trader_id as usize].orders;
                    orders.remove(&order.id);
                    let mut new_order = order.clone();
                    new_order.id = Uuid::new_v4();
                    new_order.event = EventType::Cancel;
                    new_order.sequence = sequence;
                    new_order.time = chrono::offset::Utc::now();
                    new_order
                }
                1 => {
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
                    orders.insert(order.id.clone(), order.clone());
                    order
                }
                2 => {
                    let key_id = rng.gen_range(0..limit_orders.keys().len());
                    let (key, mut order) = limit_orders.into_iter().nth(key_id).unwrap();
                    let orders = &mut self.traders[trader_id as usize].orders;

                    let (update_price, update_qty) = match rng.gen_range(0..=1) {
                        0 => {
                            price =
                                self.price * (1.0 + rng.gen_range(-self.price_dev..self.price_dev));
                            price = f64::trunc(price * 10_u64.pow(self.price_decimals) as f64)
                                / 10_u64.pow(self.price_decimals) as f64;
                            order.price = price;
                            (Some(price), None)
                        }
                        1 => {
                            qty = rng.gen_range(0.0..1.0) * rng.gen_range(0.0..self.qty_max);
                            qty = f64::trunc(qty * 10_u64.pow(self.qty_decimals) as f64)
                                / 10_u64.pow(self.qty_decimals) as f64;
                            order.qty = qty;
                            (None, Some(qty))
                        }
                        _ => (None, None),
                    };
                    orders.insert(key.clone(), order.clone());

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
                    order
                }
                _ => Order::default(),
            };

            (event, latency)
        } else {
            let order: Order = Order::new(
                trader_id,
                kind,
                side,
                price,
                qty,
                self.instrument.clone(),
                sequence,
            );
            // self.manage_order(trader_id, &order);
            let orders = &mut self.traders[trader_id as usize].orders;
            orders.insert(order.id.clone(), order.clone());
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
        price: f64,
        price_dev: f64,
        price_decimals: u32,
        latency_min: u64,
        latency_max: u64,
        qty_max: f64,
        qty_decimals: u32,
        instrument: String,
    ) -> Self {
        if latency_max < latency_min {
            panic!("Max latency has to be greater than latency_min")
        }
        if price < price_dev {
            panic!("Price has to be greater than price_dev")
        }
        let (sender, _receiver) = broadcast::channel(1_000_000);
        let traders: Vec<Trader> = generate_traders(n_traders);
        let generator = OrderGenerator {
            max_orders,
            traders,
            price,
            price_dev,
            price_decimals,
            latency_min,
            latency_max,
            qty_max,
            qty_decimals,
            instrument,
        };
        Self { generator, sender }
    }

    pub async fn run(&self) -> Receiver<Order> {
        let simulation = self.clone();
        let n_chunks = 100;
        let mut order_chunks = Vec::with_capacity(n_chunks);
        for chunk in 1..=n_chunks {
            let prior = simulation.generator.max_orders / n_chunks as u64 * (chunk as u64 - 1);
            let next = simulation.generator.max_orders / n_chunks as u64 * chunk as u64;
            order_chunks.push(prior..next);
        }
        let _silly_receiver = self.sender.subscribe();
        for chunk in order_chunks {
            let mut simulation = self.clone();
            println!("running chunk {:?}...", chunk);
            tokio::spawn(async move {
                for i in chunk.clone() {
                    let (event, latency) = simulation.generator.gen_order(i);
                    let _ = simulation
                        .sender
                        .send(event)
                        .map(|_| {})
                        .map_err(|err| println!("Error: {}", err));
                    tokio::time::sleep(time::Duration::from_nanos(latency)).await;
                }
                println!("finished chunk {:?}", chunk);
            });
        }
        self.sender.subscribe()
    }

    pub fn get_receiver(&self) -> Receiver<Order> {
        self.sender.subscribe()
    }
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
        // Self::new(1_000_000, 100_000, 142.45, 0.5, 2, 0, 1, 10_000.0, 0, "AAPL".to_string())
        // dev
        Self::new(
            1_000,
            100,
            142.45,
            0.5,
            2,
            0,
            1,
            10_000.0,
            0,
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
