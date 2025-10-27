//! Shared utilities for theta (time decay) calculations.
//!
//! Provides period parsing, date rolling, and a generic theta calculator that
//! works for any instrument implementing the `Instrument` trait.

use crate::instruments::common::traits::Instrument;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::dates::Date;
use finstack_core::Result;
use std::any::Any;
use std::marker::PhantomData;

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
/// Computes theta as the total carry from rolling the valuation date forward:
///   Theta = PV(end_date) - PV(start_date) + Sum(Cashflows from start to end)
///
/// This accounts for:
/// - Pull-to-par effects (PV change)
/// - Coupon/interest receipts during the period
/// - Principal payments during the period
///
/// # Type Parameters
/// * `I` - Instrument type implementing `Instrument` trait
///
/// # Arguments
/// * `context` - Metric context containing instrument, market data, and pricing overrides
///
/// # Returns
/// Theta value as total carry including PV change and cashflows (in base currency units)
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
    let pv_change = bumped_value.amount() - base_pv;

    // Collect cashflows during the period (if instrument provides them)
    let cashflows_during_period =
        collect_cashflows_in_period(instrument, &context.curves, context.as_of, rolled_date)?;

    // Theta = PV change + cashflows received
    Ok(pv_change + cashflows_during_period)
}

/// Generic Theta calculator wrapper for any instrument implementing `Instrument` trait.
///
/// This eliminates the need for per-instrument theta calculator files that only
/// wrap the `generic_theta_calculator` function. Instead, instruments can directly
/// register `GenericTheta::<InstrumentType>::default()` in their metric registries.
///
/// # Type Parameters
/// * `I` - Instrument type implementing `Instrument` trait
///
/// # Examples
/// ```
/// use finstack_valuations::instruments::common::metrics::GenericTheta;
/// use finstack_valuations::instruments::Bond;
/// use finstack_valuations::metrics::{MetricRegistry, MetricId};
/// use std::sync::Arc;
///
/// let mut registry = MetricRegistry::new();
/// registry.register_metric(
///     MetricId::Theta,
///     Arc::new(GenericTheta::<Bond>::default()),
///     &["Bond"],
/// );
/// ```
pub struct GenericTheta<I> {
    _phantom: PhantomData<I>,
}

impl<I> Default for GenericTheta<I> {
    fn default() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<I> MetricCalculator for GenericTheta<I>
where
    I: Instrument + 'static,
{
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        generic_theta_calculator::<I>(context)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Collect cashflows that occur during a time period.
///
/// For instruments implementing CashflowProvider, this extracts all cashflows
/// with payment dates in (start_date, end_date].
///
/// # Returns
/// Sum of cashflow amounts in the period (converted to base currency)
fn collect_cashflows_in_period<I>(
    instrument: &I,
    curves: &finstack_core::market_data::MarketContext,
    start_date: Date,
    end_date: Date,
) -> Result<f64>
where
    I: 'static,
{
    use crate::cashflow::traits::CashflowProvider;
    use crate::instruments::*;

    // Try to downcast to known CashflowProvider implementors
    let instrument_any = instrument as &dyn std::any::Any;

    let cashflows: Option<Vec<(Date, finstack_core::money::Money)>> =
        // Bonds
        if let Some(bond) = instrument_any.downcast_ref::<Bond>() {
            bond.build_schedule(curves, start_date).ok()
        }
        // Interest Rate Swaps
        else if let Some(irs) = instrument_any.downcast_ref::<InterestRateSwap>() {
            irs.build_schedule(curves, start_date).ok()
        }
        // Deposits
        else if let Some(deposit) = instrument_any.downcast_ref::<deposit::Deposit>() {
            deposit.build_schedule(curves, start_date).ok()
        }
        // FRAs
        else if let Some(fra) = instrument_any.downcast_ref::<fra::ForwardRateAgreement>() {
            fra.build_schedule(curves, start_date).ok()
        }
        // IR Futures
        else if let Some(ir_fut) = instrument_any.downcast_ref::<ir_future::InterestRateFuture>() {
            ir_fut.build_schedule(curves, start_date).ok()
        }
        // Equity
        else if let Some(equity) = instrument_any.downcast_ref::<equity::Equity>() {
            equity.build_schedule(curves, start_date).ok()
        }
        // FX Spot
        else if let Some(fx_spot) = instrument_any.downcast_ref::<fx_spot::FxSpot>() {
            fx_spot.build_schedule(curves, start_date).ok()
        }
        // Inflation-Linked Bonds
        else if let Some(inf_bond) =
            instrument_any.downcast_ref::<inflation_linked_bond::InflationLinkedBond>()
        {
            inf_bond.build_schedule(curves, start_date).ok()
        }
        // Repos
        else if let Some(repo) = instrument_any.downcast_ref::<repo::Repo>() {
            repo.build_schedule(curves, start_date).ok()
        }
        // Structured Credit
        else if let Some(sc) = instrument_any.downcast_ref::<structured_credit::StructuredCredit>() {
            sc.build_schedule(curves, start_date).ok()
        }
        // TRS (both types)
        else if let Some(eq_trs) = instrument_any.downcast_ref::<trs::EquityTotalReturnSwap>() {
            eq_trs.build_schedule(curves, start_date).ok()
        } else if let Some(fi_trs) =
            instrument_any.downcast_ref::<trs::FIIndexTotalReturnSwap>()
        {
            fi_trs.build_schedule(curves, start_date).ok()
        }
        // Private Markets Fund
        else if let Some(pmf) =
            instrument_any.downcast_ref::<private_markets_fund::PrivateMarketsFund>()
        {
            pmf.build_schedule(curves, start_date).ok()
        }
        // Variance Swap
        else if let Some(var_swap) = instrument_any.downcast_ref::<variance_swap::VarianceSwap>() {
            var_swap.build_schedule(curves, start_date).ok()
        }
        // CDS - use premium schedule for cashflows
        else if let Some(cds) = instrument_any.downcast_ref::<cds::CreditDefaultSwap>() {
            cds.build_premium_schedule(curves, start_date).ok()
        }
        // FX Swap - has explicit cashflows at near and far dates
        else if let Some(_fx_swap) = instrument_any.downcast_ref::<fx_swap::FxSwap>() {
            // FX swaps don't have interim cashflows, only near/far settlement
            // Theta comes purely from PV change, not cashflows
            None
        }
        // Inflation-Linked Bonds
        else if let Some(ilb) = instrument_any.downcast_ref::<inflation_linked_bond::InflationLinkedBond>() {
            ilb.build_schedule(curves, start_date).ok()
        }
        // Instruments without CashflowProvider implementation:
        // - BasisSwap, CDSIndex, CdsTranche, ConvertibleBond, InflationSwap
        // - Cap/Floor, Options, Basket
        // These don't have interim cashflows or don't implement the trait
        else {
            None
        };

    // Sum cashflows in (start_date, end_date]
    if let Some(flows) = cashflows {
        let cashflow_sum: f64 = flows
            .iter()
            .filter(|(date, _)| *date > start_date && *date <= end_date)
            .map(|(_, money)| money.amount())
            .sum();
        Ok(cashflow_sum)
    } else {
        // No cashflows for this instrument type
        Ok(0.0)
    }
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
    if let Some(cds) = instrument.downcast_ref::<cds::CreditDefaultSwap>() {
        return Some(cds.premium.end);
    }
    if let Some(fx_swap) = instrument.downcast_ref::<fx_swap::FxSwap>() {
        return Some(fx_swap.far_date);
    }

    // No expiry for: equity, basket, convertible, structured_credit, private_markets_fund
    None
}
