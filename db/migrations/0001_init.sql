-- Liquidation history
CREATE TABLE liquidation_events (
    id BIGSERIAL PRIMARY KEY,
    position_owner TEXT NOT NULL,
    liquidator TEXT NOT NULL,
    symbol TEXT NOT NULL,
    liquidated_size NUMERIC(38, 8) NOT NULL,
    liquidation_price NUMERIC(38, 8) NOT NULL,
    margin_before_bps INTEGER NOT NULL,
    margin_after_bps INTEGER NOT NULL,
    liquidator_reward NUMERIC(38, 8) NOT NULL,
    bad_debt NUMERIC(38, 8) NOT NULL,
    is_full BOOLEAN NOT NULL,
    occurred_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Bad debt records
CREATE TABLE bad_debt_events (
    id BIGSERIAL PRIMARY KEY,
    position_owner TEXT NOT NULL,
    symbol TEXT NOT NULL,
    bad_debt NUMERIC(38, 8) NOT NULL,
    covered_by_insurance NUMERIC(38, 8) NOT NULL,
    uncovered NUMERIC(38, 8) NOT NULL,
    occurred_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Liquidator performance
CREATE TABLE liquidator_stats (
    liquidator TEXT PRIMARY KEY,
    total_liquidations BIGINT NOT NULL DEFAULT 0,
    total_reward NUMERIC(38, 8) NOT NULL DEFAULT 0,
    last_liquidation_at TIMESTAMP WITH TIME ZONE
);

-- Insurance fund transactions
CREATE TABLE insurance_fund_transactions (
    id BIGSERIAL PRIMARY KEY,
    tx_type TEXT NOT NULL CHECK (tx_type IN ('contribution', 'coverage', 'withdrawal')),
    amount NUMERIC(38, 8) NOT NULL,
    tx_hash TEXT,
    note TEXT,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Failed liquidation attempts
CREATE TABLE failed_liquidations (
    id BIGSERIAL PRIMARY KEY,
    position_id TEXT NOT NULL,
    liquidator TEXT NOT NULL,
    symbol TEXT NOT NULL,
    margin_ratio NUMERIC(20, 10),
    error_message TEXT NOT NULL,
    attempt_tx TEXT,
    occurred_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);
