//! Core validation traits for composable validation logic

use super::result::ValidationResult;

/// Core trait for validation operations
///
/// This trait enables composable validation logic by providing a standard
/// interface for validation operations. Validators can be chained, combined,
/// and reused across different contexts.
///
/// See unit tests and `examples/` for usage patterns.
pub trait Validator {
    /// The input type for validation
    type Input;
    /// The output type after successful validation
    type Output;

    /// Validate the input and return a validation result
    ///
    /// The implementation should:
    /// - Return `ValidationResult::pass()` for successful validation
    /// - Return `ValidationResult::pass_with_warnings()` for validation that succeeds but has concerns
    /// - Return `ValidationResult::fail()` for validation that fails
    fn validate(&self, input: &Self::Input) -> ValidationResult<Self::Output>;

    /// Optional method to provide a human-readable description of what this validator checks
    fn description(&self) -> Option<&'static str> {
        None
    }
}

/// Extension trait for combining validators
pub trait ValidatorExt: Validator + Sized {
    /// Chain this validator with another validator
    ///
    /// The second validator will only be called if this validator succeeds.
    /// Warnings from both validators will be combined.
    fn and_then<V>(self, next: V) -> ChainedValidator<Self, V>
    where
        V: Validator<Input = Self::Output>,
    {
        ChainedValidator {
            first: self,
            second: next,
        }
    }

    /// Add a transformation step after successful validation
    fn map<U, F>(self, f: F) -> MappedValidator<Self, U, F>
    where
        Self::Output: Clone,
        F: Fn(Self::Output) -> U + Send + Sync,
    {
        MappedValidator {
            validator: self,
            mapper: f,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Add a condition that must be met after validation
    fn with_condition<F>(
        self,
        condition: F,
        error_msg: &'static str,
    ) -> ConditionalValidator<Self, F>
    where
        F: Fn(&Self::Output) -> bool + Send + Sync,
    {
        ConditionalValidator {
            validator: self,
            condition,
            error_message: error_msg,
        }
    }
}

// Blanket implementation for all validators
impl<T: Validator> ValidatorExt for T {}

/// A validator that chains two validators together
pub struct ChainedValidator<A, B> {
    first: A,
    second: B,
}

impl<A, B> Validator for ChainedValidator<A, B>
where
    A: Validator,
    B: Validator<Input = A::Output>,
{
    type Input = A::Input;
    type Output = B::Output;

    fn validate(&self, input: &Self::Input) -> ValidationResult<Self::Output> {
        self.first
            .validate(input)
            .and_then(|intermediate| self.second.validate(&intermediate))
    }

    fn description(&self) -> Option<&'static str> {
        // If either validator has a description, we could combine them
        // For now, just return the first one
        self.first
            .description()
            .or_else(|| self.second.description())
    }
}

/// A validator that applies a transformation after successful validation
pub struct MappedValidator<V, U, F> {
    validator: V,
    mapper: F,
    _phantom: std::marker::PhantomData<U>,
}

impl<V, U, F> Validator for MappedValidator<V, U, F>
where
    V: Validator,
    V::Output: Clone,
    F: Fn(V::Output) -> U + Send + Sync,
{
    type Input = V::Input;
    type Output = U;

    fn validate(&self, input: &Self::Input) -> ValidationResult<Self::Output> {
        self.validator.validate(input).map(&self.mapper)
    }

    fn description(&self) -> Option<&'static str> {
        self.validator.description()
    }
}

/// A validator that adds an additional condition check
pub struct ConditionalValidator<V, F> {
    validator: V,
    condition: F,
    error_message: &'static str,
}

impl<V, F> Validator for ConditionalValidator<V, F>
where
    V: Validator,
    F: Fn(&V::Output) -> bool + Send + Sync,
{
    type Input = V::Input;
    type Output = V::Output;

    fn validate(&self, input: &Self::Input) -> ValidationResult<Self::Output> {
        self.validator.validate(input).and_then(|output| {
            if (self.condition)(&output) {
                ValidationResult::pass(output)
            } else {
                ValidationResult::fail(self.error_message)
            }
        })
    }

    fn description(&self) -> Option<&'static str> {
        self.validator.description()
    }
}

// Common validator implementations

/// Validator that checks if a numeric value is within a specified range
pub struct RangeValidator<T> {
    /// Optional minimum value for the range
    pub min: Option<T>,
    /// Optional maximum value for the range  
    pub max: Option<T>,
    /// Whether the minimum bound is inclusive (default: true)
    pub min_inclusive: bool,
    /// Whether the maximum bound is inclusive (default: true)
    pub max_inclusive: bool,
}

impl<T> RangeValidator<T>
where
    T: PartialOrd + Copy + std::fmt::Display,
{
    /// Create a new range validator
    pub fn new() -> Self {
        Self {
            min: None,
            max: None,
            min_inclusive: true,
            max_inclusive: true,
        }
    }

    /// Set minimum value (inclusive by default)
    pub fn min(mut self, min: T) -> Self {
        self.min = Some(min);
        self
    }

    /// Set maximum value (inclusive by default)  
    pub fn max(mut self, max: T) -> Self {
        self.max = Some(max);
        self
    }

    /// Set whether minimum bound is inclusive
    pub fn min_exclusive(mut self) -> Self {
        self.min_inclusive = false;
        self
    }

    /// Set whether maximum bound is inclusive
    pub fn max_exclusive(mut self) -> Self {
        self.max_inclusive = false;
        self
    }
}

impl<T> Default for RangeValidator<T>
where
    T: PartialOrd + Copy + std::fmt::Display,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Validator for RangeValidator<T>
where
    T: PartialOrd + Copy + std::fmt::Display,
{
    type Input = T;
    type Output = T;

    fn validate(&self, input: &Self::Input) -> ValidationResult<Self::Output> {
        let warnings = Vec::new();

        // Check minimum bound
        if let Some(min) = self.min {
            let valid = if self.min_inclusive {
                *input >= min
            } else {
                *input > min
            };

            if !valid {
                let op = if self.min_inclusive { ">=" } else { ">" };
                return ValidationResult::fail(format!("Value {} must be {} {}", input, op, min));
            }
        }

        // Check maximum bound
        if let Some(max) = self.max {
            let valid = if self.max_inclusive {
                *input <= max
            } else {
                *input < max
            };

            if !valid {
                let op = if self.max_inclusive { "<=" } else { "<" };
                return ValidationResult::fail(format!("Value {} must be {} {}", input, op, max));
            }
        }

        ValidationResult::pass_with_warnings(*input, warnings)
    }

    fn description(&self) -> Option<&'static str> {
        Some("Range validator")
    }
}

/// Validator that checks if a collection has the correct length
pub struct LengthValidator {
    /// Optional minimum length requirement
    pub min_length: Option<usize>,
    /// Optional maximum length requirement
    pub max_length: Option<usize>,
}

impl LengthValidator {
    /// Create a new length validator
    pub fn new() -> Self {
        Self {
            min_length: None,
            max_length: None,
        }
    }

    /// Set minimum length
    pub fn min_length(mut self, min: usize) -> Self {
        self.min_length = Some(min);
        self
    }

    /// Set maximum length
    pub fn max_length(mut self, max: usize) -> Self {
        self.max_length = Some(max);
        self
    }

    /// Set exact length requirement
    pub fn exact_length(mut self, length: usize) -> Self {
        self.min_length = Some(length);
        self.max_length = Some(length);
        self
    }
}

impl Default for LengthValidator {
    fn default() -> Self {
        Self::new()
    }
}

// Implement for Vec<i32> as a concrete example
impl Validator for LengthValidator {
    type Input = Vec<i32>;
    type Output = Vec<i32>;

    fn validate(&self, input: &Self::Input) -> ValidationResult<Self::Output> {
        let length = input.len();

        if let Some(min) = self.min_length {
            if length < min {
                return ValidationResult::fail(format!(
                    "Length {} is less than minimum required length {}",
                    length, min
                ));
            }
        }

        if let Some(max) = self.max_length {
            if length > max {
                return ValidationResult::fail(format!(
                    "Length {} exceeds maximum allowed length {}",
                    length, max
                ));
            }
        }

        ValidationResult::pass(input.clone())
    }

    fn description(&self) -> Option<&'static str> {
        Some("Length validator")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct PositiveValidator;

    impl Validator for PositiveValidator {
        type Input = f64;
        type Output = f64;

        fn validate(&self, input: &Self::Input) -> ValidationResult<Self::Output> {
            if *input > 0.0 {
                ValidationResult::pass(*input)
            } else {
                ValidationResult::fail("Value must be positive")
            }
        }
    }

    struct DoubleValidator;

    impl Validator for DoubleValidator {
        type Input = f64;
        type Output = f64;

        fn validate(&self, input: &Self::Input) -> ValidationResult<Self::Output> {
            ValidationResult::pass(*input * 2.0)
        }
    }

    #[test]
    fn basic_validator() {
        let validator = PositiveValidator;

        let result1 = validator.validate(&5.0);
        assert!(result1.is_success());
        assert_eq!(result1.value_ref(), Some(&5.0));

        let result2 = validator.validate(&-1.0);
        assert!(result2.is_failure());
        assert_eq!(result2.error_message(), Some("Value must be positive"));
    }

    #[test]
    fn chained_validators() {
        let validator = PositiveValidator.and_then(DoubleValidator);

        let result = validator.validate(&3.0);
        assert!(result.is_success());
        assert_eq!(result.value_ref(), Some(&6.0));

        let fail_result = validator.validate(&-1.0);
        assert!(fail_result.is_failure());
    }

    #[test]
    fn range_validator() {
        let validator = RangeValidator::<f64>::new().min(0.0).max(100.0);

        assert!(validator.validate(&50.0).is_success());
        assert!(validator.validate(&0.0).is_success());
        assert!(validator.validate(&100.0).is_success());

        assert!(validator.validate(&-1.0).is_failure());
        assert!(validator.validate(&101.0).is_failure());
    }

    #[test]
    fn length_validator() {
        let validator = LengthValidator::new().min_length(2).max_length(5);

        assert!(validator.validate(&vec![1i32, 2, 3]).is_success());
        assert!(validator.validate(&vec![1i32, 2]).is_success());
        assert!(validator.validate(&vec![1i32, 2, 3, 4, 5]).is_success());

        assert!(validator.validate(&vec![1i32]).is_failure());
        assert!(validator.validate(&vec![1i32, 2, 3, 4, 5, 6]).is_failure());
    }

    #[test]
    fn conditional_validator() {
        let validator =
            PositiveValidator.with_condition(|&x| x < 100.0, "Value must be less than 100");

        assert!(validator.validate(&50.0).is_success());
        assert!(validator.validate(&150.0).is_failure());
    }
}
