# Testing Guide

This project currently has tests at two levels:

1. **On-chain program unit tests** (liquidation math and margin logic).
2. **Backend executable tests** (binary builds, manual verification via HTTP).

---

## 1. On-chain Program Unit Tests

Location: `programs/liquidation_engine/src/lib.rs` (`#[cfg(test)] mod tests`).

These tests cover:

- Maintenance margin basis points per leverage tier.
- Long and short PnL calculations.
- Equity clamping (no negative collateral).
- Margin ratio calculation for a simple position.

### How to run

Use the modern Rust toolchain for tests:

```bash
cd programs/liquidation_engine
cargo +stable test
````

Expected result (example):

```text
running 5 tests
test tests::test_maintenance_bps_for_leverage ... ok
test tests::test_realized_pnl_long_gain ... ok
test tests::test_realized_pnl_short_gain ... ok
test tests::test_apply_pnl_positive_and_negative ... ok
test tests::test_compute_margin_ratio_basic ... ok

test result: ok. 5 passed; 0 failed; ...
```

These tests validate the core liquidation formulas independently of the Solana runtime.

---

## 2. Backend Tests

Currently, the backend binary builds and runs without dedicated unit tests. You can still verify basic behavior manually.

### Build & run

```bash
cd backend
cargo +stable build
cargo +stable run
```

Expected console output:

* Startup logs:

  ```text
  Starting liquidation backend...
  HTTP server listening on http://0.0.0.0:8080
  ```

* Periodic “would liquidate” logs for the dummy undercollateralized test position:

  ```text
  [EXECUTOR] Would liquidate position pos-1 on BTC-PERP at price 60000 (margin_ratio=...)
  ```

### HTTP verification

From another terminal:

```bash
curl http://localhost:8080/health
```

Expected:

```json
{"status":"ok"}
```

And:

```bash
curl http://localhost:8080/liquidations/pending
```

Expected (example; exact length may vary):

```json
{"positions":["pos-1","pos-1","pos-1"]}
```

This confirms:

* The liquidation engine loop is running.
* The undercollateralized test position is being enqueued in the priority queue.
* The `/liquidations/pending` endpoint is wired to live queue state.
