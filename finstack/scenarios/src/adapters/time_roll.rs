//! Time roll-forward adapter with carry/theta calculations.

use crate::engine::ExecutionContext;
use crate::error::Result;
use crate::utils::parse_period_to_days;
use finstack_valuations::instruments::common::traits::Instrument;

/// Report from time roll-forward operation.
#[derive(Debug, Clone)]
pub struct RollForwardReport {
    /// Original as-of date.
    pub old_date: finstack_core::dates::Date,

    /// New as-of date after roll.
    pub new_date: finstack_core::dates::Date,

    /// Number of days rolled forward.
    pub days: i64,

    /// Per-instrument carry accrual (if instruments provided).
    pub instrument_carry: Vec<(String, f64)>,

    /// Per-instrument market value change (if instruments provided).
    pub instrument_mv_change: Vec<(String, f64)>,

    /// Total P&L from carry.
    pub total_carry: f64,

    /// Total P&L from market value changes.
    pub total_mv_change: f64,
}

/// Apply time roll-forward operation.
///
/// This advances the valuation date by the specified period and computes carry/theta
/// for each instrument. Theta is calculated as the PV change from rolling the date
/// forward while keeping all market data unchanged (curves, surfaces, FX rates).
///
/// The calculation is consistent with the theta metric definition: it measures
/// the value impact of time passage with no market changes, only rolling down curves.
pub fn apply_time_roll_forward(
    ctx: &mut ExecutionContext,
    period_str: &str,
) -> Result<RollForwardReport> {
    let old_date = ctx.as_of;
    let days = parse_period_to_days(period_str)?;

    // Calculate new date by adding days
    let new_date = old_date + time::Duration::days(days);

    // Note: Proper curve rolling would require:
    // 1. Adjusting all curve base dates
    // 2. Removing expired knot points
    // 3. Shifting remaining knots forward in time
    // For now, we simply update the as_of date and compute carry

    // Update as_of in context
    ctx.as_of = new_date;

    // Calculate carry and market value changes for instruments
    let (instrument_carry, instrument_mv_change, total_carry, total_mv_change) =
        if let Some(instruments) = ctx.instruments.as_ref() {
            calculate_instrument_pnl(instruments, ctx.market, old_date, new_date, days)?
        } else {
            (vec![], vec![], 0.0, 0.0)
        };

    Ok(RollForwardReport {
        old_date,
        new_date,
        days,
        instrument_carry,
        instrument_mv_change,
        total_carry,
        total_mv_change,
    })
}

/// Calculate P&L breakdown for instruments.
///
/// Theta (carry) is calculated as the PV change from rolling the date forward
/// with no market data changes. This is consistent with the theta metric definition.
#[allow(clippy::type_complexity)]
fn calculate_instrument_pnl(
    instruments: &[Box<dyn Instrument>],
    market: &finstack_core::market_data::MarketContext,
    old_date: finstack_core::dates::Date,
    new_date: finstack_core::dates::Date,
    _days: i64,
) -> Result<(Vec<(String, f64)>, Vec<(String, f64)>, f64, f64)> {
    let mut instrument_carry = Vec::new();
    let mut instrument_mv_change = Vec::new();
    let mut total_carry = 0.0;
    let mut total_mv_change = 0.0;

    for instrument in instruments {
        let inst_id = instrument.id().to_string();

        // Calculate carry as PV change with no market changes (theta definition)
        // This is exactly what the theta metric measures, but we calculate it directly
        // to avoid needing to pass pricing_overrides through price_with_metrics
        let carry = {
            let pv_old = instrument.value(market, old_date).ok();
            let pv_new = instrument.value(market, new_date).ok();

            if let (Some(old), Some(new)) = (pv_old, pv_new) {
                new.amount() - old.amount()
            } else {
                0.0
            }
        };

        // Market value change is zero in time roll (market data unchanged)
        // All P&L comes from carry/theta
        let mv_change = 0.0;

        instrument_carry.push((inst_id.clone(), carry));
        instrument_mv_change.push((inst_id, mv_change));
        total_carry += carry;
        total_mv_change += mv_change;
    }

    Ok((
        instrument_carry,
        instrument_mv_change,
        total_carry,
        total_mv_change,
    ))
}
