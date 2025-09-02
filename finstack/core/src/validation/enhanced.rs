//! Enhanced validation features for multi-error accumulation and domain-specific validation
//!
//! This module extends the core validation framework with capabilities for:
//! - Accumulating multiple validation errors instead of failing fast
//! - Domain-specific validators for financial primitives
//! - Batch validation across multiple fields

use super::{ValidationResult, ValidationStatus, ValidationWarning, Validator};
use crate::{currency::Currency, error::InputError, money::Money, F};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Enhanced validation context that accumulates multiple errors
///
/// This struct allows running multiple validators in parallel and collecting
/// all validation errors, providing comprehensive feedback to the API consumer.
#[derive(Debug, Clone, Default)]
pub struct ValidationErrors {
    /// List of accumulated error messages with field context
    pub errors: Vec<(String, String)>, // (field, message)
    /// List of accumulated warnings
    pub warnings: Vec<ValidationWarning>,
}

impl ValidationErrors {
    /// Create a new empty validation error collector
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Add an error with field context
    pub fn add_error(&mut self, field: impl Into<String>, message: impl Into<String>) {
        self.errors.push((field.into(), message.into()));
    }

    /// Add a warning with field context
    pub fn add_warning(&mut self, field: impl Into<String>, message: impl Into<String>) {
        self.warnings.push(ValidationWarning::with_context(message, field));
    }

    /// Check if there are any errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Check if there are any warnings
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    /// Get the overall validation status
    pub fn status(&self) -> ValidationStatus {
        if self.has_errors() {
            ValidationStatus::Fail
        } else if self.has_warnings() {
            ValidationStatus::Warning
        } else {
            ValidationStatus::Pass
        }
    }

    /// Convert to a ValidationResult
    pub fn into_result<T>(self, value: T) -> ValidationResult<T> {
        if self.has_errors() {
            let error_messages: Vec<String> = self.errors
                .iter()
                .map(|(field, msg)| format!("{}: {}", field, msg))
                .collect();
            ValidationResult::fail_with_warnings(
                error_messages.join("; "),
                self.warnings,
            )
        } else {
            ValidationResult::pass_with_warnings(value, self.warnings)
        }
    }

    /// Collect errors and warnings from a ValidationResult
    pub fn collect_from_result<T>(&mut self, field: &str, result: ValidationResult<T>) {
        // Collect warnings with field context
        for warning in result.warnings() {
            let contextualized = ValidationWarning::with_context(&warning.message, field);
            self.warnings.push(contextualized);
        }

        // Collect errors
        if result.is_failure() {
            if let Some(error_msg) = result.error_message() {
                self.add_error(field, error_msg);
            }
        }
    }
}

/// Multi-validator that runs multiple validators and accumulates all errors
pub struct MultiValidator<T> {
    validators: Vec<(String, Box<dyn Fn(&T) -> ValidationResult<T> + Send + Sync>)>,
}

impl<T> MultiValidator<T> {
    /// Create a new multi-validator
    pub fn new() -> Self {
        Self {
            validators: Vec::new(),
        }
    }

    /// Add a validator with field name for context
    pub fn add_validator<V>(mut self, field_name: impl Into<String>, validator: V) -> Self
    where
        V: Validator<Input = T, Output = T> + Send + Sync + 'static,
    {
        self.validators.push((
            field_name.into(),
            Box::new(move |input| validator.validate(input)),
        ));
        self
    }

    /// Validate input using all validators and accumulate errors
    pub fn validate_all(&self, input: &T) -> ValidationErrors
    where
        T: Clone,
    {
        let mut errors = ValidationErrors::new();

        for (field_name, validator) in &self.validators {
            let result = validator(input);
            errors.collect_from_result(field_name, result);
        }

        errors
    }
}

impl<T> Default for MultiValidator<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Collection of domain-specific financial validators
pub struct FinancialValidators;

impl FinancialValidators {
    /// Validate a financial rate (with reasonable bounds and warnings for extreme values)
    pub fn validate_rate(errors: &mut ValidationErrors, field: &str, rate: F) {
        // Check for reasonable bounds (-100% to very high values)
        if rate < -1.0 {
            errors.add_error(field, format!("Rate {:.2}% is below -100%", rate * 100.0));
        } else if rate > 50.0 {
            errors.add_error(field, format!("Rate {:.2}% exceeds 5000% (possibly input error)", rate * 100.0));
        } else if rate > 5.0 {
            errors.add_warning(field, format!("Very high rate: {:.2}%", rate * 100.0));
        }
    }

    /// Validate a positive financial amount
    pub fn validate_positive_amount(errors: &mut ValidationErrors, field: &str, amount: F) {
        if amount <= 0.0 {
            errors.add_error(field, "Amount must be positive");
        }
    }

    /// Validate a non-negative financial amount
    pub fn validate_non_negative_amount(errors: &mut ValidationErrors, field: &str, amount: F) {
        if amount < 0.0 {
            errors.add_error(field, "Amount must be non-negative");
        }
    }

    /// Validate a probability value (0.0 to 1.0)
    pub fn validate_probability(errors: &mut ValidationErrors, field: &str, prob: F) {
        if prob < 0.0 || prob > 1.0 {
            errors.add_error(field, format!("Probability {} must be between 0.0 and 1.0", prob));
        }
    }

    /// Validate a monotonic sequence
    pub fn validate_monotonic_sequence(errors: &mut ValidationErrors, field: &str, sequence: &[F]) {
        if sequence.len() < 2 {
            errors.add_error(field, "At least two points required for monotonic check");
            return;
        }

        for i in 1..sequence.len() {
            if sequence[i] <= sequence[i - 1] {
                errors.add_error(field, format!(
                    "Non-monotonic sequence at index {}: {} <= {}",
                    i, sequence[i], sequence[i - 1]
                ));
                return; // Stop on first violation to avoid spam
            }
        }
    }

    /// Validate currency consistency across multiple Money values
    pub fn validate_currency_consistency(errors: &mut ValidationErrors, field: &str, amounts: &[Money]) {
        if amounts.is_empty() {
            return;
        }

        let reference_currency = amounts[0].currency();
        for (i, money) in amounts.iter().enumerate() {
            if money.currency() != reference_currency {
                errors.add_error(field, format!(
                    "Currency mismatch at index {}: expected {}, got {}",
                    i, reference_currency, money.currency()
                ));
            }
        }
    }

    /// Validate minimum data points requirement
    pub fn validate_min_points<T>(errors: &mut ValidationErrors, field: &str, data: &[T], min_count: usize) {
        if data.len() < min_count {
            errors.add_error(field, format!(
                "Insufficient data points: {} provided, {} required",
                data.len(),
                min_count
            ));
        }
    }

    /// Validate date sequence (strictly increasing)
    pub fn validate_date_sequence(errors: &mut ValidationErrors, field: &str, dates: &[crate::Date]) {
        if dates.len() < 2 {
            return;
        }

        for i in 1..dates.len() {
            if dates[i] <= dates[i - 1] {
                errors.add_error(field, format!(
                    "Date sequence violation at index {}: {} is not after {}",
                    i, dates[i], dates[i - 1]
                ));
                return; // Stop on first violation
            }
        }
    }

    /// Validate that a value is within a reasonable range with warnings for edge cases
    pub fn validate_range_with_warnings(
        errors: &mut ValidationErrors, 
        field: &str, 
        value: F, 
        min: F, 
        max: F,
        warn_near_bounds: bool
    ) {
        if value < min {
            errors.add_error(field, format!("Value {} is below minimum {}", value, min));
        } else if value > max {
            errors.add_error(field, format!("Value {} exceeds maximum {}", value, max));
        } else if warn_near_bounds {
            let range = max - min;
            let tolerance = range * 0.05; // 5% of range
            
            if value - min < tolerance {
                errors.add_warning(field, format!("Value {} is very close to minimum {}", value, min));
            } else if max - value < tolerance {
                errors.add_warning(field, format!("Value {} is very close to maximum {}", value, max));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Currency;

    #[test]
    fn test_validation_errors_accumulation() {
        let mut errors = ValidationErrors::new();
        errors.add_error("rate", "Rate must be positive");
        errors.add_error("amount", "Amount too large");
        errors.add_warning("volatility", "Volatility seems high");

        assert!(errors.has_errors());
        assert!(errors.has_warnings());
        assert_eq!(errors.status(), ValidationStatus::Fail);
        assert_eq!(errors.errors.len(), 2);
        assert_eq!(errors.warnings.len(), 1);
    }

    #[test]
    fn test_financial_validators() {
        let mut errors = ValidationErrors::new();
        
        // Valid values
        FinancialValidators::validate_rate(&mut errors, "rate", 0.05);
        FinancialValidators::validate_positive_amount(&mut errors, "amount", 1000.0);
        FinancialValidators::validate_probability(&mut errors, "prob", 0.02);

        assert!(!errors.has_errors());
        assert_eq!(errors.status(), ValidationStatus::Pass);

        // Invalid values
        let mut errors = ValidationErrors::new();
        FinancialValidators::validate_positive_amount(&mut errors, "amount", -100.0);
        FinancialValidators::validate_probability(&mut errors, "prob", 1.5);

        assert!(errors.has_errors());
        assert_eq!(errors.errors.len(), 2);
    }

    #[test]
    fn test_currency_consistency() {
        let mut errors = ValidationErrors::new();
        let usd = Currency::USD;
        let eur = Currency::EUR;

        let amounts = vec![
            Money::new(100.0, usd),
            Money::new(200.0, usd),
        ];

        FinancialValidators::validate_currency_consistency(&mut errors, "amounts", &amounts);
        assert!(!errors.has_errors());

        // Mixed currencies
        let mixed_amounts = vec![
            Money::new(100.0, usd),
            Money::new(200.0, eur),
        ];

        FinancialValidators::validate_currency_consistency(&mut errors, "mixed_amounts", &mixed_amounts);
        assert!(errors.has_errors());
    }

    #[test]
    fn test_monotonic_sequence() {
        let mut errors = ValidationErrors::new();
        
        let valid_seq = vec![1.0, 2.0, 3.0, 5.0];
        FinancialValidators::validate_monotonic_sequence(&mut errors, "times", &valid_seq);
        assert!(!errors.has_errors());

        let invalid_seq = vec![1.0, 3.0, 2.0, 5.0];
        FinancialValidators::validate_monotonic_sequence(&mut errors, "bad_times", &invalid_seq);
        assert!(errors.has_errors());
    }

    #[test]
    fn test_range_with_warnings() {
        let mut errors = ValidationErrors::new();
        
        // Value close to minimum should generate warning
        FinancialValidators::validate_range_with_warnings(&mut errors, "value", 0.02, 0.0, 1.0, true);
        assert!(!errors.has_errors());
        assert!(errors.has_warnings());
        
        // Value out of range should generate error
        let mut errors = ValidationErrors::new();
        FinancialValidators::validate_range_with_warnings(&mut errors, "value", -0.1, 0.0, 1.0, true);
        assert!(errors.has_errors());
    }
}
