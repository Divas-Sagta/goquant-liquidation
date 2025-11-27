# Liquidation Mechanics

This document describes **when** positions become liquidatable, how **partial vs full** liquidations are chosen, and how **liquidator rewards** and **bad debt** are handled.

---

## 1. Margin Ratio & Liquidation Condition

### Definitions

- `Size` – position size in contracts / base units.
- `EntryPrice` – price at which the position was opened.
- `MarkPrice` – current mark price from oracle.
- `Collateral` – user collateral backing the position.
- `PositionValue = Size * MarkPrice`
- `UnrealizedPnL`:
  - Long: `UnrealizedPnL = Size * (MarkPrice - EntryPrice)`
  - Short: `UnrealizedPnL = Size * (EntryPrice - MarkPrice)`
- `Equity = Collateral + UnrealizedPnL`
- **MarginRatio** (in basis points for on-chain calculations):

```text
margin_ratio_bps = Equity / PositionValue * 10_000
````

### Liquidation Condition

A position is **liquidatable** when:

```text
margin_ratio_bps < maintenance_margin_bps
```

where `maintenance_margin_bps` depends on leverage.

---

## 2. Maintenance Margin Tiers

Maintenance margin is a step function of leverage (same mapping on-chain and off-chain):

| Leverage Range | Maintenance Margin |
| -------------- | ------------------ |
| 1x   – 20x     | 2.5%   (250 bps)   |
| 21x  – 50x     | 1.0%   (100 bps)   |
| 51x  – 100x    | 0.5%   (50 bps)    |
| 101x – 500x    | 0.25%  (25 bps)    |
| 501x – 1000x   | 0.1%   (10 bps)    |
| default        | 2.5%   (250 bps)   |

These tiers are implemented:

* Off-chain in `LiquidationEngine::get_maintenance_margin_ratio`
* On-chain in `maintenance_bps_for_leverage(leverage: u16)`

---

## 3. Partial Liquidation

**Goal:** Reduce position risk gradually, minimizing market impact.

### On-chain Instruction

```rust
pub fn liquidate_partial(
    ctx: Context<LiquidatePartial>,
    liquidation_size: u64,
) -> Result<()> {
    // ...
}
```

### Steps

1. **Oracle Freshness**

   * Check:

     ```rust
     Clock::get()?.unix_timestamp - price_feed.last_updated <= 30
     ```

   * Prevents liquidations on stale or manipulated prices.

2. **Health Check**

   * Compute `margin_ratio_bps`.
   * Fetch `maintenance_margin_bps` from leverage tier.
   * Require:

     ```text
     margin_ratio_bps < maintenance_margin_bps
     ```

3. **Liquidation Size**

   * Compute:

     ```text
     half_size = position.size / 2
     effective_liq_size = min(half_size, liquidation_size)
     ```

   * This enforces: **partial liquidation reduces position by up to 50% of size**.

4. **PnL Realization for Liquidated Portion**

   * Compute realized PnL on the liquidated size:

     ```text
     Long:  pnl_liq = Size_liq * (MarkPrice - EntryPrice)
     Short: pnl_liq = Size_liq * (EntryPrice - MarkPrice)
     ```

   * Apply PnL to collateral:

     ```text
     Equity_after_pnl = Collateral + pnl_liq
     ```

   * If `Equity_after_pnl < 0`, equity is clamped to `0`.

5. **Liquidator Reward**

   * Compute notional of liquidated portion:

     ```text
     liq_value = Size_liq * MarkPrice
     ```

   * Reward is `2.5%` of liquidated notional:

     ```text
     reward = liq_value * 250 / 10_000
     ```

   * Reward is subtracted from post-PnL equity (never makes equity negative; remainder stays as collateral).

6. **State Updates**

   * Update position:

     ```text
     position.size       -= liq_size
     position.collateral = remaining_equity
     ```

   * Emit `LiquidationRecord` with:

     * `margin_before`
     * `margin_after` (recomputed using new size and collateral)
     * `liquidator_reward`
     * `bad_debt = 0` for partials

---

## 4. Full Liquidation

### On-chain Instruction

```rust
pub fn liquidate_full(ctx: Context<LiquidateFull>) -> Result<()> {
    // ...
}
```

### Steps

1. **Oracle Freshness & Health Check**

   * Same as partial:

     * Ensure price is fresh (`last_updated` within 30 seconds).
     * Ensure position is liquidatable (`margin_ratio_bps < maintenance_margin_bps`).

2. **Full PnL Realization**

   * Compute full PnL on entire position size.
   * Apply to collateral to get final `equity_after_pnl`.

3. **Liquidator Reward**

   * Reward is based on full notional:

     ```text
     liq_value = full_position_value
     reward    = liq_value * 2.5%   // in on-chain code: bps
     ```

4. **Equity Cases**

   * **Case A: `Equity >= reward`**

     * Liquidator receives full `reward`.
     * User receives:

       ```text
       equity_after_pnl - reward
       ```

       back.

   * **Case B: `Equity < reward`**

     * Liquidator receives only `equity_after_pnl`.
     * Remaining reward shortfall is considered **bad debt**.

5. **Bad Debt & Insurance Fund**

   * If `bad_debt > 0`:

     * Use `InsuranceFund.balance` to cover as much as possible:

       ```text
       covered = min(bad_debt, fund.balance)

       fund.balance               -= covered
       fund.total_bad_debt_covered += covered
       ```

     * Update utilization:

       ```text
       utilization_ratio =
           total_bad_debt_covered / total_contributions * 10_000
       ```

6. **Close Position**

   * After full liquidation:

     ```text
     position.size       = 0
     position.collateral = 0
     position.closed     = true
     ```

7. **Emit LiquidationRecord**

   Includes:

   * `liquidated_size` = full previous size
   * `liquidation_price`
   * `margin_before`
   * `margin_after = 0` (position fully closed)
   * `liquidator_reward` actually paid
   * `bad_debt` generated

---

## 5. Oracle Protection

To resist oracle manipulation and stale data:

### On-chain

* Enforces:

  ```text
  now - last_updated <= 30 seconds
  ```

### Off-chain (Planned / Extended)

* `PriceOracle` will be extended to:

  * Use robust feed sources (e.g. Pyth / Switchboard).
  * Incorporate confidence intervals.
  * Implement fallback strategies during outages.

---

## 6. Integration with Off-chain Engine

The off-chain `LiquidationEngine` conceptually mirrors the on-chain logic:

* Computes margin ratio using floats for simplicity.
* Uses the same **leverage → maintenance margin** mapping.
* Classifies positions as **liquidatable** or **healthy**.
* Feeds liquidatable positions into a priority queue ordered by margin ratio (lowest first).

### Liquidation Flow (Off-chain)

* Engine selects **partial** or **full** liquidation depending on how undercollateralized the account is.
* It triggers the corresponding on-chain instruction:

  * `liquidate_partial`
  * `liquidate_full`
* After the transaction, it records the result in PostgreSQL for historical / analytical purposes.

---

