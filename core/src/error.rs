//! Error types for the rfin-core library.

use super::primitives::currency::Currency;
use core::fmt;

/// Main error type for rfin-core operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Input validation errors.
    Input(InputError),
}

/// Input validation error variants.
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

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Input(err) => write!(f, "Input error: {}", err),
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

#[cfg(feature = "std")]
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Input(err) => Some(err),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for InputError {}

impl From<InputError> for Error {
    fn from(err: InputError) -> Self {
        Error::Input(err)
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
