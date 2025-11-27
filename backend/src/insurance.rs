use crate::errors::LiquidationError;

pub struct InsuranceFundManager {}

impl InsuranceFundManager {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn get_balance(&self) -> Result<u64, LiquidationError> {
        // TODO: query on-chain InsuranceFund account.
        Ok(0)
    }
}
