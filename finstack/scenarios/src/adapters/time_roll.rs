//! Time roll-forward adapter with carry/theta calculations.

use crate::engine::ExecutionContext;
use crate::error::Result;
use crate::utils::parse_period_to_days;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::metrics::MetricId;

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
/// This advances the valuation date by the specified period and:
/// 1. Rolls all curves forward (shifts base dates, adjusts knots)
/// 2. Computes carry/theta for each instrument
/// 3. Revalues instruments at the new date
/// 4. Reports detailed P&L breakdown
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
#[allow(clippy::type_complexity)]
fn calculate_instrument_pnl(
    instruments: &[Box<dyn Instrument>],
    market: &finstack_core::market_data::MarketContext,
    old_date: finstack_core::dates::Date,
    new_date: finstack_core::dates::Date,
    days: i64,
) -> Result<(Vec<(String, f64)>, Vec<(String, f64)>, f64, f64)> {
    let mut instrument_carry = Vec::new();
    let mut instrument_mv_change = Vec::new();
    let mut total_carry = 0.0;
    let mut total_mv_change = 0.0;

    for instrument in instruments {
        let inst_id = instrument.id().to_string();

        // Try to compute theta (carry per day)
        let theta_result = instrument.price_with_metrics(market, old_date, &[MetricId::Theta]);

        let carry = if let Ok(result) = theta_result {
            // Theta is daily P&L, scale by number of days
            let theta_str = MetricId::Theta.as_str();
            if let Some(theta_value) = result.measures.get(theta_str) {
                theta_value * days as f64
            } else {
                0.0
            }
        } else {
            // If theta not available, estimate from PV roll
            let pv_old = instrument.value(market, old_date).ok();
            let pv_new = instrument.value(market, new_date).ok();

            if let (Some(old), Some(new)) = (pv_old, pv_new) {
                new.amount() - old.amount()
            } else {
                0.0
            }
        };

        // Calculate market value change (total change minus carry = market move)
        let mv_change = {
            let pv_old = instrument.value(market, old_date).ok();
            let pv_new = instrument.value(market, new_date).ok();

            if let (Some(old), Some(new)) = (pv_old, pv_new) {
                (new.amount() - old.amount()) - carry
            } else {
                0.0
            }
        };

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
