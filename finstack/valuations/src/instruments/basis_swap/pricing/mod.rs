//! Basis swap pricing entrypoints and pricers.

pub mod engine;

use crate::instruments::helpers::build_with_metrics_dyn;
use crate::instruments::traits::Priceable;
use crate::instruments::basis_swap::types::{BasisSwap, BasisSwapLeg};
use crate::metrics::MetricId;
use crate::results::ValuationResult;
use finstack_core::dates::Date;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;

use engine::{BasisEngine, FloatLegParams};

fn pv_leg(swap: &BasisSwap, leg: &BasisSwapLeg, context: &MarketContext, as_of: Date) -> Result<Money> {
    let schedule = swap.leg_schedule(leg);
    let params = FloatLegParams {
        schedule: &schedule,
        notional: swap.notional,
        disc_id: swap.discount_curve_id.as_str(),
        fwd_id: leg.forward_curve_id.as_str(),
        accrual_dc: leg.day_count,
        spread: leg.spread,
        base_date: swap.start_date,
    };
    BasisEngine::pv_float_leg(params, context, as_of)
}

impl Priceable for BasisSwap {
    fn value(&self, context: &MarketContext, valuation_date: Date) -> Result<Money> {
        let primary_pv = pv_leg(self, &self.primary_leg, context, valuation_date)?;
        let reference_pv = pv_leg(self, &self.reference_leg, context, valuation_date)?;
        Ok(Money::new(
            primary_pv.amount() - reference_pv.amount(),
            primary_pv.currency(),
        ))
    }

    fn price_with_metrics(
        &self,
        context: &MarketContext,
        as_of: Date,
        metrics: &[MetricId],
    ) -> Result<ValuationResult> {
        let base = <Self as Priceable>::value(self, context, as_of)?;
        build_with_metrics_dyn(self, context, as_of, base, metrics)
    }
}


