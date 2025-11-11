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
        let dv01 = (pv_bumped.amount() - base_pv.amount()) / 10_000.0;
        series.push((label, dv01));
    }

    context.store_bucketed_series(MetricId::BucketedDv01, series.clone());
    let total: f64 = series.iter().map(|(_, v)| *v).sum();
    Ok(total)
}

/// Key-rate DV01 series using full MarketContext revaluation per bucket time.
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
        let bumped_disc = disc.with_key_rate_bump_years(t, bump_bp);
        let temp_ctx = base_ctx.clone().insert_discount(bumped_disc);
        let pv_bumped = revalue_with_context(&temp_ctx)?;
        let dv01 = (pv_bumped.amount() - base_pv.amount()) / 10_000.0;
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
        let dv01 = (pv_bumped.amount() - base_pv.amount()) / 10_000.0;
        series.push((label, dv01));
    }

    context.store_bucketed_series(base_metric_id, series.clone());
    let total: f64 = series.iter().map(|(_, v)| *v).sum();
    Ok(total)
}
