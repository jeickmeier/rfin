//! Value types for node data.

use finstack_core::{currency::Currency, money::Money};
use serde::{Deserialize, Serialize};

/// Value that can be currency-aware or unitless.
///
/// Used for node values that can represent:
/// - **Amount**: Currency-aware monetary values (e.g., USD 1,000,000)
/// - **Scalar**: Unitless values (e.g., ratios, percentages, counts)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AmountOrScalar {
    /// Currency-aware amount
    Amount(Money),
    /// Unitless scalar (ratio, percentage, count, etc.)
    Scalar(f64),
}

impl AmountOrScalar {
    /// Create a currency-aware amount.
    ///
    /// # Arguments
    /// * `value` - Numeric amount
    /// * `currency` - ISO currency of the amount
    ///
    /// # Example
    /// ```rust
    /// # use finstack_statements::types::AmountOrScalar;
    /// # use finstack_core::currency::Currency;
    /// let amount = AmountOrScalar::amount(1_000_000.0, Currency::USD);
    /// assert!(amount.is_amount());
    /// assert_eq!(amount.currency(), Some(Currency::USD));
    /// ```
    pub fn amount(value: f64, currency: Currency) -> Self {
        Self::Amount(Money::new(value, currency))
    }

    /// Create a unitless scalar.
    ///
    /// # Arguments
    /// * `value` - Numeric value interpreted as a unitless scalar
    pub fn scalar(value: f64) -> Self {
        Self::Scalar(value)
    }

    /// Extract the numeric value (ignoring currency if present).
    pub fn value(&self) -> f64 {
        match self {
            Self::Amount(money) => money.amount(),
            Self::Scalar(value) => *value,
        }
    }

    /// Get the currency if this is an amount.
    pub fn currency(&self) -> Option<Currency> {
        match self {
            Self::Amount(money) => Some(money.currency()),
            Self::Scalar(_) => None,
        }
    }

    /// Check if this is a currency-aware amount.
    pub fn is_amount(&self) -> bool {
        matches!(self, Self::Amount(_))
    }

    /// Check if this is a unitless scalar.
    pub fn is_scalar(&self) -> bool {
        matches!(self, Self::Scalar(_))
    }
}

impl From<f64> for AmountOrScalar {
    fn from(value: f64) -> Self {
        Self::Scalar(value)
    }
}

impl From<Money> for AmountOrScalar {
    fn from(money: Money) -> Self {
        Self::Amount(money)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scalar_value() {
        let scalar = AmountOrScalar::scalar(0.15);
        assert_eq!(scalar.value(), 0.15);
        assert!(scalar.is_scalar());
        assert!(!scalar.is_amount());
        assert_eq!(scalar.currency(), None);
    }

    #[test]
    fn test_amount_value() {
        let amount = AmountOrScalar::amount(1_000_000.0, Currency::USD);
        assert_eq!(amount.value(), 1_000_000.0);
        assert!(amount.is_amount());
        assert!(!amount.is_scalar());
        assert_eq!(amount.currency(), Some(Currency::USD));
    }

    #[test]
    fn test_from_f64() {
        let scalar: AmountOrScalar = 42.0.into();
        assert_eq!(scalar.value(), 42.0);
        assert!(scalar.is_scalar());
    }

    #[test]
    fn test_from_money() {
        let money = Money::new(500.0, Currency::EUR);
        let amount: AmountOrScalar = money.into();
        assert_eq!(amount.value(), 500.0);
        assert_eq!(amount.currency(), Some(Currency::EUR));
    }
}
