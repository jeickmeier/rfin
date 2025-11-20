use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount, Frequency};
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::term_structures::hazard_curve::HazardCurve;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::instruments::revolving_credit::{
    BaseRateSpec, DrawRepaySpec, RevolvingCredit, RevolvingCreditFees,
};
use time::Month;

#[test]
fn test_pricing_recovery_consistency() {
    // Scenario: 1-year fully drawn facility (bullet loan behavior).
    // Risk-free rate (r) = 5%.
    // Market Spread (s) = 2% (200 bps).
    // Recovery Rate (R) = 40%.
    // Coupon = r + s = 7%.
    //
    // Market Standard Expectation:
    // A bond paying r+s, with market spread s, should price to Par (1.0) 
    // (assuming s compensates for credit risk perfectly).
    
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let maturity = Date::from_calendar_date(2026, Month::January, 1).unwrap();
    
    // 1. Create Facility
    let commitment = Money::new(10_000_000.0, Currency::USD);
    let drawn = commitment; // Fully drawn
    
    let facility = RevolvingCredit::builder()
        .id("TEST-RCF".into())
        .commitment_amount(commitment)
        .drawn_amount(drawn)
        .commitment_date(as_of)
        .maturity_date(maturity)
        .base_rate_spec(BaseRateSpec::Fixed { rate: 0.07 }) // 7% Coupon
        .day_count(DayCount::Act365F)
        .payment_frequency(Frequency::annual()) // Single payment at end
        .fees(RevolvingCreditFees::default()) // No extra fees
        .draw_repay_spec(DrawRepaySpec::Deterministic(vec![])) // No changes
        .discount_curve_id("USD-OIS".into())
        .hazard_curve_id_opt(Some("BORROWER-HAZARD".into()))
        .recovery_rate(0.40)
        .attributes(Default::default())
        .build()
        .unwrap();

    // 2. Create Market Data
    // Discount Curve: Flat 5% (Continuous)
    // DF(t) = exp(-0.05 * t)
    let r: f64 = 0.05;
    let discount_curve = DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .day_count(DayCount::Act365F)
        .knots(vec![
            (0.0, 1.0),
            (10.0, (-r * 10.0).exp())
        ])
        .build()
        .unwrap();
    
    // Hazard Curve: Flat h = s / (1-R) = 0.02 / 0.6 = 0.033333
    let hazard_rate = 0.02 / 0.60;
    let hazard_curve = HazardCurve::builder("BORROWER-HAZARD")
        .base_date(as_of)
        .day_count(DayCount::Act365F)
        .knots(vec![(0.0, hazard_rate), (10.0, hazard_rate)])
        .build()
        .unwrap();

    let mut market = MarketContext::new();
    market = market.insert_discount(discount_curve);
    market = market.insert_hazard(hazard_curve);

    // 3. Price
    let pv = facility.value(&market, as_of).unwrap();
    let price_pct = pv.amount() / commitment.amount();
    
    println!("Price %: {:.6}", price_pct);
    
    // Check expectations
    // Implementation logic was: 1.07 * exp(-(r+h))
    let expected_impl = 1.07 * (-(r + hazard_rate)).exp();
    // Market Par logic is: 1.07 * exp(-(r+s))
    let expected_market = 1.07 * (-(r + 0.02)).exp();
    
    println!("Expected (Old Logic): {:.6}", expected_impl);
    println!("Expected (Market Par): {:.6}", expected_market);
    
    // Assert that we now MATCH Market Par
    assert!((price_pct - expected_market).abs() < 1e-4, "Price should match Market Par after fix. Got {}, Expected {}", price_pct, expected_market);
}
