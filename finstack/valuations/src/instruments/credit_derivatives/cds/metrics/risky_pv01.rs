//! CDS risky PV01 metric calculator.
//!
//! Computes the change in present value for a one basis point change in
//! spread.

use super::cs01::CdsCs01Calculator;
use crate::instruments::credit_derivatives::cds::pricer::CDSPricer;
use crate::instruments::credit_derivatives::cds::CreditDefaultSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Risky PV01 calculator for CDS
pub(crate) struct RiskyPv01Calculator;

impl MetricCalculator for RiskyPv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let use_deal_quote = {
            let cds: &CreditDefaultSwap = context.instrument_as()?;
            cds.uses_clean_price() && cds.pricing_overrides.market_quotes.cds_quote_bp.is_some()
        };
        if use_deal_quote {
            return CdsCs01Calculator.calculate(context);
        }

        let cds: &CreditDefaultSwap = context.instrument_as()?;
        let disc = context
            .curves
            .get_discount(&cds.premium.discount_curve_id)?;
        let surv = context.curves.get_hazard(&cds.protection.credit_curve_id)?;
        let pricer = CDSPricer::new();
        if cds.uses_full_premium_par_spread_denominator() {
            return pricer
                .premium_leg_pv_per_bp(cds, disc.as_ref(), surv.as_ref(), context.as_of)
                .map(|pv_per_bp| pv_per_bp * cds.notional.amount());
        }
        pricer.risky_pv01(cds, disc.as_ref(), surv.as_ref(), context.as_of)
    }
}
