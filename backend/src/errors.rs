use thiserror::Error;

#[derive(Debug, Error)]
pub enum LiquidationError {
    #[error("RPC error: {0}")]
    Rpc(#[from] solana_client::client_error::ClientError),

    #[error("Database error: {0}")]
    Db(String),

    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("Oracle error: {0}")]
    Oracle(String),

    #[error("Other: {0}")]
    Other(String),
}
