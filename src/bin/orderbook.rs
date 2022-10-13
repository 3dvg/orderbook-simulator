use app::{OrderBook, OrderEvent, convert_to_order, OrderExecution};
use csv::Writer;
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
