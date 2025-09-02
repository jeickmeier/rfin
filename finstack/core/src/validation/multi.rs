//! Multi-validator framework for accumulating validation errors
//!
//! This module provides enhanced validation capabilities that can run multiple
//! validators in parallel and accumulate all errors and warnings, providing
//! comprehensive feedback to API consumers.

use super::{ValidationResult, ValidationStatus, ValidationWarning, Validator};
use crate::{error::InputError, Result, F};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Enhanced validation context that accumulates multiple errors
///
/// This struct allows running multiple validators in parallel and collecting
/// all validation errors, providing comprehensive feedback to the API consumer.
#[derive(Debug, Clone)]
pub struct ValidationErrors {
    /// List of accumulated errors
    pub errors: Vec<ValidationError>,
    /// List of accumulated warnings
    pub warnings: Vec<ValidationWarning>,
}

/// A single validation error with context
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ValidationError {
    /// The error message
    pub message: String,
    /// Optional field or context where the error occurred
    pub field: Option<String>,
    /// Optional error code for programmatic handling
    pub code: Option<String>,
    /// The underlying input error type if applicable
    pub input_error: Option<InputError>,
}

impl ValidationError {
    /// Create a new validation error with just a message
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            field: None,
            code: None,
            input_error: None,
        }
    }

    /// Create an error with field context
    pub fn with_field(message: impl Into<String>, field: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            field: Some(field.into()),
            code: None,
            input_error: None,
        }
    }

    /// Create an error with a code
    pub fn with_code(message: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            field: None,
            code: Some(code.into()),
            input_error: None,
        }
    }

    /// Create an error from an InputError
    pub fn from_input_error(input_error: InputError, field: Option<String>) -> Self {
        Self {
            message: input_error.to_string(),
            field,
            code: None,
            input_error: Some(input_error),
        }
    }
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ref field) = self.field {
            write!(f, "{}: {}", field, self.message)
        } else {
            write!(f, "{}", self.message)
        }
        if let Some(ref code) = self.code {
            write!(f, " [{}]", code)
        }
        Ok(())
    }
}

impl From<InputError> for ValidationError {
    fn from(error: InputError) -> Self {
        Self::from_input_error(error, None)
    }
}

impl ValidationErrors {
    /// Create a new empty validation error collector
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Add a validation error
    pub fn add_error(&mut self, error: ValidationError) {
        self.errors.push(error);
    }

    /// Add a validation warning
    pub fn add_warning(&mut self, warning: ValidationWarning) {
        self.warnings.push(warning);
    }

    /// Add an error with field context
    pub fn add_field_error(&mut self, field: impl Into<String>, message: impl Into<String>) {
        self.errors.push(ValidationError::with_field(message, field));
    }

    /// Add a warning with field context
    pub fn add_field_warning(&mut self, field: impl Into<String>, message: impl Into<String>) {
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
            let error_messages: Vec<String> = self.errors.iter().map(|e| e.to_string()).collect();
            ValidationResult::fail_with_warnings(
                error_messages.join("; "),
                self.warnings,
            )
        } else {
            ValidationResult::pass_with_warnings(value, self.warnings)
        }
    }

    /// Convert to a Result type with accumulated error messages
    pub fn into_std_result<T>(self, value: T) -> Result<T> {
        if self.has_errors() {
            let _error_messages: Vec<String> = self.errors.iter().map(|e| e.to_string()).collect();
            Err(crate::Error::Input(InputError::Invalid))
        } else {
            Ok(value)
        }
    }

    /// Helper to collect errors and warnings from a ValidationResult
    pub fn collect_from_result<T>(&mut self, field: &str, result: ValidationResult<T>) {
        // Collect warnings with field context
        for warning in result.warnings() {
            let contextualized = ValidationWarning::with_context(&warning.message, field);
            self.warnings.push(contextualized);
        }

        // Collect errors
        if result.is_failure() {
            if let Some(error_msg) = result.error_message() {
                self.add_field_error(field, error_msg);
            }
        }
    }
}

impl Default for ValidationErrors {
    fn default() -> Self {
        Self::new()
    }
}

/// Multi-validator that runs multiple validators and accumulates all errors
pub struct MultiValidator<T> {
    validators: Vec<Box<dyn Fn(&T) -> ValidationResult<T> + Send + Sync>>,
    field_names: Vec<String>,
}

impl<T> MultiValidator<T> {
    /// Create a new multi-validator
    pub fn new() -> Self {
        Self {
            validators: Vec::new(),
            field_names: Vec::new(),
        }
    }

    /// Add a validator with optional field name for context
    pub fn add_validator<V>(mut self, field_name: impl Into<String>, validator: V) -> Self
    where
        V: Validator<Input = T, Output = T> + Send + Sync + 'static,
    {
        self.field_names.push(field_name.into());
        self.validators.push(Box::new(move |input| validator.validate(input)));
        self
    }

    /// Validate input using all validators and accumulate errors
    pub fn validate_all(&self, input: &T) -> ValidationErrors
    where
        T: Clone,
    {
        let mut errors = ValidationErrors::new();

        for (field_name, validator) in self.field_names.iter().zip(&self.validators) {
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

/// Batch validator for running multiple independent validators
pub struct BatchValidator<T> {
    validators: Vec<(String, Box<dyn Fn(&T) -> ValidationResult<T> + Send + Sync>)>,
}

impl<T> BatchValidator<T> {
    /// Create a new batch validator
    pub fn new() -> Self {
        Self {
            validators: Vec::new(),
        }
    }

    /// Add a validator to the batch with a name for error reporting
    pub fn add<V>(mut self, name: impl Into<String>, validator: V) -> Self
    where
        V: Validator<Input = T, Output = T> + Send + Sync + 'static,
    {
        self.validators.push((
            name.into(),
            Box::new(move |input| validator.validate(input)),
        ));
        self
    }

    /// Run all validators and collect errors/warnings
    pub fn validate_all(&self, input: &T) -> ValidationErrors
    where
        T: Clone,
    {
        let mut errors = ValidationErrors::new();

        for (name, validator) in &self.validators {
            let result = validator(input);
            errors.collect_from_result(name, result);
        }

        errors
    }
}

impl<T> Default for BatchValidator<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Validator for BatchValidator<T> {
    type Input = T;
    type Output = T;

    fn validate(&self, input: &Self::Input) -> ValidationResult<Self::Output>
    where
        T: Clone,
    {
        let errors = self.validate_all(input);
        errors.into_result(input.clone())
    }

    fn description(&self) -> Option<&'static str> {
        Some("Batch validator")
    }
}

/// Builder pattern for creating comprehensive validation pipelines
pub struct ValidationPipeline<T> {
    validators: Vec<(String, Box<dyn Fn(&T) -> ValidationResult<T> + Send + Sync>)>,
    warnings_as_errors: bool,
}

impl<T> ValidationPipeline<T> {
    /// Create a new validation pipeline
    pub fn new() -> Self {
        Self {
            validators: Vec::new(),
            warnings_as_errors: false,
        }
    }

    /// Add a validator to the pipeline
    pub fn add<V>(mut self, name: impl Into<String>, validator: V) -> Self
    where
        V: Validator<Input = T, Output = T> + Send + Sync + 'static,
    {
        self.validators.push((
            name.into(),
            Box::new(move |input| validator.validate(input)),
        ));
        self
    }

    /// Treat warnings as errors (fail validation if any warnings)
    pub fn warnings_as_errors(mut self) -> Self {
        self.warnings_as_errors = true;
        self
    }

    /// Execute the validation pipeline
    pub fn execute(&self, input: &T) -> ValidationResult<T>
    where
        T: Clone,
    {
        let mut all_warnings = Vec::new();
        let mut has_errors = false;
        let mut error_messages = Vec::new();

        for (name, validator) in &self.validators {
            let result = validator(input);

            // Collect warnings
            for warning in result.warnings() {
                let contextualized = ValidationWarning::with_context(&warning.message, name);
                all_warnings.push(contextualized);
            }

            // Collect errors
            if result.is_failure() {
                has_errors = true;
                if let Some(error_msg) = result.error_message() {
                    error_messages.push(format!("{}: {}", name, error_msg));
                }
            }
        }

        // Check if warnings should be treated as errors
        if self.warnings_as_errors && !all_warnings.is_empty() {
            has_errors = true;
            let warning_messages: Vec<String> = all_warnings.iter()
                .map(|w| w.to_string())
                .collect();
            error_messages.extend(warning_messages);
            all_warnings.clear(); // Move warnings to errors
        }

        if has_errors {
            ValidationResult::fail_with_warnings(
                error_messages.join("; "),
                all_warnings,
            )
        } else {
            ValidationResult::pass_with_warnings(input.clone(), all_warnings)
        }
    }
}

impl<T> Default for ValidationPipeline<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Validator for ValidationPipeline<T> {
    type Input = T;
    type Output = T;

    fn validate(&self, input: &Self::Input) -> ValidationResult<Self::Output>
    where
        T: Clone,
    {
        self.execute(input)
    }

    fn description(&self) -> Option<&'static str> {
        Some("Validation pipeline")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validation::{RangeValidator};

    // Mock validator for testing
    struct PositiveValidator;

    impl Validator for PositiveValidator {
        type Input = F;
        type Output = F;

        fn validate(&self, input: &Self::Input) -> ValidationResult<Self::Output> {
            if *input > 0.0 {
                ValidationResult::pass(*input)
            } else {
                ValidationResult::fail("Value must be positive")
            }
        }
    }

    #[test]
    fn test_validation_errors_accumulation() {
        let mut errors = ValidationErrors::new();
        errors.add_field_error("rate", "Rate must be positive");
        errors.add_field_error("amount", "Amount too large");
        errors.add_field_warning("volatility", "Volatility seems high");

        assert!(errors.has_errors());
        assert!(errors.has_warnings());
        assert_eq!(errors.status(), ValidationStatus::Fail);
        assert_eq!(errors.errors.len(), 2);
        assert_eq!(errors.warnings.len(), 1);
    }

    #[test]
    fn test_multi_validator() {
        let validator = MultiValidator::new()
            .add_validator("positive_check", PositiveValidator)
            .add_validator("range_check", RangeValidator::new().max(100.0));

        // Valid input
        let errors = validator.validate_all(&50.0);
        assert!(!errors.has_errors());
        assert_eq!(errors.status(), ValidationStatus::Pass);

        // Invalid input (negative)
        let errors = validator.validate_all(&-10.0);
        assert!(errors.has_errors());
        assert_eq!(errors.errors.len(), 1);
        assert!(errors.errors[0].message.contains("positive"));

        // Invalid input (out of range)
        let errors = validator.validate_all(&150.0);
        assert!(errors.has_errors());
        assert_eq!(errors.errors.len(), 1);
        assert!(errors.errors[0].message.contains("must be"));
    }

    #[test]
    fn test_validation_pipeline() {
        let pipeline = ValidationPipeline::new()
            .add("positive", PositiveValidator)
            .add("range", RangeValidator::new().max(100.0));

        // Valid input
        let result = pipeline.execute(&50.0);
        assert!(result.is_success());

        // Invalid input
        let result = pipeline.execute(&-10.0);
        assert!(result.is_failure());
        assert!(result.error_message().unwrap().contains("positive"));
    }

    #[test]
    fn test_integration_with_input_error() {
        let input_error = InputError::NonPositiveValue;
        let validation_error = ValidationError::from_input_error(input_error, Some("rate".to_string()));

        assert_eq!(validation_error.field.as_deref(), Some("rate"));
        assert_eq!(validation_error.message, "Values must be positive");
    }
}
