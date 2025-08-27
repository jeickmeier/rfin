//! Rate and percentage types with conversion utilities
//!
//! This module provides strongly-typed wrappers for financial rates and percentages,
//! with clear conversion semantics and arithmetic operations.

use std::fmt;
use std::ops::{Add, Div, Mul, Neg, Sub};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// A financial rate (e.g., interest rate, discount rate)
///
/// Internally stored as a decimal value where 0.05 represents 5%.
/// Provides methods for conversion to/from basis points and percentages.
///
/// # Examples
///
/// ```
/// use finstack_core::types::{Rate, Bps, Percentage};
///
/// // Create rates in different ways
/// let rate1 = Rate::from_decimal(0.025);  // 2.5%
/// let rate2 = Rate::from_percent(2.5);    // 2.5%
/// let rate3 = Rate::from_bps(250);        // 2.5% (250 basis points)
///
/// assert_eq!(rate1, rate2);
/// assert_eq!(rate2, rate3);
///
/// // Convert to different representations
/// assert_eq!(rate1.as_decimal(), 0.025);
/// assert_eq!(rate1.as_percent(), 2.5);
/// assert_eq!(rate1.as_bps(), 250);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Rate(f64);

impl Rate {
    /// Create a rate from a decimal value (0.05 = 5%)
    pub fn from_decimal(decimal: f64) -> Self {
        Self(decimal)
    }

    /// Create a rate from a percentage value (5.0 = 5%)
    pub fn from_percent(percent: f64) -> Self {
        Self(percent / 100.0)
    }

    /// Create a rate from basis points (500 bps = 5%)
    pub fn from_bps(bps: i32) -> Self {
        Self(bps as f64 / 10_000.0)
    }

    /// Get the rate as a decimal value
    pub fn as_decimal(self) -> f64 {
        self.0
    }

    /// Get the rate as a percentage value
    pub fn as_percent(self) -> f64 {
        self.0 * 100.0
    }

    /// Get the rate as basis points (rounded to nearest integer)
    pub fn as_bps(self) -> i32 {
        (self.0 * 10_000.0).round() as i32
    }

    /// Zero rate constant
    pub const ZERO: Self = Self(0.0);

    /// Check if the rate is zero
    pub fn is_zero(self) -> bool {
        self.0 == 0.0
    }

    /// Check if the rate is positive
    pub fn is_positive(self) -> bool {
        self.0 > 0.0
    }

    /// Check if the rate is negative
    pub fn is_negative(self) -> bool {
        self.0 < 0.0
    }

    /// Get the absolute value of the rate
    pub fn abs(self) -> Self {
        Self(self.0.abs())
    }
}

impl fmt::Display for Rate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.4}%", self.as_percent())
    }
}

impl From<f64> for Rate {
    fn from(decimal: f64) -> Self {
        Self::from_decimal(decimal)
    }
}

impl From<Rate> for f64 {
    fn from(rate: Rate) -> Self {
        rate.as_decimal()
    }
}

// Arithmetic operations
impl Add for Rate {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Sub for Rate {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl Mul<f64> for Rate {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        Self(self.0 * rhs)
    }
}

impl Div<f64> for Rate {
    type Output = Self;

    fn div(self, rhs: f64) -> Self::Output {
        Self(self.0 / rhs)
    }
}

impl Neg for Rate {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}

/// Basis points - a unit of measure for rates (1 bp = 0.01%)
///
/// Commonly used in financial markets where small rate differences matter.
/// 10,000 basis points = 100%.
///
/// # Examples
///
/// ```
/// use finstack_core::types::Bps;
///
/// let spread = Bps::new(25);  // 25 basis points = 0.25%
/// assert_eq!(spread.as_decimal(), 0.0025);
/// assert_eq!(spread.as_percent(), 0.25);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Bps(i32);

impl Bps {
    /// Create a new Bps value
    pub fn new(bps: i32) -> Self {
        Self(bps)
    }

    /// Get the basis points as an integer
    pub fn as_bps(self) -> i32 {
        self.0
    }

    /// Convert to decimal representation
    pub fn as_decimal(self) -> f64 {
        self.0 as f64 / 10_000.0
    }

    /// Convert to percentage representation  
    pub fn as_percent(self) -> f64 {
        self.0 as f64 / 100.0
    }

    /// Convert to Rate
    pub fn as_rate(self) -> Rate {
        Rate::from_bps(self.0)
    }

    /// Zero basis points constant
    pub const ZERO: Self = Self(0);

    /// Check if zero
    pub fn is_zero(self) -> bool {
        self.0 == 0
    }

    /// Check if positive
    pub fn is_positive(self) -> bool {
        self.0 > 0
    }

    /// Check if negative
    pub fn is_negative(self) -> bool {
        self.0 < 0
    }

    /// Get absolute value
    pub fn abs(self) -> Self {
        Self(self.0.abs())
    }
}

impl fmt::Display for Bps {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}bp", self.0)
    }
}

impl From<i32> for Bps {
    fn from(bps: i32) -> Self {
        Self::new(bps)
    }
}

impl From<Bps> for i32 {
    fn from(bps: Bps) -> Self {
        bps.as_bps()
    }
}

impl From<Bps> for f64 {
    fn from(bps: Bps) -> Self {
        bps.as_decimal()
    }
}

// Arithmetic operations for Bps
impl Add for Bps {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Sub for Bps {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl Mul<i32> for Bps {
    type Output = Self;

    fn mul(self, rhs: i32) -> Self::Output {
        Self(self.0 * rhs)
    }
}

impl Div<i32> for Bps {
    type Output = Self;

    fn div(self, rhs: i32) -> Self::Output {
        Self(self.0 / rhs)
    }
}

impl Neg for Bps {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}

/// A percentage value
///
/// Provides a clear type for percentage values with appropriate display formatting.
///
/// # Examples
///
/// ```
/// use finstack_core::types::Percentage;
///
/// let pct = Percentage::new(12.5);
/// assert_eq!(pct.as_decimal(), 0.125);
/// assert_eq!(format!("{}", pct), "12.50%");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Percentage(f64);

impl Percentage {
    /// Create a new percentage (12.5 = 12.5%)
    pub fn new(percent: f64) -> Self {
        Self(percent)
    }

    /// Get the percentage value
    pub fn as_percent(self) -> f64 {
        self.0
    }

    /// Convert to decimal representation
    pub fn as_decimal(self) -> f64 {
        self.0 / 100.0
    }

    /// Convert to Rate
    pub fn as_rate(self) -> Rate {
        Rate::from_percent(self.0)
    }

    /// Convert to basis points
    pub fn as_bps(self) -> i32 {
        (self.0 * 100.0).round() as i32
    }

    /// Zero percentage constant
    pub const ZERO: Self = Self(0.0);

    /// Check if zero
    pub fn is_zero(self) -> bool {
        self.0 == 0.0
    }

    /// Check if positive
    pub fn is_positive(self) -> bool {
        self.0 > 0.0
    }

    /// Check if negative  
    pub fn is_negative(self) -> bool {
        self.0 < 0.0
    }

    /// Get absolute value
    pub fn abs(self) -> Self {
        Self(self.0.abs())
    }
}

impl fmt::Display for Percentage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.2}%", self.0)
    }
}

impl From<f64> for Percentage {
    fn from(percent: f64) -> Self {
        Self::new(percent)
    }
}

impl From<Percentage> for f64 {
    fn from(pct: Percentage) -> Self {
        pct.as_percent()
    }
}

// Arithmetic operations for Percentage
impl Add for Percentage {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Sub for Percentage {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl Mul<f64> for Percentage {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        Self(self.0 * rhs)
    }
}

impl Div<f64> for Percentage {
    type Output = Self;

    fn div(self, rhs: f64) -> Self::Output {
        Self(self.0 / rhs)
    }
}

impl Neg for Percentage {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}

// Cross-conversions between types
impl From<Rate> for Bps {
    fn from(rate: Rate) -> Self {
        Bps::new(rate.as_bps())
    }
}

impl From<Rate> for Percentage {
    fn from(rate: Rate) -> Self {
        Percentage::new(rate.as_percent())
    }
}

impl From<Bps> for Rate {
    fn from(bps: Bps) -> Self {
        Rate::from_bps(bps.as_bps())
    }
}

impl From<Bps> for Percentage {
    fn from(bps: Bps) -> Self {
        Percentage::new(bps.as_percent())
    }
}

impl From<Percentage> for Rate {
    fn from(pct: Percentage) -> Self {
        Rate::from_percent(pct.as_percent())
    }
}

impl From<Percentage> for Bps {
    fn from(pct: Percentage) -> Self {
        Bps::new(pct.as_bps())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_creation_and_conversion() {
        let rate = Rate::from_percent(2.5);
        assert_eq!(rate.as_decimal(), 0.025);
        assert_eq!(rate.as_percent(), 2.5);
        assert_eq!(rate.as_bps(), 250);

        let rate2 = Rate::from_bps(250);
        assert_eq!(rate, rate2);

        let rate3 = Rate::from_decimal(0.025);
        assert_eq!(rate, rate3);
    }

    #[test]
    fn rate_arithmetic() {
        let rate1 = Rate::from_percent(2.0);
        let rate2 = Rate::from_percent(1.5);

        // Use approximate equality for floating point comparisons
        let eps = 1e-15;

        let result_add = rate1 + rate2;
        let expected_add = Rate::from_percent(3.5);
        assert!((result_add.as_decimal() - expected_add.as_decimal()).abs() < eps);

        let result_sub = rate1 - rate2;
        let expected_sub = Rate::from_percent(0.5);
        assert!((result_sub.as_decimal() - expected_sub.as_decimal()).abs() < eps);

        let result_mul = rate1 * 2.0;
        let expected_mul = Rate::from_percent(4.0);
        assert!((result_mul.as_decimal() - expected_mul.as_decimal()).abs() < eps);

        let result_div = rate1 / 2.0;
        let expected_div = Rate::from_percent(1.0);
        assert!((result_div.as_decimal() - expected_div.as_decimal()).abs() < eps);

        let result_neg = -rate1;
        let expected_neg = Rate::from_percent(-2.0);
        assert!((result_neg.as_decimal() - expected_neg.as_decimal()).abs() < eps);
    }

    #[test]
    fn bps_creation_and_conversion() {
        let bps = Bps::new(250);
        assert_eq!(bps.as_bps(), 250);
        assert_eq!(bps.as_decimal(), 0.025);
        assert_eq!(bps.as_percent(), 2.5);
    }

    #[test]
    fn bps_arithmetic() {
        let bps1 = Bps::new(100);
        let bps2 = Bps::new(50);

        assert_eq!(bps1 + bps2, Bps::new(150));
        assert_eq!(bps1 - bps2, Bps::new(50));
        assert_eq!(bps1 * 2, Bps::new(200));
        assert_eq!(bps1 / 2, Bps::new(50));
        assert_eq!(-bps1, Bps::new(-100));
    }

    #[test]
    fn percentage_creation_and_conversion() {
        let pct = Percentage::new(12.5);
        assert_eq!(pct.as_percent(), 12.5);
        assert_eq!(pct.as_decimal(), 0.125);
        assert_eq!(pct.as_bps(), 1250);
    }

    #[test]
    fn percentage_arithmetic() {
        let pct1 = Percentage::new(10.0);
        let pct2 = Percentage::new(5.0);

        assert_eq!(pct1 + pct2, Percentage::new(15.0));
        assert_eq!(pct1 - pct2, Percentage::new(5.0));
        assert_eq!(pct1 * 2.0, Percentage::new(20.0));
        assert_eq!(pct1 / 2.0, Percentage::new(5.0));
        assert_eq!(-pct1, Percentage::new(-10.0));
    }

    #[test]
    fn cross_conversions() {
        let rate = Rate::from_percent(2.5);
        let bps: Bps = rate.into();
        let pct: Percentage = rate.into();

        assert_eq!(bps.as_bps(), 250);
        assert_eq!(pct.as_percent(), 2.5);

        let rate_from_bps: Rate = bps.into();
        let rate_from_pct: Rate = pct.into();

        assert_eq!(rate, rate_from_bps);
        assert_eq!(rate, rate_from_pct);
    }

    #[test]
    fn display_formatting() {
        let rate = Rate::from_percent(2.5);
        assert_eq!(format!("{}", rate), "2.5000%");

        let bps = Bps::new(250);
        assert_eq!(format!("{}", bps), "250bp");

        let pct = Percentage::new(12.75);
        assert_eq!(format!("{}", pct), "12.75%");
    }

    #[test]
    fn constants_and_predicates() {
        assert!(Rate::ZERO.is_zero());
        assert!(!Rate::ZERO.is_positive());
        assert!(!Rate::ZERO.is_negative());

        let positive = Rate::from_percent(1.0);
        assert!(positive.is_positive());
        assert!(!positive.is_negative());

        let negative = Rate::from_percent(-1.0);
        assert!(negative.is_negative());
        assert!(!negative.is_positive());

        assert_eq!(negative.abs(), positive);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_round_trip() {
        let rate = Rate::from_percent(2.5);
        let json = serde_json::to_string(&rate).unwrap();
        let deserialized: Rate = serde_json::from_str(&json).unwrap();
        assert_eq!(rate, deserialized);

        let bps = Bps::new(250);
        let json = serde_json::to_string(&bps).unwrap();
        let deserialized: Bps = serde_json::from_str(&json).unwrap();
        assert_eq!(bps, deserialized);

        let pct = Percentage::new(12.5);
        let json = serde_json::to_string(&pct).unwrap();
        let deserialized: Percentage = serde_json::from_str(&json).unwrap();
        assert_eq!(pct, deserialized);
    }
}
