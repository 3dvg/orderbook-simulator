use app::gbm;
use app::OrderSimulation;
use csv::Writer;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let simulation = OrderSimulation::default();
    let mut receiver = simulation.run().await;

    // let vals = gbm::generate_gbm(100.0, 1.0 / 31_536_000.0, 10, 0.15, 0.5);
    // println!("vals {:?}", vals);
    let mut wtr = Writer::from_path("orders.csv")?;
    while let Ok(message) = receiver.recv().await {
        // println!("{:?}", message);
        wtr.serialize(message)?;
        wtr.flush()?;
    }
    Ok(())
}