//! Validation result types supporting warnings and pass/fail status

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Status of a validation check
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ValidationStatus {
    /// Validation passed without issues
    Pass,
    /// Validation passed but with warnings
    Warning,
    /// Validation failed - result should not be used
    Fail,
}

impl ValidationStatus {
    /// Check if the validation succeeded (Pass or Warning)
    pub fn is_success(self) -> bool {
        matches!(self, ValidationStatus::Pass | ValidationStatus::Warning)
    }

    /// Check if the validation failed
    pub fn is_failure(self) -> bool {
        matches!(self, ValidationStatus::Fail)
    }

    /// Check if there are warnings (but not failures)
    pub fn has_warnings(self) -> bool {
        matches!(self, ValidationStatus::Warning)
    }
}

/// A single validation warning message
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ValidationWarning {
    /// The warning message
    pub message: String,
    /// Optional code for programmatic handling
    pub code: Option<String>,
    /// Optional context (e.g., field name, index)
    pub context: Option<String>,
}

impl ValidationWarning {
    /// Create a new warning with just a message
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            code: None,
            context: None,
        }
    }

    /// Create a warning with a code
    pub fn with_code(message: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            code: Some(code.into()),
            context: None,
        }
    }

    /// Create a warning with context
    pub fn with_context(message: impl Into<String>, context: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            code: None,
            context: Some(context.into()),
        }
    }

    /// Create a warning with both code and context
    pub fn full(
        message: impl Into<String>,
        code: impl Into<String>,
        context: impl Into<String>,
    ) -> Self {
        Self {
            message: message.into(),
            code: Some(code.into()),
            context: Some(context.into()),
        }
    }
}

impl std::fmt::Display for ValidationWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)?;
        if let Some(ref context) = self.context {
            write!(f, " (context: {})", context)?;
        }
        if let Some(ref code) = self.code {
            write!(f, " [code: {}]", code)?;
        }
        Ok(())
    }
}

/// Result of a validation operation
///
/// This type encapsulates the result of validation along with any warnings
/// that were generated during the validation process.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ValidationResult<T> {
    /// The validated value (if validation succeeded)
    value: Option<T>,
    /// Overall validation status
    status: ValidationStatus,
    /// List of warnings generated during validation
    warnings: Vec<ValidationWarning>,
    /// Optional error message (for failures)
    error_message: Option<String>,
}

impl<T> ValidationResult<T> {
    /// Create a successful validation result
    pub fn pass(value: T) -> Self {
        Self {
            value: Some(value),
            status: ValidationStatus::Pass,
            warnings: Vec::new(),
            error_message: None,
        }
    }

    /// Create a successful validation result with warnings
    pub fn pass_with_warnings(value: T, warnings: Vec<ValidationWarning>) -> Self {
        let status = if warnings.is_empty() {
            ValidationStatus::Pass
        } else {
            ValidationStatus::Warning
        };

        Self {
            value: Some(value),
            status,
            warnings,
            error_message: None,
        }
    }

    /// Create a failed validation result
    pub fn fail(error_message: impl Into<String>) -> Self {
        Self {
            value: None,
            status: ValidationStatus::Fail,
            warnings: Vec::new(),
            error_message: Some(error_message.into()),
        }
    }

    /// Create a failed validation result with warnings collected during validation
    pub fn fail_with_warnings(
        error_message: impl Into<String>,
        warnings: Vec<ValidationWarning>,
    ) -> Self {
        Self {
            value: None,
            status: ValidationStatus::Fail,
            warnings,
            error_message: Some(error_message.into()),
        }
    }

    /// Get the validation status
    pub fn status(&self) -> ValidationStatus {
        self.status
    }

    /// Check if validation succeeded (Pass or Warning)
    pub fn is_success(&self) -> bool {
        self.status.is_success()
    }

    /// Check if validation failed
    pub fn is_failure(&self) -> bool {
        self.status.is_failure()
    }

    /// Check if there are warnings
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    /// Get the warnings
    pub fn warnings(&self) -> &[ValidationWarning] {
        &self.warnings
    }

    /// Get the error message (if validation failed)
    pub fn error_message(&self) -> Option<&str> {
        self.error_message.as_deref()
    }

    /// Unwrap the value if validation succeeded
    ///
    /// # Panics
    ///
    /// Panics if the validation failed
    pub fn unwrap(self) -> T {
        match self.value {
            Some(value) => value,
            None => panic!("called `ValidationResult::unwrap()` on a failed validation"),
        }
    }

    /// Get the value if validation succeeded
    pub fn value(self) -> Option<T> {
        self.value
    }

    /// Get a reference to the value if validation succeeded
    pub fn value_ref(&self) -> Option<&T> {
        self.value.as_ref()
    }

    /// Convert into a Result type
    pub fn into_result(self) -> Result<T, String> {
        match self.value {
            Some(value) => Ok(value),
            None => Err(self
                .error_message
                .unwrap_or_else(|| "Validation failed".to_string())),
        }
    }

    /// Map the contained value to a new type using a function
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> ValidationResult<U> {
        ValidationResult {
            value: self.value.map(f),
            status: self.status,
            warnings: self.warnings,
            error_message: self.error_message,
        }
    }

    /// Chain validation results, collecting warnings from both
    pub fn and_then<U>(self, f: impl FnOnce(T) -> ValidationResult<U>) -> ValidationResult<U> {
        match self.value {
            Some(value) => {
                let mut next_result = f(value);
                // Collect warnings from both results
                let mut all_warnings = self.warnings;
                all_warnings.extend(next_result.warnings);
                next_result.warnings = all_warnings;

                // If we had warnings before, preserve the warning status
                if self.status == ValidationStatus::Warning
                    && next_result.status == ValidationStatus::Pass
                {
                    next_result.status = ValidationStatus::Warning;
                }

                next_result
            }
            None => ValidationResult {
                value: None,
                status: self.status,
                warnings: self.warnings,
                error_message: self.error_message,
            },
        }
    }

    /// Add additional warnings to the result
    pub fn with_additional_warnings(mut self, mut warnings: Vec<ValidationWarning>) -> Self {
        self.warnings.append(&mut warnings);

        // Update status if we now have warnings and previously had none
        if !self.warnings.is_empty() && self.status == ValidationStatus::Pass {
            self.status = ValidationStatus::Warning;
        }

        self
    }
}

impl<T> From<ValidationResult<T>> for Result<T, String> {
    fn from(validation_result: ValidationResult<T>) -> Self {
        validation_result.into_result()
    }
}

// Implement Display for easy error reporting
impl<T> std::fmt::Display for ValidationResult<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.status {
            ValidationStatus::Pass => write!(f, "Validation passed"),
            ValidationStatus::Warning => {
                write!(f, "Validation passed with {} warnings", self.warnings.len())
            }
            ValidationStatus::Fail => {
                if let Some(ref msg) = self.error_message {
                    write!(f, "Validation failed: {}", msg)
                } else {
                    write!(f, "Validation failed")
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_status_checks() {
        assert!(ValidationStatus::Pass.is_success());
        assert!(ValidationStatus::Warning.is_success());
        assert!(!ValidationStatus::Fail.is_success());

        assert!(!ValidationStatus::Pass.is_failure());
        assert!(!ValidationStatus::Warning.is_failure());
        assert!(ValidationStatus::Fail.is_failure());

        assert!(!ValidationStatus::Pass.has_warnings());
        assert!(ValidationStatus::Warning.has_warnings());
        assert!(!ValidationStatus::Fail.has_warnings());
    }

    #[test]
    fn validation_warning_creation() {
        let w1 = ValidationWarning::new("Simple message");
        assert_eq!(w1.message, "Simple message");
        assert!(w1.code.is_none());
        assert!(w1.context.is_none());

        let w2 = ValidationWarning::with_code("Message with code", "E001");
        assert_eq!(w2.code.as_deref(), Some("E001"));

        let w3 = ValidationWarning::with_context("Message with context", "field_name");
        assert_eq!(w3.context.as_deref(), Some("field_name"));

        let w4 = ValidationWarning::full("Full warning", "E002", "field_x");
        assert_eq!(w4.code.as_deref(), Some("E002"));
        assert_eq!(w4.context.as_deref(), Some("field_x"));
    }

    #[test]
    fn validation_result_creation() {
        let pass = ValidationResult::pass(42);
        assert!(pass.is_success());
        assert!(!pass.has_warnings());
        assert_eq!(pass.value_ref(), Some(&42));

        let pass_warn =
            ValidationResult::pass_with_warnings(100, vec![ValidationWarning::new("Minor issue")]);
        assert!(pass_warn.is_success());
        assert!(pass_warn.has_warnings());
        assert_eq!(pass_warn.warnings().len(), 1);

        let fail = ValidationResult::<i32>::fail("Something went wrong");
        assert!(fail.is_failure());
        assert_eq!(fail.error_message(), Some("Something went wrong"));
        assert_eq!(fail.value_ref(), None);
    }

    #[test]
    fn validation_result_chaining() {
        let first = ValidationResult::pass(10);
        let result = first.and_then(|x| {
            if x > 5 {
                ValidationResult::pass(x * 2)
            } else {
                ValidationResult::fail("Too small")
            }
        });

        assert!(result.is_success());
        assert_eq!(result.value_ref(), Some(&20));
    }

    #[test]
    fn validation_result_mapping() {
        let result = ValidationResult::pass(5);
        let mapped = result.map(|x| x.to_string());

        assert!(mapped.is_success());
        assert_eq!(mapped.value_ref(), Some(&"5".to_string()));
    }
}
