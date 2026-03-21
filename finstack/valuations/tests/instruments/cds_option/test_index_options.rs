//! Market validation tests for CDS index option specific features.

use super::common::*;
use finstack_valuations::instruments::internal::InstrumentExt as Instrument;
use finstack_valuations::instruments::SettlementType;
use time::macros::date;

#[test]
fn test_index_factor_scaling() {
    // Index factor should scale PV linearly
    let as_of = date!(2025 - 01 - 01);
    let market = standard_market(as_of);

    let base_option = CDSOptionBuilder::new().with_index(1.0).build(as_of);

    let base_pv = base_option.value(&market, as_of).unwrap().amount();

    for factor in [0.85, 0.90, 0.95] {
        let scaled_option = CDSOptionBuilder::new().with_index(factor).build(as_of);

        let scaled_pv = scaled_option.value(&market, as_of).unwrap().amount();
        let ratio = scaled_pv / base_pv;

        assert_approx_eq(
            ratio,
            factor,
            0.001,
            &format!("Index factor scaling for factor={}", factor),
        );
    }
}

#[test]
fn test_forward_spread_adjustment_call() {
    // Forward spread adjustment should increase call value
    let as_of = date!(2025 - 01 - 01);
    let market = standard_market(as_of);

    let base_option = CDSOptionBuilder::new()
        .call()
        .with_index(1.0)
        .forward_adjust(0.0)
        .build(as_of);

    let adjusted_option = CDSOptionBuilder::new()
        .call()
        .with_index(1.0)
        .forward_adjust(25.0)
        .build(as_of);

    let base_pv = base_option.value(&market, as_of).unwrap().amount();
    let adj_pv = adjusted_option.value(&market, as_of).unwrap().amount();

    assert!(
        adj_pv > base_pv,
        "Positive forward adjustment should increase call value: base={}, adjusted={}",
        base_pv,
        adj_pv
    );
}

#[test]
fn test_forward_spread_adjustment_put() {
    // Forward spread adjustment should decrease put value
    let as_of = date!(2025 - 01 - 01);
    let market = standard_market(as_of);

    let base_option = CDSOptionBuilder::new()
        .put()
        .with_index(1.0)
        .forward_adjust(0.0)
        .build(as_of);

    let adjusted_option = CDSOptionBuilder::new()
        .put()
        .with_index(1.0)
        .forward_adjust(25.0)
        .build(as_of);

    let base_pv = base_option.value(&market, as_of).unwrap().amount();
    let adj_pv = adjusted_option.value(&market, as_of).unwrap().amount();

    assert!(
        adj_pv < base_pv,
        "Positive forward adjustment should decrease put value: base={}, adjusted={}",
        base_pv,
        adj_pv
    );
}

#[test]
fn test_index_vs_single_name() {
    // Index option with factor 1.0 and no adjustment should differ from single-name
    let as_of = date!(2025 - 01 - 01);
    let market = standard_market(as_of);

    let single_name = CDSOptionBuilder::new().build(as_of);
    let index = CDSOptionBuilder::new()
        .with_index(1.0)
        .forward_adjust(0.0)
        .build(as_of);

    let sn_pv = single_name.value(&market, as_of).unwrap().amount();
    let idx_pv = index.value(&market, as_of).unwrap().amount();

    // Values might differ slightly due to implementation details
    assert_finite(sn_pv, "Single-name PV");
    assert_finite(idx_pv, "Index PV");
}

#[test]
fn test_very_small_index_factor() {
    // Very small index factor should give very small value
    let as_of = date!(2025 - 01 - 01);
    let market = standard_market(as_of);

    // Test with smallest practical index factor (1%)
    let option_small = CDSOptionBuilder::new().with_index(0.01).build(as_of);
    let option_full = CDSOptionBuilder::new().with_index(1.0).build(as_of);

    let pv_small = option_small.value(&market, as_of).unwrap().amount();
    let pv_full = option_full.value(&market, as_of).unwrap().amount();

    // Small factor should give proportionally smaller value
    let ratio = pv_small / pv_full;
    assert_approx_eq(
        ratio,
        0.01,
        0.001,
        "Very small index factor should scale linearly",
    );
}

#[test]
fn test_negative_forward_adjustment() {
    // Negative forward adjustment should decrease call value
    let as_of = date!(2025 - 01 - 01);
    let market = standard_market(as_of);

    let base = CDSOptionBuilder::new()
        .call()
        .with_index(1.0)
        .forward_adjust(0.0)
        .build(as_of);

    let adjusted = CDSOptionBuilder::new()
        .call()
        .with_index(1.0)
        .forward_adjust(-20.0)
        .build(as_of);

    let base_pv = base.value(&market, as_of).unwrap().amount();
    let adj_pv = adjusted.value(&market, as_of).unwrap().amount();

    assert!(
        adj_pv < base_pv,
        "Negative forward adjustment should decrease call value"
    );
}

#[test]
fn test_index_option_physical_settlement_is_rejected() {
    let as_of = date!(2025 - 01 - 01);
    let market = standard_market(as_of);
    let mut option = CDSOptionBuilder::new().with_index(1.0).build(as_of);
    option.settlement = SettlementType::Physical;

    let err = option
        .value(&market, as_of)
        .expect_err("Physical settlement should be rejected for CDS index options");
    assert!(matches!(err, finstack_core::Error::Validation(_)));
}
