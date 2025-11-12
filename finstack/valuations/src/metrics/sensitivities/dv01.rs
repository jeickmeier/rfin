//! Reusable helpers for bucketed risk metrics (DV01/CS01) using key-rate bumps.
//!
//! Provides a generic function to compute bucketed DV01 for instruments that
//! can be valued with discount and/or forward curves. Results are stored into
//! `MetricContext` via structured series using stable composite keys.

use crate::metrics::{MetricContext, MetricId};
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::term_structures::ForwardCurve;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::CurveId;

/// Identifies the type of rate curve for bucketed DV01 calculations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RatesCurveKind {
    /// Discount curve (used for present value discounting).
    Discount,
    /// Forward curve (used for floating rate projection).
    Forward,
}

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

/// Compute key-rate DV01 series for a forward curve using full MarketContext revaluation.
///
/// This function applies segment-localized key-rate bumps to a forward curve, similar to
/// how discount curves are bumped. For each bucket time, the forward rates at and beyond
/// the segment containing that time are shifted by the bump amount.
pub fn compute_key_rate_forward_series_with_context_for_id<I, RevalFn>(
    context: &mut MetricContext,
    base_metric_id: MetricId,
    forward_curve_id: &CurveId,
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
    let fwd = base_ctx.get_forward_ref(forward_curve_id.as_str())?;

    let mut series: Vec<(String, f64)> = Vec::new();
    let bump_rate = bump_bp / 10_000.0; // Convert bp to fraction

    for t in bucket_times_years.into_iter() {
        let label = if t < 1.0 {
            format!("{:.0}m", (t * 12.0).round())
        } else {
            format!("{:.0}y", t)
        };

        // Apply key-rate bump to forward curve
        // Similar to discount curve logic: find segment containing t, bump rates at and beyond
        let knots = fwd.knots();
        let forwards = fwd.forwards();

        if knots.len() < 2 {
            // Fallback to parallel bump for degenerate curves
            let bumped_rates: Vec<(f64, f64)> = knots
                .iter()
                .zip(forwards.iter())
                .map(|(&time, &rate)| (time, rate + bump_rate))
                .collect();

            let bumped_fwd = ForwardCurve::builder(forward_curve_id.clone(), fwd.tenor())
                .base_date(fwd.base_date())
                .reset_lag(fwd.reset_lag())
                .day_count(fwd.day_count())
                .knots(bumped_rates)
                .build()?;

            let temp_ctx = base_ctx.clone().insert_forward(bumped_fwd);
            let pv_bumped = revalue_with_context(&temp_ctx)?;
            let dv01 = (pv_bumped.amount() - base_pv.amount()) / bump_bp;
            series.push((label, dv01));
            continue;
        }

        // Find segment [t_i, t_{i+1}] containing t
        let mut seg_idx = 0usize;
        if t <= knots[0] {
            seg_idx = 0;
        } else if t >= knots[knots.len() - 1] {
            seg_idx = knots.len() - 2;
        } else {
            for idx in 0..knots.len() - 1 {
                if t > knots[idx] && t <= knots[idx + 1] {
                    seg_idx = idx;
                    break;
                }
            }
        }

        // Bump forward rates at and beyond the segment end (seg_idx+1 onwards)
        let bumped_rates: Vec<(f64, f64)> = knots
            .iter()
            .zip(forwards.iter())
            .enumerate()
            .map(|(idx, (&time, &rate))| {
                let new_rate = if idx > seg_idx { rate + bump_rate } else { rate };
                (time, new_rate)
            })
            .collect();

        let bumped_fwd = ForwardCurve::builder(forward_curve_id.clone(), fwd.tenor())
            .base_date(fwd.base_date())
            .reset_lag(fwd.reset_lag())
            .day_count(fwd.day_count())
            .knots(bumped_rates)
            .build()?;

        let temp_ctx = base_ctx.clone().insert_forward(bumped_fwd);
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

/// Mode for parallel DV01 calculation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParallelDv01Mode {
    /// Combined: bump all relevant curves (discount + forward + extra discount for FX) together; return scalar.
    Combined,
    /// Per-curve: bump each curve individually; store series under BucketedDv01; return sum.
    /// 
    /// TODO: Consider making DV01 return a series in a future major release (breaking change).
    PerCurve,
}

impl Default for ParallelDv01Mode {
    fn default() -> Self {
        Self::Combined
    }
}

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
/// Supports Combined (default) and Per-curve modes.
pub struct GenericParallelDv01<I> {
    mode: ParallelDv01Mode,
    _phantom: PhantomData<I>,
}

impl<I> Default for GenericParallelDv01<I> {
    fn default() -> Self {
        Self {
            mode: ParallelDv01Mode::Combined,
            _phantom: PhantomData,
        }
    }
}

impl<I> GenericParallelDv01<I> {
    /// Create with specified mode.
    pub fn with_mode(mode: ParallelDv01Mode) -> Self {
        Self {
            mode,
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
        let as_of = context.as_of;
        let base_pv = context.base_value;
        let base_ctx = context.curves.as_ref();

        // Get bump size from pricing overrides or default to 1.0 bp
        let bump_bp = context
            .pricing_overrides
            .as_ref()
            .and_then(|po| po.rate_bump_bp)
            .unwrap_or(1.0);

        // Collect all curves to bump: primary discount + additional discount (FX) + forward curves
        let mut curves_to_bump = vec![discount_curve_id.clone()];
        
        // Add additional discount curves (FX multi-curve)
        if let Some(inst_any) = (instrument as &dyn std::any::Any).downcast_ref::<I>() {
            let extra_discount = get_additional_discount_curves_if_available(inst_any);
            curves_to_bump.extend(extra_discount);
        }
        
        // Add forward curves if available
        if let Some(inst_any) = (instrument as &dyn std::any::Any).downcast_ref::<I>() {
            if let Some(fwd_curves) = get_forward_curves_if_available(inst_any) {
                curves_to_bump.extend(fwd_curves);
            }
        }

        // Filter curves that actually exist in market context
        use finstack_core::market_data::context::BumpSpec;
        use hashbrown::HashMap;
        
        let mut existing_curves = Vec::new();
        for curve_id in &curves_to_bump {
            // Check if curve exists (either discount or forward)
            if base_ctx.get_discount_ref(curve_id.as_str()).is_ok()
                || base_ctx.get_forward_ref(curve_id.as_str()).is_ok()
            {
                existing_curves.push(curve_id.clone());
            }
            // Silently skip curves that don't exist
        }

        // If no curves exist, return DV01 = 0
        if existing_curves.is_empty() {
            return Ok(0.0);
        }

        match self.mode {
            ParallelDv01Mode::Combined => {
                // Bump all existing curves together
                let mut bumps = HashMap::new();
                for curve_id in &existing_curves {
                    bumps.insert(curve_id.clone(), BumpSpec::parallel_bp(bump_bp));
                }
                
                let temp_ctx = base_ctx.bump(bumps)?;
                let inst_clone = instrument.clone();
                let pv_bumped = inst_clone.value(&temp_ctx, as_of)?;
                let dv01 = (pv_bumped.amount() - base_pv.amount()) / bump_bp;

                Ok(dv01)
            }
            ParallelDv01Mode::PerCurve => {
                // Bump each curve individually and store series
                let mut series = Vec::new();
                let mut total_dv01 = 0.0;

                for curve_id in &existing_curves {
                    let mut bumps = HashMap::new();
                    bumps.insert(curve_id.clone(), BumpSpec::parallel_bp(bump_bp));
                    
                    let temp_ctx = base_ctx.bump(bumps)?;
                    let inst_clone = instrument.clone();
                    let pv_bumped = inst_clone.value(&temp_ctx, as_of)?;
                    let dv01 = (pv_bumped.amount() - base_pv.amount()) / bump_bp;

                    series.push((curve_id.as_str().to_string(), dv01));
                    total_dv01 += dv01;
                }

                // Store per-curve series under BucketedDv01
                context.store_bucketed_series(MetricId::BucketedDv01, series);

                Ok(total_dv01)
            }
        }
    }
}

/// Collect all rate curves (discount and forward) relevant to an instrument.
///
/// Returns a vector of (CurveId, RatesCurveKind) tuples, filtered to only include
/// curves that exist in the provided MarketContext.
fn collect_rate_curves_for_instrument<I: 'static>(
    instrument: &I,
    primary_discount: &CurveId,
    market_ctx: &MarketContext,
) -> Vec<(CurveId, RatesCurveKind)> {
    let mut curves = Vec::new();

    // Primary discount curve
    if market_ctx.get_discount_ref(primary_discount.as_str()).is_ok() {
        curves.push((primary_discount.clone(), RatesCurveKind::Discount));
    }

    // Additional discount curves (FX instruments)
    let extra_discount = get_additional_discount_curves_if_available(instrument);
    for curve_id in extra_discount {
        if market_ctx.get_discount_ref(curve_id.as_str()).is_ok() {
            curves.push((curve_id, RatesCurveKind::Discount));
        }
    }

    // Forward curves
    if let Some(fwd_curves) = get_forward_curves_if_available(instrument) {
        for curve_id in fwd_curves {
            if market_ctx.get_forward_ref(curve_id.as_str()).is_ok() {
                curves.push((curve_id, RatesCurveKind::Forward));
            }
        }
    }

    curves
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
    
    // Try BasisSwap
    if let Some(bs) = any_inst.downcast_ref::<crate::instruments::BasisSwap>() {
        return Some(bs.forward_curve_ids());
    }
    
    // No forward curves
    None
}

/// Helper function to extract additional discount curves for multi-curve instruments (FX).
/// Returns empty vec if the instrument doesn't have extra discount curves.
fn get_additional_discount_curves_if_available<I: 'static>(instrument: &I) -> Vec<CurveId> {
    let any_inst = instrument as &dyn std::any::Any;
    
    // FxSwap has foreign discount curve in addition to domestic (primary)
    if let Some(fx_swap) = any_inst.downcast_ref::<crate::instruments::FxSwap>() {
        return vec![fx_swap.foreign_discount_curve_id.clone()];
    }
    
    // FxOption has foreign discount curve in addition to domestic (primary)
    if let Some(fx_option) = any_inst.downcast_ref::<crate::instruments::FxOption>() {
        return vec![fx_option.foreign_discount_curve_id.clone()];
    }
    
    // FxBarrierOption only has one discount curve (already the primary)
    // No additional curves for most instruments
    vec![]
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
        let as_of = context.as_of;

        // Get bump size from pricing overrides or default to 1.0 bp
        let bump_bp = context
            .pricing_overrides
            .as_ref()
            .and_then(|po| po.rate_bump_bp)
            .unwrap_or(1.0);

        // Standard bucket times
        let buckets = standard_ir_dv01_buckets();

        // Clone instrument once before collecting curves
        let inst_clone = instrument.clone();

        // Collect all curves relevant to this instrument
        let curves_to_bump = collect_rate_curves_for_instrument(
            instrument,
            &discount_curve_id,
            context.curves.as_ref(),
        );

        // If no curves exist, return DV01 = 0
        if curves_to_bump.is_empty() {
            return Ok(0.0);
        }

        let mut total_dv01 = 0.0;

        // Compute bucketed DV01 per curve
        for (curve_id, curve_kind) in curves_to_bump {
            let inst_for_curve = inst_clone.clone();
            let reval = move |temp_ctx: &finstack_core::market_data::MarketContext| {
                inst_for_curve.value(temp_ctx, as_of)
            };

            // Create custom metric ID for this curve's series
            let curve_metric_id = MetricId::custom(format!("bucketed_dv01::{}", curve_id.as_str()));

            let curve_total = match curve_kind {
                RatesCurveKind::Discount => {
                    compute_key_rate_series_with_context_for_id(
                        context,
                        curve_metric_id.clone(),
                        &curve_id,
                        buckets.clone(),
                        bump_bp,
                        reval,
                    )?
                }
                RatesCurveKind::Forward => {
                    compute_key_rate_forward_series_with_context_for_id(
                        context,
                        curve_metric_id.clone(),
                        &curve_id,
                        buckets.clone(),
                        bump_bp,
                        reval,
                    )?
                }
            };

            total_dv01 += curve_total;

            // Also store primary discount curve under standard BucketedDv01 key for BC
            if curve_id == discount_curve_id && curve_kind == RatesCurveKind::Discount {
                // Retrieve the series we just stored and re-store under standard key
                if let Some(series) = context.get_series(&curve_metric_id) {
                    context.store_bucketed_series(MetricId::BucketedDv01, series.clone());
                }
            }
        }

        Ok(total_dv01)
    }
}
