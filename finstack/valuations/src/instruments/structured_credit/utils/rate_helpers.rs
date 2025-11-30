//! Helpers to compute all-in rates using core market_data curves.
//!
//! These helpers properly compute floating rate projections using
//! calendar-aware tenor addition for accurate period end dates.

use finstack_core::dates::{Date, DayCount, DayCountCtx};
use finstack_core::market_data::MarketContext;

use crate::instruments::structured_credit::types::TrancheCoupon;

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
    use finstack_core::dates::{BusinessDayConvention, Tenor};
    let tenor = Tenor::from_years(tenor_years, day_count);
    tenor
        .add_to_date(start, None, BusinessDayConvention::Unadjusted)
        .expect("Date addition failed")
}

/// Compute tranche all-in rate (fixed => fixed; floating => index forward + spread with caps/floors).
///
/// For floating rate tranches, this properly calculates the period end date
/// using calendar-aware month addition based on the index tenor.
pub fn tranche_all_in_rate(coupon: &TrancheCoupon, date: Date, market: &MarketContext) -> f64 {
    match coupon {
        TrancheCoupon::Fixed { rate } => *rate,
        TrancheCoupon::Floating(spec) => {
            let fwd = match market.get_forward_ref(spec.index_id.as_str()) {
                Ok(c) => c,
                Err(_) => return spec.spread_bp / 10_000.0,
            };

            let tenor = fwd.tenor();
            let period_end = tenor_to_period_end(date, tenor, fwd.day_count());

            crate::cashflow::builder::project_floating_rate(
                date,
                period_end,
                spec.index_id.as_str(),
                spec.spread_bp,
                spec.gearing,
                spec.floor_bp,
                spec.cap_bp,
                market,
            )
            .unwrap_or(spec.spread_bp / 10_000.0)
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
        if let Ok(fwd) = market.get_forward_ref(idx) {
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
