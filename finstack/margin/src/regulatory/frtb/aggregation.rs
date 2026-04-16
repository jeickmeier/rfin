//! Cross-risk-class aggregation for FRTB SBA.
//!
//! The FRTB SBA aggregation is simple addition -- there is no cross-risk-class
//! correlation matrix for the SBA (unlike SIMM). Each risk class charge is
//! computed independently and summed.

use super::types::FrtbRiskClass;
use finstack_core::HashMap;

/// Aggregate delta+vega+curvature across risk classes for one correlation scenario.
///
/// `SBA_agg = sum_rc [ Delta_rc + Vega_rc + Curvature_rc ]`
///
/// The final capital charge picks the maximum across scenarios:
///   `Capital = max(SBA_agg_low, SBA_agg_medium, SBA_agg_high) + DRC + RRAO`
pub fn aggregate_sba(
    delta_charges: &HashMap<FrtbRiskClass, f64>,
    vega_charges: &HashMap<FrtbRiskClass, f64>,
    curvature_charges: &HashMap<FrtbRiskClass, f64>,
) -> f64 {
    let sum_delta: f64 = delta_charges.values().sum();
    let sum_vega: f64 = vega_charges.values().sum();
    let sum_curvature: f64 = curvature_charges.values().sum();
    sum_delta + sum_vega + sum_curvature
}
