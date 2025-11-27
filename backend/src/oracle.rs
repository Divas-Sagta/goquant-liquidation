use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::errors::LiquidationError;

pub struct PriceOracle {
    cache: Mutex<HashMap<String, (f64, Instant)>>,
}

impl PriceOracle {
    pub fn new() -> Self {
        Self {
            cache: Mutex::new(HashMap::new()),
        }
    }

    pub async fn get_mark_price(&self, symbol: &str) -> Result<f64, LiquidationError> {
        {
            let cache = self.cache.lock().unwrap();
            if let Some((price, ts)) = cache.get(symbol) {
                if ts.elapsed() < Duration::from_millis(200) {
                    return Ok(*price);
                }
            }
        }

        // TODO: connect to Pyth / Switchboard.
        // For now, return a dummy value so we can compile and run.
        let dummy_price = match symbol {
            "BTC-PERP" => 60_000.0,
            "ETH-PERP" => 3_000.0,
            _ => 1_000.0,
        };

        let mut cache = self.cache.lock().unwrap();
        cache.insert(symbol.to_string(), (dummy_price, Instant::now()));

        Ok(dummy_price)
    }
}
