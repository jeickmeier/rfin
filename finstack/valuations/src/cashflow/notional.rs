#![deny(missing_docs)]
use finstack_core::error::InputError;
use finstack_core::money::Money;
use super::amortization::AmortizationSpec;

// Deprecated: use `AmortizationSpec` instead. Kept temporarily for internal migration.
// pub type AmortRule = AmortizationSpec;

/// Notional amount with an optional amortisation rule.
#[derive(Clone, Debug, PartialEq)]
pub struct Notional {
    /// Initial principal amount outstanding at leg inception.
    pub initial: Money,
    /// Amortisation rule applied after each period.
    pub amort: AmortizationSpec,
}

impl Notional {
    /// Plain (non-amortising) notional helper.
    pub fn par(amount: f64, currency: finstack_core::currency::Currency) -> Self {
        Self {
            initial: Money::new(amount, currency),
            amort: AmortizationSpec::None,
        }
    }

    /// Validate amortisation schedule (sum of amort steps ≤ initial).
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
