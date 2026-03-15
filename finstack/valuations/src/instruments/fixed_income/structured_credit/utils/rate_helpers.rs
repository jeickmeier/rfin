//! Helpers to compute all-in rates using core market_data curves.
//!
//! These helpers properly compute floating rate projections using
//! calendar-aware tenor addition for accurate period end dates.

#![allow(dead_code)] // WIP: public API not yet wired into main pricing paths

use finstack_core::dates::{Date, DayCount, DayCountCtx};
use finstack_core::market_data::context::MarketContext;
use rust_decimal::prelude::ToPrimitive;

use crate::instruments::fixed_income::structured_credit::types::TrancheCoupon;

/// Calculate period end date from a tenor value in years.
///
/// Converts decimal tenor to proper month-based rolling:
/// - 0.25 years → 3 months
/// - 0.5 years → 6 months
/// - 1.0 years → 12 months
///
/// For tenors that don't map cleanly to months, uses day approximation
/// as a fallback.
///
/// # Market Standard
///
/// Most floating rate indices use standard tenors (1M, 3M, 6M, 12M) where
/// proper month arithmetic is essential:
/// - End-of-month dates should roll to end-of-month
/// - Holiday adjustments (modified following) would be applied downstream
#[inline]
pub fn tenor_to_period_end(start: Date, tenor_years: f64, day_count: DayCount) -> Date {
    // Backward-compatible, infallible-ish helper.
    //
    // For precision-first code paths, use `try_tenor_to_period_end` and propagate errors.
    use finstack_core::dates::{BusinessDayConvention, Tenor};
    let tenor = Tenor::from_years(tenor_years, day_count);
    tenor
        .add_to_date(start, None, BusinessDayConvention::Unadjusted)
        .unwrap_or(start)
}

/// Fallible variant of [`tenor_to_period_end`].
///
/// Prefer this in pricing/valuation code so date arithmetic failures are surfaced as structured
/// errors instead of panics or silent fallbacks.
#[inline]
pub fn try_tenor_to_period_end(
    start: Date,
    tenor_years: f64,
    day_count: DayCount,
) -> finstack_core::Result<Date> {
    use finstack_core::dates::{BusinessDayConvention, Tenor};
    let tenor = Tenor::from_years(tenor_years, day_count);
    tenor.add_to_date(start, None, BusinessDayConvention::Unadjusted)
}

/// Compute tranche all-in rate (fixed => fixed; floating => index forward + spread with caps/floors).
///
/// For floating rate tranches, this properly calculates the period end date
/// using calendar-aware month addition based on the index tenor.
pub fn tranche_all_in_rate(coupon: &TrancheCoupon, date: Date, market: &MarketContext) -> f64 {
    // Backward-compatible wrapper that never panics. For correctness-first valuation, prefer
    // `try_tranche_all_in_rate` and propagate errors.
    match coupon {
        TrancheCoupon::Fixed { rate } => *rate,
        TrancheCoupon::Floating(spec) => {
            let spread_bp_f64 = spec.spread_bp.to_f64().unwrap_or_default();
            let gearing_f64 = spec.gearing.to_f64().unwrap_or(1.0);
            let floor_bp_f64 = spec.floor_bp.and_then(|d| d.to_f64());
            let cap_bp_f64 = spec.cap_bp.and_then(|d| d.to_f64());
            let fallback_rate = spread_bp_f64 / 10_000.0;

            let fwd = match market.get_forward(spec.index_id.as_str()) {
                Ok(c) => c,
                Err(_) => return fallback_rate,
            };

            let tenor = fwd.tenor();
            let period_end = match try_tenor_to_period_end(date, tenor, fwd.day_count()) {
                Ok(d) => d,
                Err(_) => return fallback_rate,
            };

            let params = crate::cashflow::builder::FloatingRateParams::with_full(
                spread_bp_f64,
                gearing_f64,
                floor_bp_f64,
                cap_bp_f64,
            );
            crate::cashflow::builder::project_floating_rate_from_market(
                date,
                period_end,
                spec.index_id.as_str(),
                &params,
                market,
            )
            .unwrap_or(fallback_rate)
        }
    }
}

/// Fallible variant of [`tranche_all_in_rate`].
///
/// This returns an error if required market data is missing or the rate projection fails.
pub fn try_tranche_all_in_rate(
    coupon: &TrancheCoupon,
    date: Date,
    market: &MarketContext,
) -> finstack_core::Result<f64> {
    match coupon {
        TrancheCoupon::Fixed { rate } => Ok(*rate),
        TrancheCoupon::Floating(spec) => {
            let spread_bp_f64 = spec
                .spread_bp
                .to_f64()
                .ok_or(finstack_core::InputError::Invalid)?;
            let gearing_f64 = spec
                .gearing
                .to_f64()
                .ok_or(finstack_core::InputError::Invalid)?;
            let floor_bp_f64 = spec
                .floor_bp
                .map(|d| d.to_f64().ok_or(finstack_core::InputError::Invalid))
                .transpose()?;
            let cap_bp_f64 = spec
                .cap_bp
                .map(|d| d.to_f64().ok_or(finstack_core::InputError::Invalid))
                .transpose()?;

            let fwd = market.get_forward(spec.index_id.as_str())?;
            let tenor = fwd.tenor();
            let period_end = try_tenor_to_period_end(date, tenor, fwd.day_count())?;

            let params = crate::cashflow::builder::FloatingRateParams::with_full(
                spread_bp_f64,
                gearing_f64,
                floor_bp_f64,
                cap_bp_f64,
            );
            crate::cashflow::builder::project_floating_rate_from_market(
                date,
                period_end,
                spec.index_id.as_str(),
                &params,
                market,
            )
        }
    }
}

/// Compute asset all-in rate given optional index id and spread; falls back to provided rate.
///
/// Uses the forward curve's own day count convention for year fraction calculations
/// to ensure consistency with how the curve was calibrated.
pub fn asset_all_in_rate(
    index_id: Option<&str>,
    spread_bps: Option<f64>,
    fallback_rate: f64,
    date: Date,
    market: &MarketContext,
) -> f64 {
    if let Some(idx) = index_id {
        if let Ok(fwd) = market.get_forward(idx) {
            let base = fwd.base_date();
            let dc = fwd.day_count();
            let t2 = dc
                .year_fraction(base, date, DayCountCtx::default())
                .unwrap_or(0.0);
            let tenor = fwd.tenor();
            let t1 = (t2 - tenor).max(0.0);
            let idx_rate = fwd.rate_period(t1, t2);
            let spread = spread_bps.unwrap_or(0.0) / 10_000.0;
            return idx_rate + spread;
        }
    }
    fallback_rate
}

/// Fallible variant of [`asset_all_in_rate`].
///
/// This returns an error if the forward curve is missing or if date/year-fraction computation
/// fails. Use this in valuation code paths where silent fallbacks are unacceptable.
pub fn try_asset_all_in_rate(
    index_id: Option<&str>,
    spread_bps: Option<f64>,
    date: Date,
    market: &MarketContext,
) -> finstack_core::Result<f64> {
    let Some(idx) = index_id else {
        return Err(finstack_core::InputError::NotFound {
            id: "asset.index_id".to_string(),
        }
        .into());
    };
    let fwd = market.get_forward(idx)?;
    let base = fwd.base_date();
    let dc = fwd.day_count();
    let t2 = dc.year_fraction(base, date, DayCountCtx::default())?;
    let tenor = fwd.tenor();
    let t1 = (t2 - tenor).max(0.0);
    let idx_rate = fwd.rate_period(t1, t2);
    let spread = spread_bps.unwrap_or(0.0) / 10_000.0;
    Ok(idx_rate + spread)
}
