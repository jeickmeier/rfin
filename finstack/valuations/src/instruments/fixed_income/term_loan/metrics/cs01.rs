//! Term-loan-specific CS01 calculators with graceful handling for missing
//! credit curves.
//!
//! When a term loan has an associated credit (hazard) curve, CS01 follows the
//! [canonical CS01 convention][canonical] — a parallel 1 bp shock to par CDS
//! spreads with a symmetric (central) finite difference — by delegating to
//! [`GenericParallelCs01`] / [`GenericBucketedCs01`].
//!
//! When no credit curve is configured, CS01 is reported as `0.0`: the loan
//! has no credit-model dependency to be sensitive to.
//!
//! Sign convention is identical to the canonical reference:
//! - Long term loan → CS01 negative.
//! - Short term loan → CS01 positive.
//!
//! [canonical]: crate::metrics::sensitivities::cs01

use crate::instruments::common_impl::traits::CurveDependencies;
use crate::instruments::TermLoan;
use crate::metrics::{MetricCalculator, MetricContext};

/// Term loan parallel CS01 with graceful no-credit-curve handling.
///
/// Delegates to [`GenericParallelCs01`] when the loan references a credit
/// curve; otherwise returns 0.0.
pub(crate) struct TermLoanCs01Calculator;

impl MetricCalculator for TermLoanCs01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let loan: &TermLoan = context.instrument_as()?;
        let curves = loan.curve_dependencies()?;

        if curves.credit_curves.is_empty() {
            return Ok(0.0);
        }

        crate::metrics::GenericParallelCs01::<TermLoan>::default().calculate(context)
    }
}

/// Term loan bucketed CS01 with graceful no-credit-curve handling.
///
/// Delegates to [`GenericBucketedCs01`] when the loan references a credit
/// curve; otherwise returns 0.0.
pub(crate) struct TermLoanBucketedCs01Calculator;

impl MetricCalculator for TermLoanBucketedCs01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let loan: &TermLoan = context.instrument_as()?;
        let curves = loan.curve_dependencies()?;

        if curves.credit_curves.is_empty() {
            return Ok(0.0);
        }

        crate::metrics::GenericBucketedCs01::<TermLoan>::default().calculate(context)
    }
}
