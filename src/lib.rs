pub mod simulator;
use chrono::{DateTime, Utc};
use serde::Serialize;
pub use simulator::gbm;
use simulator::order::{EventType, OrderKind, OrderSide};
pub use simulator::order::{Order, OrderSimulation};

mod matching_engine;
pub use matching_engine::models::{FillMetadata, OrderEvent, OrderType, Side};
pub use matching_engine::orderbook::OrderBook;
use uuid::Uuid;

#[derive(Serialize)]
pub struct OrderExecution {
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

impl From<(u128, Order, String)> for OrderExecution {
    fn from((execution_time, order, status): (u128, Order, String)) -> Self {
        Self {
            id: order.id,
            order_id: order.order_id,
            trader: order.trader,
            event: order.event,
            kind: order.kind,
            side: order.side,
            price: order.price,
            qty: order.qty,
            instrument: order.instrument,
            sequence: order.sequence,
            time: order.time,
            execution_time,
            status,
        }
    }
}

pub fn convert_to_order(order: &Order) -> OrderType {
    let side = match order.side {
        OrderSide::Buy => Side::Bid,
        OrderSide::Sell => Side::Ask,
    };
    let qty = order.qty;
    let price = order.price;
    let id = order.order_id;
    match order.event {
        EventType::Cancel => OrderType::Cancel { id },
        EventType::New => match order.kind {
            OrderKind::Market => OrderType::Market { id, qty, side },
            OrderKind::Limit => OrderType::Limit {
                id,
                qty,
                side,
                price,
            },
        },
        EventType::Update => OrderType::Limit {
            id,
            qty,
            side,
            price,
        },
    }
}
