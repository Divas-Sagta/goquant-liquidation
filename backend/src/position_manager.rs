use crate::errors::LiquidationError;
use crate::models::Position;
use solana_sdk::pubkey::Pubkey;

pub struct PositionManager {}

impl PositionManager {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn get_open_positions(&self) -> Result<Vec<Position>, LiquidationError> {
        // TODO: later: load from Redis / Postgres.
        // For now: return one undercollateralized BTC-PERP position so we see the engine work.

        let owner = Pubkey::new_unique();

        let pos = Position {
            id: "pos-1".to_string(),
            owner,
            symbol: "BTC-PERP".to_string(),
            size: 1.0,          // 1 contract
            is_long: true,
            entry_price: 60_000.0,
            collateral: 50.0,   // very small collateral, so margin will be tiny
            leverage: 500,      // high leverage -> low maintenance margin ratio
        };

        Ok(vec![pos])
    }
}
