//! Cashflow primitives and classification enums.
//!
//! Defines core types for representing individual cashflows, notional amounts,
//! and amortization schedules. These primitives are used throughout the
//! valuations crate for building instrument-specific payment schedules.
//!
//! # Types
//!
//! - [`CashFlow`]: Single dated payment with classification
//! - [`CFKind`]: Cashflow type enumeration (fixed, floating, principal, etc.)
//! - [`Notional`]: Principal amount with amortization schedule
//! - [`AmortizationSpec`]: Schedule for principal reduction

use crate::currency::Currency;
use crate::dates::Date;
use crate::error::InputError;
use crate::money::Money;

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
    pub fn fixed_cf(date: Date, amount: Money) -> crate::Result<Self> {
        if amount.amount() == 0.0 {
            return Err(InputError::Invalid.into());
        }
        Ok(Self {
            date,
            reset_date: None,
            amount,
            kind: CFKind::Fixed,
            accrual_factor: 0.0,
        })
    }

    /// Create a floating coupon cash-flow (stored as `CFKind::FloatReset`).
    ///
    /// If no explicit reset date is provided, it defaults to the payment `date`.
    ///
    /// # Errors
    /// Returns [`Error::InvalidInput`] if the `amount` is zero.
    pub fn floating_cf(date: Date, amount: Money, reset_date: Option<Date>) -> crate::Result<Self> {
        if amount.amount() == 0.0 {
            return Err(InputError::Invalid.into());
        }
        Ok(Self {
            date,
            reset_date: Some(reset_date.unwrap_or(date)),
            amount,
            kind: CFKind::FloatReset,
            accrual_factor: 0.0,
        })
    }

    /// Create a Payment-in-Kind cash-flow (`CFKind::PIK`).
    ///
    /// PIK increases outstanding principal in principal accounting.
    ///
    /// # Errors
    /// Returns [`Error::InvalidInput`] if the `amount` is zero.
    pub fn pik_cf(date: Date, amount: Money) -> crate::Result<Self> {
        if amount.amount() == 0.0 {
            return Err(InputError::Invalid.into());
        }
        Ok(Self {
            date,
            reset_date: None,
            amount,
            kind: CFKind::PIK,
            accrual_factor: 0.0,
        })
    }

    /// Create an amortization principal cash-flow (`CFKind::Amortization`).
    ///
    /// Amortization reduces outstanding principal in principal accounting.
    ///
    /// # Errors
    /// Returns [`Error::InvalidInput`] if the `amount` is zero.
    pub fn amort_cf(date: Date, amount: Money) -> crate::Result<Self> {
        if amount.amount() == 0.0 {
            return Err(InputError::Invalid.into());
        }
        Ok(Self {
            date,
            reset_date: None,
            amount,
            kind: CFKind::Amortization,
            accrual_factor: 0.0,
        })
    }

    /// Create a **principal exchange** (`CFKind::Notional`) cash-flow.
    ///
    /// # Errors
    /// Returns [`Error::InvalidInput`] if the `amount` is zero.
    pub fn principal_exchange(date: Date, amount: Money) -> crate::Result<Self> {
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
    pub fn fee(date: Date, amount: Money) -> crate::Result<Self> {
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

/// Amortization specification for principal over time.
///
/// Describes how principal amortizes or is exchanged during the life of the contract.
/// Used by instruments (e.g., bonds) and cashflow legs for consistent behavior.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AmortizationSpec {
    /// No amortization – principal remains constant until final redemption.
    None,
    /// Linear principal paydown towards a target final notional amount over all periods.
    LinearTo {
        /// Target remaining principal at the end of the amortization schedule.
        final_notional: Money,
    },
    /// Explicit schedule of remaining principal amounts after given dates.
    /// Each pair stores `(date, remaining_principal_after_date)`.
    StepRemaining {
        /// Ordered list of `(date, remaining_principal_after_date)`.
        schedule: Vec<(Date, Money)>,
    },
    /// Fixed percentage of original notional paid each period (capped by remaining outstanding).
    PercentPerPeriod {
        /// Fraction of original notional paid per period (e.g., 0.05 = 5%).
        pct: f64,
    },
    /// Custom principal exchanges on specific dates (absolute cash amounts).
    /// Positive amounts reduce outstanding (i.e., principal paid by issuer).
    CustomPrincipal {
        /// List of `(date, principal_amount)` exchanges; amounts are absolute cashflows.
        items: Vec<(Date, Money)>,
    },
}

impl Default for AmortizationSpec {
    fn default() -> Self {
        Self::None
    }
}

/// Notional amount with an optional amortisation rule.
///
/// Combines initial principal with amortization behavior for complete
/// notional lifecycle management.
#[derive(Clone, Debug, PartialEq)]
pub struct Notional {
    /// Initial principal amount outstanding at leg inception.
    pub initial: Money,
    /// Amortisation rule applied after each period.
    pub amort: AmortizationSpec,
}

impl Notional {
    /// Plain (non-amortising) notional helper.
    ///
    /// # Example
    /// ```rust
    /// use finstack_core::cashflow::primitives::Notional;
    /// use finstack_core::currency::Currency;
    ///
    /// let notional = Notional::par(1_000_000.0, Currency::USD);
    /// assert_eq!(notional.initial.amount(), 1_000_000.0);
    /// assert!(matches!(notional.amort, finstack_core::cashflow::primitives::AmortizationSpec::None));
    /// ```
    pub fn par(amount: f64, currency: Currency) -> Self {
        Self {
            initial: Money::new(amount, currency),
            amort: AmortizationSpec::None,
        }
    }

    /// Validate amortisation schedule (sum of amort steps ≤ initial).
    ///
    /// # Errors
    /// Returns an error if the amortization schedule is invalid:
    /// - Currency mismatch between initial and final amounts
    /// - Final amount exceeds initial amount
    /// - Step schedule has invalid progression
    pub fn validate(&self) -> crate::Result<()> {
        match &self.amort {
            AmortizationSpec::None => Ok(()),
            AmortizationSpec::LinearTo { final_notional } => {
                if final_notional.currency() != self.initial.currency()
                    || final_notional.amount() > self.initial.amount()
                {
                    return Err(InputError::Invalid.into());
                }
                Ok(())
            }
            AmortizationSpec::StepRemaining { schedule } => {
                // Enforce strictly increasing dates with no duplicates and validate amounts
                // in chronological order. We sort a local copy to check timeline consistency
                // and then ensure the provided input is already strictly increasing.
                let mut sorted = schedule.clone();
                sorted.sort_by_key(|(d, _)| *d);

                // Reject duplicates and non-increasing dates
                let mut prev_date: Option<Date> = None;
                for (d, _) in &sorted {
                    if let Some(p) = prev_date {
                        if *d <= p {
                            return Err(InputError::Invalid.into());
                        }
                    }
                    prev_date = Some(*d);
                }

                // Require that input is already strictly increasing by date
                let input_dates_iter = schedule.iter().map(|(d, _)| *d);
                let sorted_dates_iter = sorted.iter().map(|(d, _)| *d);
                if !input_dates_iter.eq(sorted_dates_iter) {
                    return Err(InputError::Invalid.into());
                }

                // Validate currency consistency and non-increasing remaining amounts
                let mut remaining = self.initial.amount();
                for (_, notl) in &sorted {
                    if notl.currency() != self.initial.currency() || notl.amount() > remaining {
                        return Err(InputError::Invalid.into());
                    }
                    remaining = notl.amount();
                }
                Ok(())
            }
            AmortizationSpec::PercentPerPeriod { .. } => Ok(()),
            AmortizationSpec::CustomPrincipal { items } => {
                for (_d, amt) in items {
                    if amt.currency() != self.initial.currency() {
                        return Err(InputError::Invalid.into());
                    }
                }
                Ok(())
            }
        }
    }

    /// Convenience accessor for currency.
    pub fn currency(&self) -> Currency {
        self.initial.currency()
    }
}

// -------------------------------------------------------------------------
// Compile-time size assertion (≤ 48 bytes with `f64` path) – phase-2 goal.
// -------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use crate::currency::Currency;
    use core::mem::size_of;
    use time::Month;

    #[test]
    fn cashflow_size_is_reasonable() {
        assert!(size_of::<CashFlow>() <= 48);
    }

    #[test]
    fn fixed_cf_constructor_stores_fields() {
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
