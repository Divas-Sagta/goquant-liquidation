use anchor_lang::prelude::*;

// IMPORTANT: replace this with the program ID you copied earlier
declare_id!("EHGrMFLNaYrKDt6cp5b3iABFwUsS5mCNa1EfwNARWm5n");

pub const BPS_DENOM: u64 = 10_000;
pub const LIQUIDATOR_REWARD_BPS: u64 = 250; // 2.5%
pub const MAX_ORACLE_STALENESS_SECS: i64 = 30;
pub const SCALE: u128 = 1_000_000; // fixed-point scale for prices

#[program]
pub mod liquidation_engine {
    use super::*;

    // --------- PARTIAL LIQUIDATION ---------
    pub fn liquidate_partial(
        ctx: Context<LiquidatePartial>,
        liquidation_size: u64,
    ) -> Result<()> {
        let clock = Clock::get()?;
        let position = &mut ctx.accounts.position;
        let price_feed = &ctx.accounts.price_feed;

        require!(!position.closed, ErrorCode::PositionClosed);

        // 1) Check oracle price is fresh
        let age = clock.unix_timestamp - price_feed.last_updated;
        require!(age <= MAX_ORACLE_STALENESS_SECS, ErrorCode::StaleOraclePrice);

        let mark_price = price_feed.price;

        // 2) Check if position is liquidatable
        let (margin_ratio_bps, _) = compute_margin_ratio(
            position.size,
            position.entry_price,
            position.collateral,
            position.is_long,
            mark_price,
        )?;

        let maintenance_bps = maintenance_bps_for_leverage(position.leverage);
        require!(margin_ratio_bps < maintenance_bps, ErrorCode::PositionHealthy);

        // 3) Compute liquidation size (50% or user-specified smaller)
        let half_size = position.size / 2;
        let liq_size = half_size.min(liquidation_size);
        require!(liq_size > 0, ErrorCode::InvalidLiquidationSize);

        // 4) Realize PnL on liquidated portion
        let pnl_on_liq = realized_pnl_for_size(
            liq_size,
            position.entry_price,
            mark_price,
            position.is_long,
        )?;
        let liq_value = position_value_for_size(liq_size, mark_price)?;

        // 5) Compute liquidator reward
        let liquidator_reward = (liq_value
            .checked_mul(LIQUIDATOR_REWARD_BPS as u128)
            .ok_or(error!(ErrorCode::MathOverflow))?
            / BPS_DENOM as u128) as u64;

        // 6) Update collateral: apply PnL then pay reward out of equity
        let equity_after_pnl = apply_pnl(position.collateral, pnl_on_liq)?;
        let remaining_equity = equity_after_pnl.saturating_sub(liquidator_reward);

        position.collateral = remaining_equity;
        position.size = position
            .size
            .checked_sub(liq_size)
            .ok_or(error!(ErrorCode::MathOverflow))?;

        // 7) Emit event (for off-chain DB)
        emit!(LiquidationRecord {
            position_owner: position.owner,
            liquidator: ctx.accounts.liquidator.key(),
            symbol: position.symbol.clone(),
            liquidated_size: liq_size,
            liquidation_price: mark_price,
            margin_before: margin_ratio_bps,
            margin_after: compute_margin_ratio(
                position.size,
                position.entry_price,
                position.collateral,
                position.is_long,
                mark_price,
            )?
            .0,
            liquidator_reward,
            bad_debt: 0,
            timestamp: clock.unix_timestamp,
        });

        Ok(())
    }

    // --------- FULL LIQUIDATION ---------
    pub fn liquidate_full(ctx: Context<LiquidateFull>) -> Result<()> {
        let clock = Clock::get()?;
        let position = &mut ctx.accounts.position;
        let price_feed = &ctx.accounts.price_feed;
        let insurance = &mut ctx.accounts.insurance_fund;

        require!(!position.closed, ErrorCode::PositionClosed);

        // 1) Oracle freshness
        let age = clock.unix_timestamp - price_feed.last_updated;
        require!(age <= MAX_ORACLE_STALENESS_SECS, ErrorCode::StaleOraclePrice);

        let mark_price = price_feed.price;

        // 2) Check liquidation condition again on-chain
        let (margin_ratio_bps, position_value) = compute_margin_ratio(
            position.size,
            position.entry_price,
            position.collateral,
            position.is_long,
            mark_price,
        )?;
        let maintenance_bps = maintenance_bps_for_leverage(position.leverage);
        require!(margin_ratio_bps < maintenance_bps, ErrorCode::PositionHealthy);

        // 3) Full PnL
        let pnl_full = realized_pnl_for_size(
            position.size,
            position.entry_price,
            mark_price,
            position.is_long,
        )?;

        let equity_after_pnl = apply_pnl(position.collateral, pnl_full)?;

        // 4) Reward based on full notional
        let liq_value = position_value;
        let liquidator_reward = (liq_value
            .checked_mul(LIQUIDATOR_REWARD_BPS as u128)
            .ok_or(error!(ErrorCode::MathOverflow))?
            / BPS_DENOM as u128) as u64;

        let mut reward_paid: u64 = 0;
        let mut bad_debt: u64 = 0;
        let mut _user_refund: u64 = 0;

        if equity_after_pnl >= liquidator_reward {
            reward_paid = liquidator_reward;
            _user_refund = equity_after_pnl - liquidator_reward;
        } else {
            // not enough margin to fully pay reward
            reward_paid = equity_after_pnl;
            bad_debt = liquidator_reward - equity_after_pnl;
        }

        // 5) Cover bad debt from insurance fund
        if bad_debt > 0 {
            let covered = insurance.balance.min(bad_debt);
            insurance.balance = insurance.balance.saturating_sub(covered);
            insurance.total_bad_debt_covered = insurance
                .total_bad_debt_covered
                .saturating_add(covered);
        }

        // Update utilization
        if insurance.total_contributions > 0 {
            insurance.utilization_ratio = insurance
                .total_bad_debt_covered
                .saturating_mul(BPS_DENOM)
                / insurance.total_contributions;
        }

        // 6) Close position logically
        let old_size = position.size;
        position.size = 0;
        position.collateral = 0;
        position.closed = true;

        emit!(LiquidationRecord {
            position_owner: position.owner,
            liquidator: ctx.accounts.liquidator.key(),
            symbol: position.symbol.clone(),
            liquidated_size: old_size,
            liquidation_price: mark_price,
            margin_before: margin_ratio_bps,
            margin_after: 0,
            liquidator_reward: reward_paid,
            bad_debt,
            timestamp: clock.unix_timestamp,
        });

        Ok(())
    }
}

// ------------- ACCOUNTS / STRUCTS -------------

#[account]
pub struct Position {
    pub owner: Pubkey,
    pub symbol: String,    // e.g. "BTC-PERP"
    pub size: u64,         // contracts, or base amount in 1e6
    pub is_long: bool,
    pub entry_price: u64,  // price * 1e6
    pub collateral: u64,   // quote * 1e6
    pub leverage: u16,
    pub closed: bool,
}

#[account]
pub struct InsuranceFund {
    pub authority: Pubkey,
    pub balance: u64,                // total funds
    pub total_contributions: u64,
    pub total_bad_debt_covered: u64,
    pub utilization_ratio: u64,      // basis points
}

#[account]
pub struct PriceFeed {
    pub price: u64,          // mark price * 1e6
    pub last_updated: i64,   // unix timestamp
}

// This matches your assignment's LiquidationRecord structure (as an event)
#[event]
pub struct LiquidationRecord {
    pub position_owner: Pubkey,
    pub liquidator: Pubkey,
    pub symbol: String,
    pub liquidated_size: u64,
    pub liquidation_price: u64,
    pub margin_before: u64,  // bps
    pub margin_after: u64,   // bps
    pub liquidator_reward: u64,
    pub bad_debt: u64,
    pub timestamp: i64,
}

// ------------- INSTRUCTION CONTEXTS -------------

#[derive(Accounts)]
pub struct LiquidatePartial<'info> {
    #[account(mut)]
    pub position: Account<'info, Position>,

    #[account(mut)]
    pub liquidator: Signer<'info>,

    #[account()]
    pub price_feed: Account<'info, PriceFeed>,
}

#[derive(Accounts)]
pub struct LiquidateFull<'info> {
    #[account(mut)]
    pub position: Account<'info, Position>,

    #[account(mut)]
    pub liquidator: Signer<'info>,

    #[account()]
    pub price_feed: Account<'info, PriceFeed>,

    #[account(mut)]
    pub insurance_fund: Account<'info, InsuranceFund>,
}

// ------------- HELPER FUNCTIONS -------------

fn maintenance_bps_for_leverage(leverage: u16) -> u64 {
    match leverage {
        1..=20 => 250,      // 2.5%
        21..=50 => 100,     // 1.0%
        51..=100 => 50,     // 0.5%
        101..=500 => 25,    // 0.25%
        501..=1000 => 10,   // 0.1%
        _ => 250,
    }
}

fn position_value_for_size(size: u64, price: u64) -> Result<u128> {
    let s = size as u128;
    let p = price as u128;
    let v = s
        .checked_mul(p)
        .ok_or(error!(ErrorCode::MathOverflow))?;
    Ok(v / SCALE)
}

fn realized_pnl_for_size(
    size: u64,
    entry_price: u64,
    mark_price: u64,
    is_long: bool,
) -> Result<i128> {
    let size_u = size as u128;
    let entry_u = entry_price as u128;
    let mark_u = mark_price as u128;

    let diff: i128 = if is_long {
        mark_u as i128 - entry_u as i128
    } else {
        entry_u as i128 - mark_u as i128
    };

    let pnl = diff
        .checked_mul(size_u as i128)
        .ok_or(error!(ErrorCode::MathOverflow))?
        / SCALE as i128;

    Ok(pnl)
}

fn compute_margin_ratio(
    size: u64,
    entry_price: u64,
    collateral: u64,
    is_long: bool,
    mark_price: u64,
) -> Result<(u64, u128)> {
    let position_value = position_value_for_size(size, mark_price)?;
    if position_value == 0 {
        return Ok((u64::MAX, position_value));
    }

    let unrealized = realized_pnl_for_size(size, entry_price, mark_price, is_long)?;
    let equity = collateral as i128 + unrealized;

    if equity <= 0 {
        return Ok((0, position_value));
    }

    let equity_u = equity as u128;
    let ratio_bps = equity_u
        .checked_mul(BPS_DENOM as u128)
        .ok_or(error!(ErrorCode::MathOverflow))?
        / position_value;

    Ok((ratio_bps as u64, position_value))
}

fn apply_pnl(collateral: u64, pnl: i128) -> Result<u64> {
    let base = collateral as i128;
    let after = base
        .checked_add(pnl)
        .ok_or(error!(ErrorCode::MathOverflow))?;
    if after <= 0 {
        Ok(0)
    } else {
        Ok(after as u64)
    }
}

// ------------- ERRORS -------------

#[error_code]
pub enum ErrorCode {
    #[msg("Math overflow")]
    MathOverflow,
    #[msg("Position is healthy, cannot be liquidated")]
    PositionHealthy,
    #[msg("Oracle price is stale")]
    StaleOraclePrice,
    #[msg("Invalid liquidation size")]
    InvalidLiquidationSize,
    #[msg("Position already closed")]
    PositionClosed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_maintenance_bps_for_leverage() {
        assert_eq!(maintenance_bps_for_leverage(10), 250);
        assert_eq!(maintenance_bps_for_leverage(30), 100);
        assert_eq!(maintenance_bps_for_leverage(75), 50);
        assert_eq!(maintenance_bps_for_leverage(200), 25);
        assert_eq!(maintenance_bps_for_leverage(700), 10);
    }

    #[test]
    fn test_realized_pnl_long_gain() {
        // 1 contract (scaled), entry 10,000, mark 11,000
        let size = SCALE as u64;              // 1 * 1e6
        let entry_price = 10 * SCALE as u64;  // 10 * 1e6
        let mark_price = 11 * SCALE as u64;   // 11 * 1e6

        let pnl = realized_pnl_for_size(size, entry_price, mark_price, true).unwrap();
        // Expect +1 (scaled)
        assert_eq!(pnl, SCALE as i128);
    }

    #[test]
    fn test_realized_pnl_short_gain() {
        // short from 10k to 9k
        let size = SCALE as u64;
        let entry_price = 10 * SCALE as u64;
        let mark_price = 9 * SCALE as u64;

        let pnl = realized_pnl_for_size(size, entry_price, mark_price, false).unwrap();
        // Expect +1 (scaled)
        assert_eq!(pnl, SCALE as i128);
    }

    #[test]
    fn test_apply_pnl_positive_and_negative() {
        let collateral = 5 * SCALE as u64;

        // +2
        let after_gain = apply_pnl(collateral, 2 * SCALE as i128).unwrap();
        assert_eq!(after_gain, 7 * SCALE as u64);

        // -10 (should clamp at 0, not underflow)
        let after_loss = apply_pnl(collateral, -10 * SCALE as i128).unwrap();
        assert_eq!(after_loss, 0);
    }

    #[test]
    fn test_compute_margin_ratio_basic() {
        // size 1, price 10, collateral 1 -> equity 1, value 10 -> MR = 10%
        let size = SCALE as u64;
        let price = 10 * SCALE as u64;
        let collateral = 1 * SCALE as u64;

        let (mr_bps, position_value) =
            compute_margin_ratio(size, price, collateral, true, price).unwrap();

        // Position value ~= 10 * SCALE
        assert_eq!(position_value, 10 * SCALE as u128);

        // 10% margin -> 1000 bps
        assert_eq!(mr_bps, 1000);
    }
}

