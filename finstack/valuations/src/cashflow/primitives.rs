#![deny(missing_docs)]
//! Cash-flow primitives and enums.

use finstack_core::dates::Date;
use finstack_core::error::InputError;
use finstack_core::money::Money;

/// Enumeration of cash-flow kinds as per §5.1 of the design document.
///
/// `non_exhaustive` – downstream crates must handle unknown variants.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CFKind {
    /// Fixed-rate coupon cash-flow.
    Fixed,
    /// Floating-rate reset (index fixing).
    FloatReset,
    /// Principal exchange / notional flow.
    Notional,
    /// Up-front fee or cost.
    Fee,
    /// Irregular stub period.
    Stub,
    // Future variants will be added in line with design §5.1 (PIK, StepUp, …).
}

/// A single dated cash-flow (payment or reset).
///
/// See §5.2 of the design document for details.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CashFlow {
    /// Payment date (or payment date for principal/fee, or reset date for `CFKind::FloatReset`).
    pub date: Date,
    /// Optional index reset date (for floating coupons).
    pub reset_date: Option<Date>,
    /// Monetary amount including its currency.
    pub amount: Money,
    /// Category/kind of cash-flow.
    pub kind: CFKind,
    /// Cached year-fraction used for sensitivity (populated elsewhere).
    pub accrual_factor: f64,
}

impl CashFlow {
    /// Create a **principal exchange** (`CFKind::Notional`) cash-flow.
    ///
    /// # Errors
    /// Returns [`Error::InvalidInput`] if the `amount` is zero.
    pub fn principal_exchange(date: Date, amount: Money) -> finstack_core::Result<Self> {
        if amount.amount() == 0.0 {
            return Err(InputError::Invalid.into());
        }
        Ok(Self {
            date,
            reset_date: None,
            amount,
            kind: CFKind::Notional,
            accrual_factor: 0.0,
        })
    }

    /// Create a **fee** cash-flow (`CFKind::Fee`).
    ///
    /// # Errors
    /// Returns [`Error::InvalidInput`] if the `amount` is zero.
    pub fn fee(date: Date, amount: Money) -> finstack_core::Result<Self> {
        if amount.amount() == 0.0 {
            return Err(InputError::Invalid.into());
        }
        Ok(Self {
            date,
            reset_date: None,
            amount,
            kind: CFKind::Fee,
            accrual_factor: 0.0,
        })
    }
    /// Construct a new cash-flow with the given date, amount, and kind.
    ///
    /// `reset_date` is initialised to `None`, and the `accrual_factor` starts
    /// at `0.0` (to be filled by the accrual cache in a later phase).
    #[must_use]
    pub const fn new(date: Date, amount: Money, kind: CFKind) -> Self {
        Self {
            date,
            reset_date: None,
            amount,
            kind,
            accrual_factor: 0.0,
        }
    }
}

// -------------------------------------------------------------------------
// Compile-time size assertion (≤ 48 bytes with `f64` path) – phase-2 goal.
// -------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::size_of;

    #[test]
    fn cashflow_size_is_reasonable() {
        assert!(size_of::<CashFlow>() <= 48);
    }

    #[test]
    fn new_constructor_stores_fields() {
        use finstack_core::currency::Currency;
        use time::Month;
        let date = Date::from_calendar_date(2025, Month::January, 15).unwrap();
        let amount = Money::new(100.0, Currency::USD);
        let cf = CashFlow::new(date, amount, CFKind::Fixed);
        assert_eq!(cf.date, date);
        assert_eq!(cf.amount, amount);
        assert_eq!(cf.kind, CFKind::Fixed);
        assert!(cf.reset_date.is_none());
        assert_eq!(cf.accrual_factor, 0.0);
    }

    #[test]
    fn factory_helpers_work() {
        use finstack_core::currency::Currency;
        use time::Month;
        let date = Date::from_calendar_date(2025, Month::March, 1).unwrap();
        let amt = Money::new(1_000.0, Currency::EUR);

        let princ = CashFlow::principal_exchange(date, amt).unwrap();
        assert_eq!(princ.kind, CFKind::Notional);

        let fee = CashFlow::fee(date, amt).unwrap();
        assert_eq!(fee.kind, CFKind::Fee);

        // Zero amount returns error
        let zero = Money::new(0.0, Currency::EUR);
        assert!(CashFlow::fee(date, zero).is_err());
    }
}
