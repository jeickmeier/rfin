//! FxSpot BucketedDv01 smoke tests

use crate::fx_spot::common::{market_full, sample_eurusd, test_date};
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::metrics::MetricId;

#[test]
fn test_fx_spot_bucketed_dv01_computed() {
    let as_of = test_date();
    let fx_spot = sample_eurusd();
    let market = market_full();

    let result = fx_spot
        .price_with_metrics(&market, as_of, &[MetricId::BucketedDv01])
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
}

