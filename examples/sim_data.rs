use app::{OrderBook, OrderType, Side, simulator::order::{EventType, OrderKind, OrderSide}, OrderEvent};
use chrono::{DateTime, Utc};
use csv::Writer;
use serde::Serialize;
use uuid::Uuid;
use std::{time::Instant};
use app::Order;
use anyhow::{Result, Error};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path("order_simulations/orders.csv")?;
    
    let mut ob = OrderBook::default();
    let mut wtr_orders = Writer::from_path("./executions/orders.csv")?;
    // let begin = Instant::now();
    for msg in rdr.deserialize() {
        let begin = Instant::now();
        let order_request: Order = msg?;
        let order = convert_to_order(&order_request);
        let event = ob.execute(order);
        let elapsed = begin.elapsed().as_nanos();
        // println!("-- {:?}", elapsed);
        let status = match event {
            OrderEvent::Unfilled { id } => "Unfilled".to_string(),
            OrderEvent::Placed { id } => "Placed".to_string(),
            OrderEvent::Canceled { id } => "Canceled".to_string(),
            OrderEvent::PartiallyFilled { id, filled_qty, fills } => "PartiallyFilled".to_string(),
            OrderEvent::Filled { id, filled_qty, fills } => "Filled".to_string(),

        };
        wtr_orders.serialize(OrderExecution::from((elapsed, order_request,status)))?;
        wtr_orders.flush()?;
    }
    // let elapsed = begin.elapsed().as_millis();
    // println!("-- {:?}", elapsed);
    Ok(())
}

#[derive(Serialize)]
struct OrderExecution {
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
    pub execution_time: u128,
    pub status: String,
}

impl From<(u128, Order, String)> for OrderExecution{
    fn from((execution_time, order, status): (u128, Order, String)) -> Self {
        Self {
            id:order.id,
            order_id:order.order_id,
            trader:order.trader,
            event:order.event,
            kind:order.kind,
            side:order.side,
            price:order.price,
            qty:order.qty,
            instrument:order.instrument,
            sequence:order.sequence,
            time:order.time,
            execution_time,
            status,
        }
    }
}

fn convert_to_order(order: &Order) -> OrderType {
    let side = match order.side {
        OrderSide::Buy => Side::Bid,
        OrderSide::Sell => Side::Ask,
    };
    let qty = order.qty;
    let price = order.price;
    let id = order.order_id;
    match order.event {
        EventType::Cancel => {
            OrderType::Cancel {
                    id,
                }
            },
        EventType::New => {
            match order.kind {
                OrderKind::Market => {
                    OrderType::Market {
                        id,
                        qty,
                        side,
                    }
                },
                OrderKind::Limit => {
                    OrderType::Limit {
                        id,
                        qty,
                        side,
                        price 
                    }
                },
            }
        },
        EventType::Update => {
                    OrderType::Limit {
                        id,
                        qty,
                        side,
                        price 
                    }
            },
    }

}