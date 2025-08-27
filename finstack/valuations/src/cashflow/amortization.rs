#![deny(missing_docs)]
use finstack_core::dates::Date;
use finstack_core::money::Money;

/// Amortization specification for principal over time.
///
/// This unified enum is shared by instruments (e.g., bonds) and cashflow legs
/// to describe how principal amortizes or is exchanged during the life of the contract.
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


