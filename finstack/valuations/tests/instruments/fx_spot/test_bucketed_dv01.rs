//! FxSpot BucketedDv01 registry tests.

use crate::fx_spot::common::{market_full, sample_eurusd, test_date};
use finstack_valuations::instruments::Instrument;
use finstack_valuations::metrics::MetricId;

#[test]
fn test_fx_spot_bucketed_dv01_not_applicable() {
    let result = sample_eurusd().price_with_metrics(
        &market_full(),
        test_date(),
        &[MetricId::BucketedDv01, MetricId::Dv01],
        finstack_valuations::instruments::PricingOptions::default(),
    );

    let result = result.expect("FxSpot should still price without rate DV01 metrics");
    assert!(
        !result.measures.contains_key("bucketed_dv01"),
        "FxSpot should not emit BucketedDv01"
    );
    assert!(
        !result.measures.contains_key("dv01"),
        "FxSpot should not emit Dv01"
    );
}
