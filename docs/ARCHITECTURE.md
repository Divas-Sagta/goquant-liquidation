# GoQuant Liquidation Engine – Architecture

## High-Level Overview

This project implements a liquidation system for a high-leverage perpetual futures exchange, split into:

- **On-chain Solana program (Anchor):**
  - Validates liquidation conditions (margin ratio, oracle freshness).
  - Executes partial and full liquidations.
  - Updates `Position` & `InsuranceFund` accounts.
  - Emits `LiquidationRecord` events.

- **Off-chain Rust backend (“Liquidation Engine”):**
  - Monitors open positions in real time.
  - Identifies undercollateralized positions.
  - Queues liquidations by urgency.
  - (Planned) Executes on-chain liquidations via Solana RPC.
  - Exposes monitoring APIs for operations and analytics.

- **Database (PostgreSQL):**
  - Stores liquidation history.
  - Tracks bad debt.
  - Tracks liquidator performance.
  - Stores insurance fund transaction history.
  - Logs failed liquidation attempts.

---

## Components & Interactions

### 1. On-chain Program (`programs/liquidation_engine`)

**Main instructions:**

- `liquidate_partial(ctx, liquidation_size: u64)`  
  - Checks price freshness and margin ratio.
  - Partially reduces position size (up to 50% or requested size).
  - Realizes PnL for the liquidated portion.
  - Pays liquidator reward from remaining equity.
  - Updates `Position.collateral` and `Position.size`.
  - Emits `LiquidationRecord` event with before/after margin.

- `liquidate_full(ctx)`  
  - Checks price freshness and margin ratio.
  - Closes the entire position.
  - Computes full PnL and resulting equity.
  - Pays liquidator reward if equity is positive.
  - If equity is insufficient, calculates bad debt.
  - Covers bad debt from `InsuranceFund` balance (up to available funds).
  - Updates insurance fund utilization.
  - Marks `Position` as closed and zeroes size/collateral.
  - Emits `LiquidationRecord` event with bad debt info.

**Key accounts:**

- `Position`
  - `owner: Pubkey`
  - `symbol: String` (e.g. `"BTC-PERP"`)
  - `size: u64`
  - `is_long: bool`
  - `entry_price: u64`
  - `collateral: u64`
  - `leverage: u16`
  - `closed: bool`

- `InsuranceFund`
  - `authority: Pubkey`
  - `balance: u64`
  - `total_contributions: u64`
  - `total_bad_debt_covered: u64`
  - `utilization_ratio: u64` (basis points)

- `PriceFeed`
  - `price: u64` (mark price * 1e6)
  - `last_updated: i64` (unix timestamp)

- `LiquidationRecord` (event)
  - Mirrors the assignment’s liquidation record fields and is used for off-chain indexing.

---

### 2. Off-chain Backend (`backend`)

**Major modules:**

- `engine.rs` – `LiquidationEngine`
  - Periodic loop (every `check_interval_ms`, default 1000 ms).
  - Fetches open positions from `PositionManager`.
  - Fetches mark prices from `PriceOracle`.
  - Computes unrealized PnL, equity, and margin ratio.
  - Compares margin ratio to maintenance margin ratio based on leverage.
  - Enqueues undercollateralized positions into `LiquidationQueue`.
  - Invokes `LiquidationExecutor` to process queue candidates.

- `queue.rs` – `LiquidationQueue`
  - Priority queue (binary heap) ordered by **lowest margin ratio first**.
  - Stores `LiquidationCandidate` (position + price + margin ratio).
  - Methods:
    - `enqueue(...)`
    - `pop()` – used by executor in a realistic setting.
    - `snapshot_ids()` – used by HTTP API to list pending position IDs.
    - `snapshot()` – used by executor to inspect queue without consuming it.

- `executor.rs` – `LiquidationExecutor`
  - Currently **logs** which positions would be liquidated, using `queue.snapshot()`.
  - In a full implementation, this component would:
    - Build Solana transactions for `liquidate_partial` / `liquidate_full`.
    - Sign with a liquidator keypair.
    - Submit with a high-priority fee.
    - Handle retries and failure logging (to DB).

- `oracle.rs` – `PriceOracle`
  - Provides an abstraction over price feeds.
  - Currently uses a simple in-memory cache and dummy prices.
  - In a full version, this would:
    - Subscribe to Pyth / Switchboard streams.
    - Validate confidence intervals.
    - Handle oracle downtime gracefully.

- `position_manager.rs` – `PositionManager`
  - Abstracts position storage (Redis/Postgres).
  - Currently returns a single dummy undercollateralized BTC-PERP position
    for demonstration and testing of the liquidation loop.

- `insurance.rs` – `InsuranceFundManager`
  - Stub abstraction over the on-chain insurance fund state.
  - Would be responsible for querying fund balance, utilization, and
    alerting when the fund runs low.

- `models.rs`
  - Houses data structs used by the backend (e.g. `Position`, `LiquidationCandidate`).

- `errors.rs`
  - Defines a unified `LiquidationError` type for error propagation across modules.

---

### 3. Database

Schema defined in `db/migrations/0001_init.sql`. Tables include:

- `liquidation_events`
- `bad_debt_events`
- `liquidator_stats`
- `insurance_fund_transactions`
- `failed_liquidations`

The backend will write to and read from these tables for history, analytics, and monitoring endpoints.

---

## Data & Control Flow

### Liquidation Flow 

1. **Price / Position Updates**
   - Off-chain engine periodically queries open positions and the latest mark prices.

2. **Risk Check**
   - For each position:
     - Compute unrealized PnL and position value.
     - Compute margin ratio.
     - Compare to maintenance margin ratio tier based on leverage.

3. **Queueing**
   - If `margin_ratio < maintenance_margin_ratio`:
     - Enqueue into `LiquidationQueue` (lower margin ratio = higher priority).

4. **Execution**
   - `LiquidationExecutor` inspects (`snapshot`) the queue.
   - For each candidate:
     - Build transaction calling `liquidate_partial` or `liquidate_full`.
     - Submit to Solana and track confirmation.
     - Log success/failure into DB.

5. **On-chain logic**
   - Smart contract checks price freshness, margin health, and performs state updates.
   - Emits `LiquidationRecord` events for off-chain indexing.
   - Updates `InsuranceFund` balances and utilization when covering bad debt.

6. **Monitoring**
   - Backend exposes HTTP endpoints like `/liquidations/pending` which currently read directly from `LiquidationQueue`.
   - Future endpoints will read from Postgres for history and analytics.

---

## Threading & Async Model

- Backend runs on `tokio` async runtime.
- `LiquidationEngine::start` runs as a background async task.
- HTTP server (Axum) runs concurrently in the same runtime.
- Shared structures (like `LiquidationQueue`) are wrapped in `Arc<...>` and internal `Mutex` where needed.
- On-chain program execution remains single-threaded per transaction and leverages Solana’s runtime for concurrency across accounts.
