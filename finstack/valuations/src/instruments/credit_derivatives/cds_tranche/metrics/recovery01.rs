//! Recovery01 calculator for CDS Tranche.
//!
//! Computes Recovery01 (recovery rate sensitivity) using finite differences.
//! Recovery01 measures the change in PV for a 1% (100bp) change in recovery rate.
//!
//! Note: Recovery rate is stored in the credit index, so we need to bump
//! the recovery rate in the CreditIndexData.
//!
//! ## Limitation: Frozen Hazard Curve
//!
//! This calculator bumps the recovery rate in the credit index data but does **not**
//! recalibrate the underlying hazard curves. Since `h ≈ S / (1 - R)`, changing R
//! without recalibrating understates the true recovery sensitivity. Professional
//! systems (Bloomberg, QuantLib) recalibrate. This provides a "local" or "partial"
//! recovery sensitivity only.

use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::credit_derivatives::cds_tranche::CDSTranche;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Standard recovery rate bump: 1% (0.01)
const RECOVERY_BUMP: f64 = 0.01;

/// Recovery01 calculator for CDS Tranche.
pub(crate) struct Recovery01Calculator;

impl MetricCalculator for Recovery01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let tranche: &CDSTranche = context.instrument_as()?;
        let as_of = context.as_of;

        // Get the credit index
        let original_index = context.curves.get_credit_index(&tranche.credit_index_id)?;
        let base_recovery = original_index.recovery_rate;

        // Create bumped credit index (up)
        use finstack_core::market_data::term_structures::CreditIndexData;
        let bumped_recovery_up = (base_recovery + RECOVERY_BUMP).clamp(0.0, 1.0);
        let bumped_index_up = CreditIndexData::builder()
            .num_constituents(original_index.num_constituents)
            .recovery_rate(bumped_recovery_up)
            .index_credit_curve(original_index.index_credit_curve.clone())
            .base_correlation_curve(original_index.base_correlation_curve.clone())
            .issuer_curves(
                original_index
                    .issuer_credit_curves
                    .clone()
                    .unwrap_or_default(),
            )
            .build()?;

        let curves_up = context
            .curves
            .as_ref()
            .clone()
            .insert_credit_index(&tranche.credit_index_id, bumped_index_up);
        let pv_up = tranche.value(&curves_up, as_of)?.amount();

        // Create bumped credit index (down)
        let bumped_recovery_down = (base_recovery - RECOVERY_BUMP).clamp(0.0, 1.0);
        let bumped_index_down = CreditIndexData::builder()
            .num_constituents(original_index.num_constituents)
            .recovery_rate(bumped_recovery_down)
            .index_credit_curve(original_index.index_credit_curve.clone())
            .base_correlation_curve(original_index.base_correlation_curve.clone())
            .issuer_curves(
                original_index
                    .issuer_credit_curves
                    .clone()
                    .unwrap_or_default(),
            )
            .build()?;

        let curves_down = context
            .curves
            .as_ref()
            .clone()
            .insert_credit_index(&tranche.credit_index_id, bumped_index_down);
        let pv_down = tranche.value(&curves_down, as_of)?.amount();

        // Recovery01 = (PV_up - PV_down) / (2 * bump_size)
        let recovery01 = (pv_up - pv_down) / (2.0 * RECOVERY_BUMP);

        Ok(recovery01)
    }
}
