//! Helpers to compute all-in rates using core market_data curves.

use finstack_core::dates::{Date, DayCount, DayCountCtx};
use finstack_core::market_data::MarketContext;

use super::tranches::TrancheCoupon;

/// Compute tranche all-in rate (fixed => fixed; floating => index forward + spread with caps/floors).
pub fn tranche_all_in_rate(coupon: &TrancheCoupon, date: Date, market: &MarketContext) -> f64 {
    match coupon {
        TrancheCoupon::Fixed { rate } => *rate,
        TrancheCoupon::Floating {
            forward_curve_id,
            spread_bp,
            floor,
            cap,
        } => {
            // Look up forward curve
            let fwd = match market.get_forward_ref(forward_curve_id.as_str()) {
                Ok(c) => c,
                Err(_) => return *spread_bp / 10_000.0,
            };

            let base = fwd.base_date();
            let dc = fwd.day_count();
            let t2 = dc
                .year_fraction(base, date, DayCountCtx::default())
                .unwrap_or(0.0);
            let tenor = fwd.tenor();
            let t1 = (t2 - tenor).max(0.0);
            let idx_rate = fwd.rate_period(t1, t2);

            let mut all_in = idx_rate + (*spread_bp / 10_000.0);
            if let Some(c) = cap {
                all_in = all_in.min(*c);
            }
            if let Some(f) = floor {
                all_in = all_in.max(*f);
            }
            all_in
        }
    }
}

/// Compute asset all-in rate given optional index id and spread; falls back to provided rate.
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
            let dc = DayCount::Act365F; // consistent with other call sites
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
