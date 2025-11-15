//! DV01 (Dollar Value of a Basis Point) risk metric calculations.
//!
//! # Migration Notice
//!
//! **New users should use [`crate::metrics::UnifiedDv01Calculator`]** which provides
//! a cleaner, more flexible API with type-safe curve discovery via the 
//! [`crate::instruments::common::traits::CurveDependencies`] trait.
//!
//! This module contains legacy implementations maintained for backward compatibility.
//! The old implementation will be deprecated in a future release.
//!
//! ## Quick Migration Guide
//!
//! ```ignore
//! // Old approach:
//! let calculator = GenericBucketedDv01::<Bond>::default();
//! 
//! // New approach:
//! let calculator = UnifiedDv01Calculator::<Bond>::new(
//!     Dv01CalculatorConfig::key_rate()
//! );
//! ```
//!
//! # Legacy Documentation
//!
//! Provides generic functions to compute bucketed DV01 for instruments that
//! can be valued with discount and/or forward curves. Results are stored into
//! `MetricContext` via structured series using stable composite keys.
//!
//! # Important Implementation Note
//!
//! All DV01 calculation functions recalculate the base PV rather than using 
//! `context.base_value`. This is necessary to ensure consistency with bumped 
//! calculations and avoid potential inconsistencies that can arise from cached 
//! base values. This recalculation ensures accurate bucketed DV01 values.
//!
//! # Quick Start
//!
//! ## Example 1: Computing Key-Rate DV01 for a Bond
//!
//! ```ignore
//! use finstack_valuations::instruments::Bond;
//! use finstack_valuations::metrics::{
//!     compute_key_rate_dv01_series, standard_ir_dv01_buckets, MetricContext, MetricId
//! };
//! use finstack_core::dates::{create_date, Month};
//! use finstack_core::types::{CurveId, Rate, Currency};
//! use finstack_core::money::Money;
//! use finstack_core::market_data::MarketContext;
//! use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
//! use finstack_core::dates::day_count::DayCount;
//! use std::sync::Arc;
//!
//! # fn main() -> finstack_core::Result<()> {
//! // Setup: Create a 5-year bond
//! let as_of = create_date(2024, Month::January, 1)?;
//! let bond = Bond::builder("BOND-001")
//!     .issue_date(as_of)
//!     .maturity(create_date(2029, Month::January, 1)?)
//!     .coupon_rate(Rate::from_bps(500)) // 5.00% coupon
//!     .face_value(Money::new(100_000.0, Currency::USD))
//!     .build()?;
//!
//! // Create discount curve
//! let curve_id = CurveId::from("USD-OIS");
//! let discount_curve = DiscountCurve::builder(curve_id.clone())
//!     .base_date(as_of)
//!     .day_count(DayCount::Act365F)
//!     .knots(vec![
//!         (0.0, 1.0),
//!         (1.0, 0.96),
//!         (2.0, 0.93),
//!         (5.0, 0.85),
//!         (10.0, 0.70),
//!     ])
//!     .build()?;
//!
//! let market = MarketContext::new(as_of)
//!     .insert_discount(discount_curve);
//!
//! // Price the bond to get base PV
//! let base_value = bond.value(&market, as_of)?;
//!
//! // Create metric context
//! let mut context = MetricContext {
//!     instrument: &bond as &dyn finstack_valuations::instruments::common::traits::Instrument,
//!     curves: Arc::new(market),
//!     as_of,
//!     base_value,
//!     pricing_overrides: None,
//!     bucketed_series: Default::default(),
//!     structured_2d: Default::default(),
//!     structured_3d: Default::default(),
//! };
//!
//! // Standard buckets: [3M, 6M, 1Y, 2Y, 3Y, 5Y, 7Y, 10Y, 15Y, 20Y, 30Y]
//! let buckets = standard_ir_dv01_buckets();
//!
//! // Compute key-rate DV01 series
//! let total_dv01 = compute_key_rate_dv01_series(
//!     &mut context,
//!     &curve_id,
//!     buckets,
//!     1.0, // 1bp bump
//!     |bumped_curve| {
//!         // Rebuild cashflows with original market, discount with bumped curve
//!         use finstack_valuations::cashflow::traits::CashflowProvider;
//!         let flows = bond.build_schedule(context.curves.as_ref(), as_of)?;
//!         finstack_valuations::instruments::common::discountable::npv_static(
//!             bumped_curve,
//!             bumped_curve.base_date(),
//!             bumped_curve.day_count(),
//!             &flows,
//!         )
//!     }
//! )?;
//!
//! println!("Total DV01: ${:.2} per bp", total_dv01);
//!
//! // Access bucketed series
//! if let Some(series) = context.bucketed_series.get(&MetricId::BucketedDv01) {
//!     println!("\nKey-Rate DV01 Breakdown:");
//!     for (bucket, dv01) in series {
//!         println!("  {}: ${:.2} per bp", bucket, dv01);
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Example 2: Computing Parallel DV01 (Full Curve Bump)
//!
//! ```ignore
//! use finstack_valuations::instruments::Bond;
//! use finstack_valuations::metrics::{compute_parallel_dv01, MetricContext};
//! use finstack_core::dates::{create_date, Month};
//! use finstack_core::types::{CurveId, Rate, Currency};
//! use finstack_core::money::Money;
//! use finstack_core::market_data::MarketContext;
//! use std::sync::Arc;
//!
//! # fn main() -> finstack_core::Result<()> {
//! let as_of = create_date(2024, Month::January, 1)?;
//! let bond = Bond::builder("BOND-001")
//!     .issue_date(as_of)
//!     .maturity(create_date(2029, Month::January, 1)?)
//!     .coupon_rate(Rate::from_bps(500))
//!     .face_value(Money::new(100_000.0, Currency::USD))
//!     .build()?;
//!
//! // Setup market (abbreviated)
//! # use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
//! # use finstack_core::dates::day_count::DayCount;
//! # let curve_id = CurveId::from("USD-OIS");
//! # let discount_curve = DiscountCurve::builder(curve_id.clone())
//! #     .base_date(as_of)
//! #     .day_count(DayCount::Act365F)
//! #     .knots(vec![(0.0, 1.0), (5.0, 0.85)])
//! #     .build()?;
//! # let market = MarketContext::new(as_of).insert_discount(discount_curve);
//! let base_value = bond.value(&market, as_of)?;
//!
//! let mut context = MetricContext {
//!     instrument: &bond as &dyn finstack_valuations::instruments::common::traits::Instrument,
//!     curves: Arc::new(market),
//!     as_of,
//!     base_value,
//!     pricing_overrides: None,
//!     bucketed_series: Default::default(),
//!     structured_2d: Default::default(),
//!     structured_3d: Default::default(),
//! };
//!
//! // Parallel bump the entire curve by 1bp
//! let parallel_dv01 = compute_parallel_dv01(
//!     &mut context,
//!     &curve_id,
//!     1.0, // 1bp bump
//!     |bumped_curve| {
//!         // Reprice with bumped curve
//!         use finstack_valuations::cashflow::traits::CashflowProvider;
//!         let flows = bond.build_schedule(context.curves.as_ref(), as_of)?;
//!         finstack_valuations::instruments::common::discountable::npv_static(
//!             bumped_curve,
//!             bumped_curve.base_date(),
//!             bumped_curve.day_count(),
//!             &flows,
//!         )
//!     }
//! )?;
//!
//! println!("Parallel DV01: ${:.2} per bp", parallel_dv01);
//! // For a bond, DV01 is typically negative (price falls as rates rise)
//! # Ok(())
//! # }
//! ```
//!
//! ## Example 3: Multi-Curve DV01 (Interest Rate Swap)
//!
//! For instruments with multiple rate curves (e.g., IRS with separate discount and forward curves),
//! the `GenericBucketedDv01WithContext` calculator automatically computes DV01 for all relevant curves:
//!
//! ```ignore
//! use finstack_valuations::instruments::InterestRateSwap;
//! use finstack_valuations::metrics::{standard_registry, MetricId};
//! use finstack_core::dates::{create_date, Month};
//! use finstack_core::types::{Rate, Currency};
//! use finstack_core::money::Money;
//!
//! # fn main() -> finstack_core::Result<()> {
//! let as_of = create_date(2024, Month::January, 1)?;
//! let swap = InterestRateSwap::builder("SWAP-001")
//!     .start_date(as_of)
//!     .maturity(create_date(2029, Month::January, 1)?)
//!     .notional(Money::new(10_000_000.0, Currency::USD))
//!     .fixed_rate(Rate::from_bps(300))
//!     .is_receive_fixed(true)
//!     .build()?;
//!
//! // Setup market with discount and forward curves (abbreviated)
//! # use finstack_core::market_data::MarketContext;
//! # let market = MarketContext::new(as_of);
//!
//! let registry = standard_registry();
//! let metrics = vec![MetricId::BucketedDv01];
//!
//! let result = swap.price_with_metrics(&market, as_of, &metrics)?;
//!
//! // Total DV01 (sum across all curves and buckets)
//! if let Some(total) = result.measures.get(&MetricId::BucketedDv01) {
//!     println!("Total DV01: ${:.2} per bp", total);
//! }
//!
//! // Access per-curve bucketed series
//! // Example: "bucketed_dv01::USD-OIS::1y" for discount curve 1-year bucket
//! // Example: "bucketed_dv01::USD-LIBOR-3M::5y" for forward curve 5-year bucket
//! for (key, series) in &result.bucketed_series {
//!     println!("\n{}: {} entries", key, series.len());
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Key-Rate vs Parallel DV01
//!
//! - **Parallel DV01** (`compute_parallel_dv01`): Bumps the entire curve uniformly.
//!   - Use when measuring overall interest rate exposure.
//!   - Returns a single scalar value.
//!   - Faster to compute (single bump).
//!
//! - **Key-Rate DV01** (`compute_key_rate_dv01_series`): Bumps individual maturity points.
//!   - Use for hedging and understanding where risk is concentrated.
//!   - Returns a series of values (one per bucket).
//!   - More granular risk breakdown.
//!   - Sum of key-rate DV01s ≈ parallel DV01 (not exact due to curve interpolation).

use crate::metrics::{MetricContext, MetricId};
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::term_structures::ForwardCurve;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::CurveId;

// ===== Internal Helper Functions =====

/// Generate bucket label from years.
#[inline]
fn bucket_label(years: f64) -> String {
    if years < 1.0 {
        format!("{:.0}m", (years * 12.0).round())
    } else {
        format!("{:.0}y", years)
    }
}

/// Calculate DV01 from PV changes.
#[inline]
fn calculate_dv01(base_pv: Money, bumped_pv: Money, bump_bp: f64) -> f64 {
    (bumped_pv.amount() - base_pv.amount()) / bump_bp
}

/// Find the segment index containing time t for forward curve bumping.
fn find_forward_segment_index(knots: &[f64], t: f64) -> usize {
    if knots.len() < 2 {
        return 0;
    }
    
    if t <= knots[0] {
        0
    } else if t >= knots[knots.len() - 1] {
        knots.len() - 2
    } else {
        // Find segment [t_i, t_{i+1}] containing t
        for idx in 0..knots.len() - 1 {
            if t > knots[idx] && t <= knots[idx + 1] {
                return idx;
            }
        }
        0  // Fallback
    }
}

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
    let disc = context
        .curves
        .get_discount_ref(discount_curve_id.as_str())?;

    // Recalculate base PV (see module docs for rationale)
    let base_pv = revalue_with_disc(disc)?;
    
    // Parallel bump the entire curve
    let bumped = disc.try_with_parallel_bump(bump_bp)?;
    let pv_bumped = revalue_with_disc(&bumped)?;
    let dv01 = calculate_dv01(base_pv, pv_bumped, bump_bp);

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

    let base_ctx = context.curves.as_ref();
    
    // Recalculate base PV (see module docs for rationale)
    let base_pv = revalue_with_context(base_ctx)?;

    // Use the MarketContext.bump() method which correctly replaces curves under original IDs
    let mut bumps = HashMap::new();
    bumps.insert(discount_curve_id.clone(), BumpSpec::parallel_bp(bump_bp));
    let temp_ctx = base_ctx.bump(bumps)?;

    let pv_bumped = revalue_with_context(&temp_ctx)?;
    let dv01 = calculate_dv01(base_pv, pv_bumped, bump_bp);

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
    revalue_with_disc: RevalFn,
) -> finstack_core::Result<f64>
where
    I: IntoIterator<Item = f64>,
    RevalFn: FnMut(&DiscountCurve) -> finstack_core::Result<Money>,
{
    // Delegate to _for_id version with standard BucketedDv01 metric ID
    compute_key_rate_series_for_id(
        context,
        MetricId::BucketedDv01,
        discount_curve_id,
        bucket_times_years,
        bump_bp,
        revalue_with_disc,
    )
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
    revalue_with_context: RevalFn,
) -> finstack_core::Result<f64>
where
    I: IntoIterator<Item = f64>,
    RevalFn: FnMut(&MarketContext) -> finstack_core::Result<Money>,
{
    // Delegate to _for_id version with standard BucketedDv01 metric ID
    compute_key_rate_series_with_context_for_id(
        context,
        MetricId::BucketedDv01,
        discount_curve_id,
        bucket_times_years,
        bump_bp,
        revalue_with_context,
    )
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
    let disc = context
        .curves
        .get_discount_ref(discount_curve_id.as_str())?;
    
    // Recalculate base PV (see module docs for rationale)
    let base_pv = revalue_with_disc(disc)?;

    let mut series: Vec<(String, f64)> = Vec::new();
    for t in bucket_times_years.into_iter() {
        let label = bucket_label(t);
        let bumped = disc.try_with_key_rate_bump_years(t, bump_bp)?;
        let pv_bumped = revalue_with_disc(&bumped)?;
        let dv01 = calculate_dv01(base_pv, pv_bumped, bump_bp);
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
    let base_ctx = context.curves.as_ref();
    let disc = base_ctx.get_discount_ref(discount_curve_id.as_str())?;
    
    // Recalculate base PV (see module docs for rationale)
    let base_pv = revalue_with_context(base_ctx)?;

    let mut series: Vec<(String, f64)> = Vec::new();
    for t in bucket_times_years.into_iter() {
        let label = bucket_label(t);
        let bumped_disc = disc.try_with_key_rate_bump_years(t, bump_bp)?;
        let temp_ctx = base_ctx.clone().insert_discount(bumped_disc);
        let pv_bumped = revalue_with_context(&temp_ctx)?;
        let dv01 = calculate_dv01(base_pv, pv_bumped, bump_bp);
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
    let base_ctx = context.curves.as_ref();
    let fwd = base_ctx.get_forward_ref(forward_curve_id.as_str())?;
    
    // Recalculate base PV (see module docs for rationale)
    let base_pv = revalue_with_context(base_ctx)?;

    let mut series: Vec<(String, f64)> = Vec::new();
    let bump_rate = bump_bp / 10_000.0; // Convert bp to fraction

    for t in bucket_times_years.into_iter() {
        let label = bucket_label(t);

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
            let dv01 = calculate_dv01(base_pv, pv_bumped, bump_bp);
            series.push((label, dv01));
            continue;
        }

        // Find segment [t_i, t_{i+1}] containing t
        let seg_idx = find_forward_segment_index(knots, t);

        // Bump forward rates at and beyond the segment end (seg_idx+1 onwards)
        let bumped_rates: Vec<(f64, f64)> = knots
            .iter()
            .zip(forwards.iter())
            .enumerate()
            .map(|(idx, (&time, &rate))| {
                let new_rate = if idx > seg_idx {
                    rate + bump_rate
                } else {
                    rate
                };
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
        let dv01 = calculate_dv01(base_pv, pv_bumped, bump_bp);
        series.push((label, dv01));
    }

    context.store_bucketed_series(base_metric_id, series.clone());
    let total: f64 = series.iter().map(|(_, v)| *v).sum();
    Ok(total)
}

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
