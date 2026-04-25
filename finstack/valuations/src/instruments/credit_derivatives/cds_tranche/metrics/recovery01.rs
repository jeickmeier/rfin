//! Recovery01 calculator for CDS Tranche.
//!
//! Computes Recovery01 (recovery rate sensitivity) using finite differences.
//! Recovery01 measures the change in PV for a 1% (100bp) change in recovery rate.
//!
//! Note: Recovery rate is stored in the credit index, so we need to bump
//! the recovery rate in the CreditIndexData.
//!
//! ## Limitation: Partial Recovery Sensitivity Only
//!
//! This calculator bumps the recovery rate in the credit index data but does **not**
//! recalibrate the underlying hazard curves. Since `h ≈ S / (1 - R)`, changing R
//! without recalibrating understates the true recovery sensitivity. Professional
//! systems (Bloomberg, QuantLib) recalibrate the hazard curve after bumping recovery.
//! This implementation provides a **partial / local** recovery sensitivity only —
//! traders hedging recovery risk should expect this to differ materially from a
//! full recalibrated bump for spread-bootstrapped curves (typically by 2-5x for
//! distressed credits).

use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::credit_derivatives::cds_tranche::CDSTranche;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::market_data::term_structures::CreditIndexData;
use finstack_core::Result;

/// Standard recovery rate bump: 1% (0.01)
const RECOVERY_BUMP: f64 = 0.01;

/// Recovery01 calculator for CDS Tranche.
pub(crate) struct Recovery01Calculator;

/// Build a copy of `original` with `recovery_rate` replaced, preserving all
/// per-issuer overrides (curves, recovery rates, weights). Mirrors the
/// pricer's own rebuild logic so that heterogeneous bespoke tranches are
/// not silently downgraded to the homogeneous binomial path during bumping.
fn rebuild_with_recovery(
    original: &CreditIndexData,
    new_recovery: f64,
) -> Result<CreditIndexData> {
    let mut builder = CreditIndexData::builder()
        .num_constituents(original.num_constituents)
        .recovery_rate(new_recovery)
        .index_credit_curve(original.index_credit_curve.clone())
        .base_correlation_curve(original.base_correlation_curve.clone());

    if let Some(curves) = &original.issuer_credit_curves {
        builder = builder.issuer_curves(curves.clone());
    }
    if let Some(rates) = &original.issuer_recovery_rates {
        builder = builder.issuer_recovery_rates(rates.clone());
    }
    if let Some(weights) = &original.issuer_weights {
        builder = builder.issuer_weights(weights.clone());
    }

    builder.build()
}

impl MetricCalculator for Recovery01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let tranche: &CDSTranche = context.instrument_as()?;
        let as_of = context.as_of;

        let original_index = context.curves.get_credit_index(&tranche.credit_index_id)?;
        let base_recovery = original_index.recovery_rate;

        let bumped_recovery_up = (base_recovery + RECOVERY_BUMP).clamp(0.0, 1.0);
        let up_delta = bumped_recovery_up - base_recovery;
        let bumped_index_up = rebuild_with_recovery(original_index.as_ref(), bumped_recovery_up)?;
        let curves_up = context
            .curves
            .as_ref()
            .clone()
            .insert_credit_index(&tranche.credit_index_id, bumped_index_up);
        let pv_up = tranche.value(&curves_up, as_of)?.amount();

        let bumped_recovery_down = (base_recovery - RECOVERY_BUMP).clamp(0.0, 1.0);
        let down_delta = base_recovery - bumped_recovery_down;
        let bumped_index_down =
            rebuild_with_recovery(original_index.as_ref(), bumped_recovery_down)?;
        let curves_down = context
            .curves
            .as_ref()
            .clone()
            .insert_credit_index(&tranche.credit_index_id, bumped_index_down);
        let pv_down = tranche.value(&curves_down, as_of)?.amount();

        let span = up_delta + down_delta;
        if span <= 0.0 {
            return Ok(0.0);
        }
        let recovery01 = (pv_up - pv_down) / span * RECOVERY_BUMP;

        Ok(recovery01)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::market_data::term_structures::{BaseCorrelationCurve, HazardCurve};
    use std::sync::Arc;

    fn sample_index(
        with_per_issuer_recovery: bool,
        with_per_issuer_weight: bool,
    ) -> CreditIndexData {
        let base = time::macros::date!(2024 - 01 - 01);
        let hz = Arc::new(
            HazardCurve::builder("HZ-IDX")
                .base_date(base)
                .recovery_rate(0.40)
                .knots([(1.0, 0.02), (5.0, 0.025)])
                .build()
                .expect("hazard curve"),
        );
        let bc = Arc::new(
            BaseCorrelationCurve::builder("BC-IDX")
                .knots([(3.0, 0.30), (7.0, 0.35), (15.0, 0.45)])
                .build()
                .expect("base correlation"),
        );

        let mut issuer_curves = finstack_core::HashMap::default();
        for i in 0..3 {
            let id = format!("ISS-{i}");
            issuer_curves.insert(id, Arc::clone(&hz));
        }

        let mut builder = CreditIndexData::builder()
            .num_constituents(3)
            .recovery_rate(0.40)
            .index_credit_curve(Arc::clone(&hz))
            .base_correlation_curve(Arc::clone(&bc))
            .issuer_curves(issuer_curves);

        if with_per_issuer_recovery {
            let mut rates = finstack_core::HashMap::default();
            rates.insert("ISS-0".to_string(), 0.10);
            rates.insert("ISS-1".to_string(), 0.50);
            rates.insert("ISS-2".to_string(), 0.70);
            builder = builder.issuer_recovery_rates(rates);
        }

        if with_per_issuer_weight {
            let mut weights = finstack_core::HashMap::default();
            weights.insert("ISS-0".to_string(), 0.50);
            weights.insert("ISS-1".to_string(), 0.30);
            weights.insert("ISS-2".to_string(), 0.20);
            builder = builder.issuer_weights(weights);
        }

        builder.build().expect("credit index")
    }

    #[test]
    fn rebuild_with_recovery_preserves_per_issuer_recovery_rates() {
        let original = sample_index(true, false);
        let bumped = rebuild_with_recovery(&original, 0.41).expect("rebuild");

        assert!((bumped.recovery_rate - 0.41).abs() < 1e-12);

        // Per-issuer recovery overrides survive intact (this is the regression).
        assert!(
            bumped.issuer_recovery_rates.is_some(),
            "issuer_recovery_rates dropped"
        );
        let rates = bumped
            .issuer_recovery_rates
            .as_ref()
            .expect("rates present");
        assert!((rates.get("ISS-0").copied().unwrap_or(0.0) - 0.10).abs() < 1e-12);
        assert!((rates.get("ISS-1").copied().unwrap_or(0.0) - 0.50).abs() < 1e-12);
        assert!((rates.get("ISS-2").copied().unwrap_or(0.0) - 0.70).abs() < 1e-12);
    }

    #[test]
    fn rebuild_with_recovery_preserves_per_issuer_weights() {
        let original = sample_index(false, true);
        let bumped = rebuild_with_recovery(&original, 0.39).expect("rebuild");

        assert!((bumped.recovery_rate - 0.39).abs() < 1e-12);

        // Per-issuer weights survive intact (the second leg of the regression).
        assert!(bumped.issuer_weights.is_some(), "issuer_weights dropped");
        let weights = bumped.issuer_weights.as_ref().expect("weights present");
        assert!((weights.get("ISS-0").copied().unwrap_or(0.0) - 0.50).abs() < 1e-12);
        assert!((weights.get("ISS-1").copied().unwrap_or(0.0) - 0.30).abs() < 1e-12);
        assert!((weights.get("ISS-2").copied().unwrap_or(0.0) - 0.20).abs() < 1e-12);
    }

    #[test]
    fn rebuild_with_recovery_preserves_all_three_overrides_together() {
        let original = sample_index(true, true);
        let bumped = rebuild_with_recovery(&original, 0.42).expect("rebuild");

        assert!((bumped.recovery_rate - 0.42).abs() < 1e-12);
        assert!(bumped.issuer_credit_curves.is_some(), "issuer_curves dropped");
        assert!(
            bumped.issuer_recovery_rates.is_some(),
            "issuer_recovery_rates dropped"
        );
        assert!(bumped.issuer_weights.is_some(), "issuer_weights dropped");
        assert_eq!(bumped.num_constituents, original.num_constituents);
    }

    #[test]
    fn rebuild_with_recovery_handles_homogeneous_index() {
        // Homogeneous (no per-issuer recovery/weight overrides) — must not
        // panic and must propagate the bumped recovery without inventing fields.
        let original = sample_index(false, false);
        let bumped = rebuild_with_recovery(&original, 0.45).expect("rebuild");

        assert!((bumped.recovery_rate - 0.45).abs() < 1e-12);
        assert!(bumped.issuer_credit_curves.is_some());
        assert!(bumped.issuer_recovery_rates.is_none());
        assert!(bumped.issuer_weights.is_none());
    }
}
