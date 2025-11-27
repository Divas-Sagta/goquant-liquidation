use std::sync::Arc;

use crate::errors::LiquidationError;
use crate::queue::LiquidationQueue;

pub struct LiquidationExecutor {}

impl LiquidationExecutor {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn drain_queue(
        &self,
        queue: Arc<LiquidationQueue>,
    ) -> Result<(), LiquidationError> {
        // NOTE: we use snapshot() so we don't consume the queue.
        let candidates = queue.snapshot();

        for candidate in candidates {
            println!(
                "[EXECUTOR] Would liquidate position {} on {} at price {} (margin_ratio={})",
                candidate.position.id,
                candidate.position.symbol,
                candidate.mark_price,
                candidate.margin_ratio
            );
        }

        Ok(())
    }
}
