//! CDS Option CS01 metric calculator.
//!
//! Implements the [canonical CS01 convention][canonical]: a parallel 1 bp shock
//! to the par CDS quotes underlying the option, re-bootstrapped under the
//! synthetic underlying CDS's doc clause and valuation convention, with a
//! symmetric (central) finite difference `(PV(s + 1bp) − PV(s − 1bp)) / 2`.
//!
//! Direct hazard-rate CS01 is intentionally **not** exposed for CDS options,
//! so callers cannot accidentally mix quote-spread and hazard-rate
//! conventions; CDS option CS01 always means "quote-spread risk".
//!
//! Sign convention (per canonical reference):
//! - Payer option / receiver of credit risk: signs follow the underlying
//!   delta — typically positive when long protection exposure, negative when
//!   short. Long bond / sell protection equivalents are negative; short bond
//!   / buy protection equivalents are positive.
//!
//! [canonical]: crate::metrics::sensitivities::cs01

use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::credit_derivatives::cds::metrics::market_doc_clause;
use crate::instruments::credit_derivatives::cds_option::pricer::synthetic_underlying_cds;
use crate::instruments::credit_derivatives::cds_option::CDSOption;
use crate::metrics::sensitivities::config as sens_config;
use crate::metrics::sensitivities::cs01::compute_parallel_cs01_with_context_raw_and_doc_clause_and_valuation_convention;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Quoted-spread CS01 calculator for CDS Option instruments.
pub(crate) struct Cs01Calculator;

impl MetricCalculator for Cs01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let cds_option: CDSOption = context.instrument_as::<CDSOption>()?.clone();
        let as_of = context.as_of;

        if as_of >= cds_option.expiry {
            tracing::debug!(
                instrument_id = %cds_option.id,
                as_of = %as_of,
                expiry = %cds_option.expiry,
                "CDS Option CS01: Instrument already expired, returning 0.0"
            );
            return Ok(0.0);
        }

        let hazard = context.curves.get_hazard(&cds_option.credit_curve_id)?;
        if hazard.par_spread_points().next().is_none() {
            return Err(finstack_core::Error::Calibration {
                message: format!(
                    "CDS option '{}' CS01 requires CDS quote/par-spread points on hazard curve '{}'",
                    cds_option.id,
                    cds_option.credit_curve_id.as_str()
                ),
                category: "cs01_quote_bump".to_string(),
            });
        }

        let synthetic = synthetic_underlying_cds(&cds_option, as_of)?;
        let defaults =
            sens_config::from_context_or_default(context.config(), context.get_metric_overrides())?;
        let bump_bp = defaults.credit_spread_bump_bp;

        compute_parallel_cs01_with_context_raw_and_doc_clause_and_valuation_convention(
            context,
            &cds_option.credit_curve_id,
            Some(&cds_option.discount_curve_id),
            bump_bp,
            Some(market_doc_clause(&synthetic)),
            Some(synthetic.valuation_convention),
            |bumped_market| cds_option.value(bumped_market, as_of).map(|pv| pv.amount()),
        )
    }
}
