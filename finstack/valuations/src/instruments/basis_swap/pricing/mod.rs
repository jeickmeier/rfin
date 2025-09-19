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

/// Calculates the present value of a single leg of a basis swap.
///
/// # Arguments
/// * `swap` — The basis swap containing the leg
/// * `leg` — The specific leg to price
/// * `context` — Market context with curves and rates
/// * `as_of` — Valuation date
///
/// # Returns
/// The present value of the leg as a `Money` amount.
fn pv_leg(swap: &BasisSwap, leg: &BasisSwapLeg, context: &MarketContext, as_of: Date) -> Result<Money> {
    let schedule = swap.leg_schedule(leg);
    let params = FloatLegParams {
        schedule: &schedule,
        notional: swap.notional,
        disc_id: swap.discount_curve_id.as_str(),
        fwd_id: leg.forward_curve_id.as_str(),
        accrual_dc: leg.day_count,
        spread: leg.spread,
    };
    BasisEngine::pv_float_leg(params, context, as_of)
}

impl Priceable for BasisSwap {
    /// Calculates the present value of the basis swap.
    ///
    /// The value is computed as the difference between the primary leg PV
    /// (which typically receives a spread) and the reference leg PV.
    ///
    /// # Arguments
    /// * `context` — Market context containing curves and rates
    /// * `valuation_date` — Date for present value calculation
    ///
    /// # Returns
    /// The net present value of the basis swap.
    fn value(&self, context: &MarketContext, valuation_date: Date) -> Result<Money> {
        let primary_pv = pv_leg(self, &self.primary_leg, context, valuation_date)?;
        let reference_pv = pv_leg(self, &self.reference_leg, context, valuation_date)?;
        Ok(Money::new(
            primary_pv.amount() - reference_pv.amount(),
            primary_pv.currency(),
        ))
    }

    /// Calculates the present value with additional metrics.
    ///
    /// # Arguments
    /// * `context` — Market context containing curves and rates
    /// * `as_of` — Valuation date
    /// * `metrics` — List of metrics to calculate
    ///
    /// # Returns
    /// A `ValuationResult` containing the base value and requested metrics.
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


