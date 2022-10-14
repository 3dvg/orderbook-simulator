use anyhow::{Error, Result};
use app::Order;
use app::{convert_to_order, OrderBook, OrderEvent, OrderExecution};
use csv::{Writer};
use indicatif::ProgressBar;
use log::{info, LevelFilter};
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Error> {
    pretty_env_logger::formatted_timed_builder()
        .filter_level(LevelFilter::Info)
        .init();

    let reader_path = "././order_simulations/orders_non_normdistr.csv";
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(reader_path)?;

    let total_orders = rdr.records().count() as u64;
    info!("Initialized CSV reader from {reader_path}, found {total_orders} orders");

    // records().count() consumes the iterator, need to recreate it again to execute orders
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(reader_path)?;

    let mut ob = OrderBook::default();
    info!("Initialized Orderbook");

    let executions_path = "././executions/orders_non_normdistr.csv";
    let mut wtr = Writer::from_path(executions_path)?;
    let bar = ProgressBar::new(total_orders);
    info!("Executing orders and saving the executions in {executions_path}");

    let total_begin = Instant::now();
    for msg in rdr.deserialize() {
        let begin = Instant::now();
        let order_request: Order = msg?;
        let order = convert_to_order(&order_request);
        let event = ob.execute(order);
        let elapsed = begin.elapsed().as_nanos();
        let status = match event {
            OrderEvent::Unfilled { id: _ } => "Unfilled".to_string(),
            OrderEvent::Placed { id: _ } => "Placed".to_string(),
            OrderEvent::Canceled { id: _ } => "Canceled".to_string(),
            OrderEvent::PartiallyFilled {
                id: _,
                filled_qty: _,
                fills: _,
            } => "PartiallyFilled".to_string(),
            OrderEvent::Filled {
                id: _,
                filled_qty: _,
                fills: _,
            } => "Filled".to_string(),
        };
        wtr.serialize(OrderExecution::from((elapsed, order_request, status)))?;
        wtr.flush()?;
        bar.inc(1);
    }
    bar.finish();
    let total_elapsed = total_begin.elapsed().as_millis();
    info!("Finished execution in {total_elapsed}ms");
    Ok(())
}
