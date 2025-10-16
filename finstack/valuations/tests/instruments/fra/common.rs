//! Shared test utilities for FRA tests.
//!
//! Provides common fixtures, builders, and assertion helpers to reduce
//! duplication across the test suite and ensure consistent test setup.

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::{DiscountCurve, ForwardCurve};
use finstack_core::math::interp::InterpStyle;
use finstack_core::money::Money;
use finstack_valuations::instruments::fra::ForwardRateAgreement;
use time::macros::date;

/// Standard test base date
pub const BASE_DATE: Date = date!(2024 - 01 - 01);

/// Standard FRA periods (3M x 6M means 3M forward, 3M tenor)
pub fn standard_fra_dates() -> (Date, Date, Date) {
    let fixing = date!(2024 - 04 - 01); // 3M forward
    let start = date!(2024 - 04 - 01);
    let end = date!(2024 - 07 - 01); // 3M tenor
    (fixing, start, end)
}

/// Creates a flat forward curve at the given rate
pub fn build_flat_forward_curve(rate: f64, base_date: Date, curve_id: &str) -> ForwardCurve {
    ForwardCurve::builder(curve_id, 0.25)
        .base_date(base_date)
        .day_count(DayCount::Act360)
        .knots([(0.0, rate), (10.0, rate)])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap()
}

/// Creates a flat discount curve at the given rate
pub fn build_flat_discount_curve(rate: f64, base_date: Date, curve_id: &str) -> DiscountCurve {
    // DF = exp(-rate * time)
    DiscountCurve::builder(curve_id)
        .base_date(base_date)
        .day_count(DayCount::Act360)
        .knots([
            (0.0, 1.0),
            (1.0, (-rate).exp() as f64),
            (5.0, (-rate * 5.0).exp() as f64),
            (10.0, (-rate * 10.0).exp() as f64),
        ])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap()
}

/// Creates an upward sloping forward curve (normal term structure)
pub fn build_upward_forward_curve(base_date: Date, curve_id: &str) -> ForwardCurve {
    ForwardCurve::builder(curve_id, 0.25)
        .base_date(base_date)
        .day_count(DayCount::Act360)
        .knots([
            (0.0, 0.03),  // 3% at short end
            (2.0, 0.04),  // 4% at 2Y
            (5.0, 0.045), // 4.5% at 5Y
            (10.0, 0.05), // 5% at 10Y
        ])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap()
}

/// Creates an inverted forward curve (stressed scenario)
pub fn build_inverted_forward_curve(base_date: Date, curve_id: &str) -> ForwardCurve {
    ForwardCurve::builder(curve_id, 0.25)
        .base_date(base_date)
        .day_count(DayCount::Act360)
        .knots([
            (0.0, 0.06), // 6% at short end
            (1.0, 0.05), // 5% at 1Y
            (2.0, 0.04), // 4% at 2Y
            (5.0, 0.03), // 3% at 5Y
        ])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap()
}

/// Standard market context with flat 5% curves
pub fn standard_market() -> MarketContext {
    let disc = build_flat_discount_curve(0.05, BASE_DATE, "USD_OIS");
    let fwd = build_flat_forward_curve(0.05, BASE_DATE, "USD_LIBOR_3M");
    MarketContext::new()
        .insert_discount(disc)
        .insert_forward(fwd)
}

/// Creates a standard 3M x 6M FRA for testing
pub fn create_standard_fra() -> ForwardRateAgreement {
    let (fixing, start, end) = standard_fra_dates();
    ForwardRateAgreement {
        id: "FRA_TEST".into(),
        notional: Money::new(1_000_000.0, Currency::USD),
        fixing_date: fixing,
        start_date: start,
        end_date: end,
        fixed_rate: 0.05,
        day_count: DayCount::Act360,
        reset_lag: 2,
        disc_id: "USD_OIS".into(),
        forward_id: "USD_LIBOR_3M".into(),
        pay_fixed: true, // true = receive fixed (confusing naming!)
        attributes: Default::default(),
    }
}

/// Fluent builder for test FRAs with sensible defaults
#[derive(Clone)]
pub struct TestFraBuilder {
    id: String,
    notional: Money,
    fixing_date: Date,
    start_date: Date,
    end_date: Date,
    fixed_rate: f64,
    day_count: DayCount,
    reset_lag: i32,
    disc_id: String,
    forward_id: String,
    pay_fixed: bool,
}

impl Default for TestFraBuilder {
    fn default() -> Self {
        let (fixing, start, end) = standard_fra_dates();
        Self {
            id: "FRA_TEST".to_string(),
            notional: Money::new(1_000_000.0, Currency::USD),
            fixing_date: fixing,
            start_date: start,
            end_date: end,
            fixed_rate: 0.05,
            day_count: DayCount::Act360,
            reset_lag: 2,
            disc_id: "USD_OIS".to_string(),
            forward_id: "USD_LIBOR_3M".to_string(),
            pay_fixed: true, // true = receive fixed
        }
    }
}

impl TestFraBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn id(mut self, id: &str) -> Self {
        self.id = id.to_string();
        self
    }

    pub fn notional(mut self, amount: f64, currency: Currency) -> Self {
        self.notional = Money::new(amount, currency);
        self
    }

    pub fn dates(mut self, fixing: Date, start: Date, end: Date) -> Self {
        self.fixing_date = fixing;
        self.start_date = start;
        self.end_date = end;
        self
    }

    pub fn fixed_rate(mut self, rate: f64) -> Self {
        self.fixed_rate = rate;
        self
    }

    pub fn day_count(mut self, dc: DayCount) -> Self {
        self.day_count = dc;
        self
    }

    pub fn curves(mut self, disc: &str, fwd: &str) -> Self {
        self.disc_id = disc.to_string();
        self.forward_id = fwd.to_string();
        self
    }

    pub fn pay_fixed(mut self, pay: bool) -> Self {
        self.pay_fixed = pay;
        self
    }

    pub fn build(self) -> ForwardRateAgreement {
        ForwardRateAgreement {
            id: self.id.into(),
            notional: self.notional,
            fixing_date: self.fixing_date,
            start_date: self.start_date,
            end_date: self.end_date,
            fixed_rate: self.fixed_rate,
            day_count: self.day_count,
            reset_lag: self.reset_lag,
            disc_id: self.disc_id.into(),
            forward_id: self.forward_id.into(),
            pay_fixed: self.pay_fixed,
            attributes: Default::default(),
        }
    }
}

/// Assertion helpers for FRA tests

/// Assert value is finite (not NaN or infinite)
pub fn assert_finite(value: f64, msg: &str) {
    assert!(value.is_finite(), "{}: value is not finite: {}", msg, value);
}

/// Assert value is positive
pub fn assert_positive(value: f64, msg: &str) {
    assert!(
        value > 0.0,
        "{}: value should be positive, got {}",
        msg,
        value
    );
}

/// Assert value is negative
pub fn assert_negative(value: f64, msg: &str) {
    assert!(
        value < 0.0,
        "{}: value should be negative, got {}",
        msg,
        value
    );
}

/// Assert value is within range (inclusive)
pub fn assert_in_range(value: f64, min: f64, max: f64, msg: &str) {
    assert!(
        value >= min && value <= max,
        "{}: value {} not in range [{}, {}]",
        msg,
        value,
        min,
        max
    );
}

/// Assert value is near zero (within tolerance)
pub fn assert_near_zero(value: f64, tolerance: f64, msg: &str) {
    assert!(
        value.abs() < tolerance,
        "{}: |value| = {} exceeds tolerance {}",
        msg,
        value.abs(),
        tolerance
    );
}

/// Assert two values are approximately equal
pub fn assert_approx_equal(a: f64, b: f64, tolerance: f64, msg: &str) {
    let diff = (a - b).abs();
    assert!(
        diff < tolerance,
        "{}: |{} - {}| = {} exceeds tolerance {}",
        msg,
        a,
        b,
        diff,
        tolerance
    );
}
