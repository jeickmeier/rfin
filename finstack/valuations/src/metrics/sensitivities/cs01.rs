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
use finstack_core::types::CurveId;
use std::sync::Arc;

/// Minimum bump size threshold (in basis points) to avoid division by near-zero.
const MIN_BUMP_BP_THRESHOLD: f64 = 1e-10;

/// Central-difference sensitivity: `(pv_up - pv_down) / (2 * bump_bp)`.
///
/// Returns 0.0 when `bump_bp` is below [`MIN_BUMP_BP_THRESHOLD`] to avoid
/// numerically unstable division.
#[inline]
fn sensitivity_central_diff(pv_up: f64, pv_down: f64, bump_bp: f64) -> f64 {
    if bump_bp.abs() <= MIN_BUMP_BP_THRESHOLD {
        return 0.0;
    }
    (pv_up - pv_down) / (2.0 * bump_bp)
}

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
/// * `revalue_raw` - Closure that reprices the instrument with a bumped context,
///   returning raw f64 for precision
///
/// # Errors
///
/// Returns an error if hazard curve re-calibration fails. This ensures that CS01
/// is computed under a consistent definition (par spread bump + rebootstrap) rather
/// than silently falling back to a different methodology.
pub fn compute_parallel_cs01_with_context_raw<RevalFn>(
    context: &mut MetricContext,
    hazard_id: &CurveId,
    discount_id: Option<&CurveId>,
    bump_bp: f64,
    mut revalue_raw: RevalFn,
) -> finstack_core::Result<f64>
where
    RevalFn: FnMut(&MarketContext) -> finstack_core::Result<f64>,
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
    // Central differencing does not need the base PV, but we still probe whether
    // par-spread re-bootstrapping is available so both legs use the same methodology.
    let used_rebootstrap = if discount_id.is_some() && has_par_points {
        bump_hazard_spreads(
            hazard_ref,
            base_ctx,
            &BumpRequest::Parallel(0.0),
            discount_id,
        )
        .map_err(|e| finstack_core::Error::Calibration {
            message: format!(
                "CS01 hazard curve re-calibration failed for '{}': {} \
                 (cannot compute CS01 under market-standard par spread bump methodology)",
                hazard_id.as_str(),
                e
            ),
            category: "cs01_rebootstrap".to_string(),
        })?;
        true
    } else {
        false
    };

    let bump_request_up = BumpRequest::Parallel(bump_bp);
    let bump_request_down = BumpRequest::Parallel(-bump_bp);

    let bumped_hazard_up = if used_rebootstrap {
        bump_hazard_spreads(hazard_ref, base_ctx, &bump_request_up, discount_id).map_err(|e| {
            finstack_core::Error::Calibration {
                message: format!(
                    "CS01 up-bumped hazard curve re-calibration failed for '{}': {}",
                    hazard_id.as_str(),
                    e
                ),
                category: "cs01_rebootstrap".to_string(),
            }
        })?
    } else {
        bump_hazard_shift(hazard_ref, &bump_request_up)?
    };

    let bumped_hazard_down = if used_rebootstrap {
        bump_hazard_spreads(hazard_ref, base_ctx, &bump_request_down, discount_id).map_err(|e| {
            finstack_core::Error::Calibration {
                message: format!(
                    "CS01 down-bumped hazard curve re-calibration failed for '{}': {}",
                    hazard_id.as_str(),
                    e
                ),
                category: "cs01_rebootstrap".to_string(),
            }
        })?
    } else {
        bump_hazard_shift(hazard_ref, &bump_request_down)?
    };

    let temp_ctx_up = base_ctx.clone().insert(bumped_hazard_up);
    let pv_bumped_up = revalue_raw(&temp_ctx_up)?;

    let temp_ctx_down = base_ctx.clone().insert(bumped_hazard_down);
    let pv_bumped_down = revalue_raw(&temp_ctx_down)?;

    Ok(sensitivity_central_diff(
        pv_bumped_up,
        pv_bumped_down,
        bump_bp,
    ))
}

/// Compute key-rate CS01 series by bumping par spreads at specific tenors.
///
/// - `bucket_times_years` are maturities in years (e.g., 0.25, 0.5, 1.0, ...)
/// - For bootstrapped curves, bumps the par quote corresponding to the bucket.
/// - `bump_bp` is the bump size in basis points (typically 1.0 for CS01)
///
/// # Errors
///
/// Returns an error if hazard curve re-calibration fails. This ensures that CS01
/// is computed under a consistent definition rather than silently falling back.
pub fn compute_key_rate_cs01_series_with_context_raw<I, RevalFn>(
    context: &mut MetricContext,
    hazard_id: &CurveId,
    discount_id: Option<&CurveId>,
    series_id: MetricId,
    bucket_times_years: I,
    bump_bp: f64,
    mut revalue_raw: RevalFn,
) -> finstack_core::Result<f64>
where
    I: IntoIterator<Item = f64>,
    RevalFn: FnMut(&MarketContext) -> finstack_core::Result<f64>,
{
    let base_ctx = context.curves.as_ref();
    let hazard = base_ctx.get_hazard(hazard_id.as_str())?;
    let hazard_ref = hazard.as_ref();
    let has_par_points = hazard_ref.par_spread_points().next().is_some();

    // Central differencing does not need the base PV, but we still probe whether
    // par-spread re-bootstrapping is available so all buckets use the same methodology.
    let used_rebootstrap = if discount_id.is_some() && has_par_points {
        bump_hazard_spreads(
            hazard_ref,
            base_ctx,
            &BumpRequest::Parallel(0.0),
            discount_id,
        )
        .map_err(|e| finstack_core::Error::Calibration {
            message: format!(
                "CS01 hazard curve re-calibration failed for '{}': {}",
                hazard_id.as_str(),
                e
            ),
            category: "cs01_rebootstrap".to_string(),
        })?;
        true
    } else {
        false
    };

    let mut series: Vec<(std::borrow::Cow<'static, str>, f64)> = Vec::new();
    let mut total = 0.0;

    for t in bucket_times_years.into_iter() {
        let label = super::config::format_bucket_label_cow(t);

        let bump_request_up = BumpRequest::Tenors(vec![(t, bump_bp)]);
        let bump_request_down = BumpRequest::Tenors(vec![(t, -bump_bp)]);

        let bumped_hazard_up = if used_rebootstrap {
            bump_hazard_spreads(hazard_ref, base_ctx, &bump_request_up, discount_id).map_err(
                |e| finstack_core::Error::Calibration {
                    message: format!(
                        "CS01 bucket '{}' up-bump hazard re-calibration failed: {}",
                        label, e
                    ),
                    category: "cs01_rebootstrap".to_string(),
                },
            )?
        } else {
            bump_hazard_shift(hazard_ref, &bump_request_up)?
        };

        let bumped_hazard_down = if used_rebootstrap {
            bump_hazard_spreads(hazard_ref, base_ctx, &bump_request_down, discount_id).map_err(
                |e| finstack_core::Error::Calibration {
                    message: format!(
                        "CS01 bucket '{}' down-bump hazard re-calibration failed: {}",
                        label, e
                    ),
                    category: "cs01_rebootstrap".to_string(),
                },
            )?
        } else {
            bump_hazard_shift(hazard_ref, &bump_request_down)?
        };

        let temp_ctx_up = base_ctx.clone().insert(bumped_hazard_up);
        let pv_bumped_up = revalue_raw(&temp_ctx_up)?;

        let temp_ctx_down = base_ctx.clone().insert(bumped_hazard_down);
        let pv_bumped_down = revalue_raw(&temp_ctx_down)?;

        let cs01 = sensitivity_central_diff(pv_bumped_up, pv_bumped_down, bump_bp);
        series.push((label, cs01));
        total += cs01;
    }

    context.store_bucketed_series(series_id, series);
    Ok(total)
}

// ===== Generic Calculators =====

use crate::instruments::common_impl::traits::{CurveDependencies, Instrument};
use crate::metrics::MetricCalculator;
use std::marker::PhantomData;

/// Resolve the primary credit (hazard) and discount curve IDs from an instrument's
/// declared curve dependencies. Returns an error when no credit curve is declared.
fn resolve_cs01_curves<I: Instrument + CurveDependencies>(
    instrument: &I,
    metric_name: &str,
) -> finstack_core::Result<(CurveId, Option<CurveId>)> {
    let curves = instrument.curve_dependencies()?;
    let hazard_id = curves.credit_curves.first().cloned().ok_or_else(|| {
        finstack_core::Error::Validation(format!(
            "Instrument {} has no credit curve dependencies for {} calculation",
            instrument.id(),
            metric_name
        ))
    })?;
    let discount_id = curves.discount_curves.first().cloned();
    Ok((hazard_id, discount_id))
}

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
        let (hazard_id, discount_id) = resolve_cs01_curves(instrument, "CS01")?;

        let bump_bp = sens_config::from_context_or_default(
            context.config(),
            context.metric_overrides.as_ref(),
        )?
        .credit_spread_bump_bp;

        let inst_arc = Arc::clone(&context.instrument);
        let as_of = context.as_of;

        let reval = move |temp_ctx: &finstack_core::market_data::context::MarketContext| {
            inst_arc.value_raw(temp_ctx, as_of)
        };

        let cs01 = compute_parallel_cs01_with_context_raw(
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
        let (hazard_id, discount_id) = resolve_cs01_curves(instrument, "CS01")?;

        let defaults = sens_config::from_context_or_default(
            context.config(),
            context.metric_overrides.as_ref(),
        )?;
        let buckets = defaults.cs01_buckets_years;
        let bump_bp = defaults.credit_spread_bump_bp;

        let inst_arc = Arc::clone(&context.instrument);
        let as_of = context.as_of;

        let reval = move |temp_ctx: &finstack_core::market_data::context::MarketContext| {
            inst_arc.value_raw(temp_ctx, as_of)
        };

        let series_id = MetricId::custom(format!("bucketed_cs01::{}", hazard_id.as_str()));

        let total = compute_key_rate_cs01_series_with_context_raw(
            context,
            &hazard_id,
            discount_id.as_ref(),
            series_id,
            buckets,
            bump_bp,
            reval,
        )?;

        Ok(total)
    }
}

// ===== Hazard-Rate CS01 Calculators =====

/// Generic parallel CS01 calculator using direct hazard-rate bumps.
///
/// Unlike `GenericParallelCs01` which bumps par spreads and re-bootstraps,
/// this directly shifts hazard rates. Registered as `MetricId::Cs01Hazard`.
pub struct GenericParallelCs01Hazard<I> {
    _phantom: PhantomData<I>,
}

impl<I> Default for GenericParallelCs01Hazard<I> {
    fn default() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<I> MetricCalculator for GenericParallelCs01Hazard<I>
where
    I: Instrument + CurveDependencies + 'static,
{
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let instrument: &I = context.instrument_as()?;
        let (hazard_id, _discount_id) = resolve_cs01_curves(instrument, "CS01Hazard")?;

        let bump_bp = sens_config::from_context_or_default(
            context.config(),
            context.metric_overrides.as_ref(),
        )?
        .credit_spread_bump_bp;

        let base_ctx = context.curves.as_ref();
        let hazard = base_ctx.get_hazard(hazard_id.as_str())?;
        let hazard_ref = hazard.as_ref();

        let inst_arc = Arc::clone(&context.instrument);
        let as_of = context.as_of;

        let bumped_up = bump_hazard_shift(hazard_ref, &BumpRequest::Parallel(bump_bp))?;
        let bumped_down = bump_hazard_shift(hazard_ref, &BumpRequest::Parallel(-bump_bp))?;

        let pv_up = inst_arc.value_raw(&base_ctx.clone().insert(bumped_up), as_of)?;
        let pv_down = inst_arc.value_raw(&base_ctx.clone().insert(bumped_down), as_of)?;

        let cs01 = sensitivity_central_diff(pv_up, pv_down, bump_bp);

        context.computed.insert(
            MetricId::custom(format!("cs01_hazard::{}", hazard_id.as_str())),
            cs01,
        );

        Ok(cs01)
    }
}

/// Generic bucketed CS01 calculator using direct hazard-rate bumps.
///
/// Unlike `GenericBucketedCs01` which bumps par spreads and re-bootstraps,
/// this directly shifts hazard rates at each tenor. Registered as
/// `MetricId::BucketedCs01Hazard`.
pub struct GenericBucketedCs01Hazard<I> {
    _phantom: PhantomData<I>,
}

impl<I> Default for GenericBucketedCs01Hazard<I> {
    fn default() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<I> MetricCalculator for GenericBucketedCs01Hazard<I>
where
    I: Instrument + CurveDependencies + 'static,
{
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let instrument: &I = context.instrument_as()?;
        let (hazard_id, _discount_id) = resolve_cs01_curves(instrument, "BucketedCs01Hazard")?;

        let defaults = sens_config::from_context_or_default(
            context.config(),
            context.metric_overrides.as_ref(),
        )?;
        let buckets = defaults.cs01_buckets_years;
        let bump_bp = defaults.credit_spread_bump_bp;

        let base_ctx = context.curves.as_ref();
        let hazard = base_ctx.get_hazard(hazard_id.as_str())?;
        let hazard_ref = hazard.as_ref();

        let inst_arc = Arc::clone(&context.instrument);
        let as_of = context.as_of;

        let mut series: Vec<(std::borrow::Cow<'static, str>, f64)> = Vec::new();
        let mut total = 0.0;

        for t in buckets {
            let label = super::config::format_bucket_label_cow(t);

            let bumped_up =
                bump_hazard_shift(hazard_ref, &BumpRequest::Tenors(vec![(t, bump_bp)]))?;
            let bumped_down =
                bump_hazard_shift(hazard_ref, &BumpRequest::Tenors(vec![(t, -bump_bp)]))?;

            let pv_up = inst_arc.value_raw(&base_ctx.clone().insert(bumped_up), as_of)?;
            let pv_down = inst_arc.value_raw(&base_ctx.clone().insert(bumped_down), as_of)?;

            let cs01 = sensitivity_central_diff(pv_up, pv_down, bump_bp);
            series.push((label, cs01));
            total += cs01;
        }

        let series_id = MetricId::custom(format!("bucketed_cs01_hazard::{}", hazard_id.as_str()));
        context.store_bucketed_series(series_id, series);

        Ok(total)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    #[test]
    fn test_central_vs_forward_difference() {
        // Central difference: (f(x+h) - f(x-h)) / (2h) = f'(x) + O(h^2)
        // Forward difference: (f(x+h) - f(x)) / h = f'(x) + O(h)
        // For f(x) = x^2, f'(x) = 2x at x=1, h=0.1:
        // Central: (1.21 - 0.81) / 0.2 = 2.0 (exact)
        // Forward: (1.21 - 1.0) / 0.1 = 2.1 (has error)
        let f = |x: f64| x * x;
        let x = 1.0;
        let h = 0.1;
        let central = (f(x + h) - f(x - h)) / (2.0 * h);
        let forward = (f(x + h) - f(x)) / h;
        assert!(
            (central - 2.0).abs() < 1e-14,
            "Central difference should be exact for quadratics"
        );
        assert!(
            (forward - 2.0).abs() > 0.09,
            "Forward difference should have O(h) error"
        );
    }
}
