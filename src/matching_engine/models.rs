
#[derive(Debug, Copy, Clone, PartialEq)]
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
        id: u128,
        side: Side,
        qty: u64,
    },
    Limit {
        id: u128,
        side: Side,
        qty: u64,
        price: u64,
    },
    Cancel {
        id: u128,
    },
}

#[derive(Debug, PartialEq, Clone)]
pub enum OrderEvent {
    Unfilled {
        id: u128,
    },
    Placed {
        id: u128,
    },
    Canceled {
        id: u128,
    },
    PartiallyFilled {
        id: u128,
        filled_qty: u64,
        fills: Vec<FillMetadata>,
    },
    Filled {
        id: u128,
        filled_qty: u64,
        fills: Vec<FillMetadata>,
    },
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct FillMetadata {
    pub order_1: u128,
    pub order_2: u128,
    pub qty: u64,
    pub price: u64,
    pub taker_side: Side,
    pub total_fill: bool,
}

#[derive(Debug, Copy, Clone)]
pub struct Trade {
    pub total_qty: u64,
    pub avg_price: f64,
    pub last_price: u64,
    pub last_qty: u64,
}

#[derive(Debug, PartialEq, Default)]
pub struct LimitOrder {
    pub id: u128,
    pub qty: u64,
    pub price: u64,
}
