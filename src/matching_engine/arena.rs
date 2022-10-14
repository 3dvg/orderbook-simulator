use crate::matching_engine::models::LimitOrder;
use indexmap::IndexMap;
use std::ops::{Index, IndexMut};
use uuid::Uuid;

#[derive(Debug)]
pub struct OrderArena {
    order_map: IndexMap<Uuid, LimitOrder>,
}

impl OrderArena {
    pub fn new(capacity: usize) -> Self {
        Self {
            order_map: IndexMap::with_capacity(capacity),
        }
    }

    pub fn get(&self, id: Uuid) -> Option<(f64, usize)> {
        self.order_map
            .get_full(&id)
            .map(|(index, _key, order)| (order.price, index))
    }

    pub fn insert(&mut self, id: Uuid, price: f64, qty: f64) -> usize {
        let (index, _limit_order) = self
            .order_map
            .insert_full(id, LimitOrder { id, price, qty });
        index
    }

    pub fn delete(&mut self, key: &Uuid) -> bool {
        if let Some((_index, _key, order)) = self.order_map.get_full_mut(key) {
            order.qty = 0.0;
            return true;
        }
        false
    }
}

impl Index<usize> for OrderArena {
    type Output = LimitOrder;

    #[inline]
    fn index(&self, index: usize) -> &LimitOrder {
        let (_key, order) = self.order_map.get_index(index).unwrap();
        order
    }
}

impl IndexMut<usize> for OrderArena {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut LimitOrder {
        let (_key, order) = self.order_map.get_index_mut(index).unwrap();
        order
    }
}
