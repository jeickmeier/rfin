//! Standard financial validators following the Validator trait pattern
//!
//! This module provides domain-specific validators for common financial validation
//! scenarios, all implementing the core Validator trait for composability.

use super::{ValidationResult, ValidationWarning, Validator};
use crate::{dates::Date, F};

/// Validator for monotonic term structures (e.g., discount factors, hazard rates)
///
/// Ensures that a sequence of values is monotonic (either increasing or decreasing)
/// which is critical for no-arbitrage conditions in financial models.
#[derive(Debug, Clone)]
pub struct MonotonicTermStructureValidator {
    /// Whether the sequence should be increasing (true) or decreasing (false)
    pub increasing: bool,
    /// Whether to allow equal adjacent values (non-strict monotonicity)
    pub allow_equal: bool,
    /// Context description for error messages
    pub context: &'static str,
}

impl MonotonicTermStructureValidator {
    /// Create a validator for strictly decreasing sequences (typical for discount factors)
    pub fn decreasing(context: &'static str) -> Self {
        Self {
            increasing: false,
            allow_equal: false,
            context,
        }
    }

    /// Create a validator for strictly increasing sequences (typical for hazard rates)
    pub fn increasing(context: &'static str) -> Self {
        Self {
            increasing: true,
            allow_equal: false,
            context,
        }
    }

    /// Create a validator allowing equal adjacent values
    pub fn allow_equal(mut self) -> Self {
        self.allow_equal = true;
        self
    }
}

impl Validator for MonotonicTermStructureValidator {
    type Input = Vec<F>;
    type Output = Vec<F>;

    fn validate(&self, input: &Self::Input) -> ValidationResult<Self::Output> {
        if input.len() < 2 {
            return ValidationResult::pass(input.clone());
        }

        let mut warnings = Vec::new();

        for i in 1..input.len() {
            let curr = input[i];
            let prev = input[i - 1];

            let violation = if self.increasing {
                if self.allow_equal {
                    curr < prev
                } else {
                    curr <= prev
                }
            } else if self.allow_equal {
                curr > prev
            } else {
                curr >= prev
            };

            if violation {
                let direction = if self.increasing { "increasing" } else { "decreasing" };
                let strictness = if self.allow_equal { "non-strictly" } else { "strictly" };
                return ValidationResult::fail(format!(
                    "{} violation at index {}: expected {} {} sequence, but {} {} {}",
                    self.context,
                    i,
                    strictness,
                    direction,
                    prev,
                    if self.increasing { ">" } else { "<" },
                    curr
                ));
            }

            // Add warnings for very small changes that might indicate data issues
            let relative_change = (curr - prev).abs() / prev.abs().max(F::EPSILON);
            if relative_change < 1e-8 {
                warnings.push(ValidationWarning::with_context(
                    format!("Very small change at index {}: {} to {}", i, prev, curr),
                    format!("{}[{}]", self.context, i),
                ));
            }
        }

        ValidationResult::pass_with_warnings(input.clone(), warnings)
    }

    fn description(&self) -> Option<&'static str> {
        Some("Monotonic term structure validator")
    }
}

/// Validator for discount factors ensuring they're in the range (0, 1]
///
/// Discount factors must be positive and cannot exceed 1.0 (which would imply
/// negative interest rates beyond reasonable bounds or arbitrage opportunities).
#[derive(Debug, Clone)]
pub struct DiscountFactorValidator {
    /// Whether to allow discount factors exactly equal to 1.0
    pub allow_unity: bool,
    /// Tolerance for floating-point comparison near boundaries
    pub tolerance: F,
}

impl DiscountFactorValidator {
    /// Create a new discount factor validator
    pub fn new() -> Self {
        Self {
            allow_unity: true,
            tolerance: 1e-12,
        }
    }

    /// Disallow discount factors equal to 1.0 (for curves that must decay)
    pub fn strict(mut self) -> Self {
        self.allow_unity = false;
        self
    }

    /// Set custom tolerance for boundary checks
    pub fn with_tolerance(mut self, tolerance: F) -> Self {
        self.tolerance = tolerance;
        self
    }
}

impl Default for DiscountFactorValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Validator for DiscountFactorValidator {
    type Input = Vec<F>;
    type Output = Vec<F>;

    fn validate(&self, input: &Self::Input) -> ValidationResult<Self::Output> {
        let mut warnings = Vec::new();

        for (i, &df) in input.iter().enumerate() {
            // Check if discount factor is positive
            if df <= 0.0 {
                return ValidationResult::fail(format!(
                    "Discount factor at index {} is non-positive: {}",
                    i, df
                ));
            }

            // Check upper bound
            if df > 1.0 + self.tolerance {
                return ValidationResult::fail(format!(
                    "Discount factor at index {} exceeds 1.0: {}",
                    i, df
                ));
            }

            // Check if exactly 1.0 is allowed
            if !self.allow_unity && (df - 1.0).abs() < self.tolerance {
                return ValidationResult::fail(format!(
                    "Discount factor at index {} equals 1.0 but strict mode is enabled",
                    i
                ));
            }

            // Warning for very high discount factors (might indicate data issues)
            if df > 0.999 && df <= 1.0 {
                warnings.push(ValidationWarning::with_context(
                    format!("Very high discount factor: {}", df),
                    format!("index[{}]", i),
                ));
            }

            // Warning for very low discount factors (might indicate extreme scenarios)
            if df < 0.001 {
                warnings.push(ValidationWarning::with_context(
                    format!("Very low discount factor: {}", df),
                    format!("index[{}]", i),
                ));
            }
        }

        ValidationResult::pass_with_warnings(input.clone(), warnings)
    }

    fn description(&self) -> Option<&'static str> {
        Some("Discount factor range validator (0, 1]")
    }
}

/// Validator for interest rate bounds with domain-specific warnings
///
/// Validates that interest rates fall within reasonable bounds for financial modeling,
/// with configurable thresholds for warnings about extreme values.
#[derive(Debug, Clone)]
pub struct RateBoundsValidator {
    /// Minimum allowed rate (e.g., -0.10 for -10%)
    pub min_rate: F,
    /// Maximum allowed rate (e.g., 1.0 for 100%)
    pub max_rate: F,
    /// Threshold for warning about low rates
    pub low_warning_threshold: F,
    /// Threshold for warning about high rates  
    pub high_warning_threshold: F,
    /// Context description for error messages
    pub context: &'static str,
}

impl RateBoundsValidator {
    /// Create a validator with standard bounds for interest rates
    /// Default: [-50%, 100%] with warnings at [-10%, 25%]
    pub fn interest_rate() -> Self {
        Self {
            min_rate: -0.50,  // -50%
            max_rate: 1.00,   // 100%
            low_warning_threshold: -0.10,  // -10%
            high_warning_threshold: 0.25,   // 25%
            context: "Interest rate",
        }
    }

    /// Create a validator with bounds for credit spreads
    /// Default: [0%, 50%] with warnings at [20%]
    pub fn credit_spread() -> Self {
        Self {
            min_rate: 0.0,    // 0%
            max_rate: 0.50,   // 50%
            low_warning_threshold: F::NEG_INFINITY, // No low warning for spreads
            high_warning_threshold: 0.20,           // 20%
            context: "Credit spread",
        }
    }

    /// Create a validator with bounds for volatility
    /// Default: [0%, 300%] with warnings at [100%]
    pub fn volatility() -> Self {
        Self {
            min_rate: 0.0,    // 0%
            max_rate: 3.0,    // 300%
            low_warning_threshold: F::NEG_INFINITY, // No low warning for vol
            high_warning_threshold: 1.0,            // 100%
            context: "Volatility",
        }
    }

    /// Set custom bounds
    pub fn with_bounds(mut self, min: F, max: F) -> Self {
        self.min_rate = min;
        self.max_rate = max;
        self
    }

    /// Set custom warning thresholds
    pub fn with_warning_thresholds(mut self, low: F, high: F) -> Self {
        self.low_warning_threshold = low;
        self.high_warning_threshold = high;
        self
    }

    /// Set custom context for error messages
    pub fn with_context(mut self, context: &'static str) -> Self {
        self.context = context;
        self
    }
}

impl Validator for RateBoundsValidator {
    type Input = F;
    type Output = F;

    fn validate(&self, input: &Self::Input) -> ValidationResult<Self::Output> {
        let rate = *input;
        let mut warnings = Vec::new();

        // Check hard bounds
        if rate < self.min_rate {
            return ValidationResult::fail(format!(
                "{} {:.4} ({:.2}%) is below minimum allowed {:.2}%",
                self.context,
                rate,
                rate * 100.0,
                self.min_rate * 100.0
            ));
        }

        if rate > self.max_rate {
            return ValidationResult::fail(format!(
                "{} {:.4} ({:.2}%) exceeds maximum allowed {:.2}%",
                self.context,
                rate,
                rate * 100.0,
                self.max_rate * 100.0
            ));
        }

        // Check warning thresholds
        if self.low_warning_threshold.is_finite() && rate < self.low_warning_threshold {
            warnings.push(ValidationWarning::new(format!(
                "{} {:.4} ({:.2}%) is unusually low",
                self.context,
                rate,
                rate * 100.0
            )));
        }

        if self.high_warning_threshold.is_finite() && rate > self.high_warning_threshold {
            warnings.push(ValidationWarning::new(format!(
                "{} {:.4} ({:.2}%) is unusually high",
                self.context,
                rate,
                rate * 100.0
            )));
        }

        ValidationResult::pass_with_warnings(rate, warnings)
    }

    fn description(&self) -> Option<&'static str> {
        Some("Financial rate bounds validator")
    }
}

/// Validator for date ranges ensuring start < end with business day awareness
///
/// Validates date ranges with configurable policies for weekends, holidays,
/// and minimum/maximum range duration constraints.
#[derive(Debug, Clone)]
pub struct DateRangeValidator {
    /// Minimum allowed duration in days
    pub min_duration_days: Option<i32>,
    /// Maximum allowed duration in days  
    pub max_duration_days: Option<i32>,
    /// Whether to warn about weekend dates
    pub warn_weekends: bool,
    /// Whether to allow same start and end dates
    pub allow_same_date: bool,
}

impl DateRangeValidator {
    /// Create a new date range validator with default settings
    pub fn new() -> Self {
        Self {
            min_duration_days: None,
            max_duration_days: None,
            warn_weekends: true,
            allow_same_date: false,
        }
    }

    /// Set minimum duration requirement
    pub fn min_duration_days(mut self, days: i32) -> Self {
        self.min_duration_days = Some(days);
        self
    }

    /// Set maximum duration limit
    pub fn max_duration_days(mut self, days: i32) -> Self {
        self.max_duration_days = Some(days);
        self
    }

    /// Enable warnings for weekend dates
    pub fn warn_weekends(mut self, warn: bool) -> Self {
        self.warn_weekends = warn;
        self
    }

    /// Allow start and end dates to be the same
    pub fn allow_same_date(mut self) -> Self {
        self.allow_same_date = true;
        self
    }

    /// Preset for standard swap/bond date validation (1 day to 50 years)
    pub fn financial_instrument() -> Self {
        Self::new()
            .min_duration_days(1)
            .max_duration_days(365 * 50) // 50 years
            .warn_weekends(true)
    }

    /// Preset for daily observation periods (allow same date, max 1 year)
    pub fn observation_period() -> Self {
        Self::new()
            .allow_same_date()
            .max_duration_days(365)
            .warn_weekends(false)
    }
}

impl Default for DateRangeValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Validator for DateRangeValidator {
    type Input = (Date, Date);
    type Output = (Date, Date);

    fn validate(&self, input: &Self::Input) -> ValidationResult<Self::Output> {
        let (start_date, end_date) = *input;
        let mut warnings = Vec::new();

        // Basic ordering check
        if start_date > end_date {
            return ValidationResult::fail(format!(
                "Start date {} must be before or equal to end date {}",
                start_date, end_date
            ));
        }

        // Check if same date is allowed
        if start_date == end_date && !self.allow_same_date {
            return ValidationResult::fail(format!(
                "Start and end dates cannot be the same: {}",
                start_date
            ));
        }

        // Calculate duration in days
        let duration_days = (end_date - start_date).whole_days() as i32;

        // Check minimum duration
        if let Some(min_days) = self.min_duration_days {
            if duration_days < min_days {
                return ValidationResult::fail(format!(
                    "Date range duration {} days is below minimum {} days",
                    duration_days, min_days
                ));
            }
        }

        // Check maximum duration
        if let Some(max_days) = self.max_duration_days {
            if duration_days > max_days {
                return ValidationResult::fail(format!(
                    "Date range duration {} days exceeds maximum {} days",
                    duration_days, max_days
                ));
            }
        }

        // Weekend warnings
        if self.warn_weekends {
            if start_date.weekday().number_from_monday() > 5 {
                warnings.push(ValidationWarning::with_context(
                    format!("Start date {} falls on a weekend", start_date),
                    "start_date",
                ));
            }

            if end_date.weekday().number_from_monday() > 5 {
                warnings.push(ValidationWarning::with_context(
                    format!("End date {} falls on a weekend", end_date),
                    "end_date",
                ));
            }
        }

        // Warning for very long periods
        if duration_days > 365 * 30 {
            warnings.push(ValidationWarning::new(format!(
                "Very long date range: {} years ({} days)",
                duration_days / 365,
                duration_days
            )));
        }

        ValidationResult::pass_with_warnings((start_date, end_date), warnings)
    }

    fn description(&self) -> Option<&'static str> {
        Some("Date range validator")
    }
}

/// Validator for arrays of discount factors with term structure constraints
///
/// Combines range validation (0, 1] with monotonic decreasing validation
/// specifically tailored for discount factor curves.
#[derive(Debug, Clone)]
pub struct DiscountFactorCurveValidator {
    /// Whether to require strict monotonic decrease
    pub require_monotonic: bool,
    /// Tolerance for floating-point comparisons
    pub tolerance: F,
}

impl DiscountFactorCurveValidator {
    /// Create a new discount factor curve validator
    pub fn new() -> Self {
        Self {
            require_monotonic: true,
            tolerance: 1e-12,
        }
    }

    /// Allow non-monotonic discount factors (not recommended for most use cases)
    pub fn allow_non_monotonic(mut self) -> Self {
        self.require_monotonic = false;
        self
    }

    /// Set custom tolerance for comparisons
    pub fn with_tolerance(mut self, tolerance: F) -> Self {
        self.tolerance = tolerance;
        self
    }
}

impl Default for DiscountFactorCurveValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Validator for DiscountFactorCurveValidator {
    type Input = Vec<F>;
    type Output = Vec<F>;

    fn validate(&self, input: &Self::Input) -> ValidationResult<Self::Output> {
        // First validate individual discount factor ranges
        let df_validator = DiscountFactorValidator::new().with_tolerance(self.tolerance);
        let range_result = df_validator.validate(input);

        if range_result.is_failure() {
            return range_result;
        }

        // Then validate monotonic structure if required
        if self.require_monotonic {
            let monotonic_validator = MonotonicTermStructureValidator::decreasing("Discount factor curve")
                .allow_equal(); // Allow equal values for flat segments

            let monotonic_result = monotonic_validator.validate(input);
            if monotonic_result.is_failure() {
                return monotonic_result;
            }

            // Combine warnings from both validators
            let mut all_warnings = range_result.warnings().to_vec();
            all_warnings.extend_from_slice(monotonic_result.warnings());

            return ValidationResult::pass_with_warnings(input.clone(), all_warnings);
        }

        range_result
    }

    fn description(&self) -> Option<&'static str> {
        Some("Discount factor curve validator")
    }
}

/// Validator for term structure knots (time points)
///
/// Ensures time points are strictly increasing and within reasonable bounds
/// for financial modeling.
#[derive(Debug, Clone)]
pub struct TermStructureKnotsValidator {
    /// Minimum allowed time (typically 0.0 for spot)
    pub min_time: F,
    /// Maximum allowed time in years (e.g., 100 years)
    pub max_time: F,
    /// Whether to allow negative times (for backtesting scenarios)
    pub allow_negative: bool,
}

impl TermStructureKnotsValidator {
    /// Create validator for standard curve knots (0 to 100 years)
    pub fn standard_curve() -> Self {
        Self {
            min_time: 0.0,
            max_time: 100.0,
            allow_negative: false,
        }
    }

    /// Create validator allowing negative times (for backtesting)
    pub fn with_backtesting() -> Self {
        Self {
            min_time: -50.0,
            max_time: 100.0,
            allow_negative: true,
        }
    }

    /// Set custom time bounds
    pub fn with_bounds(mut self, min: F, max: F) -> Self {
        self.min_time = min;
        self.max_time = max;
        self
    }
}

impl Default for TermStructureKnotsValidator {
    fn default() -> Self {
        Self::standard_curve()
    }
}

impl Validator for TermStructureKnotsValidator {
    type Input = Vec<F>;
    type Output = Vec<F>;

    fn validate(&self, input: &Self::Input) -> ValidationResult<Self::Output> {
        if input.len() < 2 {
            return ValidationResult::fail("Term structure requires at least 2 time points".to_string());
        }

        let mut warnings = Vec::new();

        // Validate individual time points
        for (i, &time) in input.iter().enumerate() {
            if !self.allow_negative && time < 0.0 {
                return ValidationResult::fail(format!(
                    "Time point at index {} is negative: {}",
                    i, time
                ));
            }

            if time < self.min_time {
                return ValidationResult::fail(format!(
                    "Time point at index {} ({}) is below minimum allowed time {}",
                    i, time, self.min_time
                ));
            }

            if time > self.max_time {
                return ValidationResult::fail(format!(
                    "Time point at index {} ({}) exceeds maximum allowed time {}",
                    i, time, self.max_time
                ));
            }

            // Warning for very long maturities
            if time > 50.0 {
                warnings.push(ValidationWarning::with_context(
                    format!("Very long maturity: {} years", time),
                    format!("index[{}]", i),
                ));
            }
        }

        // Validate strict increasing sequence
        for i in 1..input.len() {
            if input[i] <= input[i - 1] {
                return ValidationResult::fail(format!(
                    "Time points must be strictly increasing: {} at index {} is not greater than {} at index {}",
                    input[i], i, input[i - 1], i - 1
                ));
            }

            // Warning for very small time steps
            let step = input[i] - input[i - 1];
            if step < 1.0 / 365.0 {  // Less than 1 day
                warnings.push(ValidationWarning::with_context(
                    format!("Very small time step: {:.6} years ({:.1} days)", step, step * 365.0),
                    format!("step[{}-{}]", i - 1, i),
                ));
            }
        }

        ValidationResult::pass_with_warnings(input.clone(), warnings)
    }

    fn description(&self) -> Option<&'static str> {
        Some("Term structure knots validator")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dates::Date;
    use time::{Duration, Month};

    #[test]
    fn test_monotonic_term_structure_decreasing() {
        let validator = MonotonicTermStructureValidator::decreasing("Test curve");
        
        // Valid decreasing sequence
        let valid_data = vec![1.0, 0.95, 0.90, 0.85];
        let result = validator.validate(&valid_data);
        assert!(result.is_success());

        // Invalid increasing sequence
        let invalid_data = vec![0.85, 0.90, 0.95, 1.0];
        let result = validator.validate(&invalid_data);
        assert!(result.is_failure());
        assert!(result.error_message().unwrap().contains("violation"));
    }

    #[test]
    fn test_monotonic_term_structure_increasing() {
        let validator = MonotonicTermStructureValidator::increasing("Hazard rates");
        
        // Valid increasing sequence
        let valid_data = vec![0.01, 0.02, 0.03, 0.04];
        let result = validator.validate(&valid_data);
        assert!(result.is_success());

        // Invalid decreasing sequence
        let invalid_data = vec![0.04, 0.03, 0.02, 0.01];
        let result = validator.validate(&invalid_data);
        assert!(result.is_failure());
    }

    #[test]
    fn test_discount_factor_validator() {
        let validator = DiscountFactorValidator::new();
        
        // Valid discount factors
        let valid_data = vec![1.0, 0.95, 0.90, 0.85];
        let result = validator.validate(&valid_data);
        assert!(result.is_success());

        // Invalid negative discount factor
        let invalid_data = vec![1.0, -0.95, 0.90];
        let result = validator.validate(&invalid_data);
        assert!(result.is_failure());

        // Invalid discount factor > 1
        let invalid_data = vec![1.0, 1.05, 0.90];
        let result = validator.validate(&invalid_data);
        assert!(result.is_failure());
    }

    #[test]
    fn test_discount_factor_validator_strict() {
        let validator = DiscountFactorValidator::new().strict();
        
        // Should fail with exactly 1.0
        let data_with_unity = vec![1.0, 0.95, 0.90];
        let result = validator.validate(&data_with_unity);
        assert!(result.is_failure());
    }

    #[test]
    fn test_rate_bounds_validator() {
        let validator = RateBoundsValidator::interest_rate();
        
        // Valid rate
        let result = validator.validate(&0.05); // 5%
        assert!(result.is_success());

        // Rate too high
        let result = validator.validate(&1.5); // 150%
        assert!(result.is_failure());

        // Rate with warning (high but valid)
        let result = validator.validate(&0.30); // 30%
        assert!(result.is_success());
        assert!(result.has_warnings());
    }

    #[test]
    fn test_date_range_validator() {
        let validator = DateRangeValidator::new();
        
        let start = Date::from_calendar_date(2024, Month::January, 15).unwrap();
        let end = Date::from_calendar_date(2024, Month::February, 15).unwrap();
        
        // Valid range
        let result = validator.validate(&(start, end));
        assert!(result.is_success());

        // Invalid inverted range
        let result = validator.validate(&(end, start));
        assert!(result.is_failure());
    }

    #[test]
    fn test_date_range_validator_duration_limits() {
        let validator = DateRangeValidator::new()
            .min_duration_days(7)
            .max_duration_days(30);
        
        let start = Date::from_calendar_date(2024, Month::January, 1).unwrap();
        
        // Too short duration
        let end_short = start + Duration::days(3);
        let result = validator.validate(&(start, end_short));
        assert!(result.is_failure());

        // Valid duration
        let end_valid = start + Duration::days(14);
        let result = validator.validate(&(start, end_valid));
        assert!(result.is_success());

        // Too long duration
        let end_long = start + Duration::days(45);
        let result = validator.validate(&(start, end_long));
        assert!(result.is_failure());
    }

    #[test]
    fn test_discount_factor_curve_validator() {
        let validator = DiscountFactorCurveValidator::new();
        
        // Valid decreasing discount factor curve
        let valid_curve = vec![1.0, 0.98, 0.95, 0.90, 0.85];
        let result = validator.validate(&valid_curve);
        assert!(result.is_success());

        // Invalid: contains negative value
        let invalid_curve = vec![1.0, 0.98, -0.95, 0.90];
        let result = validator.validate(&invalid_curve);
        assert!(result.is_failure());

        // Invalid: non-monotonic (arbitrage)
        let arbitrage_curve = vec![1.0, 0.95, 0.97, 0.90]; // increases then decreases
        let result = validator.validate(&arbitrage_curve);
        assert!(result.is_failure());
    }

    #[test]
    fn test_term_structure_knots_validator() {
        let validator = TermStructureKnotsValidator::standard_curve();
        
        // Valid increasing time points
        let valid_knots = vec![0.0, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0];
        let result = validator.validate(&valid_knots);
        assert!(result.is_success());

        // Invalid: non-increasing
        let invalid_knots = vec![0.0, 1.0, 0.5, 2.0];
        let result = validator.validate(&invalid_knots);
        assert!(result.is_failure());

        // Invalid: too few points
        let insufficient_knots = vec![1.0];
        let result = validator.validate(&insufficient_knots);
        assert!(result.is_failure());
    }
}
