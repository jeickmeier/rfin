//! CDS tranche–specific CS01 calculators.
//!
//! The generic CS01 calculator assumes `CurveDependencies::credit_curves`
//! contains direct hazard curve IDs.  For CDS tranches, however, the credit
//! dependency is a **credit index ID** (e.g. `"CDX.NA.IG.HAZARD"`), and the
//! actual hazard curve sits inside the `CreditIndexData` under a different ID
//! (e.g. `"CDX-HAZ"`).
//!
//! These calculators resolve the index → hazard mapping before delegating to
//! the shared CS01 bump helpers. This applies to both par-spread and
//! direct hazard-rate bump variants.

use crate::calibration::bumps::hazard::bump_hazard_shift;
use crate::calibration::bumps::BumpRequest;
use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::credit_derivatives::cds_tranche::CDSTranche;
use crate::metrics::sensitivities::config as sens_config;
use crate::metrics::sensitivities::cs01::sensitivity_central_diff;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::types::CurveId;
use std::sync::Arc;

/// Resolve the hazard curve ID and discount curve ID for a CDS tranche.
///
/// Fetches the `CreditIndexData` from the market context, extracts the
/// actual hazard curve ID from `index_credit_curve`, and returns it
/// alongside the discount curve ID.
fn resolve_tranche_cs01_curves(
    tranche: &CDSTranche,
    market: &finstack_core::market_data::context::MarketContext,
) -> finstack_core::Result<(CurveId, Option<CurveId>)> {
    let index_data = market
        .get_credit_index(&tranche.credit_index_id)
        .map_err(|_| {
            finstack_core::Error::Validation(format!(
                "Credit index '{}' not found for tranche '{}' CS01 calculation",
                tranche.credit_index_id,
                tranche.id()
            ))
        })?;

    let hazard_id = CurveId::from(index_data.index_credit_curve.id().as_str());
    let discount_id = Some(tranche.discount_curve_id.clone());
    Ok((hazard_id, discount_id))
}

/// CDS tranche parallel CS01 that resolves the credit index → hazard curve mapping.
pub(crate) struct CdsTrancheCs01Calculator;

impl MetricCalculator for CdsTrancheCs01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let tranche: &CDSTranche = context.instrument_as()?;
        let (hazard_id, discount_id) =
            resolve_tranche_cs01_curves(tranche, context.curves.as_ref())?;

        let bump_bp = sens_config::from_context_or_default(
            context.config(),
            context.metric_overrides.as_ref(),
        )?
        .credit_spread_bump_bp;

        let inst_arc = Arc::clone(&context.instrument);
        let model = context.pricing_model;
        let registry = context.pricer_registry.clone();
        let as_of = context.as_of;

        let reval = move |temp_ctx: &finstack_core::market_data::context::MarketContext| {
            if let (Some(model), Some(registry)) = (model, registry.as_ref()) {
                return registry
                    .price_raw(inst_arc.as_ref(), model, temp_ctx, as_of)
                    .map_err(Into::into);
            }
            inst_arc.value_raw(temp_ctx, as_of)
        };

        let cs01 = crate::metrics::sensitivities::cs01::compute_parallel_cs01_with_context_raw(
            context,
            &hazard_id,
            discount_id.as_ref(),
            bump_bp,
            reval,
        )?;

        context.computed.insert(
            MetricId::custom(format!("cs01::{}", hazard_id.as_str())),
            cs01,
        );

        Ok(cs01)
    }
}

/// CDS tranche bucketed CS01 that resolves the credit index → hazard curve mapping.
pub(crate) struct CdsTrancheBucketedCs01Calculator;

impl MetricCalculator for CdsTrancheBucketedCs01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let tranche: &CDSTranche = context.instrument_as()?;
        let (hazard_id, discount_id) =
            resolve_tranche_cs01_curves(tranche, context.curves.as_ref())?;

        let defaults = sens_config::from_context_or_default(
            context.config(),
            context.metric_overrides.as_ref(),
        )?;
        let buckets = defaults.cs01_buckets_years;
        let bump_bp = defaults.credit_spread_bump_bp;

        let inst_arc = Arc::clone(&context.instrument);
        let model = context.pricing_model;
        let registry = context.pricer_registry.clone();
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

        crate::metrics::sensitivities::cs01::compute_key_rate_cs01_series_with_context_raw(
            context,
            &hazard_id,
            discount_id.as_ref(),
            series_id,
            buckets,
            bump_bp,
            reval,
        )
    }
}

/// CDS tranche parallel CS01 (direct hazard-rate bump) with index → hazard resolution.
pub(crate) struct CdsTrancheCs01HazardCalculator;

impl MetricCalculator for CdsTrancheCs01HazardCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let tranche: &CDSTranche = context.instrument_as()?;
        let (hazard_id, _discount_id) =
            resolve_tranche_cs01_curves(tranche, context.curves.as_ref())?;

        let bump_bp = sens_config::from_context_or_default(
            context.config(),
            context.metric_overrides.as_ref(),
        )?
        .credit_spread_bump_bp;

        let base_ctx = context.curves.as_ref();
        let hazard = base_ctx.get_hazard(hazard_id.as_str())?;
        let hazard_ref = hazard.as_ref();

        let as_of = context.as_of;

        let bumped_up = bump_hazard_shift(hazard_ref, &BumpRequest::Parallel(bump_bp))?;
        let bumped_down = bump_hazard_shift(hazard_ref, &BumpRequest::Parallel(-bump_bp))?;

        let pv_up = context.reprice_raw(&base_ctx.clone().insert(bumped_up), as_of)?;
        let pv_down = context.reprice_raw(&base_ctx.clone().insert(bumped_down), as_of)?;

        let cs01 = sensitivity_central_diff(pv_up, pv_down, bump_bp);

        context.computed.insert(
            MetricId::custom(format!("cs01_hazard::{}", hazard_id.as_str())),
            cs01,
        );

        Ok(cs01)
    }
}

/// CDS tranche bucketed CS01 (direct hazard-rate bump) with index → hazard resolution.
pub(crate) struct CdsTrancheBucketedCs01HazardCalculator;

impl MetricCalculator for CdsTrancheBucketedCs01HazardCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let tranche: &CDSTranche = context.instrument_as()?;
        let (hazard_id, _discount_id) =
            resolve_tranche_cs01_curves(tranche, context.curves.as_ref())?;

        let defaults = sens_config::from_context_or_default(
            context.config(),
            context.metric_overrides.as_ref(),
        )?;
        let buckets = defaults.cs01_buckets_years;
        let bump_bp = defaults.credit_spread_bump_bp;

        let base_ctx = context.curves.as_ref();
        let hazard = base_ctx.get_hazard(hazard_id.as_str())?;
        let hazard_ref = hazard.as_ref();

        let as_of = context.as_of;

        let mut series: Vec<(std::borrow::Cow<'static, str>, f64)> = Vec::new();
        let mut total = 0.0;

        for t in buckets {
            let label = sens_config::format_bucket_label_cow(t);

            let bumped_up =
                bump_hazard_shift(hazard_ref, &BumpRequest::Tenors(vec![(t, bump_bp)]))?;
            let bumped_down =
                bump_hazard_shift(hazard_ref, &BumpRequest::Tenors(vec![(t, -bump_bp)]))?;

            let pv_up = context.reprice_raw(&base_ctx.clone().insert(bumped_up), as_of)?;
            let pv_down = context.reprice_raw(&base_ctx.clone().insert(bumped_down), as_of)?;

            let cs01 = sensitivity_central_diff(pv_up, pv_down, bump_bp);
            series.push((label, cs01));
            total += cs01;
        }

        let series_id = MetricId::custom(format!("bucketed_cs01_hazard::{}", hazard_id.as_str()));
        context.store_bucketed_series(series_id, series);

        Ok(total)
    }
}
