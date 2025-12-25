//! Fee cashflow emission (periodic, commitment, usage, facility).

use crate::cashflow::primitives::{CFKind, CashFlow};
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::error::InputError;
use finstack_core::money::Money;

use super::super::compiler::PeriodicFee;
use super::super::specs::FeeBase;

/// Internal generic helper for fee emission.
///
/// Creates a single fee cashflow with the specified kind if the computed fee amount
/// is positive, otherwise returns an empty vector.
fn emit_fee_generic(
    d: Date,
    base_amount: f64,
    fee_bp: f64,
    year_fraction: f64,
    ccy: Currency,
    kind: CFKind,
) -> Vec<CashFlow> {
    let fee_amt = base_amount * (fee_bp * 1e-4 * year_fraction);
    if fee_amt > 0.0 {
        vec![CashFlow {
            date: d,
            reset_date: None,
            amount: Money::new(fee_amt, ccy),
            kind,
            accrual_factor: year_fraction,
            rate: Some(fee_bp * 1e-4),
        }]
    } else {
        vec![]
    }
}

/// Emit commitment fee cashflow (fee on undrawn balance).
///
/// Commitment fees are charged on the undrawn portion of a credit facility.
/// Returns a single cashflow with `CFKind::CommitmentFee`.
///
/// # Arguments
///
/// * `d` - Payment date for the fee
/// * `undrawn_balance` - Undrawn balance amount
/// * `commitment_fee_bp` - Fee rate in basis points
/// * `year_fraction` - Accrual period in years
/// * `ccy` - Currency for the cashflow
///
/// # Returns
///
/// Vector containing zero or one cashflow (empty if fee amount is zero)
pub fn emit_commitment_fee_on(
    d: Date,
    undrawn_balance: f64,
    commitment_fee_bp: f64,
    year_fraction: f64,
    ccy: Currency,
) -> Vec<CashFlow> {
    emit_fee_generic(
        d,
        undrawn_balance,
        commitment_fee_bp,
        year_fraction,
        ccy,
        CFKind::CommitmentFee,
    )
}

/// Emit usage fee cashflow (fee on drawn balance).
///
/// Usage fees are charged on the drawn portion of a credit facility.
/// Returns a single cashflow with `CFKind::UsageFee`.
///
/// # Arguments
///
/// * `d` - Payment date for the fee
/// * `drawn_balance` - Drawn balance amount
/// * `usage_fee_bp` - Fee rate in basis points
/// * `year_fraction` - Accrual period in years
/// * `ccy` - Currency for the cashflow
///
/// # Returns
///
/// Vector containing zero or one cashflow (empty if fee amount is zero)
pub fn emit_usage_fee_on(
    d: Date,
    drawn_balance: f64,
    usage_fee_bp: f64,
    year_fraction: f64,
    ccy: Currency,
) -> Vec<CashFlow> {
    emit_fee_generic(
        d,
        drawn_balance,
        usage_fee_bp,
        year_fraction,
        ccy,
        CFKind::UsageFee,
    )
}

/// Emit facility fee cashflow (fee on total commitment).
///
/// Facility fees are charged on the entire commitment amount regardless of utilization.
/// Returns a single cashflow with `CFKind::FacilityFee`.
///
/// # Arguments
///
/// * `d` - Payment date for the fee
/// * `commitment_amount` - Total commitment amount
/// * `facility_fee_bp` - Fee rate in basis points
/// * `year_fraction` - Accrual period in years
/// * `ccy` - Currency for the cashflow
///
/// # Returns
///
/// Vector containing zero or one cashflow (empty if fee amount is zero)
pub fn emit_facility_fee_on(
    d: Date,
    commitment_amount: f64,
    facility_fee_bp: f64,
    year_fraction: f64,
    ccy: Currency,
) -> Vec<CashFlow> {
    emit_fee_generic(
        d,
        commitment_amount,
        facility_fee_bp,
        year_fraction,
        ccy,
        CFKind::FacilityFee,
    )
}

/// Emit fee cashflows on a specific date.
///
/// Processes both periodic fees (based on drawn/undrawn balances) and fixed
/// fees (explicit amounts) that fall on the given date.
///
/// For periodic fees, computes the fee amount as `base * bps * year_fraction`
/// where base is either the drawn balance or the undrawn balance (facility_limit - outstanding).
pub(in crate::cashflow::builder) fn emit_fees_on(
    d: Date,
    periodic_fees: &[PeriodicFee],
    fixed_fees: &[(Date, Money)],
    outstanding: f64,
    ccy: Currency,
    new_flows: &mut Vec<CashFlow>,
) -> finstack_core::Result<()> {
    for pf in periodic_fees {
        if let Some(&prev) = pf.prev.get(&d) {
            let yf = pf
                .dc
                .year_fraction(prev, d, finstack_core::dates::DayCountCtx::default())?;
            let base_amt = match &pf.base {
                FeeBase::Drawn => outstanding,
                FeeBase::Undrawn { facility_limit } => {
                    if facility_limit.currency() != ccy {
                        return Err(InputError::Invalid.into());
                    }
                    (facility_limit.amount() - outstanding).max(0.0)
                }
            };
            let fee_amt = base_amt * (pf.bps * 1e-4 * yf);
            if fee_amt > 0.0 {
                new_flows.push(CashFlow {
                    date: d,
                    reset_date: None,
                    amount: Money::new(fee_amt, ccy),
                    kind: CFKind::Fee,
                    accrual_factor: yf,
                    rate: Some(pf.bps * 1e-4),
                });
            }
        }
    }

    for (fd, amt) in fixed_fees {
        if *fd == d && amt.amount() != 0.0 {
            new_flows.push(CashFlow {
                date: d,
                reset_date: None,
                amount: *amt,
                kind: CFKind::Fee,
                // Fixed fees don't have an accrual period - use 0.0
                accrual_factor: 0.0,
                rate: None,
            });
        }
    }
    Ok(())
}
