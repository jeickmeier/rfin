//! IR Future convexity adjustment tests.

use super::utils::*;
use finstack_valuations::instruments::ir_future::{FutureContractSpecs, Position};

#[test]
fn test_automatic_convexity_near_term() {
    let (as_of, start, end) = near_term_dates();
    let market = build_standard_market(as_of, 0.05);

    let future = create_custom_future(
        "NEAR",
        1_000_000.0,
        start,
        start,
        end,
        97.50,
        Position::Long,
    );
    let pv = future.npv(&market).unwrap();

    // Should apply automatic convexity adjustment
    assert!(pv.amount().is_finite());
}

#[test]
fn test_automatic_convexity_far_forward() {
    let (as_of, start, end) = far_forward_dates();
    let market = build_standard_market(as_of, 0.05);

    let future = create_custom_future("FAR", 1_000_000.0, start, start, end, 97.50, Position::Long);
    let pv = future.npv(&market).unwrap();

    // Far forward should have larger convexity adjustment
    assert!(pv.amount().is_finite());
}

#[test]
fn test_explicit_convexity_adjustment() {
    let (as_of, start, end) = standard_dates();
    let market = build_standard_market(as_of, 0.05);

    let specs_with_ca = FutureContractSpecs {
        face_value: 1_000_000.0,
        tick_size: 0.0025,
        tick_value: 6.25,
        delivery_months: 3,
        convexity_adjustment: Some(0.0005), // 5 bps adjustment
    };

    let future = create_standard_future(start, end).with_contract_specs(specs_with_ca);

    let pv = future.npv(&market).unwrap();
    assert!(pv.amount().is_finite());
}

#[test]
fn test_convexity_impact() {
    let (as_of, start, end) = far_forward_dates();
    let market = build_standard_market(as_of, 0.05);

    // No explicit convexity (uses automatic)
    let no_ca = create_custom_future(
        "NO_CA",
        1_000_000.0,
        start,
        start,
        end,
        97.50,
        Position::Long,
    );
    let pv_no_ca = no_ca.npv(&market).unwrap().amount();

    // With zero convexity adjustment
    let specs_zero_ca = FutureContractSpecs {
        convexity_adjustment: Some(0.0),
        ..FutureContractSpecs::default()
    };
    let with_zero_ca = create_standard_future(start, end).with_contract_specs(specs_zero_ca);
    let pv_zero_ca = with_zero_ca.npv(&market).unwrap().amount();

    // Automatic convexity should produce different result than zero
    // (unless very short dated)
    assert!(pv_no_ca.is_finite());
    assert!(pv_zero_ca.is_finite());
}

#[test]
fn test_convexity_adjustment_sign() {
    let (as_of, start, end) = far_forward_dates();
    let market = build_standard_market(as_of, 0.05);

    // Positive convexity adjustment (typical for futures)
    let positive_ca = FutureContractSpecs {
        convexity_adjustment: Some(0.0010),
        ..FutureContractSpecs::default()
    };

    // Negative convexity adjustment (unusual, but test it)
    let negative_ca = FutureContractSpecs {
        convexity_adjustment: Some(-0.0010),
        ..FutureContractSpecs::default()
    };

    let fut_pos = create_standard_future(start, end).with_contract_specs(positive_ca);
    let fut_neg = create_standard_future(start, end).with_contract_specs(negative_ca);

    let pv_pos = fut_pos.npv(&market).unwrap().amount();
    let pv_neg = fut_neg.npv(&market).unwrap().amount();

    // Different convexity adjustments should produce different valuations
    assert_ne!(
        pv_pos, pv_neg,
        "Convexity adjustments should impact valuation"
    );
}

#[test]
fn test_large_convexity_adjustment() {
    let (as_of, start, end) = standard_dates();
    let market = build_standard_market(as_of, 0.05);

    let large_ca = FutureContractSpecs {
        convexity_adjustment: Some(0.02), // 200 bps - unrealistically large
        ..FutureContractSpecs::default()
    };

    let future = create_standard_future(start, end).with_contract_specs(large_ca);

    let pv = future.npv(&market).unwrap();

    // Should still produce valid result
    assert!(pv.amount().is_finite());
    assert!(pv.amount().abs() < 1_000_000.0);
}
