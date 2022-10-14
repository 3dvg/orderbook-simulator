use app::{OrderSimulation};
use csv::Writer;
use indicatif::ProgressBar;
use log::{info, LevelFilter};
use std::{error::Error, fs};
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::formatted_timed_builder()
        .filter_level(LevelFilter::Info)
        .init();

    // let simulation = OrderSimulation::default();
    
    fs::create_dir_all("././order_simulations")?;

    let max_orders = 1_000_000;
    let n_traders = 100_000;
    let n_tasks = 1_000;
    let price = 100.0;
    let price_dev = 2.0;
    let price_decimals = 2;
    let latency_min = 0;
    let latency_max = 1;
    let qty_max = 10_000.0;
    let qty_decimals = 0;
    let pct_limit_orders = 0.75;
    let instrument = "AAPL".to_string();

    let simulation = OrderSimulation::new(
        max_orders,
        n_traders,
        n_tasks,
        price,
        price_dev,
        price_decimals,
        latency_min,
        latency_max,
        qty_max,
        qty_decimals,
        pct_limit_orders,
        instrument,
    );

    let mut receiver = simulation.run().await;

    let path = "././order_simulations/orders.csv";
    let mut wtr = Writer::from_path(path)?;
    info!("Saving simulated orders in {path}");
    let bar = ProgressBar::new(max_orders);
    while let Ok(message) = receiver.recv().await {
        wtr.serialize(message)?;
        wtr.flush()?;
        if receiver.is_empty() {
            break;
        }
        bar.inc(1);
    }
    bar.finish();
    // let vals = gbm::generate_gbm(100.0, 1.0 / 31_536_000.0, 10, 0.15, 50.0);
    // info!("vals {:?}", vals);
    Ok(())
}
