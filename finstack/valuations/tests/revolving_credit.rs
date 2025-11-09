//! Comprehensive integration tests for revolving credit facilities.

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount, Frequency};
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::instruments::revolving_credit::{
    BaseRateSpec, DrawRepayEvent, DrawRepaySpec, RevolvingCredit, RevolvingCreditFees,
};
use finstack_valuations::metrics::MetricId;
use time::macros::date;

fn build_flat_discount_curve(rate: f64, base_date: Date, curve_id: &str) -> DiscountCurve {
    DiscountCurve::builder(curve_id)
        .base_date(base_date)
        .day_count(DayCount::Act360)
        .knots([
            (0.0, 1.0),
            (1.0, (-rate).exp()),
            (5.0, (-rate * 5.0).exp()),
            (10.0, (-rate * 10.0).exp()),
            (30.0, (-rate * 30.0).exp()),
        ])
        .build()
        .unwrap()
}

#[test]
fn test_revolving_credit_basic_pricing() {
    let val_date = date!(2025 - 01 - 01);
    let commitment_date = date!(2025 - 01 - 01);
    let maturity_date = date!(2026 - 01 - 01);

    let facility = RevolvingCredit::builder()
        .id("RC-001".into())
        .commitment_amount(Money::new(10_000_000.0, Currency::USD))
        .drawn_amount(Money::new(5_000_000.0, Currency::USD))
        .commitment_date(commitment_date)
        .maturity_date(maturity_date)
        .base_rate_spec(BaseRateSpec::Fixed { rate: 0.05 })
        .day_count(DayCount::Act360)
        .payment_frequency(Frequency::quarterly())
        .fees({
            let mut fees = RevolvingCreditFees::flat(25.0, 10.0, 5.0);
            fees.upfront_fee = Some(Money::new(50_000.0, Currency::USD));
            fees
        })
        .draw_repay_spec(DrawRepaySpec::Deterministic(vec![]))
        .discount_curve_id("USD-OIS".into())
        .build()
        .unwrap();

    let disc_curve = build_flat_discount_curve(0.03, val_date, "USD-OIS");
    let market = MarketContext::new().insert_discount(disc_curve);

    let pv = facility.value(&market, val_date).unwrap();

    // PV from lender's perspective includes:
    // - Interest and fees received (positive)
    // - Principal repaid at maturity (positive, ~5M)
    // The net PV should be close to the drawn amount plus net carry

    // Rough estimate:
    // Principal repaid: 5M @ 97% DF ~= 4.85M
    // Interest: 5M * 0.05 * 97% DF = 242.5k
    // Fees: ~22.5k * 97% DF = 21.8k
    // Upfront: 50k
    // Total ~= 5.16M
    assert!(
        pv.amount() > 4_800_000.0 && pv.amount() < 5_500_000.0,
        "PV should include principal repayment, got {}",
        pv.amount()
    );
}

#[test]
fn test_revolving_credit_with_draws_and_repayments() {
    let val_date = date!(2025 - 01 - 01);
    let commitment_date = date!(2025 - 01 - 01);
    let maturity_date = date!(2026 - 01 - 01);

    let facility = RevolvingCredit::builder()
        .id("RC-002".into())
        .commitment_amount(Money::new(10_000_000.0, Currency::USD))
        .drawn_amount(Money::new(3_000_000.0, Currency::USD))
        .commitment_date(commitment_date)
        .maturity_date(maturity_date)
        .base_rate_spec(BaseRateSpec::Fixed { rate: 0.04 })
        .day_count(DayCount::Act360)
        .payment_frequency(Frequency::quarterly())
        .fees(RevolvingCreditFees::flat(20.0, 0.0, 0.0))
        .draw_repay_spec(DrawRepaySpec::Deterministic(vec![
            DrawRepayEvent {
                date: date!(2025 - 04 - 01),
                amount: Money::new(2_000_000.0, Currency::USD),
                is_draw: true, // Draw 2M
            },
            DrawRepayEvent {
                date: date!(2025 - 07 - 01),
                amount: Money::new(1_000_000.0, Currency::USD),
                is_draw: false, // Repay 1M
            },
        ]))
        .discount_curve_id("USD-OIS".into())
        .build()
        .unwrap();

    let disc_curve = build_flat_discount_curve(0.03, val_date, "USD-OIS");
    let market = MarketContext::new().insert_discount(disc_curve);

    let pv = facility.value(&market, val_date).unwrap();

    // From lender's perspective: principal repaid + interest/fees
    // Net drawn balance at maturity after events: 3M + 2M - 1M = 4M
    // PV should include this principal repayment plus carry
    assert!(
        pv.amount() > 3_000_000.0,
        "PV should include principal repayment, got {}",
        pv.amount()
    );
}

#[test]
fn test_revolving_credit_utilization_metrics() {
    let val_date = date!(2025 - 01 - 01);
    let commitment_date = date!(2025 - 01 - 01);
    let maturity_date = date!(2026 - 01 - 01);

    let facility = RevolvingCredit::builder()
        .id("RC-003".into())
        .commitment_amount(Money::new(10_000_000.0, Currency::USD))
        .drawn_amount(Money::new(6_000_000.0, Currency::USD)) // 60% utilization
        .commitment_date(commitment_date)
        .maturity_date(maturity_date)
        .base_rate_spec(BaseRateSpec::Fixed { rate: 0.05 })
        .day_count(DayCount::Act360)
        .payment_frequency(Frequency::quarterly())
        .fees(RevolvingCreditFees::flat(25.0, 10.0, 5.0))
        .draw_repay_spec(DrawRepaySpec::Deterministic(vec![]))
        .discount_curve_id("USD-OIS".into())
        .build()
        .unwrap();

    let disc_curve = build_flat_discount_curve(0.03, val_date, "USD-OIS");
    let market = MarketContext::new().insert_discount(disc_curve);

    // Test utilization rate metric
    let result = facility
        .price_with_metrics(
            &market,
            val_date,
            &[
                MetricId::custom("utilization_rate"),
                MetricId::custom("available_capacity"),
                MetricId::custom("weighted_average_cost"),
            ],
        )
        .unwrap();

    // Check utilization rate
    let utilization = result.measures.get("utilization_rate").unwrap();
    assert!(
        (utilization - 0.6).abs() < 1e-6,
        "Utilization should be 60%"
    );

    // Check available capacity
    let capacity = result.measures.get("available_capacity").unwrap();
    assert!(
        (capacity - 4_000_000.0).abs() < 1.0,
        "Available capacity should be 4M"
    );

    // Check weighted average cost is computed
    let wac = result.measures.get("weighted_average_cost").unwrap();
    assert!(*wac > 0.0, "Weighted average cost should be positive");
}

#[test]
fn test_revolving_credit_standard_metrics() {
    let val_date = date!(2025 - 01 - 01);
    let commitment_date = date!(2025 - 01 - 01);
    let maturity_date = date!(2027 - 01 - 01); // 2 years

    let facility = RevolvingCredit::builder()
        .id("RC-004".into())
        .commitment_amount(Money::new(5_000_000.0, Currency::USD))
        .drawn_amount(Money::new(3_000_000.0, Currency::USD))
        .commitment_date(commitment_date)
        .maturity_date(maturity_date)
        .base_rate_spec(BaseRateSpec::Fixed { rate: 0.06 })
        .day_count(DayCount::Act360)
        .payment_frequency(Frequency::quarterly())
        .fees(RevolvingCreditFees::flat(30.0, 15.0, 10.0))
        .draw_repay_spec(DrawRepaySpec::Deterministic(vec![]))
        .discount_curve_id("USD-OIS".into())
        .build()
        .unwrap();

    let disc_curve = build_flat_discount_curve(0.04, val_date, "USD-OIS");
    let market = MarketContext::new().insert_discount(disc_curve);

    // Test standard metrics
    let result = facility
        .price_with_metrics(
            &market,
            val_date,
            &[MetricId::Dv01, MetricId::Cs01, MetricId::Theta],
        )
        .unwrap();

    // DV01 should be positive and significant
    let dv01 = result.measures.get("dv01").unwrap();
    println!("DV01: {}", dv01);
    println!("Base PV: {}", result.value.amount());
    assert!(*dv01 > 0.0, "DV01 should be positive, got {}", dv01);

    // CS01 should be similar to DV01 for this instrument
    let cs01 = result.measures.get("cs01").unwrap();
    assert!(*cs01 > 0.0, "CS01 should be positive");
    assert!(
        (dv01 - cs01).abs() < 100.0,
        "DV01 and CS01 should be similar"
    );

    // Theta (1-day time decay) - for a lending position with positive carry,
    // theta can be positive (earning interest/fees) or negative depending on
    // the relationship between earned carry and discount rate effects
    let theta = result.measures.get("theta").unwrap();
    println!("Theta: {}", theta);
    // Just verify theta is computed
    assert!(theta.is_finite(), "Theta should be a finite number");
}

#[test]
fn test_revolving_credit_bucketed_dv01() {
    let val_date = date!(2025 - 01 - 01);
    let commitment_date = date!(2025 - 01 - 01);
    let maturity_date = date!(2028 - 01 - 01); // 3 years

    let facility = RevolvingCredit::builder()
        .id("RC-005".into())
        .commitment_amount(Money::new(20_000_000.0, Currency::USD))
        .drawn_amount(Money::new(10_000_000.0, Currency::USD))
        .commitment_date(commitment_date)
        .maturity_date(maturity_date)
        .base_rate_spec(BaseRateSpec::Fixed { rate: 0.055 })
        .day_count(DayCount::Act360)
        .payment_frequency(Frequency::quarterly())
        .fees(RevolvingCreditFees::flat(25.0, 10.0, 5.0))
        .draw_repay_spec(DrawRepaySpec::Deterministic(vec![]))
        .discount_curve_id("USD-OIS".into())
        .build()
        .unwrap();

    let disc_curve = build_flat_discount_curve(0.04, val_date, "USD-OIS");
    let market = MarketContext::new().insert_discount(disc_curve);

    // Test bucketed DV01
    let result = facility
        .price_with_metrics(&market, val_date, &[MetricId::BucketedDv01])
        .unwrap();

    // Should have bucketed DV01 metric
    let bucketed_total = result.measures.get("bucketed_dv01").unwrap();
    println!("Bucketed DV01: {}", bucketed_total);
    // Bucketed DV01 total should be finite
    assert!(
        bucketed_total.is_finite(),
        "Total bucketed DV01 should be finite, got {}",
        bucketed_total
    );
}

#[test]
fn test_revolving_credit_helpers() {
    let val_date = date!(2025 - 01 - 01);
    let facility = RevolvingCredit::builder()
        .id("RC-006".into())
        .commitment_amount(Money::new(10_000_000.0, Currency::USD))
        .drawn_amount(Money::new(7_500_000.0, Currency::USD))
        .commitment_date(val_date)
        .maturity_date(date!(2026 - 01 - 01))
        .base_rate_spec(BaseRateSpec::Fixed { rate: 0.05 })
        .day_count(DayCount::Act360)
        .payment_frequency(Frequency::quarterly())
        .fees(RevolvingCreditFees::default())
        .draw_repay_spec(DrawRepaySpec::Deterministic(vec![]))
        .discount_curve_id("USD-OIS".into())
        .build()
        .unwrap();

    // Test helper methods
    assert_eq!(facility.utilization_rate(), 0.75);
    assert!(facility.is_deterministic());
    assert!(!facility.is_stochastic());

    let undrawn = facility.undrawn_amount().unwrap();
    assert_eq!(undrawn.amount(), 2_500_000.0);
}

#[test]
fn test_term_forward_with_floor() {
    use finstack_core::market_data::term_structures::ForwardCurve;
    
    let val_date = date!(2025 - 01 - 01);
    let commitment_date = date!(2025 - 01 - 01);
    let maturity_date = date!(2025 - 07 - 01);

    // Build forward curve with very low rates that will be affected by floor
    // Use small positive rates that become negative after subtracting margin
    let fwd_curve = ForwardCurve::builder("USD-SOFR-3M", 0.25)
        .base_date(val_date)
        .day_count(DayCount::Act360)
        .knots([
            (0.0, 0.0001),  // 1 bp (very low)
            (0.5, 0.0001),
            (1.0, 0.0002),
        ])
        .build()
        .unwrap();

    let disc_curve = build_flat_discount_curve(0.03, val_date, "USD-OIS");
    
    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve);

    // Facility with floor at 0 bps
    let facility_with_floor = RevolvingCredit::builder()
        .id("RC-FLOOR".into())
        .commitment_amount(Money::new(1_000_000.0, Currency::USD))
        .drawn_amount(Money::new(1_000_000.0, Currency::USD))
        .commitment_date(commitment_date)
        .maturity_date(maturity_date)
        .base_rate_spec(BaseRateSpec::Floating {
            index_id: "USD-SOFR-3M".into(),
            margin_bp: 500.0, // +500 bps margin = +5%
            reset_freq: Frequency::quarterly(),
            floor_bp: Some(100.0), // 1% floor on base rate (floors 1bp to 1%)
        })
        .day_count(DayCount::Act360)
        .payment_frequency(Frequency::quarterly())
        .fees(RevolvingCreditFees::default())
        .draw_repay_spec(DrawRepaySpec::Deterministic(vec![]))
        .discount_curve_id("USD-OIS".into())
        .build()
        .unwrap();

    // Facility without floor
    let facility_no_floor = RevolvingCredit::builder()
        .id("RC-NO-FLOOR".into())
        .commitment_amount(Money::new(1_000_000.0, Currency::USD))
        .drawn_amount(Money::new(1_000_000.0, Currency::USD))
        .commitment_date(commitment_date)
        .maturity_date(maturity_date)
        .base_rate_spec(BaseRateSpec::Floating {
            index_id: "USD-SOFR-3M".into(),
            margin_bp: 500.0, // +500 bps margin = +5%
            reset_freq: Frequency::quarterly(),
            floor_bp: None, // No floor, so 1bp base passes through
        })
        .day_count(DayCount::Act360)
        .payment_frequency(Frequency::quarterly())
        .fees(RevolvingCreditFees::default())
        .draw_repay_spec(DrawRepaySpec::Deterministic(vec![]))
        .discount_curve_id("USD-OIS".into())
        .build()
        .unwrap();

    let pv_with_floor = facility_with_floor.value(&market, val_date).unwrap();
    let pv_no_floor = facility_no_floor.value(&market, val_date).unwrap();

    // With floor: max(0.01%, 1%) + 5% margin = 1% + 5% = 6% (borrower pays 6%)
    // Without floor: 0.01% + 5% margin = 5.01% (borrower pays 5.01%)
    // With floor, the borrower pays more interest, so from lender perspective PV is higher
    assert!(pv_with_floor.amount() > pv_no_floor.amount(), 
        "Floor should increase PV (lender receives more interest). With floor: {}, Without floor: {}",
        pv_with_floor.amount(), pv_no_floor.amount());
}

#[test]
fn test_overdraw_validation() {
    let val_date = date!(2025 - 01 - 01);
    let commitment_date = date!(2025 - 01 - 01);
    let maturity_date = date!(2026 - 01 - 01);

    let disc_curve = build_flat_discount_curve(0.03, val_date, "USD-OIS");
    let market = MarketContext::new().insert_discount(disc_curve);

    // Create a facility with a draw that would exceed commitment
    let facility = RevolvingCredit::builder()
        .id("RC-OVERDRAW".into())
        .commitment_amount(Money::new(1_000_000.0, Currency::USD))
        .drawn_amount(Money::new(500_000.0, Currency::USD))
        .commitment_date(commitment_date)
        .maturity_date(maturity_date)
        .base_rate_spec(BaseRateSpec::Fixed { rate: 0.05 })
        .day_count(DayCount::Act360)
        .payment_frequency(Frequency::quarterly())
        .fees(RevolvingCreditFees::default())
        .draw_repay_spec(DrawRepaySpec::Deterministic(vec![
            DrawRepayEvent {
                date: date!(2025 - 03 - 01),
                amount: Money::new(600_000.0, Currency::USD), // This would take us to 1.1M > 1M commitment
                is_draw: true,
            },
        ]))
        .discount_curve_id("USD-OIS".into())
        .build()
        .unwrap();

    // This should error due to overdraw
    let result = facility.value(&market, val_date);
    assert!(result.is_err(), "Should error on overdraw");
    
    if let Err(e) = result {
        let err_msg = format!("{}", e);
        assert!(err_msg.contains("exceed commitment") || err_msg.contains("Validation"),
            "Error should mention exceeding commitment, got: {}", err_msg);
    }
}
