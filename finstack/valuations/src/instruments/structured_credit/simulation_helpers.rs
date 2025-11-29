//! Helper types for structured credit simulation.
//!
//! This module contains internal helpers used by the simulation engine
//! in `instrument_trait.rs`. These are implementation details not exposed
//! in the public API.

use finstack_core::dates::months_between;
use finstack_core::dates::Date;
use finstack_core::money::Money;
use std::collections::{HashMap, VecDeque};

/// Recovery lag buffer for delayed recovery processing.
///
/// Recoveries from defaulted assets are typically received 6-12 months after
/// the default event. This buffer holds pending recoveries until they can be
/// released based on the configured recovery lag.
#[derive(Debug, Default)]
pub(crate) struct RecoveryLagBuffer {
    /// Queue of pending recoveries: (origination_date, recovery_amount)
    pending: VecDeque<(Date, Money)>,
}

impl RecoveryLagBuffer {
    /// Create a new empty recovery lag buffer.
    pub fn new() -> Self {
        Self {
            pending: VecDeque::new(),
        }
    }

    /// Add a new recovery to the buffer.
    pub fn add_recovery(&mut self, origination_date: Date, amount: Money) {
        if amount.amount() > 0.0 {
            self.pending.push_back((origination_date, amount));
        }
    }

    /// Release all recoveries that have matured based on the lag period.
    ///
    /// Returns the total amount of recoveries released this period.
    pub fn release_matured(
        &mut self,
        current_date: Date,
        recovery_lag_months: u32,
        base_currency: finstack_core::currency::Currency,
    ) -> finstack_core::Result<Money> {
        let mut released = Money::new(0.0, base_currency);

        // Pop all recoveries that have matured
        while let Some((orig_date, _)) = self.pending.front() {
            let months_elapsed = months_between(*orig_date, current_date);
            if months_elapsed >= recovery_lag_months {
                if let Some((_, amount)) = self.pending.pop_front() {
                    released = released.checked_add(amount)?;
                }
            } else {
                // Remaining entries are not yet mature
                break;
            }
        }

        Ok(released)
    }
}

/// Cashflows generated in a single payment period
pub(crate) struct PeriodFlows {
    pub interest_collections: Money,
    pub prepayments: Money,
    pub defaults: Money,
    #[allow(dead_code)]
    pub recoveries: Money,
}

impl PeriodFlows {
    /// Total cash available for distribution
    #[allow(dead_code)]
    pub fn total_cash(&self) -> finstack_core::Result<Money> {
        let principal = self.prepayments.checked_add(self.recoveries)?;
        self.interest_collections.checked_add(principal)
    }
}

/// Update tranche balance after payment (helper function)
pub(crate) fn update_tranche_balance(
    tranche_balances: &mut HashMap<String, Money>,
    tranche_id: &str,
    payment: Money,
    interest_portion: Money,
) -> finstack_core::Result<()> {
    let principal_payment = payment
        .checked_sub(interest_portion)
        .unwrap_or(Money::new(0.0, payment.currency()));

    if let Some(current) = tranche_balances.get_mut(tranche_id) {
        *current = current.checked_sub(principal_payment).unwrap_or(*current);
    }

    Ok(())
}
