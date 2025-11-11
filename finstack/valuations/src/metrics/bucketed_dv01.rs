//! Reusable helpers for bucketed risk metrics (DV01/CS01) using key-rate bumps.
//!
//! Provides a generic function to compute bucketed DV01 for instruments that
//! can be valued with discount and/or forward curves. Results are stored into
//! `MetricContext` via structured series using stable composite keys.

use crate::metrics::{MetricContext, MetricId};
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::CurveId;

/// Standard IR key-rate buckets in years used for quick demos/tests.
/// Example: [0.25, 0.5, 1, 2, 3, 5, 7, 10, 15, 20, 30]
pub fn standard_ir_dv01_buckets() -> Vec<f64> {
    vec![0.25, 0.5, 1.0, 2.0, 3.0, 5.0, 7.0, 10.0, 15.0, 20.0, 30.0]
}

/// Compute parallel DV01 by bumping the entire discount curve uniformly.
///
/// Returns the DV01 as a single scalar value (PV change per 1bp parallel shift).
/// Does not store bucketed series in the context.
pub fn compute_parallel_dv01<RevalFn>(
    context: &mut MetricContext,
    discount_curve_id: &CurveId,
    bump_bp: f64,
    mut revalue_with_disc: RevalFn,
) -> finstack_core::Result<f64>
where
    RevalFn: FnMut(&DiscountCurve) -> finstack_core::Result<Money>,
{
    let base_pv = context.base_value;
    let disc = context
        .curves
        .get_discount_ref(discount_curve_id.as_str())?;

    // Parallel bump the entire curve
    let bumped = disc.with_parallel_bump(bump_bp);
    let pv_bumped = revalue_with_disc(&bumped)?;
    // DV01 = PV change per 1bp move. If bump is N bp, divide by N to get per-bp sensitivity
    let dv01 = (pv_bumped.amount() - base_pv.amount()) / bump_bp;

    Ok(dv01)
}

/// Compute parallel DV01 using full MarketContext revaluation.
///
/// Returns the DV01 as a single scalar value.
pub fn compute_parallel_dv01_with_context<RevalFn>(
    context: &mut MetricContext,
    discount_curve_id: &CurveId,
    bump_bp: f64,
    mut revalue_with_context: RevalFn,
) -> finstack_core::Result<f64>
where
    RevalFn: FnMut(&MarketContext) -> finstack_core::Result<Money>,
{
    use finstack_core::market_data::context::BumpSpec;
    use hashbrown::HashMap;

    let base_pv = context.base_value;
    let base_ctx = context.curves.as_ref();

    // Use the MarketContext.bump() method which correctly replaces curves under original IDs
    let mut bumps = HashMap::new();
    bumps.insert(discount_curve_id.clone(), BumpSpec::parallel_bp(bump_bp));
    let temp_ctx = base_ctx.bump(bumps)?;

    let pv_bumped = revalue_with_context(&temp_ctx)?;
    // DV01 = PV change per 1bp move. If bump is N bp, divide by N to get per-bp sensitivity
    let dv01 = (pv_bumped.amount() - base_pv.amount()) / bump_bp;

    Ok(dv01)
}

// Note: prior versions supported “parallel bump” DV01 per label; this was incorrect.
// All bucketed DV01 now uses key‑rate bumps at per-bucket maturities.

/// Compute key-rate DV01 series by bumping only the forward segment that contains each bucket time.
///
/// - `bucket_times_years` are maturities in years (e.g., 0.25, 0.5, 1.0, ...)
/// - Labels are derived from times using the standard m/y formatting
pub fn compute_key_rate_dv01_series<I, RevalFn>(
    context: &mut MetricContext,
    discount_curve_id: &CurveId,
    bucket_times_years: I,
    bump_bp: f64,
    mut revalue_with_disc: RevalFn,
) -> finstack_core::Result<f64>
where
    I: IntoIterator<Item = f64>,
    RevalFn: FnMut(&DiscountCurve) -> finstack_core::Result<Money>,
{
    let base_pv = context.base_value;
    let disc = context
        .curves
        .get_discount_ref(discount_curve_id.as_str())?;

    let mut series: Vec<(String, f64)> = Vec::new();
    for t in bucket_times_years.into_iter() {
        let label = if t < 1.0 {
            format!("{:.0}m", (t * 12.0).round())
        } else {
            format!("{:.0}y", t)
        };
        let bumped = disc.with_key_rate_bump_years(t, bump_bp);
        let pv_bumped = revalue_with_disc(&bumped)?;
        // DV01 per bucket: PV change per 1bp move in this bucket
        let dv01 = (pv_bumped.amount() - base_pv.amount()) / bump_bp;
        series.push((label, dv01));
    }

    context.store_bucketed_series(MetricId::BucketedDv01, series.clone());
    let total: f64 = series.iter().map(|(_, v)| *v).sum();
    Ok(total)
}

/// Key-rate DV01 series using full MarketContext revaluation per bucket time.
///
/// Note: This function uses a workaround - it creates a new MarketContext with the bumped
/// curve using the original curve ID. This requires using the bump() method with a temporary ID.
pub fn compute_key_rate_dv01_series_with_context<I, RevalFn>(
    context: &mut MetricContext,
    discount_curve_id: &CurveId,
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
    let disc = base_ctx.get_discount_ref(discount_curve_id.as_str())?;

    let mut series: Vec<(String, f64)> = Vec::new();
    for t in bucket_times_years.into_iter() {
        let label = if t < 1.0 {
            format!("{:.0}m", (t * 12.0).round())
        } else {
            format!("{:.0}y", t)
        };
        // Create bumped curve, then rebuild it with the ORIGINAL ID so instruments can find it
        // Note: We lose the original interpolation style but use safe defaults (Linear)
        let bumped_tmp = disc.with_key_rate_bump_years(t, bump_bp);
        let bumped_points: Vec<(f64, f64)> = bumped_tmp
            .knots()
            .iter()
            .copied()
            .zip(bumped_tmp.dfs().iter().copied())
            .collect();
        let bumped_disc = DiscountCurve::builder(discount_curve_id.clone())
            .base_date(bumped_tmp.base_date())
            .day_count(bumped_tmp.day_count())
            .knots(bumped_points)
            .build()?;
        let temp_ctx = base_ctx.clone().insert_discount(bumped_disc);

        let pv_bumped = revalue_with_context(&temp_ctx)?;
        // DV01 per bucket: PV change per 1bp move in this bucket
        let dv01 = (pv_bumped.amount() - base_pv.amount()) / bump_bp;
        series.push((label, dv01));
    }

    context.store_bucketed_series(MetricId::BucketedDv01, series.clone());
    let total: f64 = series.iter().map(|(_, v)| *v).sum();
    Ok(total)
}

/// Generic helper: key-rate series under a custom base metric ID using DiscountCurve revaluation.
pub fn compute_key_rate_series_for_id<I, RevalFn>(
    context: &mut MetricContext,
    base_metric_id: MetricId,
    discount_curve_id: &CurveId,
    bucket_times_years: I,
    bump_bp: f64,
    mut revalue_with_disc: RevalFn,
) -> finstack_core::Result<f64>
where
    I: IntoIterator<Item = f64>,
    RevalFn: FnMut(&DiscountCurve) -> finstack_core::Result<Money>,
{
    let base_pv = context.base_value;
    let disc = context
        .curves
        .get_discount_ref(discount_curve_id.as_str())?;

    let mut series: Vec<(String, f64)> = Vec::new();
    for t in bucket_times_years.into_iter() {
        let label = if t < 1.0 {
            format!("{:.0}m", (t * 12.0).round())
        } else {
            format!("{:.0}y", t)
        };
        let bumped = disc.with_key_rate_bump_years(t, bump_bp);
        let pv_bumped = revalue_with_disc(&bumped)?;
        let dv01 = (pv_bumped.amount() - base_pv.amount()) / 10_000.0;
        series.push((label, dv01));
    }

    context.store_bucketed_series(base_metric_id, series.clone());
    let total: f64 = series.iter().map(|(_, v)| *v).sum();
    Ok(total)
}

/// Generic helper: key-rate series under a custom base metric ID using full MarketContext revaluation.
pub fn compute_key_rate_series_with_context_for_id<I, RevalFn>(
    context: &mut MetricContext,
    base_metric_id: MetricId,
    discount_curve_id: &CurveId,
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
    let disc = base_ctx.get_discount_ref(discount_curve_id.as_str())?;

    let mut series: Vec<(String, f64)> = Vec::new();
    for t in bucket_times_years.into_iter() {
        let label = if t < 1.0 {
            format!("{:.0}m", (t * 12.0).round())
        } else {
            format!("{:.0}y", t)
        };
        let bumped_disc = disc.with_key_rate_bump_years(t, bump_bp);
        let temp_ctx = base_ctx.clone().insert_discount(bumped_disc);
        let pv_bumped = revalue_with_context(&temp_ctx)?;
        // DV01 per bucket: PV change per 1bp move in this bucket
        let dv01 = (pv_bumped.amount() - base_pv.amount()) / bump_bp;
        series.push((label, dv01));
    }

    context.store_bucketed_series(base_metric_id, series.clone());
    let total: f64 = series.iter().map(|(_, v)| *v).sum();
    Ok(total)
}

// ===== Generic Calculators =====

use crate::instruments::common::traits::Instrument;
use crate::metrics::traits::MetricCalculator;
use std::marker::PhantomData;

// Re-export traits from pricing for convenience
pub use crate::instruments::common::pricing::{HasDiscountCurve, HasForwardCurves};

/// Generic BucketedDv01 calculator that works for any instrument implementing
/// the required traits.
///
/// Requires the instrument to implement `HasDiscountCurve`.
pub struct GenericBucketedDv01<I> {
    _phantom: PhantomData<I>,
}

impl<I> Default for GenericBucketedDv01<I> {
    fn default() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<I> MetricCalculator for GenericBucketedDv01<I>
where
    I: Instrument + crate::cashflow::traits::CashflowProvider + HasDiscountCurve + Clone + 'static,
{
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let instrument: &I = context.instrument_as()?;
        let discount_curve_id = instrument.discount_curve_id().clone();

        // Standard bucket times (years) - shared across all instruments
        let buckets = standard_ir_dv01_buckets();

        // Generic revaluation using cashflow building and discounting
        let inst_clone = instrument.clone();
        let curves = context.curves.clone();
        let as_of = context.as_of;

        let reval = move |
            bumped_disc: &finstack_core::market_data::term_structures::discount_curve::DiscountCurve|
         {
            // Build flows using original curves (preserves forward projections)
            let flows = inst_clone.build_schedule(&curves, as_of)?;
            let base = bumped_disc.base_date();
            let dc = bumped_disc.day_count();

            // Discount using bumped curve
            crate::instruments::common::discountable::npv_static(
                bumped_disc,
                base,
                dc,
                &flows,
            )
        };

        let total = compute_key_rate_dv01_series(context, &discount_curve_id, buckets, 1.0, reval)?;

        Ok(total)
    }
}

/// Alternative generic calculator for instruments that need full MarketContext revaluation.
///
/// Use this for instruments whose pricing requires access to multiple curves or
/// complex pricing models that can't be reduced to simple cashflow discounting.
pub struct GenericBucketedDv01WithContext<I> {
    _phantom: PhantomData<I>,
}

/// Generic parallel DV01 calculator that returns a scalar (not bucketed).
///
/// Computes DV01 by applying a parallel bump to the entire discount curve.
pub struct GenericParallelDv01<I> {
    _phantom: PhantomData<I>,
}

impl<I> Default for GenericParallelDv01<I> {
    fn default() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<I> MetricCalculator for GenericParallelDv01<I>
where
    I: Instrument + HasDiscountCurve + Clone + 'static,
{
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let instrument: &I = context.instrument_as()?;
        let discount_curve_id = instrument.discount_curve_id().clone();

        let inst_clone = instrument.clone();
        let as_of = context.as_of;
        let base_pv = context.base_value;
        let base_ctx = context.curves.as_ref();

        // Collect all curves to bump: discount curve + forward curves (if any)
        let mut curves_to_bump = vec![discount_curve_id.clone()];
        
        // Check if instrument has forward curves and add them
        if let Some(inst_with_fwd) = (instrument as &dyn std::any::Any).downcast_ref::<I>() {
            // Try to get forward curves using a helper that checks if I implements HasForwardCurves
            if let Some(fwd_curves) = get_forward_curves_if_available(inst_with_fwd) {
                curves_to_bump.extend(fwd_curves);
            }
        }

        // Bump all curves with the same parallel shift
        // Only bump curves that exist in the market context
        use finstack_core::market_data::context::BumpSpec;
        use hashbrown::HashMap;
        
        let mut bumps = HashMap::new();
        for curve_id in &curves_to_bump {
            // Check if the curve exists before trying to bump it
            // Discount curves we know exist (required by HasDiscountCurve)
            // Forward curves might not exist in all market setups
            if curve_id == &discount_curve_id {
                bumps.insert(curve_id.clone(), BumpSpec::parallel_bp(1.0));
            } else if base_ctx.get_forward_ref(curve_id.as_str()).is_ok() {
                bumps.insert(curve_id.clone(), BumpSpec::parallel_bp(1.0));
            }
            // Silently skip curves that don't exist in market context
        }
        
        let temp_ctx = base_ctx.bump(bumps)?;
        let pv_bumped = inst_clone.value(&temp_ctx, as_of)?;
        let dv01 = pv_bumped.amount() - base_pv.amount();

        Ok(dv01)
    }
}

/// Helper function to extract forward curves if the instrument implements HasForwardCurves.
/// Returns None if the instrument doesn't implement the trait.
fn get_forward_curves_if_available<I: 'static>(instrument: &I) -> Option<Vec<CurveId>> {
    // This is a bit of a workaround - we try to downcast to &dyn HasForwardCurves
    // If it succeeds, the instrument implements the trait
    let any_inst = instrument as &dyn std::any::Any;
    
    // We need to check each concrete type that implements both HasDiscountCurve and HasForwardCurves
    // This is not ideal but necessary without specialization
    
    // Try FRA
    if let Some(fra) = any_inst.downcast_ref::<crate::instruments::ForwardRateAgreement>() {
        return Some(fra.forward_curve_ids());
    }
    
    // Try IRS
    if let Some(irs) = any_inst.downcast_ref::<crate::instruments::InterestRateSwap>() {
        return Some(irs.forward_curve_ids());
    }
    
    // Try IR Future
    if let Some(irf) = any_inst.downcast_ref::<crate::instruments::InterestRateFuture>() {
        return Some(irf.forward_curve_ids());
    }
    
    // Try TRS variants
    if let Some(trs) = any_inst.downcast_ref::<crate::instruments::trs::EquityTotalReturnSwap>() {
        return Some(trs.forward_curve_ids());
    }
    if let Some(trs) = any_inst.downcast_ref::<crate::instruments::trs::FIIndexTotalReturnSwap>() {
        return Some(trs.forward_curve_ids());
    }
    
    // No forward curves
    None
}

impl<I> Default for GenericBucketedDv01WithContext<I> {
    fn default() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<I> MetricCalculator for GenericBucketedDv01WithContext<I>
where
    I: Instrument + HasDiscountCurve + Clone + 'static,
{
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let instrument: &I = context.instrument_as()?;
        let discount_curve_id = instrument.discount_curve_id().clone();

        // Standard bucket times
        let buckets = standard_ir_dv01_buckets();

        // Revaluation using full MarketContext (for complex pricers)
        let inst_clone = instrument.clone();
        let as_of = context.as_of;

        let reval = move |temp_ctx: &finstack_core::market_data::MarketContext| {
            inst_clone.value(temp_ctx, as_of)
        };

        let total = compute_key_rate_dv01_series_with_context(
            context,
            &discount_curve_id,
            buckets,
            1.0,
            reval,
        )?;

        Ok(total)
    }
}
