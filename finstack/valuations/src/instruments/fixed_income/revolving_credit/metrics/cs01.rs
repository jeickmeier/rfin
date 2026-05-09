//! Revolving-credit-specific CS01 calculators with graceful handling for
//! missing credit curves.
//!
//! When a facility has an associated credit (hazard) curve, CS01 follows the
//! [canonical CS01 convention][canonical] — a parallel 1 bp shock to par CDS
//! spreads with a symmetric (central) finite difference — by delegating to
//! [`GenericParallelCs01`] / [`GenericBucketedCs01`]. Direct hazard-rate
//! bump variants (same canonical sign convention, hazard-rate shift instead
//! of par-spread shock) are exposed via [`GenericParallelCs01Hazard`] /
//! [`GenericBucketedCs01Hazard`].
//!
//! When no credit curve is configured, all CS01 variants return `0.0`: the
//! facility has no credit-model dependency to be sensitive to.
//!
//! Sign convention is identical to the canonical reference:
//! - Long facility (drawn) → CS01 negative.
//! - Short facility → CS01 positive.
//!
//! [canonical]: crate::metrics::sensitivities::cs01

use crate::instruments::common_impl::traits::CurveDependencies;
use crate::instruments::RevolvingCredit;
use crate::metrics::{MetricCalculator, MetricContext};

/// Revolving credit parallel CS01 with graceful no-credit-curve handling.
///
/// Delegates to [`GenericParallelCs01`] when the facility references a credit
/// curve; otherwise returns 0.0.
pub(crate) struct RevolvingCreditCs01Calculator;

impl MetricCalculator for RevolvingCreditCs01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let facility: &RevolvingCredit = context.instrument_as()?;
        let curves = facility.curve_dependencies()?;

        if curves.credit_curves.is_empty() {
            return Ok(0.0);
        }

        crate::metrics::GenericParallelCs01::<RevolvingCredit>::default().calculate(context)
    }
}

/// Revolving credit bucketed CS01 with graceful no-credit-curve handling.
///
/// Delegates to [`GenericBucketedCs01`] when the facility references a credit
/// curve; otherwise returns 0.0.
pub(crate) struct RevolvingCreditBucketedCs01Calculator;

impl MetricCalculator for RevolvingCreditBucketedCs01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let facility: &RevolvingCredit = context.instrument_as()?;
        let curves = facility.curve_dependencies()?;

        if curves.credit_curves.is_empty() {
            return Ok(0.0);
        }

        crate::metrics::GenericBucketedCs01::<RevolvingCredit>::default().calculate(context)
    }
}

/// Revolving credit parallel CS01 (hazard-rate bump) with graceful handling.
///
/// Delegates to [`GenericParallelCs01Hazard`] when the facility references a
/// credit curve; otherwise returns 0.0.
pub(crate) struct RevolvingCreditCs01HazardCalculator;

impl MetricCalculator for RevolvingCreditCs01HazardCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let facility: &RevolvingCredit = context.instrument_as()?;
        let curves = facility.curve_dependencies()?;

        if curves.credit_curves.is_empty() {
            return Ok(0.0);
        }

        crate::metrics::GenericParallelCs01Hazard::<RevolvingCredit>::default().calculate(context)
    }
}

/// Revolving credit bucketed CS01 (hazard-rate bump) with graceful handling.
///
/// Delegates to [`GenericBucketedCs01Hazard`] when the facility references a
/// credit curve; otherwise returns 0.0.
pub(crate) struct RevolvingCreditBucketedCs01HazardCalculator;

impl MetricCalculator for RevolvingCreditBucketedCs01HazardCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let facility: &RevolvingCredit = context.instrument_as()?;
        let curves = facility.curve_dependencies()?;

        if curves.credit_curves.is_empty() {
            return Ok(0.0);
        }

        crate::metrics::GenericBucketedCs01Hazard::<RevolvingCredit>::default().calculate(context)
    }
}
