//! CDS-specific CS01 calculator.

use super::market_doc_clause;
use crate::instruments::common_impl::traits::CurveDependencies;
use crate::instruments::credit_derivatives::cds::CreditDefaultSwap;
use crate::metrics::sensitivities::config as sens_config;
use crate::metrics::sensitivities::cs01::compute_parallel_cs01_with_context_raw_and_doc_clause_and_valuation_convention;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::Result;
use std::sync::Arc;

/// CDS parallel CS01 calculator.
pub(crate) struct CdsCs01Calculator;

impl MetricCalculator for CdsCs01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let cds: &CreditDefaultSwap = context.instrument_as()?;

        let curves = cds.curve_dependencies()?;
        let hazard_id = curves.credit_curves.first().cloned().ok_or_else(|| {
            finstack_core::Error::Validation(format!(
                "Instrument {} has no credit curve dependencies for CS01 calculation",
                cds.id
            ))
        })?;
        let discount_id = curves.discount_curves.first().cloned();
        let bump_bp =
            sens_config::from_context_or_default(context.config(), context.get_metric_overrides())?
                .credit_spread_bump_bp;

        let inst_arc = Arc::clone(&context.instrument);
        let (model, registry) = context.clone_pricer_dispatch();
        let as_of = context.as_of;
        let reval = move |temp_ctx: &finstack_core::market_data::context::MarketContext| {
            if let (Some(model), Some(registry)) = (model, registry.as_ref()) {
                return registry
                    .price_raw(inst_arc.as_ref(), model, temp_ctx, as_of)
                    .map_err(Into::into);
            }
            inst_arc.value_raw(temp_ctx, as_of)
        };

        let cs01 = compute_parallel_cs01_with_context_raw_and_doc_clause_and_valuation_convention(
            context,
            &hazard_id,
            discount_id.as_ref(),
            bump_bp,
            Some(market_doc_clause(cds)),
            Some(cds.valuation_convention),
            reval,
        )?;
        context.computed.insert(
            MetricId::custom(format!("cs01::{}", hazard_id.as_str())),
            cs01,
        );
        Ok(cs01)
    }
}
