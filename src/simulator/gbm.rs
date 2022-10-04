use rand_distr::{Distribution, Normal};

/// Generate geometric brownian motion
/// dS — Change in asset price over the time period
/// s — Asset price for the previous (or initial) period
/// µ — Expected return for the time period or the Drift
/// dt — The change in time (one period of time)
/// σ — Volatility term (a measure of spread)
/// dW — Change in Brownian motion term
pub fn generate_gbm(s: f64, dt: f64, length: usize, drift: f64, volatility: f64) -> Vec<f64> {
    let mut rng = rand::thread_rng();
    // let dist = Normal::new(0.0, 1.0).unwrap();
    let dist2 = Normal::new(0.0, dt.sqrt()).unwrap();
    let mut prices = Vec::<f64>::with_capacity(length);
    prices.push(s);
    let mut current_price = s;
    for _ in 0..length {
        let dw = dist2.sample(&mut rng);
        let ds = current_price * drift * dt + current_price * volatility * dt.sqrt() * dw;
        current_price += ds;
        prices.push(current_price);
    }
    prices
}

#[cfg(test)]
mod tests {
    // use super::*;
    // use crate::plot_values;

    #[test]
    fn generate_gbm() -> Result<(), Box<dyn std::error::Error>> {
        // let vals = generate_gbm(100.0, 1.5 / 365.0, 256, 0.15, 0.5);

        // let filename = "img/geometric_brownian_motion.png";
        // plot_values(vals, filename)
        Ok(())
    }
}
