use ordered_float::OrderedFloat;
use std::collections::BTreeMap;
use uuid::Uuid;

use crate::matching_engine::arena::OrderArena;
use crate::matching_engine::models::{FillMetadata, OrderEvent, OrderType, Side, Trade};

use super::models::{BookDepth, BookLevel};

const DEFAULT_ARENA_CAPACITY: usize = 1_000_000;
const DEFAULT_QUEUE_CAPACITY: usize = 100_000;

#[derive(Debug)]
pub struct OrderBook {
    last_trade: Option<Trade>,
    traded_volume: f64,
    best_ask: Option<OrderedFloat<f64>>,
    best_bid: Option<OrderedFloat<f64>>,
    asks: BTreeMap<OrderedFloat<f64>, Vec<usize>>,
    bids: BTreeMap<OrderedFloat<f64>, Vec<usize>>,
    arena: OrderArena,
    default_queue_capacity: usize,
}

impl Default for OrderBook {
    fn default() -> Self {
        Self::new(DEFAULT_ARENA_CAPACITY, DEFAULT_QUEUE_CAPACITY)
    }
}

impl OrderBook {
    pub fn new(arena_capacity: usize, queue_capacity: usize) -> Self {
        Self {
            last_trade: None,
            traded_volume: 0.0,
            best_ask: None,
            best_bid: None,
            asks: BTreeMap::new(),
            bids: BTreeMap::new(),
            arena: OrderArena::new(arena_capacity),
            default_queue_capacity: queue_capacity,
        }
    }

    pub fn get_asks(&self) -> BTreeMap<OrderedFloat<f64>, Vec<usize>> {
        self.asks.clone()
    }

    #[inline(always)]
    pub fn best_ask(&self) -> Option<OrderedFloat<f64>> {
        self.best_ask
    }

    #[inline(always)]
    pub fn best_bid(&self) -> Option<OrderedFloat<f64>> {
        self.best_bid
    }

    #[inline(always)]
    pub fn spread(&self) -> Option<OrderedFloat<f64>> {
        match (self.best_bid, self.best_ask) {
            (Some(b), Some(a)) => Some(a - b),
            _ => None,
        }
    }

    #[inline(always)]
    pub fn last_trade(&self) -> Option<Trade> {
        self.last_trade
    }

    #[inline(always)]
    pub fn traded_volume(&self) -> f64 {
        self.traded_volume
    }

    pub fn execute(&mut self, event: OrderType) -> OrderEvent {
        let event = self._execute(event);
        // uncomment to track total_volume, last_trade
        // match event.clone() {
        //     OrderEvent::Filled {
        //         id: _,
        //         filled_qty,
        //         fills,
        //     } => {
        //         self.traded_volume += filled_qty;
        //         let last_fill = fills.last().unwrap();
        //         self.last_trade = Some(Trade {
        //             total_qty: filled_qty,
        //             avg_price: fills.iter().map(|fm| fm.price * fm.qty).sum::<f64>() as f64
        //                 / (filled_qty as f64),
        //             last_qty: last_fill.qty,
        //             last_price: last_fill.price,
        //         });
        //     }
        //     OrderEvent::PartiallyFilled {
        //         id: _,
        //         filled_qty,
        //         fills,
        //     } => {
        //         self.traded_volume += filled_qty;
        //         let last_fill = fills.last().unwrap();
        //         self.last_trade = Some(Trade {
        //             total_qty: filled_qty,
        //             avg_price: fills.iter().map(|fm| fm.price * fm.qty).sum::<f64>() as f64
        //                 / (filled_qty as f64),
        //             last_qty: last_fill.qty,
        //             last_price: last_fill.price,
        //         });
        //     }
        //     _ => {}
        // }
        event
    }

    fn _execute(&mut self, event: OrderType) -> OrderEvent {
        match event {
            OrderType::Market { id, side, qty } => {
                let (fills, partial, filled_qty) = self.market(id, side, qty);
                if fills.is_empty() {
                    OrderEvent::Unfilled { id }
                } else if partial {
                    OrderEvent::PartiallyFilled {
                        id,
                        filled_qty,
                        fills,
                    }
                } else {
                    OrderEvent::Filled {
                        id,
                        filled_qty,
                        fills,
                    }
                }
            }
            OrderType::Limit {
                id,
                side,
                qty,
                price,
            } => {
                let (fills, partial, filled_qty) = self.limit(id, side, qty, price);

                if fills.is_empty() {
                    OrderEvent::Placed { id }
                } else if partial {
                    OrderEvent::PartiallyFilled {
                        id,
                        filled_qty,
                        fills,
                    }
                } else {
                    OrderEvent::Filled {
                        id,
                        filled_qty,
                        fills,
                    }
                }
            }
            OrderType::Cancel { id } => {
                self.cancel(id);
                OrderEvent::Canceled { id }
            }
        }
    }

    fn cancel(&mut self, id: Uuid) -> bool {
        if let Some((price, idx)) = self.arena.get(id) {
            if let Some(queue) = self.asks.get_mut(&OrderedFloat(price)) {
                if let Some(i) = queue.iter().position(|i| *i == idx) {
                    queue.remove(i);
                    self.update_best_ask();
                }
            }
            if let Some(queue) = self.bids.get_mut(&OrderedFloat(price)) {
                if let Some(i) = queue.iter().position(|i| *i == idx) {
                    queue.remove(i);
                }
                self.update_best_bid();
            }
        }
        self.arena.delete(&id)
    }

    fn market(&mut self, id: Uuid, side: Side, qty: f64) -> (Vec<FillMetadata>, bool, f64) {
        let mut fills = Vec::new();

        let remaining_qty = match side {
            Side::Bid => self.match_with_asks(id, qty, &mut fills, None),
            Side::Ask => self.match_with_bids(id, qty, &mut fills, None),
        };

        let partial = remaining_qty > 0.0;

        (fills, partial, qty - remaining_qty)
    }

    fn limit(
        &mut self,
        id: Uuid,
        side: Side,
        qty: f64,
        price: f64,
    ) -> (Vec<FillMetadata>, bool, f64) {
        let mut partial = false;
        let remaining_qty;
        let mut fills: Vec<FillMetadata> = Vec::new();

        match side {
            Side::Bid => {
                remaining_qty = self.match_with_asks(id, qty, &mut fills, Some(price));
                if remaining_qty > 0.0 {
                    partial = true;
                    let index = self.arena.insert(id, price, remaining_qty);
                    let queue_capacity = self.default_queue_capacity;
                    self.bids
                        .entry(OrderedFloat(price))
                        .or_insert_with(|| Vec::with_capacity(queue_capacity))
                        .push(index);
                    match self.best_bid {
                        None => {
                            self.best_bid = Some(OrderedFloat(price));
                        }
                        Some(b) if price > *b => {
                            self.best_bid = Some(OrderedFloat(price));
                        }
                        _ => {}
                    };
                }
            }
            Side::Ask => {
                remaining_qty = self.match_with_bids(id, qty, &mut fills, Some(price));
                if remaining_qty > 0.0 {
                    partial = true;
                    let index = self.arena.insert(id, price, remaining_qty);
                    if let Some(a) = self.best_ask {
                        if price < *a {
                            self.best_ask = Some(OrderedFloat(price));
                        }
                    }
                    let queue_capacity = self.default_queue_capacity;
                    self.asks
                        .entry(OrderedFloat(price))
                        .or_insert_with(|| Vec::with_capacity(queue_capacity))
                        .push(index);
                    match self.best_ask {
                        None => {
                            self.best_ask = Some(OrderedFloat(price));
                        }
                        Some(a) if price < *a => {
                            self.best_ask = Some(OrderedFloat(price));
                        }
                        _ => {}
                    };
                }
            }
        }

        (fills, partial, qty - remaining_qty)
    }

    fn match_with_asks(
        &mut self,
        id: Uuid,
        qty: f64,
        fills: &mut Vec<FillMetadata>,
        limit_price: Option<f64>,
    ) -> f64 {
        let mut remaining_qty = qty;
        let mut update_bid_ask = false;
        for (ask_price, queue) in self.asks.iter_mut() {
            if queue.is_empty() {
                continue;
            }
            if (update_bid_ask || self.best_ask.is_none()) && !queue.is_empty() {
                self.best_ask = Some(*ask_price);
                update_bid_ask = false;
            }
            if let Some(lp) = limit_price {
                if lp < **ask_price {
                    break;
                }
            }
            if remaining_qty == 0.0 {
                break;
            }
            let filled_qty =
                Self::process_queue(&mut self.arena, queue, remaining_qty, id, Side::Bid, fills);
            if queue.is_empty() {
                update_bid_ask = true;
            }
            remaining_qty -= filled_qty;
        }

        self.update_best_ask();
        remaining_qty
    }

    fn match_with_bids(
        &mut self,
        id: Uuid,
        qty: f64,
        fills: &mut Vec<FillMetadata>,
        limit_price: Option<f64>,
    ) -> f64 {
        let mut remaining_qty = qty;
        let mut update_bid_ask = false;
        for (bid_price, queue) in self.bids.iter_mut().rev() {
            if queue.is_empty() {
                continue;
            }
            if (update_bid_ask || self.best_bid.is_none()) && !queue.is_empty() {
                self.best_bid = Some(*bid_price);
                update_bid_ask = false;
            }
            if let Some(lp) = limit_price {
                if lp > **bid_price {
                    break;
                }
            }
            if remaining_qty == 0.0 {
                break;
            }
            let filled_qty =
                Self::process_queue(&mut self.arena, queue, remaining_qty, id, Side::Ask, fills);
            if queue.is_empty() {
                update_bid_ask = true;
            }
            remaining_qty -= filled_qty;
        }

        self.update_best_bid();
        remaining_qty
    }

    fn update_best_ask(&mut self) {
        let mut cur_asks = self.asks.iter().filter(|(_, q)| !q.is_empty());
        self.best_ask = cur_asks.next().map(|(p, _)| *p);
    }

    fn update_best_bid(&mut self) {
        let mut cur_bids = self.bids.iter().rev().filter(|(_, q)| !q.is_empty());
        self.best_bid = cur_bids.next().map(|(p, _)| *p);
    }

    fn process_queue(
        arena: &mut OrderArena,
        opposite_orders: &mut Vec<usize>,
        remaining_qty: f64,
        id: Uuid,
        side: Side,
        fills: &mut Vec<FillMetadata>,
    ) -> f64 {
        let mut qty_to_fill = remaining_qty;
        let mut filled_qty = 0.0;
        let mut filled_index = None;

        for (index, head_order_idx) in opposite_orders.iter_mut().enumerate() {
            if qty_to_fill == 0.0 {
                break;
            }
            let head_order = &mut arena[*head_order_idx];
            let traded_price = head_order.price;
            let available_qty = head_order.qty;
            if available_qty == 0.0 {
                filled_index = Some(index);
                continue;
            }
            let traded_quantity: f64;
            let filled;

            if qty_to_fill >= available_qty {
                traded_quantity = available_qty;
                qty_to_fill -= available_qty;
                filled_index = Some(index);
                filled = true;
            } else {
                traded_quantity = qty_to_fill;
                qty_to_fill = 0.0;
                filled = false;
            }
            head_order.qty -= traded_quantity;
            let fill = FillMetadata {
                order_1: id,
                order_2: head_order.id,
                qty: traded_quantity,
                price: traded_price,
                taker_side: side,
                total_fill: filled,
            };
            fills.push(fill);
            filled_qty += traded_quantity;
        }
        if let Some(index) = filled_index {
            opposite_orders.drain(0..index + 1);
        }

        filled_qty
    }

    pub fn depth(&self, levels: usize) -> BookDepth {
        let mut asks: Vec<BookLevel> = Vec::with_capacity(levels);
        let mut bids: Vec<BookLevel> = Vec::with_capacity(levels);

        for (i, (ask_price, queue)) in self.asks.iter().enumerate() {
            if i > levels {
                break;
            }
            let mut qty = 0.0;
            for idx in queue {
                qty += self.arena[*idx].qty;
            }
            if qty > 0.0 {
                asks.push(BookLevel {
                    price: **ask_price,
                    qty,
                });
            }
        }

        for (i, (bid_price, queue)) in self.bids.iter().enumerate() {
            if i > levels {
                break;
            }
            let mut qty = 0.0;
            for idx in queue {
                qty += self.arena[*idx].qty;
            }
            if qty > 0.0 {
                bids.push(BookLevel {
                    price: **bid_price,
                    qty,
                });
            }
        }
        BookDepth { levels, asks, bids }
    }
}
