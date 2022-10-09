use app::gbm;
use app::OrderSimulation;
use csv::Writer;
use std::collections::HashMap;
use std::error::Error;
use indexmap::IndexMap;
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // simulator ----------------------------------------------------
    let simulation = OrderSimulation::default();
    let mut receiver = simulation.run().await;
    
    
    let mut wtr = Writer::from_path("./order_simulations/orders.csv")?;
    while let Ok(message) = receiver.recv().await {
        wtr.serialize(message)?;
        wtr.flush()?;
    }
    Ok(())
}
// let vals = gbm::generate_gbm(100.0, 1.0 / 31_536_000.0, 10, 0.15, 0.5);
// println!("vals {:?}", vals);

// use app::{FillMetadata, OrderBook, OrderEvent, OrderType, Side};
// fn main() {
//     let mut ob = OrderBook::default();
//     let event = ob.execute(OrderType::Market {
//         id: 0,
//         qty: 1,
//         side: Side::Bid,
//     });
//     assert_eq!(event, OrderEvent::Unfilled { id: 0 });
//     println!("event: {:?}", event);

//     let event = ob.execute(OrderType::Limit {
//         id: 1,
//         price: 120,
//         qty: 3,
//         side: Side::Ask,
//     });
//     assert_eq!(event, OrderEvent::Placed { id: 1 });
//     println!("event: {:?}", event);

//     let event = ob.execute(OrderType::Market {
//         id: 2,
//         qty: 4,
//         side: Side::Bid,
//     });
//     assert_eq!(
//         event,
//         OrderEvent::PartiallyFilled {
//             id: 2,
//             filled_qty: 3,
//             fills: vec![FillMetadata {
//                 order_1: 2,
//                 order_2: 1,
//                 qty: 3,
//                 price: 120,
//                 taker_side: Side::Bid,
//                 total_fill: true,
//             }],
//         },
//     );
//     println!("event: {:?}", event);
// }