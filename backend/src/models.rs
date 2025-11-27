use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub id: String,
    pub owner: Pubkey,
    pub symbol: String,
    pub size: f64,
    pub is_long: bool,
    pub entry_price: f64,
    pub collateral: f64,
    pub leverage: u16,
}

#[derive(Debug, Clone)]
pub struct LiquidationCandidate {
    pub position: Position,
    pub mark_price: f64,
    pub margin_ratio: f64,
}
