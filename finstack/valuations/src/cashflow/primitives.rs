//! Cash-flow primitives and enums.

use finstack_core::dates::Date;
use finstack_core::error::InputError;
use finstack_core::money::Money;

/// Enumeration of cash-flow kinds for classification and ordering.
///
/// Used to distinguish between different types of cashflows for
/// proper sequencing, risk calculation, and accounting treatment.
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
    /// Payment-in-kind interest capitalization (adds to principal).
    PIK,
    /// Amortization principal repayment (reduces principal).
    Amortization,
    /// Up-front fee or cost.
    Fee,
    /// Irregular stub period.
    Stub,
}

/// A single dated cash-flow (payment or reset).
///
/// Represents a monetary flow at a specific date with metadata
/// for proper classification and risk calculation.
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
    /// Accrual factor used for coupon amount and sensitivity.
    pub accrual_factor: f64,
}

impl CashFlow {
    /// Create a fixed coupon cash-flow (`CFKind::Fixed`).
    /// 
    /// # Errors
    /// Returns [`Error::InvalidInput`] if the `amount` is zero.
    /// 
    /// See unit tests and `examples/` for usage.
    pub fn fixed_cf(date: Date, amount: Money) -> finstack_core::Result<Self> {
        if amount.amount() == 0.0 {
            return Err(InputError::Invalid.into());
        }
        Ok(Self { date, reset_date: None, amount, kind: CFKind::Fixed, accrual_factor: 0.0 })
    }

    /// Create a floating coupon cash-flow (stored as `CFKind::FloatReset`).
    ///
    /// If no explicit reset date is provided, it defaults to the payment `date`.
    ///
    /// # Errors
    /// Returns [`Error::InvalidInput`] if the `amount` is zero.
    pub fn floating_cf(date: Date, amount: Money, reset_date: Option<Date>) -> finstack_core::Result<Self> {
        if amount.amount() == 0.0 {
            return Err(InputError::Invalid.into());
        }
        Ok(Self { date, reset_date: Some(reset_date.unwrap_or(date)), amount, kind: CFKind::FloatReset, accrual_factor: 0.0 })
    }

    /// Create a Payment-in-Kind cash-flow (`CFKind::PIK`).
    ///
    /// PIK increases outstanding principal in principal accounting.
    ///
    /// # Errors
    /// Returns [`Error::InvalidInput`] if the `amount` is zero.
    pub fn pik_cf(date: Date, amount: Money) -> finstack_core::Result<Self> {
        if amount.amount() == 0.0 {
            return Err(InputError::Invalid.into());
        }
        Ok(Self { date, reset_date: None, amount, kind: CFKind::PIK, accrual_factor: 0.0 })
    }

    /// Create an amortization principal cash-flow (`CFKind::Amortization`).
    ///
    /// Amortization reduces outstanding principal in principal accounting.
    ///
    /// # Errors
    /// Returns [`Error::InvalidInput`] if the `amount` is zero.
    pub fn amort_cf(date: Date, amount: Money) -> finstack_core::Result<Self> {
        if amount.amount() == 0.0 {
            return Err(InputError::Invalid.into());
        }
        Ok(Self { date, reset_date: None, amount, kind: CFKind::Amortization, accrual_factor: 0.0 })
    }

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
    fn fixed_cf_constructor_stores_fields() {
        use finstack_core::currency::Currency;
        use time::Month;
        let date = Date::from_calendar_date(2025, Month::January, 15).unwrap();
        let amount = Money::new(100.0, Currency::USD);
        let cf = CashFlow::fixed_cf(date, amount).unwrap();
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

        let pik = CashFlow::pik_cf(date, amt).unwrap();
        assert_eq!(pik.kind, CFKind::PIK);

        let amort = CashFlow::amort_cf(date, amt).unwrap();
        assert_eq!(amort.kind, CFKind::Amortization);

        // Zero amount returns error
        let zero = Money::new(0.0, Currency::EUR);
        assert!(CashFlow::fee(date, zero).is_err());
        assert!(CashFlow::fixed_cf(date, zero).is_err());
        assert!(CashFlow::floating_cf(date, zero, None).is_err());
        assert!(CashFlow::pik_cf(date, zero).is_err());
        assert!(CashFlow::amort_cf(date, zero).is_err());
    }
}
