//! Shared utilities for theta (time decay) calculations.
//!
//! Provides period parsing, date rolling, and a generic theta calculator that
//! works for any instrument implementing the `Instrument` trait.

use crate::instruments::common::traits::Instrument;
use crate::metrics::MetricContext;
use finstack_core::dates::Date;
use finstack_core::Result;
use std::any::Any;

/// Parse a period string to calendar days.
///
/// Supported formats:
/// - "1D", "2D", etc. -> days
/// - "1W", "2W", etc. -> weeks (7 days each)
/// - "1M", "2M", etc. -> months (30 days each)
/// - "3M", "6M", etc. -> months (30 days each)
/// - "1Y", "2Y", etc. -> years (365 days each)
///
/// # Examples
/// ```
/// # use finstack_valuations::instruments::common::metrics::theta_utils::parse_period_days;
/// assert_eq!(parse_period_days("1D").unwrap(), 1);
/// assert_eq!(parse_period_days("1W").unwrap(), 7);
/// assert_eq!(parse_period_days("1M").unwrap(), 30);
/// assert_eq!(parse_period_days("3M").unwrap(), 90);
/// assert_eq!(parse_period_days("1Y").unwrap(), 365);
/// ```
pub fn parse_period_days(period: &str) -> Result<i64> {
    let period = period.trim().to_uppercase();

    if period.is_empty() {
        return Err(finstack_core::Error::from(
            finstack_core::error::InputError::Invalid,
        ));
    }

    // Extract number and unit
    let (num_str, unit) = if let Some(pos) = period.find(|c: char| c.is_alphabetic()) {
        (&period[..pos], &period[pos..])
    } else {
        return Err(finstack_core::Error::from(
            finstack_core::error::InputError::Invalid,
        ));
    };

    let num: i64 = num_str
        .parse()
        .map_err(|_| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;

    let days = match unit {
        "D" => num,
        "W" => num * 7,
        "M" => num * 30,
        "Y" => num * 365,
        _ => {
            return Err(finstack_core::Error::from(
                finstack_core::error::InputError::Invalid,
            ))
        }
    };

    Ok(days)
}

/// Calculate the rolled forward date for theta calculation.
///
/// Advances the base date by the specified period (in calendar days), but caps
/// at the expiry date if the instrument expires before the period ends.
///
/// # Arguments
/// * `base_date` - Starting valuation date
/// * `period_str` - Period string (e.g., "1D", "1W", "1M")
/// * `expiry_date` - Optional instrument expiry date
///
/// # Returns
/// The rolled forward date, capped at expiry if applicable
pub fn calculate_theta_date(
    base_date: Date,
    period_str: &str,
    expiry_date: Option<Date>,
) -> Result<Date> {
    let days = parse_period_days(period_str)?;
    let rolled_date = base_date + time::Duration::days(days);

    // Cap at expiry if instrument expires before the rolled date
    if let Some(expiry) = expiry_date {
        if rolled_date > expiry {
            return Ok(expiry);
        }
    }

    Ok(rolled_date)
}

/// Generic theta calculator for any instrument implementing `Instrument` trait.
///
/// Computes theta as the change in present value when rolling the valuation
/// date forward by the specified period (default "1D"), holding all market
/// data constant.
///
/// # Type Parameters
/// * `I` - Instrument type implementing `Instrument` trait
///
/// # Arguments
/// * `context` - Metric context containing instrument, market data, and pricing overrides
///
/// # Returns
/// Theta value as the change in present value (in base currency units)
pub fn generic_theta_calculator<I>(context: &MetricContext) -> Result<f64>
where
    I: Instrument + 'static,
{
    // Downcast to concrete instrument type
    let instrument: &I = context
        .instrument
        .as_any()
        .downcast_ref::<I>()
        .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;

    // Get theta period from pricing overrides, default to "1D"
    let period_str = context
        .pricing_overrides
        .as_ref()
        .and_then(|po| po.theta_period.as_deref())
        .unwrap_or("1D");

    // Get expiry date if available (instrument-specific)
    let expiry_date = get_instrument_expiry(instrument);

    // Calculate rolled date
    let rolled_date = calculate_theta_date(context.as_of, period_str, expiry_date)?;

    // If already expired or rolling to same date, theta is zero
    if rolled_date <= context.as_of {
        return Ok(0.0);
    }

    // Base PV from context
    let base_pv = context.base_value.amount();

    // Reprice at rolled date with same market context
    let bumped_value = instrument.value(&context.curves, rolled_date)?;

    Ok(bumped_value.amount() - base_pv)
}

/// Helper to extract expiry date from an instrument (trait object).
///
/// Uses Any downcasting to check for common expiry field patterns.
/// Returns None if the instrument doesn't have a clear expiry concept.
fn get_instrument_expiry(instrument: &dyn Any) -> Option<Date> {
    use crate::instruments::*;

    // Try downcasting to each instrument type with expiry
    if let Some(bond) = instrument.downcast_ref::<Bond>() {
        return Some(bond.maturity);
    }
    if let Some(cds) = instrument.downcast_ref::<cds::CreditDefaultSwap>() {
        return Some(cds.premium.end);
    }
    if let Some(cds_idx) = instrument.downcast_ref::<cds_index::CDSIndex>() {
        return Some(cds_idx.premium.end);
    }
    if let Some(cds_tr) = instrument.downcast_ref::<cds_tranche::CdsTranche>() {
        return Some(cds_tr.maturity);
    }
    if let Some(cap) = instrument.downcast_ref::<cap_floor::InterestRateOption>() {
        return Some(cap.end_date);
    }
    if let Some(eq_opt) = instrument.downcast_ref::<equity_option::EquityOption>() {
        return Some(eq_opt.expiry);
    }
    if let Some(fx_opt) = instrument.downcast_ref::<fx_option::FxOption>() {
        return Some(fx_opt.expiry);
    }
    if let Some(swaption) = instrument.downcast_ref::<swaption::Swaption>() {
        return Some(swaption.expiry);
    }
    if let Some(cds_opt) = instrument.downcast_ref::<cds_option::CdsOption>() {
        return Some(cds_opt.expiry);
    }
    if let Some(fra) = instrument.downcast_ref::<fra::ForwardRateAgreement>() {
        return Some(fra.end_date);
    }
    if let Some(irs) = instrument.downcast_ref::<InterestRateSwap>() {
        return Some(irs.fixed.end);
    }
    if let Some(basis) = instrument.downcast_ref::<basis_swap::BasisSwap>() {
        return Some(basis.maturity_date);
    }
    if let Some(deposit) = instrument.downcast_ref::<deposit::Deposit>() {
        return Some(deposit.end);
    }
    if let Some(inf_swap) = instrument.downcast_ref::<inflation_swap::InflationSwap>() {
        return Some(inf_swap.maturity);
    }
    if let Some(inf_bond) = instrument.downcast_ref::<inflation_linked_bond::InflationLinkedBond>()
    {
        return Some(inf_bond.maturity);
    }
    if let Some(repo) = instrument.downcast_ref::<repo::Repo>() {
        return Some(repo.maturity);
    }
    if let Some(eq_trs) = instrument.downcast_ref::<trs::EquityTotalReturnSwap>() {
        return Some(eq_trs.schedule.end);
    }
    if let Some(fi_trs) = instrument.downcast_ref::<trs::FIIndexTotalReturnSwap>() {
        return Some(fi_trs.schedule.end);
    }
    if let Some(var_swap) = instrument.downcast_ref::<variance_swap::VarianceSwap>() {
        return Some(var_swap.maturity);
    }
    if let Some(ir_fut) = instrument.downcast_ref::<ir_future::InterestRateFuture>() {
        return Some(ir_fut.expiry_date);
    }

    // No expiry for: fx_spot, fx_swap, equity, basket, convertible, structured_credit, private_markets_fund
    None
}
