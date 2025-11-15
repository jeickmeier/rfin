//! Reusable helpers for bucketed CS01 (credit spread sensitivity) using key-rate bumps.
//!
//! Provides generic functions to compute bucketed CS01 for instruments that
//! depend on hazard curves. Results are stored into `MetricContext` via structured
//! series using stable composite keys.

use crate::metrics::{MetricContext, MetricId};
use finstack_core::market_data::term_structures::hazard_curve::HazardCurve;
use finstack_core::market_data::MarketContext;
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
/// ```rust
/// use finstack_valuations::metrics::standard_credit_cs01_buckets;
///
/// let buckets = standard_credit_cs01_buckets();
/// assert_eq!(buckets.len(), 11);
/// assert_eq!(buckets[0], 0.25); // 3 months
/// assert_eq!(buckets[10], 30.0); // 30 years
/// ```
pub fn standard_credit_cs01_buckets() -> Vec<f64> {
    vec![0.25, 0.5, 1.0, 2.0, 3.0, 5.0, 7.0, 10.0, 15.0, 20.0, 30.0]
}

/// Compute parallel CS01 by bumping the entire hazard curve uniformly.
///
/// Calculates credit spread sensitivity by shifting the entire hazard curve in parallel
/// and measuring the present value change. This is the most common CS01 measure, representing
/// the P&L impact of a 1bp parallel shift in credit spreads.
///
/// # Arguments
///
/// * `context` - Metric context containing instrument and market data
/// * `hazard_id` - ID of the hazard curve to bump
/// * `bump_bp` - Bump size in basis points (typically 1.0 for CS01)
/// * `revalue_with_hazard` - Closure that reprices the instrument with a bumped hazard curve
///
/// # Returns
///
/// CS01 value representing the present value change per 1bp parallel shift in credit spreads.
/// A positive CS01 means the instrument gains value when spreads widen.
///
/// # Errors
///
/// Returns an error if:
/// - The hazard curve is not found in the market context
/// - The bump operation fails due to invalid curve data
/// - Revaluation fails
///
/// # Examples
///
/// ```rust,ignore
/// use finstack_valuations::metrics::cs01::compute_parallel_cs01;
///
/// let cs01 = compute_parallel_cs01(
///     &mut context,
///     &hazard_curve_id,
///     1.0, // 1bp bump
///     |hazard| instrument.value_with_hazard(hazard)
/// )?;
///
/// println!("CS01: ${:.2} per bp", cs01);
/// ```
pub fn compute_parallel_cs01<RevalFn>(
    context: &mut MetricContext,
    hazard_id: &CurveId,
    bump_bp: f64,
    mut revalue_with_hazard: RevalFn,
) -> finstack_core::Result<f64>
where
    RevalFn: FnMut(&HazardCurve) -> finstack_core::Result<Money>,
{
    let base_pv = context.base_value;
    let hazard = context.curves.get_hazard_ref(hazard_id.as_str())?;

    // Parallel bump the entire hazard curve (convert bp to decimal)
    let bump_decimal = bump_bp * 1e-4;
    let bumped_hazard = hazard.with_hazard_shift(bump_decimal)?;
    let pv_bumped = revalue_with_hazard(&bumped_hazard)?;
    let cs01 = (pv_bumped.amount() - base_pv.amount()) / 10_000.0;

    Ok(cs01)
}

/// Compute parallel CS01 using full MarketContext revaluation.
///
/// Returns the CS01 as a single scalar value.
pub fn compute_parallel_cs01_with_context<RevalFn>(
    context: &mut MetricContext,
    hazard_id: &CurveId,
    bump_bp: f64,
    mut revalue_with_context: RevalFn,
) -> finstack_core::Result<f64>
where
    RevalFn: FnMut(&MarketContext) -> finstack_core::Result<Money>,
{
    let base_pv = context.base_value;
    let base_ctx = context.curves.as_ref();
    let hazard = base_ctx.get_hazard_ref(hazard_id.as_str())?;

    // Parallel bump the entire hazard curve
    let bump_decimal = bump_bp * 1e-4;
    let bumped_hazard = hazard.with_hazard_shift(bump_decimal)?;
    let temp_ctx = base_ctx.clone().insert_hazard(bumped_hazard);
    let pv_bumped = revalue_with_context(&temp_ctx)?;
    let cs01 = (pv_bumped.amount() - base_pv.amount()) / 10_000.0;

    Ok(cs01)
}

/// Compute key-rate CS01 series by bumping hazard rates at specific tenors.
///
/// - `bucket_times_years` are maturities in years (e.g., 0.25, 0.5, 1.0, ...)
/// - Each bucket bumps only the hazard rate segment containing that tenor
/// - `bump_bp` is the bump size in basis points (typically 1.0 for CS01)
/// - Labels are derived from times using standard m/y formatting
pub fn compute_key_rate_cs01_series<I, RevalFn>(
    context: &mut MetricContext,
    hazard_id: &CurveId,
    bucket_times_years: I,
    bump_bp: f64,
    mut revalue_with_hazard: RevalFn,
) -> finstack_core::Result<f64>
where
    I: IntoIterator<Item = f64>,
    RevalFn: FnMut(&HazardCurve) -> finstack_core::Result<Money>,
{
    let base_pv = context.base_value;
    let hazard = context.curves.get_hazard_ref(hazard_id.as_str())?;

    let mut series: Vec<(String, f64)> = Vec::new();
    for t in bucket_times_years.into_iter() {
        let label = if t < 1.0 {
            format!("{:.0}m", (t * 12.0).round())
        } else {
            format!("{:.0}y", t)
        };

        // Bump hazard rate at the specific tenor
        // For key-rate bumping, we bump the segment containing the target time
        let bumped_hazard = with_key_rate_hazard_bump(hazard, t, bump_bp)?;

        let pv_bumped = revalue_with_hazard(&bumped_hazard)?;
        // CS01 is PV change per 1bp, so divide by 10,000 to normalize
        let cs01 = (pv_bumped.amount() - base_pv.amount()) / 10_000.0;
        series.push((label, cs01));
    }

    context.store_bucketed_series(MetricId::BucketedCs01, series.clone());
    let total: f64 = series.iter().map(|(_, v)| *v).sum();
    Ok(total)
}

/// Compute key-rate CS01 series using full MarketContext revaluation per bucket time.
pub fn compute_key_rate_cs01_series_with_context<I, RevalFn>(
    context: &mut MetricContext,
    hazard_id: &CurveId,
    bucket_times_years: I,
    bump_bp: f64,
    mut revalue_with_context: RevalFn,
) -> finstack_core::Result<f64>
where
    I: IntoIterator<Item = f64>,
    RevalFn: FnMut(&MarketContext) -> finstack_core::Result<Money>,
{
    let base_pv = context.base_value;
    let base_ctx = context.curves.as_ref();
    let hazard = base_ctx.get_hazard_ref(hazard_id.as_str())?;

    let mut series: Vec<(String, f64)> = Vec::new();
    for t in bucket_times_years.into_iter() {
        let label = if t < 1.0 {
            format!("{:.0}m", (t * 12.0).round())
        } else {
            format!("{:.0}y", t)
        };

        // Bump hazard at key rate
        let bumped_hazard = with_key_rate_hazard_bump(hazard, t, bump_bp)?;

        // Create new MarketContext with bumped hazard
        let temp_ctx = base_ctx.clone().insert_hazard(bumped_hazard);
        let pv_bumped = revalue_with_context(&temp_ctx)?;
        let cs01 = (pv_bumped.amount() - base_pv.amount()) / 10_000.0;
        series.push((label, cs01));
    }

    context.store_bucketed_series(MetricId::BucketedCs01, series.clone());
    let total: f64 = series.iter().map(|(_, v)| *v).sum();
    Ok(total)
}

/// Helper to apply a key-rate bump to a hazard curve at a specific tenor.
///
/// This bumps only the hazard rate segment containing the target time `t_years`.
/// If `t_years` is before the first knot, bumps the first segment.
/// If `t_years` is after the last knot, bumps the last segment.
fn with_key_rate_hazard_bump(
    hazard: &HazardCurve,
    t_years: f64,
    bump_bp: f64,
) -> finstack_core::Result<HazardCurve> {
    // Convert bump from bp to hazard rate units (1bp = 0.0001 in decimal)
    let bump_decimal = bump_bp * 1e-4;

    // Get knot points
    let knots: Vec<f64> = hazard.knot_points().map(|(t, _)| t).collect();
    let hazard_rates: Vec<f64> = hazard.knot_points().map(|(_, lambda)| lambda).collect();

    if knots.len() < 2 {
        // Parallel bump if curve has < 2 knots
        return hazard.with_hazard_shift(bump_decimal);
    }

    // Find segment containing t_years
    let mut target_segment = 0usize;
    if t_years <= knots[0] {
        target_segment = 0;
    } else if t_years >= knots[knots.len() - 1] {
        target_segment = knots.len() - 2;
    } else {
        for i in 0..knots.len() - 1 {
            if t_years > knots[i] && t_years <= knots[i + 1] {
                target_segment = i;
                break;
            }
        }
    }

    // Bump the hazard rate in the target segment
    // For piecewise constant curves, we bump the constant rate in that segment
    let mut bumped_rates = hazard_rates.clone();
    bumped_rates[target_segment] = (bumped_rates[target_segment] + bump_decimal).max(0.0);

    // Rebuild hazard curve with bumped rates
    let bumped_points: Vec<(f64, f64)> = knots
        .iter()
        .zip(bumped_rates.iter())
        .map(|(&t, &lambda)| (t, lambda))
        .collect();

    // Use builder to recreate curve
    let mut builder = hazard
        .to_builder_with_id(hazard.id().clone())
        .knots(bumped_points);

    // Add par spread points if available
    builder = builder.par_spreads(hazard.par_spread_points());

    builder
        .build()
        .map_err(|_e| finstack_core::Error::from(finstack_core::error::InputError::Invalid))
}

// ===== Generic Calculators =====

use crate::instruments::common::traits::Instrument;
use crate::metrics::MetricCalculator;
use std::marker::PhantomData;

/// Trait for instruments that have a primary credit curve.
///
/// Used by generic bucketed CS01 calculators to identify which credit curve
/// to bump for credit spread sensitivity calculations.
pub trait HasCreditCurve {
    /// Returns the ID of the primary credit curve used for credit spread sensitivity.
    fn credit_curve_id(&self) -> &finstack_core::types::CurveId;
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
    I: Instrument + HasCreditCurve + 'static,
{
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let instrument: &I = context.instrument_as()?;
        let hazard_id = instrument.credit_curve_id().clone();

        let inst_arc = Arc::clone(&context.instrument);
        let as_of = context.as_of;

        let reval = move |temp_ctx: &finstack_core::market_data::MarketContext| {
            inst_arc.value(temp_ctx, as_of)
        };

        compute_parallel_cs01_with_context(context, &hazard_id, 1.0, reval)
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
    I: Instrument + HasCreditCurve + 'static,
{
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let instrument: &I = context.instrument_as()?;
        let hazard_id = instrument.credit_curve_id().clone();

        // Standard credit bucket times
        let buckets = standard_credit_cs01_buckets();

        // Generic revaluation using full MarketContext (for complex pricers)
        let inst_arc = Arc::clone(&context.instrument);
        let as_of = context.as_of;

        let reval = move |temp_ctx: &finstack_core::market_data::MarketContext| {
            inst_arc.value(temp_ctx, as_of)
        };

        let total =
            compute_key_rate_cs01_series_with_context(context, &hazard_id, buckets, 1.0, reval)?;

        Ok(total)
    }
}
