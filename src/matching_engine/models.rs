use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Copy, Clone, PartialEq, Serialize)]
pub enum Side {
    Bid,
    Ask,
}

impl std::ops::Not for Side {
    type Output = Side;

    fn not(self) -> Self::Output {
        match self {
            Side::Bid => Side::Ask,
            Side::Ask => Side::Bid,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum OrderType {
    Market {
        id: Uuid,
        side: Side,
        qty: f64,
    },
    Limit {
        id: Uuid,
        side: Side,
        qty: f64,
        price: f64,
    },
    Cancel {
        id: Uuid,
    },
}

#[derive(Debug, PartialEq, Clone, Serialize)]
pub enum OrderEvent {
    Unfilled {
        id: Uuid,
    },
    Placed {
        id: Uuid,
    },
    Canceled {
        id: Uuid,
    },
    PartiallyFilled {
        id: Uuid,
        filled_qty: f64,
        fills: Vec<FillMetadata>,
    },
    Filled {
        id: Uuid,
        filled_qty: f64,
        fills: Vec<FillMetadata>,
    },
}

#[derive(Debug, PartialEq, Copy, Clone, Serialize)]
pub struct FillMetadata {
    pub order_1: Uuid,
    pub order_2: Uuid,
    pub qty: f64,
    pub price: f64,
    pub taker_side: Side,
    pub total_fill: bool,
}

#[derive(Debug, Copy, Clone)]
pub struct Trade {
    pub total_qty: f64,
    pub avg_price: f64,
    pub last_price: f64,
    pub last_qty: f64,
}

#[derive(Debug, PartialEq, Default)]
pub struct LimitOrder {
    pub id: Uuid,
    pub qty: f64,
    pub price: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BookDepth {
    pub levels: usize,
    pub asks: Vec<BookLevel>,
    pub bids: Vec<BookLevel>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BookLevel {
    pub price: f64,
    pub qty: f64,
}