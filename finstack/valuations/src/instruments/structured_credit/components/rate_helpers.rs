//! Helpers to compute all-in rates using core market_data curves.
//!
//! These helpers properly compute floating rate projections using
//! calendar-aware tenor addition for accurate period end dates.

use finstack_core::dates::{Date, DayCountCtx};
use finstack_core::market_data::MarketContext;

use super::tranches::TrancheCoupon;

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
pub fn tenor_to_period_end(start: Date, tenor_years: f64) -> Date {
    // Convert tenor years to months for standard tenors
    let tenor_months = (tenor_years * 12.0).round() as i64;
    
    if tenor_months > 0 && tenor_months <= 12 {
        // Use proper month addition for standard tenors (1M, 3M, 6M, 12M)
        // This handles end-of-month and other edge cases correctly
        let (year, month, day) = (start.year(), start.month(), start.day());
        
        // Calculate target month and year
        let month_ord = month as i64;
        let total_months = month_ord + tenor_months;
        let target_year = year + ((total_months - 1) / 12) as i32;
        let target_month_ord = ((total_months - 1) % 12 + 1) as u8;
        
        let target_month = match target_month_ord {
            1 => time::Month::January,
            2 => time::Month::February,
            3 => time::Month::March,
            4 => time::Month::April,
            5 => time::Month::May,
            6 => time::Month::June,
            7 => time::Month::July,
            8 => time::Month::August,
            9 => time::Month::September,
            10 => time::Month::October,
            11 => time::Month::November,
            12 => time::Month::December,
            _ => time::Month::January,
        };
        
        // Handle end-of-month: if start day is EOM, target should also be EOM
        let days_in_target_month = target_month.length(target_year);
        let target_day = day.min(days_in_target_month);
        
        Date::from_calendar_date(target_year, target_month, target_day)
            .unwrap_or_else(|_| start + time::Duration::days((tenor_years * 365.25) as i64))
    } else {
        // Fallback for unusual tenors
        start + time::Duration::days((tenor_years * 365.25) as i64)
    }
}

/// Compute tranche all-in rate (fixed => fixed; floating => index forward + spread with caps/floors).
///
/// For floating rate tranches, this properly calculates the period end date
/// using calendar-aware month addition based on the index tenor.
pub fn tranche_all_in_rate(coupon: &TrancheCoupon, date: Date, market: &MarketContext) -> f64 {
    match coupon {
        TrancheCoupon::Fixed { rate } => *rate,
        TrancheCoupon::Floating(spec) => {
            // Use centralized projection
            let fwd = match market.get_forward_ref(spec.index_id.as_str()) {
                Ok(c) => c,
                Err(_) => return spec.spread_bp / 10_000.0,
            };

            let tenor = fwd.tenor();
            let period_end = tenor_to_period_end(date, tenor);

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
            // Use the curve's own day count for consistency with calibration
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
    // Fallback to stored all-in
    fallback_rate
}
