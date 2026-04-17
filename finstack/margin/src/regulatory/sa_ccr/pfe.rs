//! PFE (Potential Future Exposure) computation for SA-CCR.
//!
//! PFE = multiplier * AddOn_aggregate

use super::add_on::asset_class_add_on;
use super::maturity_factor::{maturity_factor_margined, maturity_factor_unmargined};
use super::types::{SaCcrAssetClass, SaCcrNettingSetConfig, SaCcrTrade};
use finstack_core::HashMap;

/// PFE multiplier floor per BCBS 279 paragraph 149.
const MULTIPLIER_FLOOR: f64 = 0.05;

/// Compute PFE components: (multiplier, add_on_aggregate, add_on_by_class).
///
/// `multiplier = min(1, floor + (1 - floor) * exp(V_minus_C / (2 * (1-floor) * AddOn)))`
/// `PFE = multiplier * AddOn_aggregate`
pub fn pfe(
    config: &SaCcrNettingSetConfig,
    trades: &[SaCcrTrade],
) -> (f64, f64, HashMap<SaCcrAssetClass, f64>) {
    let mf = compute_maturity_factor(config, trades);

    // Compute add-on per asset class.
    let mut add_on_by_class = HashMap::default();
    let mut add_on_aggregate = 0.0;
    for &ac in SaCcrAssetClass::ALL {
        let ao = asset_class_add_on(ac, trades, mf);
        if ao > 0.0 {
            add_on_by_class.insert(ac, ao);
            add_on_aggregate += ao;
        }
    }

    // Compute multiplier. Basel 279 para 149: the multiplier input is
    // V - C where C is *total* net collateral, i.e. VM + NICA.
    let v: f64 = trades.iter().map(|t| t.mtm).sum();
    let v_minus_c = v - config.collateral - config.nica;
    let mult = multiplier(v_minus_c, add_on_aggregate);

    (mult, add_on_aggregate, add_on_by_class)
}

/// PFE multiplier per BCBS 279 paragraph 149.
///
/// Recognizes excess collateral by scaling AddOn below 1.0
/// when the netting set is over-collateralized (V - C < 0).
/// Floor of 5% prevents the multiplier from reaching zero.
pub fn multiplier(v_minus_c: f64, add_on: f64) -> f64 {
    if add_on <= 0.0 {
        return MULTIPLIER_FLOOR;
    }
    let ratio = v_minus_c / (2.0 * (1.0 - MULTIPLIER_FLOOR) * add_on);
    f64::min(
        1.0,
        MULTIPLIER_FLOOR + (1.0 - MULTIPLIER_FLOOR) * ratio.exp(),
    )
}

/// Compute the representative maturity factor for a netting set.
///
/// For margined sets: uses MPOR-based formula.
/// For unmargined sets: uses the average tenor across trades, with the
/// regulatory floor of 10 business days.
///
/// # Known limitations
///
/// * Basel 279 defines `M_i` as *remaining* maturity (from the
///   calculation date to trade end). This implementation uses
///   `end - start` because no calculation-date parameter is currently
///   plumbed into `calculate_ead`. For already-seasoned trades the two
///   differ and the result slightly overstates MF. Wire an `as_of`
///   parameter through if precision for seasoned books matters.
/// * The floor `10 / 250` is a business-day year fraction while
///   `(end - start) / 365` is calendar. The mismatch is ~4% at the
///   floor and is clamped to 1 year at the ceiling, so the effective
///   bias is small for typical derivatives maturities.
fn compute_maturity_factor(config: &SaCcrNettingSetConfig, trades: &[SaCcrTrade]) -> f64 {
    if config.is_margined {
        maturity_factor_margined(config.mpor_days)
    } else if trades.is_empty() {
        maturity_factor_unmargined(10.0 / 250.0)
    } else {
        let avg_maturity: f64 = trades
            .iter()
            .map(|t| {
                let days = (t.end_date - t.start_date).whole_days().max(0) as f64;
                days / 365.0
            })
            .sum::<f64>()
            / trades.len() as f64;
        maturity_factor_unmargined(avg_maturity)
    }
}
