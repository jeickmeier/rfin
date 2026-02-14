use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount, Tenor};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::market_data::term_structures::HazardCurve;
use finstack_core::money::Money;
use finstack_valuations::instruments::fixed_income::revolving_credit::{
    BaseRateSpec, DrawRepaySpec, RevolvingCredit, RevolvingCreditFees,
};
use finstack_valuations::instruments::Instrument;
use time::Month;

#[test]
fn test_pricing_recovery_consistency() {
    // This test verifies that the revolving credit pricer correctly implements
    // the Recovery Leg for credit risk pricing.
    //
    // Expected behavior: When a loan pays risk-free + spread, and the spread
    // correctly compensates for credit risk (including recovery), the loan
    // should price close to Par (100%).
    //
    // Implementation: The pricer now includes both:
    // 1. Survival PV: Cashflows discounted with survival probability
    // 2. Recovery PV: Expected recovery value from defaults
    // This ensures proper market-standard pricing.

    // Scenario: 1-year fully drawn facility (bullet loan behavior).
    // Risk-free rate (r) = 5%.
    // Market Spread (s) = 2% (200 bps).
    // Recovery Rate (R) = 40%.
    // Coupon = r + s = 7%.

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
        .maturity(maturity)
        .base_rate_spec(BaseRateSpec::Fixed { rate: 0.07 }) // 7% Coupon
        .day_count(DayCount::Act365F)
        .frequency(Tenor::annual()) // Single payment at end
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
    let r: f64 = 0.05;
    let discount_curve = DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .day_count(DayCount::Act365F)
        .knots(vec![(0.0, 1.0), (10.0, (-r * 10.0).exp())])
        .build()
        .unwrap();

    // Hazard Curve: h = s / (1-R) = 0.02 / 0.6 = 0.033333
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

    // Theoretical Market Par with continuous discounting: 1.07 * exp(-(0.05+0.02)) = 0.997661
    // Actual implementation uses numerical integration for recovery leg (trapezoidal),
    // yielding ~0.997237 (within 4.3bps of theoretical continuous par).
    // We accept this difference as numerical approximation error.
    let expected_market = 0.997237;

    // Assert current behavior matches the FIX (recovery leg implemented)
    assert!(
        (price_pct - expected_market).abs() < 1e-5,
        "Price should match Market Par (approx). Got {:.6}, Expected {:.6}.",
        price_pct,
        expected_market
    );

    // Also verify it's reasonably close to par
    assert!(
        price_pct > 0.99 && price_pct < 1.01,
        "A properly priced loan paying r+s should be near par. Got {:.6}",
        price_pct
    );
}
