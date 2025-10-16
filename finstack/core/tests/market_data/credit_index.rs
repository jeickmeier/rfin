use std::collections::HashMap;
use std::sync::Arc;

use super::common::{sample_base_correlation_curve, sample_hazard_curve};
use finstack_core::market_data::term_structures::credit_index::{
    CreditIndexData, CreditIndexDataBuilder,
};

#[test]
fn credit_index_builder_validates_inputs() {
    let hazard = Arc::new(sample_hazard_curve("CDX"));
    let base_corr = Arc::new(sample_base_correlation_curve("CDX-BC"));

    let data = CreditIndexData::builder()
        .num_constituents(125)
        .recovery_rate(0.4)
        .index_credit_curve(hazard.clone())
        .base_correlation_curve(base_corr.clone())
        .build()
        .expect("valid credit index");

    assert_eq!(data.num_constituents, 125);
    assert!(!data.has_issuer_curves());
    assert!(Arc::ptr_eq(&data.index_credit_curve, &hazard));
    assert!(Arc::ptr_eq(&data.base_correlation_curve, &base_corr));
}

#[test]
fn credit_index_builder_supports_issuer_curves() {
    let hazard = Arc::new(sample_hazard_curve("CDX"));
    let base_corr = Arc::new(sample_base_correlation_curve("CDX-BC"));

    let mut issuers: HashMap<String, Arc<_>> = HashMap::new();
    issuers.insert(
        "IssuerA".to_string(),
        Arc::new(sample_hazard_curve("IssuerA")),
    );

    let data = CreditIndexDataBuilder::default()
        .num_constituents(1)
        .recovery_rate(0.35)
        .index_credit_curve(hazard.clone())
        .base_correlation_curve(base_corr.clone())
        .with_issuer_curves(issuers.clone())
        .build()
        .expect("builder with issuer curves");

    assert!(data.has_issuer_curves());
    assert_eq!(data.issuer_ids(), vec!["IssuerA".to_string()]);

    // Test that we get the right curves - compare IDs since we can't directly compare references
    let issuer_curve = data.get_issuer_curve("IssuerA");
    assert_eq!(issuer_curve.id().as_str(), "IssuerA");

    // Unknown issuer should fall back to index curve
    let unknown_curve = data.get_issuer_curve("Unknown");
    assert_eq!(unknown_curve.id().as_str(), "CDX");
}

#[test]
fn credit_index_builder_validates_bad_input() {
    let hazard = Arc::new(sample_hazard_curve("CDX"));
    let base_corr = Arc::new(sample_base_correlation_curve("CDX-BC"));

    let err = CreditIndexData::builder()
        .recovery_rate(-0.1)
        .index_credit_curve(hazard)
        .base_correlation_curve(base_corr)
        .build()
        .expect_err("invalid recovery should fail");
    assert!(matches!(err, finstack_core::Error::Input(_)));
}
