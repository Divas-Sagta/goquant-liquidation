use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{interval, Duration};

use crate::errors::LiquidationError;
use crate::models::Position;
use crate::oracle::PriceOracle;
use crate::position_manager::PositionManager;
use crate::queue::LiquidationQueue;
use crate::executor::LiquidationExecutor;

pub struct LiquidationEngine {
    pub check_interval_ms: u64,
    pub oracle: Arc<PriceOracle>,
    pub position_manager: Arc<PositionManager>,
    pub queue: Arc<LiquidationQueue>,
    pub executor: Arc<LiquidationExecutor>,
}

impl LiquidationEngine {
    pub fn new() -> Self {
        Self {
            check_interval_ms: 1000,
            oracle: Arc::new(PriceOracle::new()),
            position_manager: Arc::new(PositionManager::new()),
            queue: Arc::new(LiquidationQueue::new()),
            executor: Arc::new(LiquidationExecutor::new()),
        }
    }

    pub async fn start(&self) {
        let mut timer = interval(Duration::from_millis(self.check_interval_ms));

        loop {
            timer.tick().await;
            if let Err(e) = self.check_all_positions().await {
                eprintln!("[ENGINE] Error checking positions: {e:?}");
            }

            if let Err(e) = self.executor.drain_queue(self.queue.clone()).await {
                eprintln!("[ENGINE] Error executing liquidations: {e:?}");
            }
        }
    }

    async fn check_all_positions(&self) -> Result<(), LiquidationError> {
        let open_positions: Vec<Position> =
            self.position_manager.get_open_positions().await?;

        let mut price_cache: HashMap<String, f64> = HashMap::new();

        for position in open_positions {
            let mark_price = if let Some(&cached) = price_cache.get(&position.symbol) {
                cached
            } else {
                let price = self.oracle.get_mark_price(&position.symbol).await?;
                price_cache.insert(position.symbol.clone(), price);
                price
            };

            let unrealized_pnl = if position.is_long {
                position.size * (mark_price - position.entry_price)
            } else {
                position.size * (position.entry_price - mark_price)
            };

            let position_value = position.size * mark_price;
            if position_value <= 0.0 {
                continue;
            }

            let equity = position.collateral + unrealized_pnl;
            let margin_ratio = equity / position_value;

            let maintenance_margin_ratio =
                Self::get_maintenance_margin_ratio(position.leverage);

            if margin_ratio < maintenance_margin_ratio {
                self.queue.enqueue(position, mark_price, margin_ratio)?;
            }
        }

        Ok(())
    }

    fn get_maintenance_margin_ratio(leverage: u16) -> f64 {
        match leverage {
            1..=20 => 0.025,
            21..=50 => 0.01,
            51..=100 => 0.005,
            101..=500 => 0.0025,
            501..=1000 => 0.001,
            _ => 0.025,
        }
    }
}
