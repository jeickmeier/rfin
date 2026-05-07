//! CDS Index CS01 metric calculators.
//!
//! - `Cs01Calculator`: parallel CS01 from per-name finite-difference, summed
//!   over surviving constituents (or computed on the synthetic CDS in
//!   `SingleCurve` mode). Routed through `CDSIndexPricer::cs01`.
//! - `Cs01HazardCalculator`: parallel hazard-shift CS01 that correctly bumps
//!   ALL credit curves used by the index and reprices. Replaces the generic
//!   `GenericParallelCs01Hazard`, which would only bump the (unused) index-
//!   level curve in `Constituents` mode.

use crate::calibration::bumps::hazard::bump_hazard_shift;
use crate::calibration::bumps::BumpRequest;
use crate::instruments::common_impl::traits::CurveDependencies;
use crate::instruments::credit_derivatives::cds_index::{CDSIndex, IndexPricing};
use crate::metrics::sensitivities::config as sens_config;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::market_data::context::MarketContext;
use finstack_core::Result;

/// Parallel CS01 calculator for CDS Index (per-name finite difference).
pub(crate) struct Cs01Calculator;

impl MetricCalculator for Cs01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let idx: &CDSIndex = context.instrument_as()?;
        idx.cs01(&context.curves, context.as_of)
    }
}

/// Parallel hazard-shift CS01 for CDS Index.
///
/// Bumps every credit curve declared as a dependency by the instrument
/// (in `Constituents` mode this is N hazard curves, one per surviving name),
/// reprices, and computes a central difference. This is correct for
/// `IndexPricing::Constituents` where the generic single-curve form would
/// only bump the unused index-level curve.
pub(crate) struct Cs01HazardCalculator;

impl MetricCalculator for Cs01HazardCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let index: &CDSIndex = context.instrument_as()?;

        let bump_bp =
            sens_config::from_context_or_default(context.config(), context.get_metric_overrides())?
                .credit_spread_bump_bp;

        // Determine which credit curves to bump. In SingleCurve mode this is
        // just the index-level curve; in Constituents mode it's the union
        // of surviving constituent curves.
        let credit_ids: Vec<_> = match index.pricing {
            IndexPricing::SingleCurve => {
                vec![index.protection.credit_curve_id.clone()]
            }
            IndexPricing::Constituents => {
                // Pull from curve_dependencies but skip the index-level curve
                // because it is informational only in Constituents mode.
                let curves = index.curve_dependencies()?;
                curves
                    .credit_curves
                    .into_iter()
                    .filter(|id| id != &index.protection.credit_curve_id)
                    .collect()
            }
        };

        if credit_ids.is_empty() {
            return Ok(0.0);
        }

        let bump_all = |ctx: &MarketContext, bp: f64| -> Result<MarketContext> {
            let mut out = ctx.clone();
            for id in &credit_ids {
                let hazard = ctx.get_hazard(id.as_str())?;
                let bumped = bump_hazard_shift(hazard.as_ref(), &BumpRequest::Parallel(bp))?;
                out = out.insert(bumped);
            }
            Ok(out)
        };

        let base_ctx = context.curves.as_ref();
        let ctx_up = bump_all(base_ctx, bump_bp)?;
        let ctx_down = bump_all(base_ctx, -bump_bp)?;

        let as_of = context.as_of;
        let pv_up = context.reprice_raw(&ctx_up, as_of)?;
        let pv_down = context.reprice_raw(&ctx_down, as_of)?;

        Ok((pv_up - pv_down) / (2.0 * bump_bp))
    }
}
