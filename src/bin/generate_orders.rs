use app::OrderSimulation;
use csv::Writer;
use std::error::Error;
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // simulator ----------------------------------------------------
    let simulation = OrderSimulation::default();
    let mut receiver = simulation.run().await;

    let mut wtr = Writer::from_path("./order_simulations/orders.csv")?;
    while let Ok(message) = receiver.recv().await {
        println!("{:?}", message);
        wtr.serialize(message)?;
        wtr.flush()?;
        if receiver.is_empty() {
            break;
        }
    }
    Ok(())
    // }
    // let vals = gbm::generate_gbm(100.0, 1.0 / 31_536_000.0, 10, 0.15, 0.5);
    // println!("vals {:?}", vals);
}
