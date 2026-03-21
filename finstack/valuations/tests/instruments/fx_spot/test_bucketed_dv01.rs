//! FxSpot BucketedDv01 smoke tests

use crate::fx_spot::common::{market_full, sample_eurusd, test_date};
use finstack_valuations::instruments::internal::InstrumentExt as Instrument;
use finstack_valuations::metrics::MetricId;

fn sum_bucketed_dv01(result: &finstack_valuations::results::ValuationResult) -> f64 {
    result
        .measures
        .iter()
        .filter(|(id, _)| id.as_str().starts_with("bucketed_dv01::"))
        .map(|(_, v)| *v)
        .sum()
}

#[test]
fn test_fx_spot_bucketed_dv01_computed() {
    let as_of = test_date();
    let fx_spot = sample_eurusd();
    let market = market_full();

    let result = fx_spot
        .price_with_metrics(
            &market,
            as_of,
            &[MetricId::BucketedDv01, MetricId::Dv01],
            finstack_valuations::instruments::PricingOptions::default(),
        )
        .unwrap();

    // BucketedDv01 should be present
    assert!(
        result.measures.contains_key("bucketed_dv01"),
        "BucketedDv01 should be computed"
    );

    let bucketed_dv01 = *result.measures.get("bucketed_dv01").unwrap();
    assert!(
        bucketed_dv01.is_finite(),
        "BucketedDv01 should be finite, got {}",
        bucketed_dv01
    );

    let dv01 = *result.measures.get("dv01").unwrap();
    let bucket_sum = sum_bucketed_dv01(&result);
    let diff = (bucket_sum - dv01).abs();
    let tol = 1e-6_f64.max(1e-3 * dv01.abs());
    assert!(
        diff < tol,
        "Sum of bucketed DV01 should match parallel DV01: bucket_sum={}, dv01={}, diff={}",
        bucket_sum,
        dv01,
        diff
    );
}
