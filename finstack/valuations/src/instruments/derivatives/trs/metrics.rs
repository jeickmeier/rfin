//! Risk metrics for Total Return Swaps.

use super::{EquityTotalReturnSwap, FIIndexTotalReturnSwap, TrsEngine};
use crate::metrics::{MetricCalculator, MetricContext, MetricId, MetricRegistry};
use finstack_core::{Error, Result, F};

/// Calculates the par spread for a TRS (spread that makes NPV = 0)
pub struct ParSpreadCalculator;

impl MetricCalculator for ParSpreadCalculator {
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::FinancingAnnuity]
    }

    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        // Get the financing annuity
        // Calculate financing annuity first
        let annuity_calc = FinancingAnnuityCalculator;
        let annuity = annuity_calc.calculate(context)?;

        if annuity.abs() < 1e-10 {
            return Err(Error::Validation(
                "Financing annuity too small for par spread calculation".into(),
            ));
        }

        // Calculate PV of total return leg with zero spread
        let tr_pv = if let Some(equity_trs) = context
            .instrument
            .as_any()
            .downcast_ref::<EquityTotalReturnSwap>()
        {
            equity_trs.pv_total_return_leg(context.curves.as_ref(), context.as_of)?
        } else if let Some(fi_trs) = context
            .instrument
            .as_any()
            .downcast_ref::<FIIndexTotalReturnSwap>()
        {
            fi_trs.pv_total_return_leg(context.curves.as_ref(), context.as_of)?
        } else {
            return Err(Error::Input(finstack_core::error::InputError::Invalid));
        };

        // Par spread in basis points = TR PV / Annuity * 10000
        let par_spread_bp = tr_pv.amount() / annuity * 10000.0;

        Ok(par_spread_bp)
    }
}

/// Calculates the financing annuity (sum of discounted year fractions)
pub struct FinancingAnnuityCalculator;

impl MetricCalculator for FinancingAnnuityCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        if let Some(equity_trs) = context
            .instrument
            .as_any()
            .downcast_ref::<EquityTotalReturnSwap>()
        {
            TrsEngine::financing_annuity(
                &equity_trs.financing,
                &equity_trs.schedule,
                equity_trs.notional,
                context.curves.as_ref(),
                context.as_of,
            )
        } else if let Some(fi_trs) = context
            .instrument
            .as_any()
            .downcast_ref::<FIIndexTotalReturnSwap>()
        {
            TrsEngine::financing_annuity(
                &fi_trs.financing,
                &fi_trs.schedule,
                fi_trs.notional,
                context.curves.as_ref(),
                context.as_of,
            )
        } else {
            Err(Error::Input(finstack_core::error::InputError::Invalid))
        }
    }
}

/// Calculates IR01 (PV change for 1bp parallel shift in rates)
pub struct TrsIR01Calculator;

impl MetricCalculator for TrsIR01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        // Get base NPV
        let _base_npv = context
            .instrument
            .value(context.curves.as_ref(), context.as_of)?;

        // Create a bumped market context (1bp up)
        let bump_size = 0.0001; // 1bp

        // For simplicity, we'll approximate by bumping the discount curve
        // In production, we'd properly bump all relevant curves
        let _bumped_context = context.curves.clone();

        // Note: This is simplified - real implementation would bump the curves properly
        // For now, we'll use finite difference approximation

        // Calculate financing leg sensitivity
        let financing_sensitivity = if let Some(equity_trs) = context
            .instrument
            .as_any()
            .downcast_ref::<EquityTotalReturnSwap>(
        ) {
            // Approximate: duration of financing leg * notional * 1bp
            let annuity = TrsEngine::financing_annuity(
                &equity_trs.financing,
                &equity_trs.schedule,
                equity_trs.notional,
                context.curves.as_ref(),
                context.as_of,
            )?;
            annuity * bump_size
        } else if let Some(fi_trs) = context
            .instrument
            .as_any()
            .downcast_ref::<FIIndexTotalReturnSwap>()
        {
            let annuity = TrsEngine::financing_annuity(
                &fi_trs.financing,
                &fi_trs.schedule,
                fi_trs.notional,
                context.curves.as_ref(),
                context.as_of,
            )?;
            annuity * bump_size
        } else {
            return Err(Error::Input(finstack_core::error::InputError::Invalid));
        };

        Ok(financing_sensitivity)
    }
}

/// Calculates delta to the underlying index level
pub struct IndexDeltaCalculator;

impl MetricCalculator for IndexDeltaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        if let Some(equity_trs) = context
            .instrument
            .as_any()
            .downcast_ref::<EquityTotalReturnSwap>()
        {
            // For equity TRS, delta is approximately notional / spot
            let spot = match context.curves.price(&equity_trs.underlying.spot_id)? {
                finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
                finstack_core::market_data::scalars::MarketScalar::Price(p) => p.amount(),
            };

            if spot.abs() < 1e-10 {
                return Err(Error::Validation(
                    "Spot price too small for delta calculation".into(),
                ));
            }

            // Delta = notional * contract_size / spot
            let delta = equity_trs.notional.amount() * equity_trs.underlying.contract_size / spot;

            // Adjust for trade side
            let signed_delta = match equity_trs.side {
                super::TrsSide::ReceiveTotalReturn => delta,
                super::TrsSide::PayTotalReturn => -delta,
            };

            Ok(signed_delta)
        } else if let Some(fi_trs) = context
            .instrument
            .as_any()
            .downcast_ref::<FIIndexTotalReturnSwap>()
        {
            // For FI index TRS, use duration if available
            let duration = fi_trs
                .underlying
                .duration_id
                .as_ref()
                .and_then(|id| {
                    context.curves.price(id.as_str()).ok().map(|s| match s {
                        finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
                        finstack_core::market_data::scalars::MarketScalar::Price(p) => {
                            p.amount()
                        }
                    })
                })
                .unwrap_or(5.0); // Default duration assumption

            // Delta ≈ notional * duration * 0.0001 (for 1bp move)
            let delta = fi_trs.notional.amount() * duration * 0.0001;

            // Adjust for trade side
            let signed_delta = match fi_trs.side {
                super::TrsSide::ReceiveTotalReturn => delta,
                super::TrsSide::PayTotalReturn => -delta,
            };

            Ok(signed_delta)
        } else {
            Err(Error::Input(finstack_core::error::InputError::Invalid))
        }
    }
}

/// Register TRS metrics with the metric registry
pub fn register_trs_metrics(registry: &mut MetricRegistry) {
    use std::sync::Arc;

    registry.register_metric(
        MetricId::ParSpread,
        Arc::new(ParSpreadCalculator),
        &["EquityTotalReturnSwap", "FIIndexTotalReturnSwap"],
    );
    registry.register_metric(
        MetricId::FinancingAnnuity,
        Arc::new(FinancingAnnuityCalculator),
        &["EquityTotalReturnSwap", "FIIndexTotalReturnSwap"],
    );
    registry.register_metric(
        MetricId::Ir01,
        Arc::new(TrsIR01Calculator),
        &["EquityTotalReturnSwap", "FIIndexTotalReturnSwap"],
    );
    registry.register_metric(
        MetricId::IndexDelta,
        Arc::new(IndexDeltaCalculator),
        &["EquityTotalReturnSwap", "FIIndexTotalReturnSwap"],
    );
}
