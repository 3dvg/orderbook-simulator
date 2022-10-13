use app::{FillMetadata, OrderBook, OrderEvent, OrderType, Side};
use uuid::Uuid;

fn main() {
    let mut ob = OrderBook::default();
    let id0 = Uuid::new_v4();
    let event = ob.execute(OrderType::Market {
        id: id0,
        qty: 1.0,
        side: Side::Bid,
    });
    assert_eq!(event, OrderEvent::Unfilled { id: id0 });

    let id1 = Uuid::new_v4();
    let event = ob.execute(OrderType::Limit {
        id: id1,
        price: 120.0,
        qty: 3.0,
        side: Side::Ask,
    });
    assert_eq!(event, OrderEvent::Placed { id: id1 });

    let id2 = Uuid::new_v4();
    let event = ob.execute(OrderType::Market {
        id: id2,
        qty: 4.0,
        side: Side::Bid,
    });
    assert_eq!(
        event,
        OrderEvent::PartiallyFilled {
            id: id2,
            filled_qty: 3.0,
            fills: vec![FillMetadata {
                order_1: id2,
                order_2: id1,
                qty: 3.0,
                price: 120.0,
                taker_side: Side::Bid,
                total_fill: true,
            }],
        },
    );

    println!("ob {:?}", ob.depth(5));
    println!("ob {:?}", ob.traded_volume());
    println!("ob {:?}", ob.last_trade());
    println!("ob {:?}", ob.spread());
}
