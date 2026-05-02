//! CDS-specific DV01 calculator.
//!
//! CDS rate risk is a cross-curve sensitivity when the credit curve is stored
//! with the market par spreads used to build it: after a rate-curve bump, the
//! hazard curve must be re-bootstrapped from unchanged CDS spreads. This matches
//! Bloomberg-style IR DV01 for CDS screens.

use super::market_doc_clause;
use crate::calibration::bumps::hazard::bump_hazard_spreads_with_doc_clause;
use crate::calibration::bumps::BumpRequest;
use crate::instruments::credit_derivatives::cds::CreditDefaultSwap;
use crate::metrics::sensitivities::config as sens_config;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::market_data::bumps::BumpSpec;
use finstack_core::market_data::context::MarketContext;
use finstack_core::Result;

const MIN_BUMP_BP: f64 = 1e-10;

/// CDS DV01 calculator with par-spread hazard re-bootstrap when possible.
pub(crate) struct CdsDv01Calculator;

impl CdsDv01Calculator {
    fn price_at_rate_bump(
        cds: &CreditDefaultSwap,
        context: &MetricContext,
        bump_bp: f64,
        rebootstrap_hazard: bool,
    ) -> Result<f64> {
        let mut bumped_market: MarketContext = context.curves.as_ref().clone();
        bumped_market.apply_curve_bump_in_place(
            &cds.premium.discount_curve_id,
            BumpSpec::parallel_bp(bump_bp),
        )?;

        if rebootstrap_hazard {
            let base_hazard = context
                .curves
                .get_hazard(cds.protection.credit_curve_id.as_str())?;
            let recalibrated = bump_hazard_spreads_with_doc_clause(
                base_hazard.as_ref(),
                &bumped_market,
                &BumpRequest::Parallel(0.0),
                Some(&cds.premium.discount_curve_id),
                Some(market_doc_clause(cds)),
            )?;
            bumped_market = bumped_market.insert(recalibrated);
        }

        context.reprice_raw(&bumped_market, context.as_of)
    }
}

impl MetricCalculator for CdsDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let cds: &CreditDefaultSwap = context.instrument_as()?;
        let defaults =
            sens_config::from_context_or_default(context.config(), context.get_metric_overrides())?;
        let bump_bp = defaults.rate_bump_bp;
        if bump_bp.abs() <= MIN_BUMP_BP {
            return Ok(0.0);
        }

        let hazard = context
            .curves
            .get_hazard(cds.protection.credit_curve_id.as_str())?;
        let rebootstrap_hazard = hazard.par_spread_points().next().is_some();

        let pv_up = Self::price_at_rate_bump(cds, context, bump_bp, rebootstrap_hazard)?;
        let pv_down = Self::price_at_rate_bump(cds, context, -bump_bp, rebootstrap_hazard)?;

        Ok((pv_up - pv_down) / (2.0 * bump_bp))
    }
}
