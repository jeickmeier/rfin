//! CDS Tranche metrics using Gaussian Copula model.
//!
//! Implements industry-standard metrics for CDS tranches using the base
//! correlation approach with a one-factor Gaussian Copula model.

use crate::instruments::fixed_income::cds_tranche::{model::GaussianCopulaModel, CdsTranche};
use crate::market_data::ValuationMarketContext;
use crate::metrics::{MetricCalculator, MetricContext, MetricId, MetricRegistry};
use finstack_core::F;

/// Upfront payment metric using Gaussian Copula model.
///
/// Represents the net present value of the tranche at inception,
/// which is the payment required to enter the position.
pub struct Upfront;

impl MetricCalculator for Upfront {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let tranche: &CdsTranche = context.instrument_as()?;

        // Convert MarketContext to ValuationMarketContext
        let val_market_ctx = ValuationMarketContext::from_core(context.curves.as_ref().clone());

        // Check if credit index data is available
        if val_market_ctx.has_credit_index(tranche.credit_index_id) {
            let model = GaussianCopulaModel::new();
            model.calculate_upfront(tranche, &val_market_ctx, context.as_of)
        } else {
            // Fallback when credit index data is not available
            Ok(0.0)
        }
    }
}

/// Spread DV01 (premium leg PV change for 1bp change in running coupon).
///
/// Measures the sensitivity of the tranche's value to a 1 basis point
/// change in the running coupon rate.
pub struct SpreadDv01;

impl MetricCalculator for SpreadDv01 {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let tranche: &CdsTranche = context.instrument_as()?;

        // Convert MarketContext to ValuationMarketContext
        let val_market_ctx = ValuationMarketContext::from_core(context.curves.as_ref().clone());

        // Check if credit index data is available
        if val_market_ctx.has_credit_index(tranche.credit_index_id) {
            let model = GaussianCopulaModel::new();
            model.calculate_spread_dv01(tranche, &val_market_ctx, context.as_of)
        } else {
            // Fallback when credit index data is not available
            Ok(0.0)
        }
    }
}

/// Expected loss of the tranche using Gaussian Copula model.
///
/// Calculates the total expected loss on the tranche at maturity
/// based on the portfolio loss distribution.
pub struct ExpectedLoss;

impl MetricCalculator for ExpectedLoss {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let tranche: &CdsTranche = context.instrument_as()?;

        // Convert MarketContext to ValuationMarketContext
        let val_market_ctx = ValuationMarketContext::from_core(context.curves.as_ref().clone());

        // Check if credit index data is available
        if val_market_ctx.has_credit_index(tranche.credit_index_id) {
            let model = GaussianCopulaModel::new();
            model.calculate_expected_loss(tranche, &val_market_ctx)
        } else {
            // Fallback when credit index data is not available
            Ok(0.0)
        }
    }
}

/// Jump-to-default (instantaneous loss sensitivity).
///
/// Measures the immediate impact on tranche value if a specific
/// entity in the portfolio defaults instantly. This is equivalent
/// to the correlation delta in many contexts.
pub struct JumpToDefault;

impl MetricCalculator for JumpToDefault {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let tranche: &CdsTranche = context.instrument_as()?;

        // Convert MarketContext to ValuationMarketContext
        let val_market_ctx = ValuationMarketContext::from_core(context.curves.as_ref().clone());

        // Check if credit index data is available
        if val_market_ctx.has_credit_index(tranche.credit_index_id) {
            let model = GaussianCopulaModel::new();
            model.calculate_jump_to_default(tranche, &val_market_ctx, context.as_of)
        } else {
            // Fallback when credit index data is not available
            Ok(0.0)
        }
    }
}

/// CS01 (Credit Spread 01) - sensitivity to 1bp parallel shift in credit spreads.
///
/// Measures how the tranche value changes when all underlying credit
/// spreads move by 1 basis point in parallel.
pub struct Cs01;

impl MetricCalculator for Cs01 {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let tranche: &CdsTranche = context.instrument_as()?;

        // Convert MarketContext to ValuationMarketContext
        let val_market_ctx = ValuationMarketContext::from_core(context.curves.as_ref().clone());

        // Check if credit index data is available
        if val_market_ctx.has_credit_index(tranche.credit_index_id) {
            let model = GaussianCopulaModel::new();
            model.calculate_cs01(tranche, &val_market_ctx, context.as_of)
        } else {
            // Fallback when credit index data is not available
            Ok(0.0)
        }
    }
}

/// Correlation Delta - sensitivity to changes in asset correlation.
///
/// Measures how the tranche value changes when the base correlation
/// curve shifts, capturing the correlation risk of the position.
pub struct CorrelationDelta;

impl MetricCalculator for CorrelationDelta {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let tranche: &CdsTranche = context.instrument_as()?;

        // Convert MarketContext to ValuationMarketContext
        let val_market_ctx = ValuationMarketContext::from_core(context.curves.as_ref().clone());

        // Check if credit index data is available
        if val_market_ctx.has_credit_index(tranche.credit_index_id) {
            let model = GaussianCopulaModel::new();
            model.calculate_correlation_delta(tranche, &val_market_ctx, context.as_of)
        } else {
            // Fallback when credit index data is not available
            Ok(0.0)
        }
    }
}

/// Registers CDS Tranche metrics using Gaussian Copula model
pub fn register_cds_tranche_metrics(registry: &mut MetricRegistry) {
    use std::sync::Arc;

    registry
        .register_metric(
            MetricId::custom("upfront"),
            Arc::new(Upfront),
            &["CDSTranche"],
        )
        .register_metric(
            MetricId::custom("spread_dv01"),
            Arc::new(SpreadDv01),
            &["CDSTranche"],
        )
        .register_metric(
            MetricId::ExpectedLoss,
            Arc::new(ExpectedLoss),
            &["CDSTranche"],
        )
        .register_metric(
            MetricId::JumpToDefault,
            Arc::new(JumpToDefault),
            &["CDSTranche"],
        )
        .register_metric(MetricId::custom("cs01"), Arc::new(Cs01), &["CDSTranche"])
        .register_metric(
            MetricId::custom("correlation_delta"),
            Arc::new(CorrelationDelta),
            &["CDSTranche"],
        );
}
