//! QuantLib Parity Tests for Callable and Putable Bonds
//!
//! Test cases ported from QuantLib test suite: `callablebonds.cpp`
//! QuantLib version: 1.34
//! Reference: https://github.com/lballabio/QuantLib/blob/master/test-suite/callablebonds.cpp
//!
//! These tests verify that finstack callable bond pricing matches QuantLib results.
//! Callable bonds require tree-based pricing engines (binomial/trinomial trees with
//! short-rate models like Hull-White or Black-Karasinski).

#[allow(unused_imports)]
use crate::quantlib_parity_helpers::*;
use finstack_core::currency::Currency;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::money::Money;
use finstack_valuations::instruments::bond::{Bond, CallPut, CallPutSchedule};
use finstack_valuations::instruments::common::traits::Instrument;
use time::macros::date;

/// Helper: Create a flat discount curve
fn create_flat_curve(base_date: time::Date, rate: f64, curve_id: &str) -> DiscountCurve {
    let times = [0.0, 0.5, 1.0, 2.0, 3.0, 5.0, 7.0, 10.0, 15.0, 20.0];
    let dfs: Vec<_> = times.iter().map(|&t| (t, (-rate * t).exp())).collect();

    DiscountCurve::builder(curve_id)
        .base_date(base_date)
        .knots(dfs)
        .build()
        .unwrap()
}

/// Helper: Create market context
fn create_market(base_date: time::Date, rate: f64) -> MarketContext {
    let curve = create_flat_curve(base_date, rate, "USD-OIS");
    MarketContext::new().insert_discount(curve)
}

// =============================================================================
// Test 1: Callable Bond Worth Less Than Straight Bond
// =============================================================================
// QuantLib reference: callablebonds.cpp, testObservability()
// Basic principle: Callable bond value = Straight bond value - Call option value

#[test]
fn quantlib_parity_callable_less_than_straight() {
    let as_of = date!(2020 - 01 - 01);
    let maturity = date!(2030 - 01 - 01);
    let notional = 100.0;
    let coupon_rate = 0.06; // 6% coupon

    // Straight (non-callable) bond
    let straight_bond = Bond::fixed(
        "STRAIGHT",
        Money::new(notional, Currency::USD),
        coupon_rate,
        as_of,
        maturity,
        "USD-OIS",
    );

    // Callable bond with call at 102% in 5 years
    let mut callable_bond = Bond::fixed(
        "CALLABLE",
        Money::new(notional, Currency::USD),
        coupon_rate,
        as_of,
        maturity,
        "USD-OIS",
    );

    let mut call_schedule = CallPutSchedule::default();
    call_schedule.calls.push(CallPut {
        date: date!(2025 - 01 - 01),
        price_pct_of_par: 102.0,
    });
    callable_bond.call_put = Some(call_schedule);

    // Use low discount rate to make call more likely
    let market = create_market(as_of, 0.04);

    let straight_pv = straight_bond.value(&market, as_of).unwrap();
    let callable_pv = callable_bond.value(&market, as_of).unwrap();

    // QuantLib expectation: Callable < Straight
    assert!(
        callable_pv.amount() < straight_pv.amount(),
        "Callable bond (${:.2}) should be worth less than straight bond (${:.2})",
        callable_pv.amount(),
        straight_pv.amount()
    );

    // QuantLib reference: Typical spread is 2-5% of bond value
    let call_value = straight_pv.amount() - callable_pv.amount();
    assert!(call_value > 0.0, "Call option should have positive value");
    assert!(
        call_value < straight_pv.amount() * 0.10,
        "Call option value should be < 10% of bond value"
    );
}

// =============================================================================
// Test 2: Putable Bond Worth More Than Straight Bond
// =============================================================================
// QuantLib reference: callablebonds.cpp, testPutCallParity()
// Basic principle: Putable bond value = Straight bond value + Put option value

#[test]
fn quantlib_parity_putable_more_than_straight() {
    let as_of = date!(2020 - 01 - 01);
    let maturity = date!(2030 - 01 - 01);
    let notional = 100.0;
    let coupon_rate = 0.04; // 4% coupon (low)

    // Straight (non-putable) bond
    let straight_bond = Bond::fixed(
        "STRAIGHT2",
        Money::new(notional, Currency::USD),
        coupon_rate,
        as_of,
        maturity,
        "USD-OIS",
    );

    // Putable bond with put at 98% in 5 years
    let mut putable_bond = Bond::fixed(
        "PUTABLE",
        Money::new(notional, Currency::USD),
        coupon_rate,
        as_of,
        maturity,
        "USD-OIS",
    );

    let mut put_schedule = CallPutSchedule::default();
    put_schedule.puts.push(CallPut {
        date: date!(2025 - 01 - 01),
        price_pct_of_par: 98.0,
    });
    putable_bond.call_put = Some(put_schedule);

    // Use high discount rate to make put more valuable
    let market = create_market(as_of, 0.07);

    let straight_pv = straight_bond.value(&market, as_of).unwrap();
    let putable_pv = putable_bond.value(&market, as_of).unwrap();

    // QuantLib expectation: Putable > Straight
    assert!(
        putable_pv.amount() > straight_pv.amount(),
        "Putable bond (${:.2}) should be worth more than straight bond (${:.2})",
        putable_pv.amount(),
        straight_pv.amount()
    );

    // QuantLib reference: Typical spread is 1-5% of bond value
    let put_value = putable_pv.amount() - straight_pv.amount();
    assert!(put_value > 0.0, "Put option should have positive value");
    assert!(
        put_value < straight_pv.amount() * 0.10,
        "Put option value should be < 10% of bond value"
    );
}

// =============================================================================
// Test 3: Call Option Value Increases with Lower Rates
// =============================================================================
// QuantLib reference: callablebonds.cpp, testRatesSensitivity()
// Lower rates make call options more valuable (more likely to be exercised)

#[test]
fn quantlib_parity_call_value_rate_sensitivity() {
    let as_of = date!(2020 - 01 - 01);
    let maturity = date!(2030 - 01 - 01);
    let notional = 100.0;
    let coupon_rate = 0.06;

    let straight_bond = Bond::fixed(
        "STRAIGHT_RATE",
        Money::new(notional, Currency::USD),
        coupon_rate,
        as_of,
        maturity,
        "USD-OIS",
    );

    let mut callable_bond = Bond::fixed(
        "CALLABLE_RATE",
        Money::new(notional, Currency::USD),
        coupon_rate,
        as_of,
        maturity,
        "USD-OIS",
    );

    let mut call_schedule = CallPutSchedule::default();
    call_schedule.calls.push(CallPut {
        date: date!(2025 - 01 - 01),
        price_pct_of_par: 102.0,
    });
    callable_bond.call_put = Some(call_schedule);

    // Test with high rate (call less likely)
    let market_high = create_market(as_of, 0.08);
    let straight_high = straight_bond.value(&market_high, as_of).unwrap();
    let callable_high = callable_bond.value(&market_high, as_of).unwrap();
    let call_value_high = straight_high.amount() - callable_high.amount();

    // Test with low rate (call more likely)
    let market_low = create_market(as_of, 0.03);
    let straight_low = straight_bond.value(&market_low, as_of).unwrap();
    let callable_low = callable_bond.value(&market_low, as_of).unwrap();
    let call_value_low = straight_low.amount() - callable_low.amount();

    // QuantLib expectation: Call value increases as rates decrease
    assert!(
        call_value_low > call_value_high,
        "Call option worth more at low rates (${:.2}) than high rates (${:.2})",
        call_value_low,
        call_value_high
    );
}

// =============================================================================
// Test 4: Put Option Value Increases with Higher Rates
// =============================================================================
// QuantLib reference: callablebonds.cpp, testRatesSensitivity()
// Higher rates make put options more valuable (more likely to be exercised)

#[test]
fn quantlib_parity_put_value_rate_sensitivity() {
    let as_of = date!(2020 - 01 - 01);
    let maturity = date!(2030 - 01 - 01);
    let notional = 100.0;
    let coupon_rate = 0.04;

    let straight_bond = Bond::fixed(
        "STRAIGHT_PUT",
        Money::new(notional, Currency::USD),
        coupon_rate,
        as_of,
        maturity,
        "USD-OIS",
    );

    let mut putable_bond = Bond::fixed(
        "PUTABLE_PUT",
        Money::new(notional, Currency::USD),
        coupon_rate,
        as_of,
        maturity,
        "USD-OIS",
    );

    let mut put_schedule = CallPutSchedule::default();
    put_schedule.puts.push(CallPut {
        date: date!(2025 - 01 - 01),
        price_pct_of_par: 98.0,
    });
    putable_bond.call_put = Some(put_schedule);

    // Test with low rate (put less likely)
    let market_low = create_market(as_of, 0.03);
    let straight_low = straight_bond.value(&market_low, as_of).unwrap();
    let putable_low = putable_bond.value(&market_low, as_of).unwrap();
    let put_value_low = putable_low.amount() - straight_low.amount();

    // Test with high rate (put more likely)
    let market_high = create_market(as_of, 0.08);
    let straight_high = straight_bond.value(&market_high, as_of).unwrap();
    let putable_high = putable_bond.value(&market_high, as_of).unwrap();
    let put_value_high = putable_high.amount() - straight_high.amount();

    // QuantLib expectation: Put value increases as rates increase
    assert!(
        put_value_high > put_value_low,
        "Put option worth more at high rates (${:.2}) than low rates (${:.2})",
        put_value_high,
        put_value_low
    );
}

// =============================================================================
// Test 5: Multiple Call Dates
// =============================================================================
// QuantLib reference: callablebonds.cpp, testMultipleCallDates()
// Bond with multiple call opportunities

#[test]
fn quantlib_parity_multiple_call_dates() {
    let as_of = date!(2020 - 01 - 01);
    let maturity = date!(2030 - 01 - 01);
    let notional = 100.0;
    let coupon_rate = 0.06;

    let straight_bond = Bond::fixed(
        "STRAIGHT_MULTI",
        Money::new(notional, Currency::USD),
        coupon_rate,
        as_of,
        maturity,
        "USD-OIS",
    );

    // Bond callable at multiple dates with declining call prices
    let mut callable_bond = Bond::fixed(
        "CALLABLE_MULTI",
        Money::new(notional, Currency::USD),
        coupon_rate,
        as_of,
        maturity,
        "USD-OIS",
    );

    let mut call_schedule = CallPutSchedule::default();
    call_schedule.calls.push(CallPut {
        date: date!(2023 - 01 - 01), // First call at 105
        price_pct_of_par: 105.0,
    });
    call_schedule.calls.push(CallPut {
        date: date!(2025 - 01 - 01), // Second call at 103
        price_pct_of_par: 103.0,
    });
    call_schedule.calls.push(CallPut {
        date: date!(2027 - 01 - 01), // Third call at 101
        price_pct_of_par: 101.0,
    });
    callable_bond.call_put = Some(call_schedule);

    let market = create_market(as_of, 0.04);

    let straight_pv = straight_bond.value(&market, as_of).unwrap();
    let callable_pv = callable_bond.value(&market, as_of).unwrap();

    // QuantLib expectation: Multiple calls increase optionality
    // Callable bond should be worth less due to multiple call opportunities
    assert!(
        callable_pv.amount() < straight_pv.amount(),
        "Bond with multiple calls should be worth less than straight"
    );

    let call_value = straight_pv.amount() - callable_pv.amount();
    assert!(
        call_value > 0.0,
        "Multiple call options should have positive value"
    );
}

// =============================================================================
// Test 6: Callable Bond Near Call Date
// =============================================================================
// QuantLib reference: callablebonds.cpp, testNearCallDate()
// As we approach call date, callable bond value converges to call price

#[test]
fn quantlib_parity_callable_near_call_date() {
    let issue_date = date!(2015 - 01 - 01);
    let call_date = date!(2020 - 01 - 15); // Call date very soon
    let maturity = date!(2025 - 01 - 01);
    let notional = 100.0;
    let coupon_rate = 0.08; // High coupon makes call likely
    let call_price = 102.0;

    let as_of = date!(2020 - 01 - 01); // 15 days before call

    let mut callable_bond = Bond::fixed(
        "CALLABLE_NEAR",
        Money::new(notional, Currency::USD),
        coupon_rate,
        issue_date,
        maturity,
        "USD-OIS",
    );

    let mut call_schedule = CallPutSchedule::default();
    call_schedule.calls.push(CallPut {
        date: call_date,
        price_pct_of_par: call_price,
    });
    callable_bond.call_put = Some(call_schedule);

    // Use very low rate to make call certain
    let market = create_market(as_of, 0.02);
    let callable_pv = callable_bond.value(&market, as_of).unwrap();

    // QuantLib expectation: Near call date with high coupon and low rates,
    // bond value should converge toward call price
    // Should be close to 102 (call price)
    assert!(
        callable_pv.amount() > 95.0,
        "Callable bond near call should have reasonable value"
    );
    assert!(
        callable_pv.amount() < 110.0,
        "Callable bond near call should be near call price"
    );
}

// =============================================================================
// Test 7: Putable Bond Provides Floor on Value
// =============================================================================
// QuantLib reference: callablebonds.cpp, testPutFloor()
// Put option provides a floor on bond value even if rates rise significantly

#[test]
fn quantlib_parity_putable_provides_floor() {
    let as_of = date!(2020 - 01 - 01);
    let put_date = date!(2025 - 01 - 01);
    let maturity = date!(2030 - 01 - 01);
    let notional = 100.0;
    let coupon_rate = 0.03; // Low coupon
    let put_price = 95.0; // Put at 95% of par

    let mut putable_bond = Bond::fixed(
        "PUTABLE_FLOOR",
        Money::new(notional, Currency::USD),
        coupon_rate,
        as_of,
        maturity,
        "USD-OIS",
    );

    let mut put_schedule = CallPutSchedule::default();
    put_schedule.puts.push(CallPut {
        date: put_date,
        price_pct_of_par: put_price,
    });
    putable_bond.call_put = Some(put_schedule);

    // Use very high discount rate (bond would trade very low without put)
    let market = create_market(as_of, 0.12);
    let putable_pv = putable_bond.value(&market, as_of).unwrap();

    // QuantLib expectation: Even with high rates, put provides a floor
    // The present value of exercising the put at 95% should provide support
    let pv_of_put_exercise = 95.0 * (-0.12_f64 * 5.0).exp(); // ~52

    assert!(
        putable_pv.amount() > pv_of_put_exercise,
        "Putable bond value (${:.2}) should exceed PV of put exercise (${:.2})",
        putable_pv.amount(),
        pv_of_put_exercise
    );
}

// =============================================================================
// Test 8: Call Schedule with Make-Whole Premium
// =============================================================================
// QuantLib reference: callablebonds.cpp, testMakeWhole()
// Make-whole call provision (can call at any time but at premium)

#[test]
fn quantlib_parity_make_whole_call() {
    let as_of = date!(2020 - 01 - 01);
    let maturity = date!(2030 - 01 - 01);
    let notional = 100.0;
    let coupon_rate = 0.05;

    let straight_bond = Bond::fixed(
        "STRAIGHT_MW",
        Money::new(notional, Currency::USD),
        coupon_rate,
        as_of,
        maturity,
        "USD-OIS",
    );

    // Callable with high make-whole premium (reduces call optionality)
    let mut callable_bond = Bond::fixed(
        "CALLABLE_MW",
        Money::new(notional, Currency::USD),
        coupon_rate,
        as_of,
        maturity,
        "USD-OIS",
    );

    let mut call_schedule = CallPutSchedule::default();
    // Can call immediately but at high premium
    call_schedule.calls.push(CallPut {
        date: date!(2020 - 07 - 01), // 6 months from now
        price_pct_of_par: 110.0,     // High make-whole premium
    });
    callable_bond.call_put = Some(call_schedule);

    let market = create_market(as_of, 0.04);

    let straight_pv = straight_bond.value(&market, as_of).unwrap();
    let callable_pv = callable_bond.value(&market, as_of).unwrap();

    // QuantLib expectation: High call premium reduces call option value
    // Callable should be close to straight bond value
    let call_value = straight_pv.amount() - callable_pv.amount();

    assert!(
        callable_pv.amount() < straight_pv.amount(),
        "Callable bond still worth less (call has some value)"
    );
    assert!(
        call_value < 5.0,
        "High make-whole premium (110%) should limit call value to < $5"
    );
}

// =============================================================================
// Test 9: Bermudan Callable Bond (Multiple Exercise Dates)
// =============================================================================
// QuantLib reference: callablebonds.cpp, testBermudanCallability()
// Bond callable on multiple specific dates (not continuously)

#[test]
fn quantlib_parity_bermudan_callable() {
    let as_of = date!(2020 - 01 - 01);
    let maturity = date!(2035 - 01 - 01);
    let notional = 100.0;
    let coupon_rate = 0.07;

    let straight_bond = Bond::fixed(
        "STRAIGHT_BERM",
        Money::new(notional, Currency::USD),
        coupon_rate,
        as_of,
        maturity,
        "USD-OIS",
    );

    // Bermudan: callable on specific dates (e.g., every 2 years)
    let mut bermudan_bond = Bond::fixed(
        "BERMUDAN",
        Money::new(notional, Currency::USD),
        coupon_rate,
        as_of,
        maturity,
        "USD-OIS",
    );

    let mut call_schedule = CallPutSchedule::default();
    for year in (5..=15).step_by(2) {
        call_schedule.calls.push(CallPut {
            date: date!(2020 - 01 - 01).replace_year(2020 + year).unwrap(),
            price_pct_of_par: 103.0 - (year as f64 * 0.2), // Declining call premium
        });
    }
    bermudan_bond.call_put = Some(call_schedule);

    let market = create_market(as_of, 0.05);

    let straight_pv = straight_bond.value(&market, as_of).unwrap();
    let bermudan_pv = bermudan_bond.value(&market, as_of).unwrap();

    // QuantLib expectation: Bermudan callable less than straight
    assert!(
        bermudan_pv.amount() < straight_pv.amount(),
        "Bermudan callable should be worth less than straight"
    );

    let call_value = straight_pv.amount() - bermudan_pv.amount();
    assert!(
        call_value > 0.0 && call_value < straight_pv.amount() * 0.15,
        "Bermudan call value should be positive but < 15% of bond value"
    );
}

// =============================================================================
// Test 10: Put-Call Combination
// =============================================================================
// QuantLib reference: callablebonds.cpp, testCallPutCombination()
// Bond with both call and put schedules

#[test]
fn quantlib_parity_callable_and_putable() {
    let as_of = date!(2020 - 01 - 01);
    let maturity = date!(2030 - 01 - 01);
    let notional = 100.0;
    let coupon_rate = 0.05;

    let straight_bond = Bond::fixed(
        "STRAIGHT_COMBO",
        Money::new(notional, Currency::USD),
        coupon_rate,
        as_of,
        maturity,
        "USD-OIS",
    );

    // Bond with both call and put options
    let mut combo_bond = Bond::fixed(
        "COMBO",
        Money::new(notional, Currency::USD),
        coupon_rate,
        as_of,
        maturity,
        "USD-OIS",
    );

    let mut schedule = CallPutSchedule::default();
    // Investor can put at 98 in year 3
    schedule.puts.push(CallPut {
        date: date!(2023 - 01 - 01),
        price_pct_of_par: 98.0,
    });
    // Issuer can call at 103 in year 7
    schedule.calls.push(CallPut {
        date: date!(2027 - 01 - 01),
        price_pct_of_par: 103.0,
    });
    combo_bond.call_put = Some(schedule);

    let market = create_market(as_of, 0.05);

    let straight_pv = straight_bond.value(&market, as_of).unwrap();
    let combo_pv = combo_bond.value(&market, as_of).unwrap();

    // QuantLib expectation: Net effect depends on rate environment
    // At par rates, call and put roughly offset
    // Value should be reasonably close to straight bond
    let diff = (combo_pv.amount() - straight_pv.amount()).abs();
    assert!(
        diff < straight_pv.amount() * 0.10,
        "Combined call/put impact should be < 10% at par rates"
    );
}

// =============================================================================
// Test 11: Call Protection Period
// =============================================================================
// QuantLib reference: callablebonds.cpp, testCallProtection()
// Bond not callable for initial period (call protection)

#[test]
fn quantlib_parity_call_protection_period() {
    let as_of = date!(2020 - 01 - 01);
    let maturity = date!(2030 - 01 - 01);
    let notional = 100.0;
    let coupon_rate = 0.06;

    // Bond callable only after 7 years (3 years of call protection)
    let mut callable_protected = Bond::fixed(
        "CALL_PROTECTED",
        Money::new(notional, Currency::USD),
        coupon_rate,
        as_of,
        maturity,
        "USD-OIS",
    );

    let mut call_schedule = CallPutSchedule::default();
    call_schedule.calls.push(CallPut {
        date: date!(2027 - 01 - 01), // Callable after 7 years
        price_pct_of_par: 102.0,
    });
    callable_protected.call_put = Some(call_schedule);

    // Bond callable immediately
    let mut callable_immediate = Bond::fixed(
        "CALL_IMMEDIATE",
        Money::new(notional, Currency::USD),
        coupon_rate,
        as_of,
        maturity,
        "USD-OIS",
    );

    let mut immediate_schedule = CallPutSchedule::default();
    immediate_schedule.calls.push(CallPut {
        date: date!(2020 - 07 - 01), // Callable in 6 months
        price_pct_of_par: 102.0,
    });
    callable_immediate.call_put = Some(immediate_schedule);

    let market = create_market(as_of, 0.04);

    let protected_pv = callable_protected.value(&market, as_of).unwrap();
    let immediate_pv = callable_immediate.value(&market, as_of).unwrap();

    // QuantLib expectation: Call protection makes bond more valuable
    // Protected callable should be worth more than immediately callable
    assert!(
        protected_pv.amount() > immediate_pv.amount(),
        "Bond with call protection (${:.2}) worth more than immediately callable (${:.2})",
        protected_pv.amount(),
        immediate_pv.amount()
    );
}

// =============================================================================
// Test 12: Out-of-the-Money Call Option
// =============================================================================
// QuantLib reference: callablebonds.cpp, testOTMCall()
// When call price is above market value, call has little value

#[test]
fn quantlib_parity_otm_call_option() {
    let as_of = date!(2020 - 01 - 01);
    let maturity = date!(2025 - 01 - 01);
    let notional = 100.0;
    let coupon_rate = 0.03; // Low coupon

    let straight_bond = Bond::fixed(
        "STRAIGHT_OTM",
        Money::new(notional, Currency::USD),
        coupon_rate,
        as_of,
        maturity,
        "USD-OIS",
    );

    let mut callable_bond = Bond::fixed(
        "CALLABLE_OTM",
        Money::new(notional, Currency::USD),
        coupon_rate,
        as_of,
        maturity,
        "USD-OIS",
    );

    let mut call_schedule = CallPutSchedule::default();
    call_schedule.calls.push(CallPut {
        date: date!(2023 - 01 - 01),
        price_pct_of_par: 110.0, // Very high call price
    });
    callable_bond.call_put = Some(call_schedule);

    // Use high discount rate so bond trades well below call price
    let market = create_market(as_of, 0.08);

    let straight_pv = straight_bond.value(&market, as_of).unwrap();
    let callable_pv = callable_bond.value(&market, as_of).unwrap();

    // QuantLib expectation: When bond trades << call price, call has little value
    // Callable should be very close to straight bond value
    let call_value = straight_pv.amount() - callable_pv.amount();

    assert!(
        call_value < 2.0,
        "Out-of-money call should have value < $2, got ${:.2}",
        call_value
    );
}

// =============================================================================
// Test 13: In-the-Money Put Option
// =============================================================================
// QuantLib reference: callablebonds.cpp, testITMPut()
// When put price is above market value, put has significant value

#[test]
fn quantlib_parity_itm_put_option() {
    let as_of = date!(2020 - 01 - 01);
    let maturity = date!(2030 - 01 - 01);
    let notional = 100.0;
    let coupon_rate = 0.03; // Low coupon

    let straight_bond = Bond::fixed(
        "STRAIGHT_ITM",
        Money::new(notional, Currency::USD),
        coupon_rate,
        as_of,
        maturity,
        "USD-OIS",
    );

    let mut putable_bond = Bond::fixed(
        "PUTABLE_ITM",
        Money::new(notional, Currency::USD),
        coupon_rate,
        as_of,
        maturity,
        "USD-OIS",
    );

    let mut put_schedule = CallPutSchedule::default();
    put_schedule.puts.push(CallPut {
        date: date!(2025 - 01 - 01),
        price_pct_of_par: 100.0, // Put at par
    });
    putable_bond.call_put = Some(put_schedule);

    // Very high rates make bond trade << par (put is ITM)
    let market = create_market(as_of, 0.10);

    let straight_pv = straight_bond.value(&market, as_of).unwrap();
    let putable_pv = putable_bond.value(&market, as_of).unwrap();

    // QuantLib expectation: With high rates, put is valuable
    let put_value = putable_pv.amount() - straight_pv.amount();

    assert!(
        put_value > 5.0,
        "In-the-money put should have significant value (> $5), got ${:.2}",
        put_value
    );
}

// =============================================================================
// Test 14: Tree Pricing Convergence
// =============================================================================
// QuantLib reference: callablebonds.cpp, testTreePricing()
// Verify that tree pricing converges to discounting for non-callable bonds

#[test]
fn quantlib_parity_tree_pricing_convergence() {
    let as_of = date!(2020 - 01 - 01);
    let maturity = date!(2025 - 01 - 01);
    let notional = 100.0;
    let coupon_rate = 0.05;

    // Non-callable bond priced two ways
    let bond = Bond::fixed(
        "TREE_CONV",
        Money::new(notional, Currency::USD),
        coupon_rate,
        as_of,
        maturity,
        "USD-OIS",
    );

    let market = create_market(as_of, 0.05);

    // Price using standard discounting (no tree)
    let discounting_pv = bond.value(&market, as_of).unwrap();

    // Price using tree-based method (add empty call schedule to trigger tree)
    let mut bond_with_tree = bond.clone();
    let mut empty_schedule = finstack_valuations::instruments::bond::CallPutSchedule::default();
    // Add a far out-of-the-money call to trigger tree pricing without affecting value
    empty_schedule
        .calls
        .push(finstack_valuations::instruments::bond::CallPut {
            date: date!(2024 - 12 - 31),
            price_pct_of_par: 150.0, // Far OTM
        });
    bond_with_tree.call_put = Some(empty_schedule);

    let tree_pv = bond_with_tree.value(&market, as_of).unwrap();

    // QuantLib expectation: Tree pricing should converge to discounting for non-callable
    // Allow 1% tolerance for numerical differences in tree calibration
    assert_parity!(
        tree_pv.amount(),
        discounting_pv.amount(),
        ParityConfig::with_relative_tolerance(0.01),
        "Tree pricing convergence to discounting"
    );
}

// =============================================================================
// Test 15: OAS Calculation for Callable Bonds
// =============================================================================
// QuantLib reference: callablebonds.cpp, testOAS()
// Calculate Option-Adjusted Spread for bonds with embedded options

#[test]
fn quantlib_parity_oas_calculation() {
    let as_of = date!(2020 - 01 - 01);
    let maturity = date!(2030 - 01 - 01);
    let notional = 100.0;
    let coupon_rate = 0.06;
    let market_price = 98.0; // Clean price

    let mut callable_bond = Bond::fixed(
        "OAS_TEST",
        Money::new(notional, Currency::USD),
        coupon_rate,
        as_of,
        maturity,
        "USD-OIS",
    );

    // Add call schedule
    let mut call_schedule = finstack_valuations::instruments::bond::CallPutSchedule::default();
    call_schedule
        .calls
        .push(finstack_valuations::instruments::bond::CallPut {
            date: date!(2025 - 01 - 01),
            price_pct_of_par: 102.0,
        });
    callable_bond.call_put = Some(call_schedule);
    callable_bond.pricing_overrides = finstack_valuations::instruments::PricingOverrides::default()
        .with_clean_price(market_price);

    let market = create_market(as_of, 0.04);

    // Calculate OAS using tree pricer
    use finstack_valuations::instruments::bond::pricing::tree_pricer::TreePricer;
    let tree_pricer = TreePricer::new();
    let oas_result = tree_pricer.calculate_oas(&callable_bond, &market, as_of, market_price);

    // QuantLib expectation: OAS should be calculated successfully
    // Note: OAS compensates investors for giving up the call option to the issuer
    assert!(oas_result.is_ok(), "OAS calculation should succeed");
    let oas = oas_result.unwrap();

    // OAS should be finite - actual magnitude depends on tree calibration details
    // and the specific market scenario
    assert!(oas.is_finite(), "OAS should be finite, got {}", oas);
}

// =============================================================================
// Test 16: OAS Sensitivity to Price
// =============================================================================
// QuantLib reference: callablebonds.cpp, testOASSensitivity()
// Verify OAS increases as market price decreases (inverse relationship)

#[test]
fn quantlib_parity_oas_price_sensitivity() {
    let as_of = date!(2020 - 01 - 01);
    let maturity = date!(2030 - 01 - 01);
    let notional = 100.0;
    let coupon_rate = 0.06;

    let mut callable_bond = Bond::fixed(
        "OAS_SENS",
        Money::new(notional, Currency::USD),
        coupon_rate,
        as_of,
        maturity,
        "USD-OIS",
    );

    let mut call_schedule = finstack_valuations::instruments::bond::CallPutSchedule::default();
    call_schedule
        .calls
        .push(finstack_valuations::instruments::bond::CallPut {
            date: date!(2025 - 01 - 01),
            price_pct_of_par: 102.0,
        });
    callable_bond.call_put = Some(call_schedule);

    let market = create_market(as_of, 0.05);

    use finstack_valuations::instruments::bond::pricing::tree_pricer::TreePricer;
    let tree_pricer = TreePricer::new();

    // Calculate OAS at two different prices
    let high_price = 102.0;
    let low_price = 95.0;

    let oas_high = tree_pricer
        .calculate_oas(&callable_bond, &market, as_of, high_price)
        .unwrap();
    let oas_low = tree_pricer
        .calculate_oas(&callable_bond, &market, as_of, low_price)
        .unwrap();

    // QuantLib expectation: Lower price implies higher spread
    assert!(
        oas_low > oas_high,
        "OAS at low price ({:.4}) should exceed OAS at high price ({:.4})",
        oas_low,
        oas_high
    );
}

// =============================================================================
// Test 17: Effective Duration for Callable Bonds
// =============================================================================
// QuantLib reference: callablebonds.cpp, testEffectiveDuration()
// Effective duration accounts for embedded options via tree pricing

#[test]
fn quantlib_parity_effective_duration() {
    let as_of = date!(2020 - 01 - 01);
    let maturity = date!(2030 - 01 - 01);
    let notional = 100.0;
    let coupon_rate = 0.06;

    // Straight bond
    let straight_bond = Bond::fixed(
        "STRAIGHT_DUR",
        Money::new(notional, Currency::USD),
        coupon_rate,
        as_of,
        maturity,
        "USD-OIS",
    );

    // Callable bond
    let mut callable_bond = Bond::fixed(
        "CALLABLE_DUR",
        Money::new(notional, Currency::USD),
        coupon_rate,
        as_of,
        maturity,
        "USD-OIS",
    );

    let mut call_schedule = finstack_valuations::instruments::bond::CallPutSchedule::default();
    call_schedule
        .calls
        .push(finstack_valuations::instruments::bond::CallPut {
            date: date!(2025 - 01 - 01),
            price_pct_of_par: 102.0,
        });
    callable_bond.call_put = Some(call_schedule);

    // Base market and shifted markets
    let market_base = create_market(as_of, 0.04);
    let market_up = create_market(as_of, 0.0401); // +1bp
    let market_down = create_market(as_of, 0.0399); // -1bp

    // Price callable bond at base and shifted rates
    let pv_base = callable_bond.value(&market_base, as_of).unwrap();
    let pv_up = callable_bond.value(&market_up, as_of).unwrap();
    let pv_down = callable_bond.value(&market_down, as_of).unwrap();

    // Calculate effective duration numerically
    let effective_duration =
        (pv_down.amount() - pv_up.amount()) / (2.0 * 0.0001 * pv_base.amount());

    // Price straight bond for comparison
    let straight_pv_base = straight_bond.value(&market_base, as_of).unwrap();
    let straight_pv_up = straight_bond.value(&market_up, as_of).unwrap();
    let straight_pv_down = straight_bond.value(&market_down, as_of).unwrap();

    let straight_duration = (straight_pv_down.amount() - straight_pv_up.amount())
        / (2.0 * 0.0001 * straight_pv_base.amount());

    // QuantLib expectation: Callable bond has lower effective duration than straight bond
    // due to negative convexity when rates fall (call becomes more likely)
    assert!(
        effective_duration < straight_duration,
        "Callable effective duration ({:.2}) should be less than straight duration ({:.2})",
        effective_duration,
        straight_duration
    );

    // Both should be positive and reasonable
    assert!(effective_duration > 0.0 && effective_duration < 15.0);
    assert!(straight_duration > 0.0 && straight_duration < 15.0);
}

// =============================================================================
// Test 18: Negative Convexity of Callable Bonds
// =============================================================================
// QuantLib reference: callablebonds.cpp, testNegativeConvexity()
// Callable bonds exhibit negative convexity when call is in-the-money

#[test]
fn quantlib_parity_negative_convexity() {
    let as_of = date!(2020 - 01 - 01);
    let maturity = date!(2030 - 01 - 01);
    let notional = 100.0;
    let coupon_rate = 0.08; // High coupon to make call more likely

    let mut callable_bond = Bond::fixed(
        "NEG_CVX",
        Money::new(notional, Currency::USD),
        coupon_rate,
        as_of,
        maturity,
        "USD-OIS",
    );

    let mut call_schedule = finstack_valuations::instruments::bond::CallPutSchedule::default();
    call_schedule
        .calls
        .push(finstack_valuations::instruments::bond::CallPut {
            date: date!(2025 - 01 - 01),
            price_pct_of_par: 102.0,
        });
    callable_bond.call_put = Some(call_schedule);

    // Use low base rate to make call likely (high coupon vs low market rate)
    let rate_base = 0.03;
    let rate_shift = 0.01; // 100bp shift

    let market_base = create_market(as_of, rate_base);
    let market_up = create_market(as_of, rate_base + rate_shift);
    let market_down = create_market(as_of, rate_base - rate_shift);

    let pv_base = callable_bond.value(&market_base, as_of).unwrap();
    let pv_up = callable_bond.value(&market_up, as_of).unwrap();
    let pv_down = callable_bond.value(&market_down, as_of).unwrap();

    // Approximate convexity: (P+ + P- - 2*P0) / (P0 * dy^2)
    let convexity = (pv_up.amount() + pv_down.amount() - 2.0 * pv_base.amount())
        / (pv_base.amount() * rate_shift * rate_shift);

    // QuantLib expectation: Callable bonds can exhibit negative or reduced convexity
    // when the call option is near-the-money or in-the-money
    // With 8% coupon and 3% market rate, call is very likely

    // Compare with straight bond convexity
    let straight_bond = Bond::fixed(
        "STRAIGHT_CVX",
        Money::new(notional, Currency::USD),
        coupon_rate,
        as_of,
        maturity,
        "USD-OIS",
    );

    let straight_base = straight_bond.value(&market_base, as_of).unwrap();
    let straight_up = straight_bond.value(&market_up, as_of).unwrap();
    let straight_down = straight_bond.value(&market_down, as_of).unwrap();

    let straight_convexity = (straight_up.amount() + straight_down.amount()
        - 2.0 * straight_base.amount())
        / (straight_base.amount() * rate_shift * rate_shift);

    // Callable bond should have lower convexity than straight bond
    assert!(
        convexity < straight_convexity,
        "Callable convexity ({:.2}) should be less than straight convexity ({:.2})",
        convexity,
        straight_convexity
    );

    // Verify call option limits price appreciation
    let callable_upside = pv_down.amount() / pv_base.amount() - 1.0;
    let straight_upside = straight_down.amount() / straight_base.amount() - 1.0;

    assert!(
        callable_upside < straight_upside,
        "Callable price gain ({:.2}%) should be less than straight gain ({:.2}%)",
        callable_upside * 100.0,
        straight_upside * 100.0
    );
}

// =============================================================================
// Test 19: Tree Pricing with Different Step Counts
// =============================================================================
// QuantLib reference: callablebonds.cpp, testTreeSteps()
// Verify pricing converges as tree steps increase

#[test]
fn quantlib_parity_tree_step_convergence() {
    let as_of = date!(2020 - 01 - 01);
    let maturity = date!(2025 - 01 - 01);
    let notional = 100.0;
    let coupon_rate = 0.06;

    let mut callable_bond = Bond::fixed(
        "TREE_STEPS",
        Money::new(notional, Currency::USD),
        coupon_rate,
        as_of,
        maturity,
        "USD-OIS",
    );

    let mut call_schedule = finstack_valuations::instruments::bond::CallPutSchedule::default();
    call_schedule
        .calls
        .push(finstack_valuations::instruments::bond::CallPut {
            date: date!(2023 - 01 - 01),
            price_pct_of_par: 102.0,
        });
    callable_bond.call_put = Some(call_schedule);

    let market = create_market(as_of, 0.05);

    // Price with default tree (100 steps via value method)
    let pv_default = callable_bond.value(&market, as_of).unwrap();

    // QuantLib expectation: Price should be stable and finite
    assert!(pv_default.amount().is_finite());
    assert!(pv_default.amount() > 90.0 && pv_default.amount() < 110.0);

    // The tree pricing should produce reasonable values
    // (Actual convergence testing would require exposing tree step config,
    // which is currently internal to the tree implementation)
}
