//! Reusable helpers for bucketed CS01 (credit spread sensitivity) using key-rate bumps.
//!
//! Provides generic functions to compute bucketed CS01 for instruments that
//! depend on hazard curves. Results are stored into `MetricContext` via structured
//! series using stable composite keys.
//!
//! # Units and Sign Convention
//!
//! - **CS01 is expressed in currency units per basis point (1bp = 0.0001)**
//! - A CS01 of -50 means the instrument loses $50 when credit spreads widen by 1bp
//! - For protection buyers (long CDS): CS01 is typically positive (gain when spreads widen)
//! - For protection sellers (short CDS): CS01 is typically negative (lose when spreads widen)
//! - For corporate bonds: CS01 is typically negative (lose value when spreads widen)

use crate::calibration::bumps::hazard::{bump_hazard_shift, bump_hazard_spreads};
use crate::calibration::bumps::BumpRequest;
use crate::metrics::sensitivities::config as sens_config;
use crate::metrics::{MetricContext, MetricId};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::CurveId;
use std::sync::Arc;

/// Standard credit key-rate buckets in years used for CS01.
///
/// Returns the industry-standard credit spread sensitivity buckets used for
/// key-rate CS01 calculations. These buckets cover the full maturity spectrum
/// from 3 months to 30 years.
///
/// # Returns
///
/// Vector of bucket maturities in years: [0.25, 0.5, 1, 2, 3, 5, 7, 10, 15, 20, 30]
///
/// # Examples
///
/// ```rust,ignore
/// // This function is internal - use MetricId::Cs01 for public API
/// use finstack_valuations::metrics::sensitivities::cs01::standard_credit_cs01_buckets;
///
/// let buckets = standard_credit_cs01_buckets();
/// assert_eq!(buckets.len(), 11);
/// ```
#[cfg(test)]
pub fn standard_credit_cs01_buckets() -> Vec<f64> {
    sens_config::STANDARD_BUCKETS_YEARS.to_vec()
}

// Internal helper removed. Using crate::calibration::bumps::hazard::bump_hazard_spreads directly.

/// Compute parallel CS01 by bumping par spreads and re-calibrating.
///
/// Calculates credit spread sensitivity by shifting the par spreads in parallel
/// and re-bootstrapping the hazard curve.
///
/// # Arguments
///
/// * `context` - Metric context containing instrument and market data
/// * `hazard_id` - ID of the hazard curve to bump
/// * `discount_id` - ID of the discount curve used for calibration (optional)
/// * `bump_bp` - Bump size in basis points (typically 1.0 for CS01)
/// * `revalue_with_context` - Closure that reprices the instrument with a bumped context
pub fn compute_parallel_cs01_with_context<RevalFn>(
    context: &mut MetricContext,
    hazard_id: &CurveId,
    discount_id: Option<&CurveId>,
    bump_bp: f64,
    mut revalue_with_context: RevalFn,
) -> finstack_core::Result<f64>
where
    RevalFn: FnMut(&MarketContext) -> finstack_core::Result<Money>,
{
    let base_ctx = context.curves.as_ref();
    let hazard = base_ctx.get_hazard(hazard_id.as_str())?;
    let hazard_ref = hazard.as_ref();
    let has_par_points = hazard_ref.par_spread_points().next().is_some();

    // If we have par spread points + a discount curve, CS01 is defined as the sensitivity
    // to *market par spreads* under a re-bootstrapped hazard curve. In that regime, we
    // must also compute the base PV under the unbumped (re-calibrated) curve; otherwise
    // we introduce a large "base effect" when the in-context hazard curve was not itself
    // calibrated from the stored par points.
    let base_pv = if discount_id.is_some() && has_par_points {
        match bump_hazard_spreads(
            hazard_ref,
            base_ctx,
            &BumpRequest::Parallel(0.0),
            discount_id,
        ) {
            Ok(base_recal) => {
                let base_ctx_recal = base_ctx.clone().insert_hazard(base_recal);
                revalue_with_context(&base_ctx_recal)?
            }
            // If re-calibration fails (e.g. invalid CDS schedule), fall back to the
            // in-context base PV rather than failing the metric.
            Err(_) => context.base_value,
        }
    } else {
        context.base_value
    };

    let bump_request = BumpRequest::Parallel(bump_bp);
    let bumped_hazard = if discount_id.is_some() && has_par_points {
        // Prefer market-par-spread re-calibration when possible, but fall back to a model hazard
        // bump if calibration fails (keeps CS01 available for all curves).
        match bump_hazard_spreads(hazard_ref, base_ctx, &bump_request, discount_id) {
            Ok(curve) => curve,
            Err(_) => bump_hazard_shift(hazard_ref, &bump_request)?,
        }
    } else {
        bump_hazard_shift(hazard_ref, &bump_request)?
    };

    let temp_ctx = base_ctx.clone().insert_hazard(bumped_hazard);
    let pv_bumped = revalue_with_context(&temp_ctx)?;

    // CS01 is PV change per 1bp (currency units per basis point)
    // Positive CS01: instrument gains value when spreads widen
    // Negative CS01: instrument loses value when spreads widen
    let cs01 = if bump_bp.abs() > 1e-10 {
        (pv_bumped.amount() - base_pv.amount()) / bump_bp
    } else {
        0.0
    };

    Ok(cs01)
}

/// Compute key-rate CS01 series by bumping par spreads at specific tenors.
///
/// - `bucket_times_years` are maturities in years (e.g., 0.25, 0.5, 1.0, ...)
/// - For bootstrapped curves, bumps the par quote corresponding to the bucket.
/// - `bump_bp` is the bump size in basis points (typically 1.0 for CS01)
pub fn compute_key_rate_cs01_series_with_context<I, RevalFn>(
    context: &mut MetricContext,
    hazard_id: &CurveId,
    discount_id: Option<&CurveId>,
    bucket_times_years: I,
    bump_bp: f64,
    mut revalue_with_context: RevalFn,
) -> finstack_core::Result<f64>
where
    I: IntoIterator<Item = f64>,
    RevalFn: FnMut(&MarketContext) -> finstack_core::Result<Money>,
{
    let base_ctx = context.curves.as_ref();
    let hazard = base_ctx.get_hazard(hazard_id.as_str())?;
    let hazard_ref = hazard.as_ref();
    let has_par_points = hazard_ref.par_spread_points().next().is_some();

    // Same "base effect" guard as parallel CS01: if we're bumping par spreads and
    // re-bootstrapping, the base PV should be computed under the unbumped re-calibrated curve.
    let base_pv = if discount_id.is_some() && has_par_points {
        match bump_hazard_spreads(
            hazard_ref,
            base_ctx,
            &BumpRequest::Parallel(0.0),
            discount_id,
        ) {
            Ok(base_recal) => {
                let base_ctx_recal = base_ctx.clone().insert_hazard(base_recal);
                revalue_with_context(&base_ctx_recal)?
            }
            Err(_) => context.base_value,
        }
    } else {
        context.base_value
    };

    let mut series: Vec<(String, f64)> = Vec::new();
    let mut total = 0.0;

    for t in bucket_times_years.into_iter() {
        let label = format_bucket_label(t);

        let bump_request = BumpRequest::Tenors(vec![(t, bump_bp)]);
        let bumped_hazard = if discount_id.is_some() && has_par_points {
            match bump_hazard_spreads(hazard_ref, base_ctx, &bump_request, discount_id) {
                Ok(curve) => curve,
                Err(_) => bump_hazard_shift(hazard_ref, &bump_request)?,
            }
        } else {
            bump_hazard_shift(hazard_ref, &bump_request)?
        };

        // Optimization: If the curve is identical (no bump applied because no matching par point),
        // we can skip revaluation.
        // However, comparing curves is hard. bump_hazard_curve_spreads creates a new curve anyway.
        // For correctness, we reprice. Ideally we'd check if we actually bumped anything.

        let temp_ctx = base_ctx.clone().insert_hazard(bumped_hazard);
        let pv_bumped = revalue_with_context(&temp_ctx)?;

        let cs01 = if bump_bp.abs() > 1e-10 {
            (pv_bumped.amount() - base_pv.amount()) / bump_bp
        } else {
            0.0
        };
        series.push((label, cs01));
        total += cs01;
    }

    context.store_bucketed_series(MetricId::BucketedCs01, series);
    Ok(total)
}

// Use shared bucket label formatter
use super::config::format_bucket_label;

// ===== Generic Calculators =====

use crate::instruments::common_impl::traits::{CurveDependencies, Instrument};
use crate::metrics::MetricCalculator;
use std::marker::PhantomData;

/// Generic BucketedCs01 calculator that works for any instrument implementing
/// the required traits.
pub struct GenericBucketedCs01<I> {
    _phantom: PhantomData<I>,
}

/// Generic parallel CS01 calculator that returns a scalar (not bucketed).
///
/// Computes CS01 by applying a parallel bump to the entire hazard curve.
pub struct GenericParallelCs01<I> {
    _phantom: PhantomData<I>,
}

impl<I> Default for GenericParallelCs01<I> {
    fn default() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<I> MetricCalculator for GenericParallelCs01<I>
where
    I: Instrument + CurveDependencies + 'static,
{
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let instrument: &I = context.instrument_as()?;
        let curves = instrument.curve_dependencies();
        let hazard_id = curves
            .credit_curves
            .first()
            .cloned()
            .ok_or_else(|| finstack_core::Error::from(finstack_core::InputError::Invalid))?;
        let discount_id = curves.discount_curves.first().cloned();

        let bump_bp =
            sens_config::from_finstack_config_or_default(context.config())?.credit_spread_bump_bp;

        let inst_arc = Arc::clone(&context.instrument);
        let as_of = context.as_of;

        let reval = move |temp_ctx: &finstack_core::market_data::context::MarketContext| {
            inst_arc.value(temp_ctx, as_of)
        };

        compute_parallel_cs01_with_context(
            context,
            &hazard_id,
            discount_id.as_ref(),
            bump_bp,
            reval,
        )
    }
}

impl<I> Default for GenericBucketedCs01<I> {
    fn default() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<I> MetricCalculator for GenericBucketedCs01<I>
where
    I: Instrument + CurveDependencies + 'static,
{
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let instrument: &I = context.instrument_as()?;
        let curves = instrument.curve_dependencies();
        let hazard_id = curves
            .credit_curves
            .first()
            .cloned()
            .ok_or_else(|| finstack_core::Error::from(finstack_core::InputError::Invalid))?;
        let discount_id = curves.discount_curves.first().cloned();

        let defaults = sens_config::from_finstack_config_or_default(context.config())?;
        let buckets = defaults.cs01_buckets_years;
        let bump_bp = defaults.credit_spread_bump_bp;

        // Generic revaluation using full MarketContext (for complex pricers)
        let inst_arc = Arc::clone(&context.instrument);
        let as_of = context.as_of;

        let reval = move |temp_ctx: &finstack_core::market_data::context::MarketContext| {
            inst_arc.value(temp_ctx, as_of)
        };

        let total = compute_key_rate_cs01_series_with_context(
            context,
            &hazard_id,
            discount_id.as_ref(),
            buckets,
            bump_bp,
            reval,
        )?;

        Ok(total)
    }
}
