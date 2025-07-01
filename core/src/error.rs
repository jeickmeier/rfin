//! Error types for the rfin-core library.

use super::primitives::currency::Currency;
use core::fmt;

#[cfg(not(feature = "std"))]
extern crate alloc;

/// Main error type for rfin-core operations.
///
/// This error type is non-exhaustive, meaning new variants may be added
/// in future versions without breaking existing code.
///
/// # Examples
///
/// ```
/// use rfin_core::error::{Error, InputError};
///
/// let error = Error::Input(InputError::InvalidCurrency);
/// println!("Error: {}", error);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// Input validation errors.
    Input(InputError),
    /// Calculation or computational errors.
    Calculation(CalculationError),
    /// System or configuration errors.
    System(SystemError),
}

/// Input validation error variants.
///
/// # Examples
///
/// ```
/// use rfin_core::error::InputError;
/// use rfin_core::primitives::Currency;
///
/// let mismatch = InputError::CurrencyMismatch {
///     expected: Currency::USD,
///     actual: Currency::EUR,
/// };
/// assert_eq!(format!("{}", mismatch), "Currency mismatch: expected USD, got EUR");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputError {
    /// Currency mismatch in money operations.
    CurrencyMismatch {
        /// The first currency involved in the operation.
        expected: Currency,
        /// The second currency that caused the mismatch.
        actual: Currency,
    },
    /// Invalid currency operation.
    InvalidCurrency,
    /// Numeric overflow in money calculations.
    Overflow,
    /// Division by zero.
    DivisionByZero,
}

/// Calculation error variants.
///
/// # Examples  
///
/// ```
/// use rfin_core::error::CalculationError;
///
/// let overflow = CalculationError::Overflow;
/// assert_eq!(format!("{}", overflow), "Numeric overflow or underflow");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CalculationError {
    /// Numeric overflow or underflow.
    Overflow,
    /// Precision loss in calculations.
    PrecisionLoss,
    /// Invalid result from calculation.
    InvalidResult,
}

/// System error variants.
///
/// # Examples
///
/// ```
/// use rfin_core::error::SystemError;
///
/// let config_error = SystemError::Configuration;
/// assert_eq!(format!("{}", config_error), "Configuration error");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SystemError {
    /// Configuration error.
    Configuration,
    /// Resource unavailable.
    ResourceUnavailable,
    /// Internal error.
    Internal,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Input(err) => write!(f, "Input error: {}", err),
            Error::Calculation(err) => write!(f, "Calculation error: {}", err),
            Error::System(err) => write!(f, "System error: {}", err),
        }
    }
}

impl fmt::Display for InputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InputError::CurrencyMismatch { expected, actual } => {
                write!(
                    f,
                    "Currency mismatch: expected {}, got {}",
                    expected, actual
                )
            }
            InputError::InvalidCurrency => write!(f, "Invalid currency"),
            InputError::Overflow => write!(f, "Numeric overflow"),
            InputError::DivisionByZero => write!(f, "Division by zero"),
        }
    }
}

impl fmt::Display for CalculationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CalculationError::Overflow => write!(f, "Numeric overflow or underflow"),
            CalculationError::PrecisionLoss => write!(f, "Precision loss in calculation"),
            CalculationError::InvalidResult => write!(f, "Invalid calculation result"),
        }
    }
}

impl fmt::Display for SystemError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SystemError::Configuration => write!(f, "Configuration error"),
            SystemError::ResourceUnavailable => write!(f, "Resource unavailable"),
            SystemError::Internal => write!(f, "Internal system error"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Input(err) => Some(err),
            Error::Calculation(err) => Some(err),
            Error::System(err) => Some(err),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for InputError {}

#[cfg(feature = "std")]
impl std::error::Error for CalculationError {}

#[cfg(feature = "std")]
impl std::error::Error for SystemError {}

impl From<InputError> for Error {
    fn from(err: InputError) -> Self {
        Error::Input(err)
    }
}

impl From<CalculationError> for Error {
    fn from(err: CalculationError) -> Self {
        Error::Calculation(err)
    }
}

impl From<SystemError> for Error {
    fn from(err: SystemError) -> Self {
        Error::System(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "std")]
    use std::format;

    #[test]
    #[cfg(feature = "std")]
    fn test_error_display() {
        let currency_error = InputError::CurrencyMismatch {
            expected: Currency::USD,
            actual: Currency::EUR,
        };
        assert!(format!("{}", currency_error).contains("Currency mismatch"));
        assert!(format!("{}", currency_error).contains("USD"));
        assert!(format!("{}", currency_error).contains("EUR"));

        let overflow_error = InputError::Overflow;
        assert_eq!(format!("{}", overflow_error), "Numeric overflow");
    }

    #[test]
    fn test_error_conversion() {
        let input_error = InputError::InvalidCurrency;
        let error: Error = input_error.into();
        matches!(error, Error::Input(InputError::InvalidCurrency));
    }
}
