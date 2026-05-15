//! CDS-specific CS01 calculators.
//!
//! Implements the [canonical CS01 convention][canonical]: parallel 1 bp shock
//! to the par CDS curve, re-bootstrapped under the deal's doc clause and
//! valuation convention, with a symmetric (central) finite difference
//! `(PV(s + 1bp) − PV(s − 1bp)) / 2`. The bucketed variant applies the same
//! shock one tenor at a time and reports a per-bucket series.
//!
//! Sign convention (per canonical reference):
//! - Sell protection (long credit risk) → CS01 negative.
//! - Buy protection (short credit risk) → CS01 positive.
//!
//! [canonical]: crate::metrics::sensitivities::cs01

use super::{hazard_with_deal_quote, market_doc_clause};
use crate::instruments::common_impl::traits::CurveDependencies;
use crate::instruments::credit_derivatives::cds::CreditDefaultSwap;
use crate::metrics::sensitivities::config as sens_config;
use crate::metrics::sensitivities::cs01::{
    compute_key_rate_cs01_series_with_context_raw, compute_parallel_cs01_with_context_raw,
    KeyRateCs01Request,
};
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::Result;
use std::sync::Arc;

/// CDS parallel CS01 calculator.
pub(crate) struct CdsCs01Calculator;

impl MetricCalculator for CdsCs01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let cds: CreditDefaultSwap = context.instrument_as::<CreditDefaultSwap>()?.clone();

        let curves = cds.curve_dependencies()?;
        let hazard_id = curves.credit_curves.first().cloned().ok_or_else(|| {
            finstack_core::Error::Validation(format!(
                "Instrument {} has no credit curve dependencies for CS01 calculation",
                cds.id
            ))
        })?;
        let discount_id = curves.discount_curves.first().cloned();
        let original_curves = Arc::clone(&context.curves);
        let effective_curves = {
            let hazard = original_curves.get_hazard(hazard_id.as_str())?;
            hazard_with_deal_quote(&cds, hazard.as_ref())?
                .map(|quote_hazard| Arc::new(original_curves.as_ref().clone().insert(quote_hazard)))
        };
        if let Some(curves) = effective_curves {
            context.curves = curves;
        }
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

        let cs01_result =
            compute_parallel_cs01_with_context_raw(
                context,
                &hazard_id,
                discount_id.as_ref(),
                bump_bp,
                Some(market_doc_clause(&cds)),
                Some(cds.valuation_convention),
                reval,
            );
        context.curves = original_curves;
        let cs01 = cs01_result?;
        context.computed.insert(
            MetricId::custom(format!("cs01::{}", hazard_id.as_str())),
            cs01,
        );
        Ok(cs01)
    }
}

/// CDS bucketed CS01 calculator using the same CDS bootstrap convention as parallel CS01.
pub(crate) struct CdsBucketedCs01Calculator;

impl MetricCalculator for CdsBucketedCs01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let cds: CreditDefaultSwap = context.instrument_as::<CreditDefaultSwap>()?.clone();

        let curves = cds.curve_dependencies()?;
        let hazard_id = curves.credit_curves.first().cloned().ok_or_else(|| {
            finstack_core::Error::Validation(format!(
                "Instrument {} has no credit curve dependencies for bucketed CS01 calculation",
                cds.id
            ))
        })?;
        let discount_id = curves.discount_curves.first().cloned();
        let original_curves = Arc::clone(&context.curves);
        let effective_curves = {
            let hazard = original_curves.get_hazard(hazard_id.as_str())?;
            hazard_with_deal_quote(&cds, hazard.as_ref())?
                .map(|quote_hazard| Arc::new(original_curves.as_ref().clone().insert(quote_hazard)))
        };
        if let Some(curves) = effective_curves {
            context.curves = curves;
        }

        let defaults =
            sens_config::from_context_or_default(context.config(), context.get_metric_overrides())?;
        let buckets = defaults.cs01_buckets_years;
        let bump_bp = defaults.credit_spread_bump_bp;

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

        let series_id = MetricId::custom(format!("bucketed_cs01::{}", hazard_id.as_str()));
        let bucketed_result =
            compute_key_rate_cs01_series_with_context_raw(
                context,
                &hazard_id,
                discount_id.as_ref(),
                KeyRateCs01Request {
                    series_id,
                    bucket_times_years: buckets,
                    bump_bp,
                    doc_clause: Some(market_doc_clause(&cds)),
                    cds_valuation_convention: Some(cds.valuation_convention),
                },
                reval,
            );
        context.curves = original_curves;
        bucketed_result
    }
}
