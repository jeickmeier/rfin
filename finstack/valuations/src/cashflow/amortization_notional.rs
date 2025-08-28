//! Amortization specification for principal over time.

use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_core::error::InputError;

/// Amortization specification for principal over time.
///
/// Describes how principal amortizes or is exchanged during the life of the contract.
/// Used by instruments (e.g., bonds) and cashflow legs for consistent behavior.
#[derive(Clone, Debug, PartialEq)]
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
        pct: finstack_core::F,
    },
    /// Custom principal exchanges on specific dates (absolute cash amounts).
    /// Positive amounts reduce outstanding (i.e., principal paid by issuer).
    CustomPrincipal {
        /// List of `(date, principal_amount)` exchanges; amounts are absolute cashflows.
        items: Vec<(Date, Money)>,
    },
}

impl Default for AmortizationSpec {
    fn default() -> Self { Self::None }
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
    /// use finstack_core::currency::Currency;
    /// use finstack_valuations::cashflow::amortization_notional::Notional;
    /// 
    /// let notional = Notional::par(1_000_000.0, Currency::USD);
    /// assert_eq!(notional.initial.amount(), 1_000_000.0);
    /// assert!(matches!(notional.amort, finstack_valuations::cashflow::amortization_notional::AmortizationSpec::None));
    /// ```
    pub fn par(amount: f64, currency: finstack_core::currency::Currency) -> Self {
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
    pub fn validate(&self) -> finstack_core::Result<()> {
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
                let mut remaining = self.initial.amount();
                for (_, notl) in schedule {
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
    pub fn currency(&self) -> finstack_core::currency::Currency {
        self.initial.currency()
    }
}

