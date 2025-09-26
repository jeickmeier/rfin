//! Reusable helpers for bucketed risk metrics (DV01/CS01) using curve bumps.
//!
//! Provides a generic function to compute bucketed DV01 for instruments that
//! can be valued with discount and/or forward curves. Results are stored into
//! `MetricContext` via structured series using stable composite keys.

use crate::metrics::{MetricContext, MetricId};
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::CurveId;
use finstack_core::F;

/// Standard IR key-rate buckets in years used for quick demos/tests.
/// Example: [0.25, 0.5, 1, 2, 3, 5, 7, 10, 15, 20, 30]
pub fn standard_ir_dv01_buckets() -> Vec<F> {
    vec![0.25, 0.5, 1.0, 2.0, 3.0, 5.0, 7.0, 10.0, 15.0, 20.0, 30.0]
}

/// Compute bucketed DV01 series by bumping a specific discount curve in parallel bp
/// and revaluing the instrument for each bucket label.
///
/// - `context`    MetricContext with instrument/curves/date/base PV
/// - `disc_id`    Discount curve ID to bump
/// - `bucket_labels` Human-readable labels like "1m", "3m", "1y" used as keys
/// - `bump_bp`    Basis points bump magnitude (typically 1.0)
/// - `value_fn`   A closure that values the instrument given a MarketContext
///
/// Stores a 1D series under `MetricId::BucketedDv01` and returns the sum of bucket PV01s.
pub fn compute_bucketed_dv01_series<L, I, RevalFn>(
    context: &mut MetricContext,
    disc_id: &CurveId,
    bucket_labels: I,
    bump_bp: F,
    revalue_with_disc: RevalFn,
) -> finstack_core::Result<F>
where
    L: Into<String>,
    I: IntoIterator<Item = L>,
    RevalFn: FnMut(&DiscountCurve) -> finstack_core::Result<Money>,
{
    let base_id = MetricId::BucketedDv01;
    compute_bucketed_series_for_id(
        context,
        base_id,
        disc_id,
        bucket_labels,
        bump_bp,
        revalue_with_disc,
    )
}

/// Generic helper to compute a bucketed DV01-like series and store under a custom base metric ID.
pub fn compute_bucketed_series_for_id<L, I, RevalFn>(
    context: &mut MetricContext,
    base_metric_id: MetricId,
    disc_id: &CurveId,
    bucket_labels: I,
    bump_bp: F,
    mut revalue_with_disc: RevalFn,
) -> finstack_core::Result<F>
where
    L: Into<String>,
    I: IntoIterator<Item = L>,
    RevalFn: FnMut(&DiscountCurve) -> finstack_core::Result<Money>,
{
    let base_pv = context.base_value;
    let disc = context.curves.get_discount_ref(disc_id.as_str())?;

    let mut series: Vec<(String, F)> = Vec::new();
    for label in bucket_labels.into_iter() {
        let label_str: String = label.into();

        let bumped = disc.with_parallel_bump(bump_bp);
        let pv_bumped = revalue_with_disc(&bumped)?;
        let dv01 = (pv_bumped.amount() - base_pv.amount()) / 10_000.0;
        series.push((label_str, dv01));
    }

    context.store_bucketed_series(base_metric_id, series.clone());

    let total: F = series.iter().map(|(_, v)| *v).sum();
    Ok(total)
}

/// Compute bucketed DV01 series by bumping a specific discount curve and revaluing via a provided MarketContext.
///
/// This variant is useful when the instrument's pricing requires a full MarketContext rather than a raw DiscountCurve.
pub fn compute_bucketed_dv01_series_with_context<L, I, RevalFn>(
    context: &mut MetricContext,
    disc_id: &CurveId,
    bucket_labels: I,
    bump_bp: F,
    revalue_with_context: RevalFn,
) -> finstack_core::Result<F>
where
    L: Into<String>,
    I: IntoIterator<Item = L>,
    RevalFn: FnMut(&MarketContext) -> finstack_core::Result<Money>,
{
    compute_bucketed_series_with_context_for_id(
        context,
        MetricId::BucketedDv01,
        disc_id,
        bucket_labels,
        bump_bp,
        revalue_with_context,
    )
}

/// Generic helper to compute and store bucketed series under a chosen base metric id using full MarketContext revaluation.
pub fn compute_bucketed_series_with_context_for_id<L, I, RevalFn>(
    context: &mut MetricContext,
    base_metric_id: MetricId,
    disc_id: &CurveId,
    bucket_labels: I,
    bump_bp: F,
    mut revalue_with_context: RevalFn,
) -> finstack_core::Result<F>
where
    L: Into<String>,
    I: IntoIterator<Item = L>,
    RevalFn: FnMut(&MarketContext) -> finstack_core::Result<Money>,
{
    let base_pv = context.base_value;
    let base_ctx = context.curves.as_ref();
    let disc = base_ctx.get_discount_ref(disc_id.as_str())?;

    let mut series: Vec<(String, F)> = Vec::new();
    for label in bucket_labels.into_iter() {
        let label_str: String = label.into();

        let bumped = disc.with_parallel_bump(bump_bp);
        let temp_ctx = base_ctx.clone().insert_discount(bumped);
        let pv_bumped = revalue_with_context(&temp_ctx)?;
        let dv01 = (pv_bumped.amount() - base_pv.amount()) / 10_000.0;
        series.push((label_str, dv01));
    }

    context.store_bucketed_series(base_metric_id, series.clone());

    let total: F = series.iter().map(|(_, v)| *v).sum();
    Ok(total)
}
