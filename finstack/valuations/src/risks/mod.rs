#![allow(missing_docs)]

use finstack_core::prelude::*;
use finstack_core::F;
use finstack_core::market_data::traits::Discount;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
#[allow(unused_imports)]
use finstack_core::market_data::multicurve::CurveSet;
use hashbrown::HashMap;

#[derive(Clone, Debug)]
pub struct BucketSpec {
    pub tenors: Vec<F>, // in years from curve base date
}

#[derive(Clone, Debug, Default)]
pub struct Dv01Report {
    pub by_tenor: HashMap<String, F>,
    pub total: F,
}

/// Compute precise DV01 by bucketing flows into tenor buckets and bumping discount factors per bucket.
pub fn dv01_bucketed(
    flows: &[(Date, Money)],
    disc: &dyn Discount,
    dc: DayCount,
    base: Date,
    buckets: &BucketSpec,
) -> Dv01Report {
    // Centralized label formatter
    fn bucket_label(tenor_years: F) -> String { format!("{:.2}y", tenor_years) }

    // Precompute each flow's time and its assigned bucket index (argmin distance)
    // Also build per-bucket flow lists once.
    let mut idx_to_label: HashMap<usize, String> = HashMap::new();
    let mut map: HashMap<usize, Vec<(Date, Money)>> = HashMap::new();
    let mut total_flows: Vec<(Date, Money, F, usize)> = Vec::with_capacity(flows.len());
    for (d, m) in flows {
        let t = DiscountCurve::year_fraction(base, *d, dc).max(0.0);
        let (idx, _) = buckets
            .tenors
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| (*a - t).abs().partial_cmp(&(*b - t).abs()).unwrap())
            .unwrap();
        idx_to_label.entry(idx).or_insert_with(|| bucket_label(buckets.tenors[idx]));
        map.entry(idx).or_default().push((*d, *m));
        total_flows.push((*d, *m, t, idx));
    }

    // (no-op) retained spot for potential future helpers

    // Baseline PV (single pass, precompute base DFs and reuse)
    let mut base_pv = 0.0;
    let mut df_base: Vec<F> = Vec::with_capacity(total_flows.len());
    for (d, m, _t, _idx) in &total_flows {
        let df = DiscountCurve::df_on(disc, base, *d, dc);
        base_pv += m.amount() * df;
        df_base.push(df);
    }

    // Per-bucket DV01: +1bp shift only on that bucket's flows (proxy by scaling their discount factors)
    let bp = 1e-4;
    let mut report = Dv01Report::default();
    let mut total = 0.0;
    for (bucket_idx, _flows_in_bucket) in map.iter() {
        let mut bumped_pv = 0.0;
        for ((_, m, t, idx), df) in total_flows.iter().zip(df_base.iter()) {
            let df_bumped = if idx == bucket_idx { *df * (-bp * *t).exp() } else { *df };
            bumped_pv += m.amount() * df_bumped;
        }
        let dv01 = (base_pv - bumped_pv) / bp;
        let label = idx_to_label.get(bucket_idx).cloned().unwrap_or_else(|| bucket_label(buckets.tenors[*bucket_idx]));
        report.by_tenor.insert(label, dv01);
        total += dv01;
    }
    report.total = total;
    report
}


