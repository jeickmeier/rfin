//! Shared test utilities for cashflow tests.
//!
//! # Tolerance Conventions
//!
//! - `RATE_TOLERANCE` (1e-10): For rate/factor comparisons
//! - `FACTOR_TOLERANCE` (1e-12): For year fractions
//! - `financial_tolerance(notional)`: For money amounts
//!
//! # Test Curve Conventions
//!
//! - `FlatRateCurve`: Time-dependent DF = exp(-r*t), DF(0) = 1.0
//! - `FlatHazardRateCurve`: Time-dependent SP = exp(-λ*t), SP(0) = 1.0

use finstack_core::dates::Date;
use finstack_core::market_data::traits::{Discounting, Survival, TermStructure};
use finstack_core::types::CurveId;

// =============================================================================
// Tolerance Constants
// =============================================================================

/// Tolerance for rate and factor comparisons (e.g., CPR, SMM, DF, SP).
pub const RATE_TOLERANCE: f64 = 1e-10;

/// Tolerance for year fraction comparisons.
pub const FACTOR_TOLERANCE: f64 = 1e-12;

/// Calculate appropriate tolerance for financial amounts based on notional.
///
/// Returns max(notional * 1e-8, 0.01) to ensure reasonable precision
/// while avoiding overly tight tolerances for small amounts.
pub fn financial_tolerance(notional: f64) -> f64 {
    (notional.abs() * 1e-8).max(0.01)
}

// =============================================================================
// Test Curve Implementations
// =============================================================================

/// Flat-rate discount curve with proper time-dependent discount factors.
///
/// Uses continuous compounding: DF(t) = exp(-rate * t)
///
/// Key properties:
/// - DF(0) = 1.0 (always)
/// - DF(t) decreases monotonically for positive rates
/// - DF(t) > 0 for all t
#[derive(Debug, Clone)]
pub struct FlatRateCurve {
    pub id: CurveId,
    pub base: Date,
    /// Continuous compounding rate (e.g., 0.05 for 5%)
    pub rate: f64,
}

impl FlatRateCurve {
    /// Create a new flat rate curve.
    pub fn new(id: impl Into<String>, base: Date, rate: f64) -> Self {
        Self {
            id: CurveId::new(id),
            base,
            rate,
        }
    }
}

impl TermStructure for FlatRateCurve {
    fn id(&self) -> &CurveId {
        &self.id
    }
}

impl Discounting for FlatRateCurve {
    fn base_date(&self) -> Date {
        self.base
    }

    fn df(&self, t: f64) -> f64 {
        if t <= 0.0 {
            1.0
        } else {
            (-self.rate * t).exp()
        }
    }
}

/// Flat hazard rate curve with proper time-dependent survival probabilities.
///
/// Uses exponential decay: SP(t) = exp(-lambda * t)
///
/// Key properties:
/// - SP(0) = 1.0 (always)
/// - SP(t) decreases monotonically for positive lambda
/// - SP(t) > 0 for all t
/// - SP(t) <= 1.0 for all t >= 0
#[derive(Debug, Clone)]
pub struct FlatHazardRateCurve {
    pub id: CurveId,
    #[allow(dead_code)]
    pub base: Date,
    /// Hazard rate (intensity parameter, e.g., 0.02 for ~2% annual default probability)
    pub lambda: f64,
}

impl FlatHazardRateCurve {
    /// Create a new flat hazard rate curve.
    pub fn new(id: impl Into<String>, base: Date, lambda: f64) -> Self {
        Self {
            id: CurveId::new(id),
            base,
            lambda,
        }
    }
}

impl TermStructure for FlatHazardRateCurve {
    fn id(&self) -> &CurveId {
        &self.id
    }
}

impl Survival for FlatHazardRateCurve {
    fn sp(&self, t: f64) -> f64 {
        if t <= 0.0 {
            1.0
        } else {
            (-self.lambda * t).exp()
        }
    }
}

/// Flat discount curve with constant discount factor (time-independent).
///
/// Uses a constant discount factor for all times: DF(t) = df_const
///
/// Key properties:
/// - DF(t) = df_const for all t
/// - Useful for simple tests that don't need time-dependent discounting
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct FlatDiscountCurve {
    pub id: CurveId,
    pub base: Date,
    /// Constant discount factor (e.g., 0.95 for 5% discount)
    pub df_const: f64,
}

impl TermStructure for FlatDiscountCurve {
    fn id(&self) -> &CurveId {
        &self.id
    }
}

impl Discounting for FlatDiscountCurve {
    fn base_date(&self) -> Date {
        self.base
    }

    fn df(&self, _t: f64) -> f64 {
        self.df_const
    }
}

/// Flat hazard curve with constant survival probability (time-independent).
///
/// Uses a constant survival probability for all times: SP(t) = sp_const
///
/// Key properties:
/// - SP(t) = sp_const for all t
/// - Useful for simple tests that don't need time-dependent survival probabilities
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct FlatHazardCurve {
    pub id: CurveId,
    pub base: Date,
    /// Constant survival probability (e.g., 0.90 for 90% survival)
    pub sp_const: f64,
}

impl TermStructure for FlatHazardCurve {
    fn id(&self) -> &CurveId {
        &self.id
    }
}

impl Survival for FlatHazardCurve {
    fn sp(&self, _t: f64) -> f64 {
        self.sp_const
    }
}

// =============================================================================
// Unit Tests for Test Helpers
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use time::Month;

    fn test_date() -> Date {
        Date::from_calendar_date(2025, Month::January, 1).expect("valid date")
    }

    #[test]
    fn flat_rate_curve_df_at_zero_is_one() {
        let curve = FlatRateCurve::new("TEST", test_date(), 0.05);
        assert!((curve.df(0.0) - 1.0).abs() < FACTOR_TOLERANCE);
    }

    #[test]
    fn flat_rate_curve_df_at_negative_time_is_one() {
        let curve = FlatRateCurve::new("TEST", test_date(), 0.05);
        assert!((curve.df(-1.0) - 1.0).abs() < FACTOR_TOLERANCE);
    }

    #[test]
    fn flat_rate_curve_df_decreases_monotonically() {
        let curve = FlatRateCurve::new("TEST", test_date(), 0.05);
        let mut prev_df = f64::MAX;
        for t in [0.0, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0] {
            let df = curve.df(t);
            assert!(df <= prev_df, "DF must decrease: DF({}) = {}", t, df);
            assert!(df > 0.0, "DF must be positive");
            assert!(df <= 1.0, "DF must be <= 1.0 for positive rates");
            prev_df = df;
        }
    }

    #[test]
    fn flat_rate_curve_df_golden_value() {
        let curve = FlatRateCurve::new("TEST", test_date(), 0.05);
        // At t=1, DF = exp(-0.05) ≈ 0.9512
        let expected = (-0.05_f64).exp();
        assert!(
            (curve.df(1.0) - expected).abs() < FACTOR_TOLERANCE,
            "DF(1) should be {}, got {}",
            expected,
            curve.df(1.0)
        );
    }

    #[test]
    fn flat_hazard_curve_sp_at_zero_is_one() {
        let curve = FlatHazardRateCurve::new("TEST", test_date(), 0.02);
        assert!((curve.sp(0.0) - 1.0).abs() < FACTOR_TOLERANCE);
    }

    #[test]
    fn flat_hazard_curve_sp_at_negative_time_is_one() {
        let curve = FlatHazardRateCurve::new("TEST", test_date(), 0.02);
        assert!((curve.sp(-1.0) - 1.0).abs() < FACTOR_TOLERANCE);
    }

    #[test]
    fn flat_hazard_curve_sp_decreases_monotonically() {
        let curve = FlatHazardRateCurve::new("TEST", test_date(), 0.02);
        let mut prev_sp = f64::MAX;
        for t in [0.0, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0] {
            let sp = curve.sp(t);
            assert!(sp <= prev_sp, "SP must decrease: SP({}) = {}", t, sp);
            assert!(sp > 0.0, "SP must be positive");
            assert!(sp <= 1.0, "SP must be <= 1.0");
            prev_sp = sp;
        }
    }

    #[test]
    fn flat_hazard_curve_sp_golden_value() {
        let curve = FlatHazardRateCurve::new("TEST", test_date(), 0.02);
        // At t=1, SP = exp(-0.02) ≈ 0.9802
        let expected = (-0.02_f64).exp();
        assert!(
            (curve.sp(1.0) - expected).abs() < FACTOR_TOLERANCE,
            "SP(1) should be {}, got {}",
            expected,
            curve.sp(1.0)
        );
    }

    #[test]
    fn financial_tolerance_scales_with_notional() {
        assert!((financial_tolerance(1_000_000.0) - 0.01).abs() < 1e-10);
        assert!((financial_tolerance(100.0) - 0.01).abs() < 1e-10); // min is 0.01
        assert!((financial_tolerance(10_000_000_000.0) - 100.0).abs() < 1e-10);
    }
}
