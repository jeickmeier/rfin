//! Amortization cashflow emission.

use crate::cashflow::builder::{AmortizationSpec, Notional};
use finstack_core::cashflow::primitives::{CFKind, CashFlow};
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::money::Money;

/// Amortization parameters for emission.
///
/// Contains precomputed values and maps needed by `emit_amortization_on` to
/// process various amortization specifications efficiently.
#[derive(Debug, Clone)]
pub(in crate::cashflow::builder) struct AmortizationParams<'a> {
    pub(in crate::cashflow::builder) ccy: Currency,
    pub(in crate::cashflow::builder) amort_dates: &'a hashbrown::HashSet<Date>,
    pub(in crate::cashflow::builder) linear_delta: Option<f64>,
    pub(in crate::cashflow::builder) percent_per: Option<f64>,
    pub(in crate::cashflow::builder) step_remaining_map:
        &'a Option<hashbrown::HashMap<Date, Money>>,
}

fn emit_principal_repayment(
    d: Date,
    ccy: Currency,
    outstanding: &mut f64,
    pay: f64,
    new_flows: &mut Vec<CashFlow>,
) {
    if pay <= 0.0 {
        return;
    }

    // Clamp to outstanding to guard against any numerical drift
    let pay = pay.min(*outstanding);
    if pay <= 0.0 {
        return;
    }

    new_flows.push(CashFlow {
        date: d,
        reset_date: None,
        amount: Money::new(pay, ccy),
        kind: CFKind::Amortization,
        accrual_factor: 0.0,
        rate: None,
    });
    *outstanding -= pay;
}

/// Emit amortization cashflows on a specific date.
///
/// Processes the notional's amortization specification to generate principal
/// repayment flows. Mutates the `outstanding` balance in-place to reflect
/// the reduction from amortization.
///
/// Supports:
/// - LinearTo: Equal installments over schedule
/// - StepRemaining: Specific remaining balance targets
/// - PercentPerPeriod: Percentage of current outstanding
/// - CustomPrincipal: Explicit payment amounts by date
pub(in crate::cashflow::builder) fn emit_amortization_on(
    d: Date,
    notional: &Notional,
    outstanding: &mut f64,
    params: &AmortizationParams,
    is_maturity: bool,
) -> finstack_core::Result<Vec<CashFlow>> {
    let mut new_flows: Vec<CashFlow> = Vec::new();
    match &notional.amort {
        AmortizationSpec::None => {}
        AmortizationSpec::LinearTo { .. } => {
            if params.amort_dates.contains(&d) {
                if let Some(delta) = params.linear_delta {
                    let pay = if is_maturity {
                        // Final clean-up: pay whatever remains outstanding
                        *outstanding
                    } else {
                        delta.min(*outstanding)
                    };
                    emit_principal_repayment(d, params.ccy, outstanding, pay, &mut new_flows);
                }
            }
        }
        AmortizationSpec::StepRemaining { .. } => {
            if let Some(map) = params.step_remaining_map {
                if let Some(rem_after) = map.get(&d) {
                    let target = rem_after.amount();
                    let pay = if is_maturity {
                        // On final date, ignore target and fully clean up balance
                        *outstanding
                    } else {
                        (*outstanding - target).max(0.0).min(*outstanding)
                    };
                    emit_principal_repayment(d, params.ccy, outstanding, pay, &mut new_flows);
                }
            }
        }
        AmortizationSpec::PercentPerPeriod { .. } => {
            if params.amort_dates.contains(&d) {
                if let Some(per) = params.percent_per {
                    let pay = if is_maturity {
                        *outstanding
                    } else {
                        per.min(*outstanding)
                    };
                    emit_principal_repayment(d, params.ccy, outstanding, pay, &mut new_flows);
                }
            }
        }
        AmortizationSpec::CustomPrincipal { items } => {
            for (dd, amt) in items {
                if *dd == d {
                    let pay = if is_maturity {
                        *outstanding
                    } else {
                        amt.amount().max(0.0).min(*outstanding)
                    };
                    emit_principal_repayment(d, params.ccy, outstanding, pay, &mut new_flows);
                }
            }
        }
    }
    Ok(new_flows)
}
