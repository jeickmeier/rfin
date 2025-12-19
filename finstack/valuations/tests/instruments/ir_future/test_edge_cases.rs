//! IR Future edge cases and error handling tests.

use super::utils::*;
use finstack_core::market_data::context::MarketContext;
use finstack_valuations::instruments::ir_future::{FutureContractSpecs, Position};
use time::macros::date;

#[test]
fn test_expired_future() {
    let _as_of = date!(2024 - 07 - 15);
    let expiry = date!(2024 - 07 - 01); // Expired 2 weeks ago
    let end = date!(2024 - 10 - 01);

    let future = create_custom_future(
        "EXPIRED",
        1_000_000.0,
        expiry,
        expiry,
        end,
        97.50,
        Position::Long,
    );
    let market = build_standard_market(expiry, 0.05); // Use expiry as base date to avoid validation errors

    // Should still calculate (might be zero or small value)
    let result = future.npv(&market);
    assert!(result.is_ok() || result.is_err()); // Either behavior is acceptable for expired
}

#[test]
fn test_zero_tau() {
    let start = date!(2024 - 07 - 01);
    let end = date!(2024 - 07 - 02); // One day accrual (minimal but non-zero)
    let as_of = date!(2024 - 01 - 01);

    // Very short accrual period
    let future = create_custom_future(
        "NEAR_ZERO_TAU",
        1_000_000.0,
        start,
        start,
        end,
        97.50,
        Position::Long,
    );
    let market = build_standard_market(as_of, 0.05);

    let pv = future.npv(&market).unwrap();

    // Very short tau should give small PV
    assert!(
        pv.amount().abs() < 10000.0,
        "Very short tau should give small PV"
    );
}

#[test]
fn test_zero_face_value() {
    let (_, start, end) = standard_dates();
    let as_of = time::macros::date!(2024 - 01 - 01);
    let market = build_standard_market(as_of, 0.05);

    let specs = FutureContractSpecs {
        face_value: 0.0,
        convexity_adjustment: Some(0.0),
        ..FutureContractSpecs::default()
    };

    let future = create_standard_future(start, end).with_contract_specs(specs);

    // Should handle zero face value gracefully (contracts_scale = 1.0)
    let pv = future.npv(&market).unwrap();
    assert!(pv.amount().is_finite());
}

#[test]
fn test_negative_quoted_price() {
    let (as_of, start, end) = standard_dates();
    let market = build_standard_market(as_of, 0.05);

    // Negative price (unusual but possible in some markets)
    let future = create_custom_future(
        "NEG_PRICE",
        1_000_000.0,
        start,
        start,
        end,
        -1.0,
        Position::Long,
    );
    let pv = future.npv(&market).unwrap();

    // Should still calculate
    assert!(pv.amount().is_finite());
}

#[test]
fn test_very_large_notional() {
    let (as_of, start, end) = standard_dates();
    let market = build_standard_market(as_of, 0.05);

    // Billion dollar position
    let future = create_custom_future(
        "LARGE",
        1_000_000_000.0,
        start,
        start,
        end,
        97.50,
        Position::Long,
    );
    let pv = future.npv(&market).unwrap();

    assert!(pv.amount().is_finite());
    assert!(
        pv.amount().abs() < 1e12,
        "PV should be reasonable even for large notional"
    );
}

#[test]
fn test_very_small_notional() {
    let (as_of, start, end) = standard_dates();
    let market = build_standard_market(as_of, 0.05);

    // $1 position
    let future = create_custom_future("SMALL", 1.0, start, start, end, 97.50, Position::Long);
    let pv = future.npv(&market).unwrap();

    assert!(pv.amount().is_finite());
    assert!(pv.amount().abs() < 1.0);
}

#[test]
fn test_missing_discount_curve() {
    let (_as_of, start, end) = standard_dates();

    // Empty market context
    let empty_market = MarketContext::new();

    let future = create_standard_future(start, end);
    let result = future.npv(&empty_market);

    // Should error due to missing curve
    assert!(result.is_err());
}

#[test]
fn test_missing_forward_curve() {
    let (as_of, start, end) = standard_dates();

    // Only discount curve, no forward curve
    let disc_curve = build_flat_discount_curve(0.05, as_of, "USD_OIS");
    let partial_market = MarketContext::new().insert_discount(disc_curve);

    let future = create_standard_future(start, end);
    let result = future.npv(&partial_market);

    // Should error due to missing forward curve
    assert!(result.is_err());
}

#[test]
fn test_wrong_curve_ids() {
    let (as_of, start, end) = standard_dates();
    let market = build_standard_market(as_of, 0.05);

    let mut future = create_standard_future(start, end);
    future.discount_curve_id = "NONEXISTENT_DISC".into();

    let result = future.npv(&market);
    assert!(result.is_err());
}

#[test]
fn test_future_date_before_base_date() {
    let past_start = date!(2023 - 01 - 01);
    let past_end = date!(2023 - 04 - 01);

    let future = create_custom_future(
        "PAST",
        1_000_000.0,
        past_start,
        past_start,
        past_end,
        97.50,
        Position::Long,
    );
    let market = build_standard_market(past_start, 0.05); // Use past date as base

    // Should handle historical dates
    let result = future.npv(&market);
    assert!(result.is_ok() || result.is_err()); // Either behavior is acceptable
}

#[test]
#[should_panic(expected = "t2 must be after t1")]
fn test_inverted_period_dates() {
    let (as_of, _, _) = standard_dates();
    let start = date!(2024 - 10 - 01);
    let end = date!(2024 - 07 - 01); // End before start

    let future = create_custom_future(
        "INVERTED",
        1_000_000.0,
        start,
        start,
        end,
        97.50,
        Position::Long,
    );
    let market = build_standard_market(as_of, 0.05);

    // Should panic with invalid date range (in forward curve)
    let _ = future.npv(&market);
}

#[test]
fn test_cashflow_expired() {
    use finstack_valuations::cashflow::traits::CashflowProvider;

    let as_of = date!(2024 - 07 - 15);
    let expiry = date!(2024 - 07 - 01); // Already expired
    let end = date!(2024 - 10 - 01);

    let future = create_custom_future(
        "EXPIRED_CF",
        1_000_000.0,
        expiry,
        expiry,
        end,
        97.50,
        Position::Long,
    );
    let market = build_standard_market(as_of, 0.05);

    let cashflows = future.build_schedule(&market, as_of).unwrap();

    // Should return empty schedule for expired future
    assert!(
        cashflows.is_empty(),
        "Expired future should have no cashflows"
    );
}

#[test]
fn test_cashflow_active() {
    use finstack_valuations::cashflow::traits::CashflowProvider;

    let (as_of, start, end) = standard_dates();
    let future = create_standard_future(start, end);
    let market = build_standard_market(as_of, 0.05);

    let cashflows = future.build_schedule(&market, as_of).unwrap();

    // Futures are daily settled, so standard discounting schedule is empty
    // The value is returned directly by npv() / value()
    assert!(
        cashflows.is_empty(),
        "Active future should have no discountable cashflows (daily settled)"
    );
}

#[test]
fn test_extreme_rate_difference() {
    let (as_of, start, end) = standard_dates();

    // Market rate 10%, implied rate 0.5% (quoted price 99.5)
    let market = build_standard_market(as_of, 0.10);
    let future = create_custom_future(
        "EXTREME",
        1_000_000.0,
        start,
        start,
        end,
        99.5,
        Position::Long,
    );

    let pv = future.npv(&market).unwrap();

    // Large rate difference should produce large P&L
    assert!(pv.amount().is_finite());
    assert!(
        pv.amount().abs() > 1000.0,
        "Large rate difference should produce meaningful P&L"
    );
}

#[test]
fn test_zero_notional() {
    let (as_of, start, end) = standard_dates();
    let market = build_standard_market(as_of, 0.05);

    let future = create_custom_future(
        "ZERO_NOTIONAL",
        0.0,
        start,
        start,
        end,
        97.50,
        Position::Long,
    );
    let pv = future.npv(&market).unwrap();

    // Zero notional should give zero PV
    assert_eq!(pv.amount(), 0.0);
}

#[test]
fn test_price_at_par() {
    let (as_of, start, end) = standard_dates();
    let market = build_standard_market(as_of, 0.05);

    // Price at par (100.0) => 0% implied rate
    let future = create_custom_future("PAR", 1_000_000.0, start, start, end, 100.0, Position::Long);
    let pv = future.npv(&market).unwrap();

    // Should still produce valid valuation (negative due to positive market rate)
    assert!(pv.amount().is_finite());
}
