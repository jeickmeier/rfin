#![deny(missing_docs)]
use crate::dates::Date;
use crate::error::InputError;
use crate::money::Money;

/// Amortisation rule applied to the notional over time.
#[derive(Clone, Debug, PartialEq)]
pub enum AmortRule {
    /// No amortisation – notional stays constant.
    None,
    /// Linear amortisation towards a lower final notional amount.
    Linear {
        /// Remaining notional at the end of the schedule (inclusive).
        final_notional: Money,
    },
    /// Explicit step schedule – each tuple stores the cash-flow date and the notional **remaining after** that date.
    Step {
        /// Vector of `(date, remaining_notional)` pairs in chronological order.
        schedule: Vec<(Date, Money)>,
    },
}

impl Default for AmortRule {
    fn default() -> Self {
        Self::None
    }
}

/// Notional amount with an optional amortisation rule.
#[derive(Clone, Debug, PartialEq)]
pub struct Notional {
    /// Initial principal amount outstanding at leg inception.
    pub initial: Money,
    /// Amortisation rule applied after each period.
    pub amort: AmortRule,
}

impl Notional {
    /// Plain (non-amortising) notional helper.
    pub fn par(amount: f64, currency: crate::currency::Currency) -> Self {
        Self {
            initial: Money::new(amount, currency),
            amort: AmortRule::None,
        }
    }

    /// Validate amortisation schedule (sum of amort steps ≤ initial).
    pub fn validate(&self) -> crate::Result<()> {
        match &self.amort {
            AmortRule::None => Ok(()),
            AmortRule::Linear { final_notional } => {
                if final_notional.currency() != self.initial.currency()
                    || final_notional.amount() > self.initial.amount()
                {
                    return Err(InputError::Invalid.into());
                }
                Ok(())
            }
            AmortRule::Step { schedule } => {
                let mut remaining = self.initial.amount();
                for (_, notl) in schedule {
                    if notl.currency() != self.initial.currency() || notl.amount() > remaining {
                        return Err(InputError::Invalid.into());
                    }
                    remaining = notl.amount();
                }
                Ok(())
            }
        }
    }

    /// Convenience accessor for currency.
    pub fn currency(&self) -> crate::currency::Currency {
        self.initial.currency()
    }
}
