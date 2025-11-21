//! Reusable helpers for bucketed CS01 (credit spread sensitivity) using key-rate bumps.
//!
//! Provides generic functions to compute bucketed CS01 for instruments that
//! depend on hazard curves. Results are stored into `MetricContext` via structured
//! series using stable composite keys.

use crate::metrics::{MetricContext, MetricId};
use crate::calibration::methods::hazard_curve::HazardCurveCalibrator;
use crate::calibration::CreditQuote;
use crate::calibration::Calibrator;
use finstack_core::market_data::term_structures::hazard_curve::HazardCurve;
use finstack_core::market_data::term_structures::Seniority;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, Currency};
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

/// Helper to bump hazard curve by shocking par spreads and re-calibrating.
///
/// Falls back to hazard rate shifting if par spread information is missing.
fn bump_hazard_curve_spreads(
    hazard: &HazardCurve,
    context: &MarketContext,
    discount_id: Option<&CurveId>,
    bump_bp: f64,
    bucket_year: Option<f64>, // None for parallel, Some(t) for key-rate
) -> finstack_core::Result<HazardCurve> {
    // Check if we have necessary data for re-calibration
    let par_points: Vec<(f64, f64)> = hazard.par_spread_points().collect();
    
    let Some(discount_id) = discount_id else {
        // Fallback to hazard rate shift (Model Sensitivity)
        let bump_decimal = bump_bp * 1e-4;
        if let Some(t) = bucket_year {
            return with_key_rate_hazard_bump(hazard, t, bump_bp);
        } else {
            let temp_bumped = hazard.with_hazard_shift(bump_decimal)?;
            return temp_bumped
                .to_builder_with_id(hazard.id().clone())
                .build()
                .map_err(|_| finstack_core::Error::Internal);
        }
    };

    if par_points.is_empty() {
        // Fallback to hazard rate shift (Model Sensitivity) if no par points
        let bump_decimal = bump_bp * 1e-4;
        if let Some(t) = bucket_year {
            return with_key_rate_hazard_bump(hazard, t, bump_bp);
        } else {
            let temp_bumped = hazard.with_hazard_shift(bump_decimal)?;
            return temp_bumped
                .to_builder_with_id(hazard.id().clone())
                .build()
                .map_err(|_| finstack_core::Error::Internal);
        }
    }
    
    // Construct CreditQuotes from par points with bumps applied
    let base_date = hazard.base_date();
    let currency = hazard.currency().unwrap_or(Currency::USD); // Default or error?
    let recovery = hazard.recovery_rate();
    let seniority = hazard.seniority.unwrap_or(Seniority::Senior);
    let issuer = hazard.issuer().map(|s| s.to_string()).unwrap_or_else(|| "UNKNOWN".to_string());

    let mut quotes = Vec::new();
    
    for (tenor_years, spread_bp) in par_points {
        let maturity_days = (tenor_years * 365.25).round() as i64;
        let maturity = base_date + time::Duration::days(maturity_days);
        
        let mut bumped_spread = spread_bp;
        
        // Apply bump
        if let Some(bucket_t) = bucket_year {
            // Key-rate bump: strictly bucketed or distributed?
            // Standard key-rate bump usually applies to the specific par instrument
            // matching the bucket. Here we map tenor points to buckets.
            // For simplicity/standard practice: 
            // If bucket_year matches tenor_years (approx), bump it.
            // Or we could define "buckets" as the par points themselves.
            // Since we passed in `bucket_year` from `standard_credit_cs01_buckets`,
            // we should check if this par point falls in the bucket's influence.
            // BUT: typically re-calibration bumps the *input* instruments.
            // If the curve has points at 3Y, 5Y, 7Y, and we request 5Y bucket sensitivity,
            // we bump the 5Y quote.
            // If we request 4Y bucket sensitivity (no point), we might interpolate or do nothing.
            // 
            // Current logic receives `bucket_times_years` from caller.
            // For bootstrapping consistency, we should bump the par point closest to the bucket?
            // Or assumes the caller passed `par_tenors` as buckets?
            
            // Strategy: Bump the par point if it matches the requested bucket within tolerance.
            // If `bucket_year` is not one of the par tenors, this might result in zero sensitivity
            // for that bucket, which is correct for a bootstrapped curve (local dependency).
            if (tenor_years - bucket_t).abs() < 0.1 { // 0.1 year tolerance
                bumped_spread += bump_bp;
            }
        } else {
            // Parallel bump
            bumped_spread += bump_bp;
        }

        quotes.push(CreditQuote::CDS {
            entity: issuer.clone(),
            currency,
            maturity,
            spread_bp: bumped_spread,
            recovery_rate: recovery,
        });
    }

    // Calibrate new curve
    let calibrator = HazardCurveCalibrator::new(
        issuer,
        seniority,
        recovery,
        base_date,
        currency,
        discount_id.clone(),
    );

    let (new_curve, _report) = calibrator.calibrate(&quotes, context)?;
    
    // Restore original ID to ensure it overrides correctly in MarketContext
    let final_curve = new_curve
        .to_builder_with_id(hazard.id().clone())
        .build()
        .map_err(|_| finstack_core::Error::Internal)?;

    Ok(final_curve)
}

/// Compute parallel CS01 by bumping the entire hazard curve uniformly (Hazard Rate Sensitivity).
///
/// Note: This computes Model Sensitivity (to hazard rates), not Quote Sensitivity (to par spreads).
/// For Par Spread sensitivity, use `compute_parallel_par_spread_cs01` with a discount curve.
#[allow(dead_code)] // Alternative API for hazard rate sensitivity (not currently used)
pub fn compute_parallel_hazard_cs01<RevalFn>(
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
    let temp_bumped = hazard.with_hazard_shift(bump_decimal)?;

    // Restore original ID so it overwrites correctly in MarketContext
    let bumped_hazard = temp_bumped
        .to_builder_with_id(hazard_id.clone())
        .build()
        .map_err(|_| finstack_core::Error::Internal)?;

    let pv_bumped = revalue_with_hazard(&bumped_hazard)?;
    
    let cs01 = if bump_bp.abs() > 1e-10 {
        (pv_bumped.amount() - base_pv.amount()) / bump_bp
    } else {
        0.0
    };

    Ok(cs01)
}

/// Compute key-rate CS01 series by bumping hazard rates at specific tenors (Hazard Rate Sensitivity).
///
/// Note: This computes Model Sensitivity (to hazard rates), not Quote Sensitivity (to par spreads).
#[allow(dead_code)] // Alternative API for hazard rate sensitivity (not currently used)
pub fn compute_key_rate_hazard_cs01_series<I, RevalFn>(
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
        let label = format_credit_bucket_label(t);

        // Bump hazard rate at the specific tenor
        let bumped_hazard = with_key_rate_hazard_bump(hazard, t, bump_bp)?;

        let pv_bumped = revalue_with_hazard(&bumped_hazard)?;
        let cs01 = if bump_bp.abs() > 1e-10 {
            (pv_bumped.amount() - base_pv.amount()) / bump_bp
        } else {
            0.0
        };
        series.push((label, cs01));
    }

    context.store_bucketed_series(MetricId::BucketedCs01, series.clone());
    let total: f64 = series.iter().map(|(_, v)| *v).sum();
    Ok(total)
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
    let base_pv = context.base_value;
    let base_ctx = context.curves.as_ref();
    let hazard = base_ctx.get_hazard_ref(hazard_id.as_str())?;

    // Bump spreads and re-calibrate
    let bumped_hazard = bump_hazard_curve_spreads(
        hazard, 
        base_ctx, 
        discount_id, 
        bump_bp, 
        None
    )?;

    let temp_ctx = base_ctx.clone().insert_hazard(bumped_hazard);
    let pv_bumped = revalue_with_context(&temp_ctx)?;
    
    // CS01 is PV change per 1bp
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
    let base_pv = context.base_value;
    let base_ctx = context.curves.as_ref();
    let hazard = base_ctx.get_hazard_ref(hazard_id.as_str())?;

    let mut series: Vec<(String, f64)> = Vec::new();
    let mut total = 0.0;

    for t in bucket_times_years.into_iter() {
        let label = format_credit_bucket_label(t);

        // Bump spread at key rate (bucket t)
        // Note: bump_hazard_curve_spreads handles the logic of finding the matching par point
        let bumped_hazard = bump_hazard_curve_spreads(
            hazard,
            base_ctx,
            discount_id,
            bump_bp,
            Some(t)
        )?;

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

/// Standard credit bucket labels matching standard_credit_cs01_buckets() order.
const CREDIT_BUCKET_LABELS: [&str; 11] = [
    "3m", "6m", "1y", "2y", "3y", "5y", "7y", "10y", "15y", "20y", "30y",
];

/// Generate bucket label from years.
/// Uses static labels for standard buckets, falls back to dynamic formatting for custom buckets.
#[inline]
fn format_credit_bucket_label(years: f64) -> String {
    // Check if this matches a standard bucket (with small tolerance for floating point comparison)
    let standard_buckets = standard_credit_cs01_buckets();
    for (i, &bucket_time) in standard_buckets.iter().enumerate() {
        if (years - bucket_time).abs() < 0.01 {
            return CREDIT_BUCKET_LABELS[i].to_string();
        }
    }

    // Fall back to dynamic formatting for non-standard buckets
    if years < 1.0 {
        format!("{:.0}m", (years * 12.0).round())
    } else {
        format!("{:.0}y", years)
    }
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
    let mut bumped_rates = hazard_rates;
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
use crate::instruments::common::pricing::HasDiscountCurve;
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
    I: Instrument + HasCreditCurve + HasDiscountCurve + 'static,
{
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let instrument: &I = context.instrument_as()?;
        let hazard_id = instrument.credit_curve_id().clone();
        let discount_id = instrument.discount_curve_id().clone();

        let inst_arc = Arc::clone(&context.instrument);
        let as_of = context.as_of;

        let reval = move |temp_ctx: &finstack_core::market_data::MarketContext| {
            inst_arc.value(temp_ctx, as_of)
        };

        compute_parallel_cs01_with_context(context, &hazard_id, Some(&discount_id), 1.0, reval)
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
    I: Instrument + HasCreditCurve + HasDiscountCurve + 'static,
{
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let instrument: &I = context.instrument_as()?;
        let hazard_id = instrument.credit_curve_id().clone();
        let discount_id = instrument.discount_curve_id().clone();

        // Standard credit bucket times
        let buckets = standard_credit_cs01_buckets();

        // Generic revaluation using full MarketContext (for complex pricers)
        let inst_arc = Arc::clone(&context.instrument);
        let as_of = context.as_of;

        let reval = move |temp_ctx: &finstack_core::market_data::MarketContext| {
            inst_arc.value(temp_ctx, as_of)
        };

        let total =
            compute_key_rate_cs01_series_with_context(context, &hazard_id, Some(&discount_id), buckets, 1.0, reval)?;

        Ok(total)
    }
}
