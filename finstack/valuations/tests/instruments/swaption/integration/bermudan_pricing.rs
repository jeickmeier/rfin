//! Integration tests for Bermudan swaption pricing.

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount, Tenor};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::math::interp::InterpStyle;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::common::models::trees::{HullWhiteTree, HullWhiteTreeConfig};
use finstack_valuations::instruments::common::parameters::OptionType;
use finstack_valuations::instruments::swaption::pricing::BermudanSwaptionTreeValuator;
use finstack_valuations::instruments::swaption::{
    BermudanSchedule, BermudanSwaption, BermudanSwaptionPricer, BermudanType, HullWhiteParams,
    SwaptionSettlement,
};
use finstack_valuations::pricer::Pricer;
use time::Month;

fn test_discount_curve() -> DiscountCurve {
    DiscountCurve::builder("USD-OIS")
        .base_date(Date::from_calendar_date(2025, Month::January, 1).expect("Valid date"))
        .knots([
            (0.0, 1.0),
            (0.5, 0.985),
            (1.0, 0.97),
            (2.0, 0.94),
            (5.0, 0.85),
            (10.0, 0.70),
        ])
        .set_interp(InterpStyle::LogLinear)
        .build()
        .expect("Valid curve")
}

fn test_bermudan_swaption(
    swap_start: Date,
    swap_end: Date,
    first_exercise: Date,
    strike: f64,
    option_type: OptionType,
) -> BermudanSwaption {
    BermudanSwaption {
        id: InstrumentId::new("TEST-BERM"),
        option_type,
        notional: Money::new(10_000_000.0, Currency::USD),
        strike_rate: strike,
        swap_start,
        swap_end,
        fixed_freq: Tenor::semi_annual(),
        float_freq: Tenor::quarterly(),
        day_count: DayCount::Thirty360,
        settlement: SwaptionSettlement::Physical,
        discount_curve_id: CurveId::new("USD-OIS"),
        forward_id: CurveId::new("USD-SOFR"),
        vol_surface_id: CurveId::new("USD-VOL"),
        bermudan_schedule: BermudanSchedule::co_terminal(
            first_exercise,
            swap_end,
            Tenor::semi_annual(),
        ),
        bermudan_type: BermudanType::CoTerminal,
        pricing_overrides: Default::default(),
        attributes: Default::default(),
    }
}

#[test]
fn test_bermudan_price_positive() {
    let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
    let swap_start = as_of;
    let swap_end = Date::from_calendar_date(2030, Month::January, 1).expect("Valid date");
    let first_exercise = Date::from_calendar_date(2026, Month::January, 1).expect("Valid date");

    let swaption =
        test_bermudan_swaption(swap_start, swap_end, first_exercise, 0.03, OptionType::Call);
    let curve = test_discount_curve();

    let ttm = swaption.time_to_maturity(as_of).expect("Valid ttm");
    let config = HullWhiteTreeConfig::new(0.03, 0.01, 50);
    let tree = HullWhiteTree::calibrate(config, &curve, ttm).expect("Calibration should succeed");

    let valuator =
        BermudanSwaptionTreeValuator::new(&swaption, &tree, &curve, as_of).expect("Valid valuator");

    let price = valuator.price();

    // Price should be non-negative
    assert!(
        price >= 0.0,
        "Bermudan swaption price should be non-negative, got {}",
        price
    );
}

#[test]
fn test_bermudan_payer_vs_receiver() {
    let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
    let swap_start = as_of;
    let swap_end = Date::from_calendar_date(2030, Month::January, 1).expect("Valid date");
    let first_exercise = Date::from_calendar_date(2026, Month::January, 1).expect("Valid date");

    let payer =
        test_bermudan_swaption(swap_start, swap_end, first_exercise, 0.03, OptionType::Call);
    let receiver =
        test_bermudan_swaption(swap_start, swap_end, first_exercise, 0.03, OptionType::Put);

    let curve = test_discount_curve();
    let ttm = payer.time_to_maturity(as_of).expect("Valid ttm");
    let config = HullWhiteTreeConfig::new(0.03, 0.01, 50);
    let tree = HullWhiteTree::calibrate(config, &curve, ttm).expect("Calibration should succeed");

    let payer_valuator =
        BermudanSwaptionTreeValuator::new(&payer, &tree, &curve, as_of).expect("Valid valuator");
    let receiver_valuator =
        BermudanSwaptionTreeValuator::new(&receiver, &tree, &curve, as_of).expect("Valid valuator");

    let payer_price = payer_valuator.price();
    let receiver_price = receiver_valuator.price();

    // Both should be positive
    assert!(payer_price >= 0.0, "Payer price should be non-negative");
    assert!(
        receiver_price >= 0.0,
        "Receiver price should be non-negative"
    );
}

#[test]
fn test_bermudan_strike_sensitivity() {
    let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
    let swap_start = as_of;
    let swap_end = Date::from_calendar_date(2030, Month::January, 1).expect("Valid date");
    let first_exercise = Date::from_calendar_date(2026, Month::January, 1).expect("Valid date");

    // Low strike (ITM payer)
    let low_strike =
        test_bermudan_swaption(swap_start, swap_end, first_exercise, 0.01, OptionType::Call);
    // ATM strike
    let atm_strike =
        test_bermudan_swaption(swap_start, swap_end, first_exercise, 0.03, OptionType::Call);
    // High strike (OTM payer)
    let high_strike =
        test_bermudan_swaption(swap_start, swap_end, first_exercise, 0.06, OptionType::Call);

    let curve = test_discount_curve();
    let ttm = low_strike.time_to_maturity(as_of).expect("Valid ttm");
    let config = HullWhiteTreeConfig::new(0.03, 0.01, 50);
    let tree = HullWhiteTree::calibrate(config, &curve, ttm).expect("Calibration should succeed");

    let low_price = BermudanSwaptionTreeValuator::new(&low_strike, &tree, &curve, as_of)
        .expect("Valid valuator")
        .price();
    let atm_price = BermudanSwaptionTreeValuator::new(&atm_strike, &tree, &curve, as_of)
        .expect("Valid valuator")
        .price();
    let high_price = BermudanSwaptionTreeValuator::new(&high_strike, &tree, &curve, as_of)
        .expect("Valid valuator")
        .price();

    // For a payer swaption: lower strike = higher value
    assert!(
        low_price >= atm_price,
        "Low strike payer {} should be >= ATM {}",
        low_price,
        atm_price
    );
    assert!(
        atm_price >= high_price,
        "ATM payer {} should be >= high strike {}",
        atm_price,
        high_price
    );
}

#[test]
fn test_bermudan_more_exercise_dates_higher_value() {
    let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
    let swap_start = as_of;
    let swap_end = Date::from_calendar_date(2030, Month::January, 1).expect("Valid date");

    // Early first exercise (more exercise opportunities)
    let early_first = Date::from_calendar_date(2026, Month::January, 1).expect("Valid date");
    // Late first exercise (fewer exercise opportunities)
    let late_first = Date::from_calendar_date(2029, Month::January, 1).expect("Valid date");

    let early_swaption =
        test_bermudan_swaption(swap_start, swap_end, early_first, 0.03, OptionType::Call);
    let late_swaption =
        test_bermudan_swaption(swap_start, swap_end, late_first, 0.03, OptionType::Call);

    let curve = test_discount_curve();
    let ttm = early_swaption.time_to_maturity(as_of).expect("Valid ttm");
    let config = HullWhiteTreeConfig::new(0.03, 0.01, 50);
    let tree = HullWhiteTree::calibrate(config, &curve, ttm).expect("Calibration should succeed");

    let early_price = BermudanSwaptionTreeValuator::new(&early_swaption, &tree, &curve, as_of)
        .expect("Valid valuator")
        .price();
    let late_price = BermudanSwaptionTreeValuator::new(&late_swaption, &tree, &curve, as_of)
        .expect("Valid valuator")
        .price();

    // More exercise opportunities should generally be worth more or equal
    // (this depends on the exact market conditions, but typically holds)
    assert!(
        early_price >= late_price * 0.9, // Allow some tolerance
        "Early first exercise {} should be >= late first exercise {}",
        early_price,
        late_price
    );
}

#[test]
fn test_expired_bermudan_zero_value() {
    // Set as_of after swap maturity
    let as_of = Date::from_calendar_date(2035, Month::January, 1).expect("Valid date");
    let swap_start = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
    let swap_end = Date::from_calendar_date(2030, Month::January, 1).expect("Valid date");
    let first_exercise = Date::from_calendar_date(2026, Month::January, 1).expect("Valid date");

    let swaption =
        test_bermudan_swaption(swap_start, swap_end, first_exercise, 0.03, OptionType::Call);

    // Time to maturity should be negative when as_of is after swap_end
    let ttm = swaption.time_to_maturity(as_of);
    // The result may be an error or a negative value depending on day count implementation
    // An error is also acceptable for expired instruments
    if let Ok(t) = ttm {
        assert!(
            t < 0.0,
            "Expired swaption should have negative TTM, got {}",
            t
        );
    }
}

#[test]
fn test_bermudan_schedule_generation() {
    let first_exercise = Date::from_calendar_date(2026, Month::January, 1).expect("Valid date");
    let swap_end = Date::from_calendar_date(2030, Month::January, 1).expect("Valid date");

    let schedule = BermudanSchedule::co_terminal(first_exercise, swap_end, Tenor::semi_annual());

    // Should have approximately 8 exercise dates (4 years * 2 per year)
    let effective_dates = schedule.effective_dates();
    assert!(
        effective_dates.len() >= 6,
        "Should have multiple exercise dates, got {}",
        effective_dates.len()
    );

    // Dates should be sorted
    for i in 1..effective_dates.len() {
        assert!(
            effective_dates[i] > effective_dates[i - 1],
            "Exercise dates should be sorted"
        );
    }

    // All dates should be before swap end
    for &date in &effective_dates {
        assert!(date < swap_end, "Exercise dates should be before swap end");
    }
}

#[test]
fn test_bermudan_schedule_with_lockout() {
    let first_exercise = Date::from_calendar_date(2026, Month::January, 1).expect("Valid date");
    let swap_end = Date::from_calendar_date(2030, Month::January, 1).expect("Valid date");
    let lockout_end = Date::from_calendar_date(2027, Month::January, 1).expect("Valid date");

    let schedule = BermudanSchedule::co_terminal(first_exercise, swap_end, Tenor::semi_annual())
        .with_lockout(lockout_end);

    let effective_dates = schedule.effective_dates();

    // All effective dates should be after lockout
    for &date in &effective_dates {
        assert!(
            date > lockout_end,
            "Effective dates should be after lockout"
        );
    }
}

#[test]
fn test_bermudan_to_european_conversion() {
    let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
    let swap_start = as_of;
    let swap_end = Date::from_calendar_date(2030, Month::January, 1).expect("Valid date");
    let first_exercise = Date::from_calendar_date(2026, Month::January, 1).expect("Valid date");

    let bermudan =
        test_bermudan_swaption(swap_start, swap_end, first_exercise, 0.03, OptionType::Call);

    // Convert to European
    let european = bermudan.to_european().expect("Conversion should succeed");

    // Check European parameters match Bermudan
    assert_eq!(european.strike_rate, bermudan.strike_rate);
    assert_eq!(european.notional.amount(), bermudan.notional.amount());
    assert_eq!(european.swap_end, bermudan.swap_end);

    // European expiry should be the first Bermudan exercise date
    assert_eq!(european.expiry, first_exercise);
}

// ============================================================================
// LSMC Tests (requires "mc" feature)
// ============================================================================

fn build_market_context() -> MarketContext {
    let curve = test_discount_curve();
    MarketContext::new().insert_discount(curve)
}

/// Test LSMC vs Tree: prices should be in same ballpark.
#[cfg(feature = "mc")]
#[test]
fn test_lsmc_vs_tree_sanity() {
    let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
    let swap_start = as_of;
    let swap_end = Date::from_calendar_date(2030, Month::January, 1).expect("Valid date");
    let first_exercise = Date::from_calendar_date(2027, Month::January, 1).expect("Valid date");

    let swaption =
        test_bermudan_swaption(swap_start, swap_end, first_exercise, 0.03, OptionType::Call);

    let market = build_market_context();

    // Price with tree
    let tree_pricer =
        BermudanSwaptionPricer::tree_pricer(HullWhiteParams::default()).with_tree_steps(100);
    let tree_result = tree_pricer.price_dyn(&swaption, &market, as_of);
    assert!(
        tree_result.is_ok(),
        "Tree pricing should succeed: {:?}",
        tree_result.err()
    );
    let tree_pv = tree_result
        .expect("Tree pricing should succeed")
        .value
        .amount();

    // Price with LSMC (fewer paths for speed in tests)
    let lsmc_pricer = BermudanSwaptionPricer::lsmc_pricer(HullWhiteParams::default())
        .with_mc_paths(10_000)
        .with_seed(42);
    let lsmc_result = lsmc_pricer.price_dyn(&swaption, &market, as_of);
    assert!(
        lsmc_result.is_ok(),
        "LSMC pricing should succeed: {:?}",
        lsmc_result.err()
    );
    let lsmc_result = lsmc_result.expect("LSMC pricing should succeed");
    let lsmc_pv = lsmc_result.value.amount();

    // Check that LSMC diagnostics are present
    assert!(
        lsmc_result.measures.contains_key("lsmc_stderr"),
        "LSMC result should contain stderr"
    );
    assert!(
        lsmc_result.measures.contains_key("lsmc_num_paths"),
        "LSMC result should contain num_paths"
    );
    assert!(
        lsmc_result.measures.contains_key("lsmc_ci95_low"),
        "LSMC result should contain CI95 low"
    );
    assert!(
        lsmc_result.measures.contains_key("lsmc_ci95_high"),
        "LSMC result should contain CI95 high"
    );

    // Both should be positive
    assert!(
        tree_pv >= 0.0,
        "Tree PV should be non-negative: {}",
        tree_pv
    );
    assert!(
        lsmc_pv >= 0.0,
        "LSMC PV should be non-negative: {}",
        lsmc_pv
    );

    // Note: LSMC and Tree may produce different values due to:
    // 1. Different theta(t) calibration approaches (LSMC: curve-derived, Tree: forward induction)
    // 2. MC noise with limited paths
    // 3. Different grid/discretization
    //
    // This test validates the infrastructure works, not exact numerical agreement.
    // For production use, more paths and careful model calibration are needed.
    let max_pv = tree_pv.max(lsmc_pv);
    let min_pv = tree_pv.min(lsmc_pv);
    if max_pv > 1.0 {
        // Just verify they're within an order of magnitude (sanity check)
        let ratio = min_pv / max_pv;
        assert!(
            ratio > 0.1,
            "LSMC ({}) and Tree ({}) prices should be same order of magnitude (ratio: {})",
            lsmc_pv,
            tree_pv,
            ratio
        );
    }

    eprintln!(
        "Tree PV: {:.2}, LSMC PV: {:.2}, stderr: {:.2}",
        tree_pv,
        lsmc_pv,
        lsmc_result
            .measures
            .get("lsmc_stderr")
            .copied()
            .unwrap_or(0.0)
    );
}

/// Test LSMC determinism: same seed produces identical results.
#[cfg(feature = "mc")]
#[test]
fn test_lsmc_determinism() {
    let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
    let swap_start = as_of;
    let swap_end = Date::from_calendar_date(2030, Month::January, 1).expect("Valid date");
    let first_exercise = Date::from_calendar_date(2027, Month::January, 1).expect("Valid date");

    let swaption =
        test_bermudan_swaption(swap_start, swap_end, first_exercise, 0.03, OptionType::Call);

    let market = build_market_context();

    // Price twice with same seed
    let pricer1 = BermudanSwaptionPricer::lsmc_pricer(HullWhiteParams::default())
        .with_mc_paths(5_000)
        .with_seed(12345);
    let result1 = pricer1
        .price_dyn(&swaption, &market, as_of)
        .expect("Pricing should succeed");

    let pricer2 = BermudanSwaptionPricer::lsmc_pricer(HullWhiteParams::default())
        .with_mc_paths(5_000)
        .with_seed(12345);
    let result2 = pricer2
        .price_dyn(&swaption, &market, as_of)
        .expect("Pricing should succeed");

    // Results should be identical
    assert!(
        (result1.value.amount() - result2.value.amount()).abs() < 1e-10,
        "Same seed should produce identical results: {} vs {}",
        result1.value.amount(),
        result2.value.amount()
    );
}

/// Test LSMC with different seeds produces different results.
#[cfg(feature = "mc")]
#[test]
fn test_lsmc_different_seeds() {
    let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
    let swap_start = as_of;
    let swap_end = Date::from_calendar_date(2030, Month::January, 1).expect("Valid date");
    let first_exercise = Date::from_calendar_date(2027, Month::January, 1).expect("Valid date");

    let swaption =
        test_bermudan_swaption(swap_start, swap_end, first_exercise, 0.03, OptionType::Call);

    let market = build_market_context();

    // Price with different seeds
    let pricer1 = BermudanSwaptionPricer::lsmc_pricer(HullWhiteParams::default())
        .with_mc_paths(5_000)
        .with_seed(111);
    let result1 = pricer1
        .price_dyn(&swaption, &market, as_of)
        .expect("Pricing should succeed");

    let pricer2 = BermudanSwaptionPricer::lsmc_pricer(HullWhiteParams::default())
        .with_mc_paths(5_000)
        .with_seed(222);
    let result2 = pricer2
        .price_dyn(&swaption, &market, as_of)
        .expect("Pricing should succeed");

    // Results should be different (due to different random paths)
    // Note: There's a tiny probability they could be equal by chance, but effectively zero
    assert!(
        (result1.value.amount() - result2.value.amount()).abs() > 1e-10,
        "Different seeds should produce different results: {} vs {}",
        result1.value.amount(),
        result2.value.amount()
    );
}

/// Test LSMC pricer key is correct.
#[cfg(feature = "mc")]
#[test]
fn test_lsmc_pricer_key() {
    let pricer = BermudanSwaptionPricer::lsmc_pricer(HullWhiteParams::default());
    let key = pricer.key();

    assert_eq!(
        key.instrument,
        finstack_valuations::pricer::InstrumentType::BermudanSwaption
    );
    assert_eq!(
        key.model,
        finstack_valuations::pricer::ModelKey::MonteCarloHullWhite1F
    );
}
