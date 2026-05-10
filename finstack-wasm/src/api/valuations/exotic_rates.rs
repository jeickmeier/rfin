//! Deterministic coupon / payoff helpers for exotic rate products.
//!
//! Mirrors `finstack-py`'s `valuations/exotic_rates.rs`: lightweight, market-
//! data-free helpers useful for building test fixtures and inspecting coupon
//! trajectories. Full MC / copula / LSMC pricers stay on the standard
//! `priceInstrument` / `priceInstrumentWithMetrics` pipeline.

use crate::utils::to_js_err;
use finstack_valuations::instruments::rates::exotics_shared::cumulative_coupon::CumulativeCouponTracker;
use wasm_bindgen::prelude::*;

/// Simulated TARN coupon profile along a deterministic floating-rate path.
///
/// Returns a JSON object:
/// ```text
/// {
///   "coupons_paid": number[],
///   "cumulative":   number[],
///   "redemption_index": number | null,
///   "redeemed_early":   boolean
/// }
/// ```
///
/// Each period's coupon is `max(fixed_rate - L_i, coupon_floor) * day_count_fraction`.
/// Payments accumulate in a [`CumulativeCouponTracker`] configured with
/// `target_coupon`; once cumulative hits the target, the final coupon is
/// capped and the instrument is considered redeemed.
#[wasm_bindgen(js_name = tarnCouponProfile)]
pub fn tarn_coupon_profile(
    fixed_rate: f64,
    coupon_floor: f64,
    floating_fixings: Vec<f64>,
    target_coupon: f64,
    day_count_fraction: f64,
) -> Result<JsValue, JsValue> {
    if !target_coupon.is_finite() || target_coupon <= 0.0 {
        return Err(to_js_err(format!(
            "target_coupon ({target_coupon}) must be positive and finite"
        )));
    }
    if !day_count_fraction.is_finite() || day_count_fraction <= 0.0 {
        return Err(to_js_err(format!(
            "day_count_fraction ({day_count_fraction}) must be positive and finite"
        )));
    }
    if !fixed_rate.is_finite() {
        return Err(to_js_err("fixed_rate must be finite"));
    }
    if !coupon_floor.is_finite() || coupon_floor < 0.0 {
        return Err(to_js_err(format!(
            "coupon_floor ({coupon_floor}) must be non-negative and finite"
        )));
    }

    let n = floating_fixings.len();
    let mut tracker = CumulativeCouponTracker::with_target(target_coupon);
    let mut coupons_paid: Vec<f64> = Vec::with_capacity(n);
    let mut cumulative: Vec<f64> = Vec::with_capacity(n);

    for &l_i in &floating_fixings {
        if !l_i.is_finite() {
            return Err(to_js_err("floating_fixings must all be finite"));
        }
        let raw = (fixed_rate - l_i).max(coupon_floor);
        let period_coupon = raw * day_count_fraction;
        let actual = tracker.add_coupon(period_coupon);
        coupons_paid.push(actual);
        cumulative.push(tracker.cumulative());
    }

    let (redemption_index, redeemed_early) = match tracker.knockout_period() {
        Some(idx) => (Some(idx), idx + 1 < n),
        None => (None, false),
    };
    let payload = serde_json::json!({
        "coupons_paid": coupons_paid,
        "cumulative": cumulative,
        "redemption_index": redemption_index,
        "redeemed_early": redeemed_early,
    });
    serde_wasm_bindgen::to_value(&payload).map_err(to_js_err)
}

/// Snowball / inverse-floater coupon schedule.
///
/// For snowball (`is_inverse_floater = false`):
///   `c_i = clip(c_{i-1} + fixed_rate - L_i, floor, cap)` with `c_0 = initial_coupon`.
///
/// For inverse floater (`is_inverse_floater = true`):
///   `c_i = clip(fixed_rate - leverage * L_i, floor, cap)` (path-independent).
#[wasm_bindgen(js_name = snowballCouponProfile)]
#[allow(clippy::too_many_arguments)]
pub fn snowball_coupon_profile(
    initial_coupon: f64,
    fixed_rate: f64,
    floating_fixings: Vec<f64>,
    floor: f64,
    cap: f64,
    is_inverse_floater: bool,
    leverage: Option<f64>,
) -> Result<Vec<f64>, JsValue> {
    let leverage = leverage.unwrap_or(1.0);
    if !fixed_rate.is_finite() {
        return Err(to_js_err("fixed_rate must be finite"));
    }
    if !floor.is_finite() || floor < 0.0 {
        return Err(to_js_err(format!(
            "floor ({floor}) must be non-negative and finite"
        )));
    }
    if cap.is_nan() || cap <= floor {
        return Err(to_js_err(format!(
            "cap ({cap}) must be strictly greater than floor ({floor})"
        )));
    }
    if !leverage.is_finite() || leverage <= 0.0 {
        return Err(to_js_err(format!(
            "leverage ({leverage}) must be positive and finite"
        )));
    }
    if !is_inverse_floater && initial_coupon < 0.0 {
        return Err(to_js_err(format!(
            "initial_coupon ({initial_coupon}) must be non-negative for snowball variant"
        )));
    }

    let mut prev = initial_coupon;
    let mut out: Vec<f64> = Vec::with_capacity(floating_fixings.len());
    for &l_i in &floating_fixings {
        if !l_i.is_finite() {
            return Err(to_js_err("floating_fixings must all be finite"));
        }
        let raw = if is_inverse_floater {
            fixed_rate - leverage * l_i
        } else {
            prev + fixed_rate - l_i
        };
        let floored = raw.max(floor);
        let c = if cap.is_finite() {
            floored.min(cap)
        } else {
            floored
        };
        out.push(c);
        prev = c;
    }
    Ok(out)
}

/// Intrinsic (undiscounted, unhedged) payoff of a CMS spread option.
///
/// `call:  notional * max(long_cms - short_cms - strike, 0)`
/// `put:   notional * max(strike - (long_cms - short_cms), 0)`
#[wasm_bindgen(js_name = cmsSpreadOptionIntrinsic)]
pub fn cms_spread_option_intrinsic(
    long_cms: f64,
    short_cms: f64,
    strike: f64,
    is_call: bool,
    notional: f64,
) -> Result<f64, JsValue> {
    if !long_cms.is_finite() || !short_cms.is_finite() || !strike.is_finite() {
        return Err(to_js_err(
            "long_cms, short_cms, and strike must all be finite",
        ));
    }
    if !notional.is_finite() || notional < 0.0 {
        return Err(to_js_err(format!(
            "notional ({notional}) must be non-negative and finite"
        )));
    }
    let spread = long_cms - short_cms;
    let payoff = if is_call {
        (spread - strike).max(0.0)
    } else {
        (strike - spread).max(0.0)
    };
    Ok(payoff * notional)
}

/// Accrued coupon on a range-accrual leg over a set of observations.
///
/// Counts the fraction of observations with a rate in the inclusive interval
/// `[lower, upper]` and scales by the period day-count fraction:
///
/// `accrued = coupon_rate * day_count_fraction * (#in-range / #observations)`.
///
/// The call provision is not applied here.
#[wasm_bindgen(js_name = callableRangeAccrualAccrued)]
pub fn callable_range_accrual_accrued(
    lower: f64,
    upper: f64,
    observations: Vec<f64>,
    coupon_rate: f64,
    day_count_fraction: f64,
) -> Result<f64, JsValue> {
    if !lower.is_finite() || !upper.is_finite() || lower >= upper {
        return Err(to_js_err(format!(
            "lower ({lower}) must be strictly less than upper ({upper}), and both finite"
        )));
    }
    if !coupon_rate.is_finite() || coupon_rate < 0.0 {
        return Err(to_js_err(format!(
            "coupon_rate ({coupon_rate}) must be non-negative and finite"
        )));
    }
    if !day_count_fraction.is_finite() || day_count_fraction < 0.0 {
        return Err(to_js_err(format!(
            "day_count_fraction ({day_count_fraction}) must be non-negative and finite"
        )));
    }
    if observations.is_empty() {
        return Err(to_js_err("observations must contain at least one value"));
    }

    let mut in_range = 0usize;
    for &o in &observations {
        if !o.is_finite() {
            return Err(to_js_err("observations must all be finite"));
        }
        if o >= lower && o <= upper {
            in_range += 1;
        }
    }
    let fraction = in_range as f64 / observations.len() as f64;
    Ok(coupon_rate * day_count_fraction * fraction)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snowball_honors_cap_and_floor() {
        let coupons =
            snowball_coupon_profile(0.02, 0.05, vec![0.01, 0.04, 0.03], 0.0, 0.10, false, None)
                .expect("snowball");
        assert_eq!(coupons.len(), 3);
        for c in coupons {
            assert!((0.0..=0.10).contains(&c));
        }
    }

    #[test]
    fn cms_spread_option_intrinsic_call_works() {
        let p = cms_spread_option_intrinsic(0.04, 0.02, 0.01, true, 1_000_000.0).expect("cms");
        assert!((p - 10_000.0).abs() < 1e-9);
    }
}
