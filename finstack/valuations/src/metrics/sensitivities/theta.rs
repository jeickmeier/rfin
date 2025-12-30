//! Shared utilities for theta (time decay) calculations.
//!
//! Provides period parsing, date rolling, and a generic theta calculator that
//! works for any instrument implementing the `Instrument` trait.
//!
//! # Quick Start
//!
//! ## Example 1: Computing 1-Day Theta for an Equity Option
//!
//! ```rust,no_run
//! use finstack_valuations::instruments::EquityOption;
//! use finstack_valuations::instruments::Instrument;
//! use finstack_valuations::metrics::{standard_registry, MetricId};
//! use finstack_core::dates::create_date;
//! use finstack_core::market_data::context::MarketContext;
//! use time::Month;
//!
//! # fn main() -> finstack_core::Result<()> {
//! let as_of = create_date(2024, Month::January, 1)?;
//! let expiry = create_date(2024, Month::July, 1)?; // 6 months to expiry
//!
//! let option = EquityOption::european_call(
//!     "OPT-001",
//!     "SPX",
//!     4500.0,
//!     expiry,
//!     finstack_core::money::Money::new(100_000.0, finstack_core::currency::Currency::USD),
//!     100.0,
//! )?;
//!
//! // Setup market (abbreviated)
//! # let market = MarketContext::new();
//!
//! let _registry = standard_registry();
//! let metrics = vec![MetricId::Theta];
//!
//! let result = option.price_with_metrics(&market, as_of, &metrics)?;
//!
//! if let Some(theta) = result.measures.get(MetricId::Theta.as_str()) {
//!     println!("Option value: ${:.2}", result.value.amount());
//!     println!("1-day theta: ${:.2}", theta);
//!     // Negative theta indicates time decay (option loses value each day)
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Example 2: Computing Custom Period Theta (1 Week)
//!
//! ```rust,no_run
//! use finstack_valuations::instruments::{EquityOption, PricingOverrides};
//! use finstack_valuations::instruments::Instrument;
//! use finstack_valuations::metrics::{standard_registry, MetricId};
//! use finstack_core::dates::create_date;
//! use finstack_core::market_data::context::MarketContext;
//! use time::Month;
//!
//! # fn main() -> finstack_core::Result<()> {
//! let as_of = create_date(2024, Month::January, 1)?;
//! let option = EquityOption::european_call(
//!     "OPT-001",
//!     "SPX",
//!     4500.0,
//!     create_date(2024, Month::July, 1)?,
//!     finstack_core::money::Money::new(100_000.0, finstack_core::currency::Currency::USD),
//!     100.0,
//! )?;
//!
//! // Setup market
//! # let market = MarketContext::new();
//!
//! let _registry = standard_registry();
//! let metrics = vec![MetricId::Theta];
//!
//! // Customize theta period - supported formats:
//! // "1D", "2D", ... (days)
//! // "1W", "2W", ... (weeks)
//! // "1M", "3M", "6M", ... (months)
//! // "1Y", "2Y", ... (years)
//! let result = option.price_with_metrics(&market, as_of, &metrics)?;
//!
//! if let Some(theta) = result.measures.get(MetricId::Theta.as_str()) {
//!     println!("1-week theta: ${:.2}", theta);
//!     println!("This is the expected P&L from holding the option for 1 week");
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Example 3: Bond Carry (Theta with Coupon Accrual)
//!
//! ```rust,no_run
//! use finstack_valuations::instruments::{Bond, PricingOverrides};
//! use finstack_valuations::instruments::Instrument;
//! use finstack_valuations::metrics::{standard_registry, MetricId};
//! use finstack_core::dates::create_date;
//! use finstack_core::market_data::context::MarketContext;
//! use time::Month;
//!
//! # fn main() -> finstack_core::Result<()> {
//! let as_of = create_date(2024, Month::January, 1)?;
//! let bond = Bond::example();
//!
//! // Setup market
//! # let market = MarketContext::new();
//!
//! let _registry = standard_registry();
//! let metrics = vec![MetricId::Theta];
//!
//! // Measure 1-month carry
//! let result = bond.price_with_metrics(&market, as_of, &metrics)?;
//!
//! if let Some(theta) = result.measures.get(MetricId::Theta.as_str()) {
//!     println!("Bond value: ${:.2}", result.value.amount());
//!     println!("1-month carry: ${:.2}", theta);
//!     // Theta includes both:
//!     // 1. PV change (pull-to-par effect)
//!     // 2. Coupon payments during the period
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Example 4: Computing Theta Near Expiry
//!
//! When an instrument expires before the theta period ends, theta is automatically
//! capped at the expiry date:
//!
//! ```rust,no_run
//! use finstack_valuations::instruments::{EquityOption, PricingOverrides};
//! use finstack_valuations::instruments::Instrument;
//! use finstack_valuations::metrics::{standard_registry, MetricId};
//! use finstack_core::dates::create_date;
//! use finstack_core::market_data::context::MarketContext;
//! use time::Month;
//!
//! # fn main() -> finstack_core::Result<()> {
//! let as_of = create_date(2024, Month::June, 25)?;
//! let expiry = create_date(2024, Month::July, 1)?; // Only 6 days to expiry
//!
//! let option = EquityOption::european_call(
//!     "OPT-001",
//!     "SPX",
//!     4500.0,
//!     expiry,
//!     finstack_core::money::Money::new(100_000.0, finstack_core::currency::Currency::USD),
//!     100.0,
//! )?;
//!
//! // Setup market
//! # let market = MarketContext::new();
//!
//! let _registry = standard_registry();
//! let metrics = vec![MetricId::Theta];
//!
//! // Request 1-week theta, but only 6 days remain
//! let result = option.price_with_metrics(&market, as_of, &metrics)?;
//!
//! if let Some(theta) = result.measures.get(MetricId::Theta.as_str()) {
//!     println!("Theta to expiry (6 days): ${:.2}", theta);
//!     // Theta is computed only to expiry, not the full 7-day period
//!     // This equals: PV(expiry) - PV(today)
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # How Theta is Calculated
//!
//! Theta represents the total carry (profit/loss) from holding an instrument over a time period:
//!
//! ```text
//! Theta = PV(t + period) - PV(t) + Cashflows(t, t + period)
//! ```
//!
//! Where:
//! - `PV(t)` = present value at valuation date (base value)
//! - `PV(t + period)` = present value at rolled forward date
//! - `Cashflows(t, t + period)` = sum of cashflows received during the period
//!
//! ## Components
//!
//! 1. **Pull-to-par effect**: Change in present value due to passage of time
//!    - For bonds: Price converges to par as maturity approaches
//!    - For options: Time value decays (typically negative theta)
//!
//! 2. **Cashflows**: Interest, coupons, or other payments during the period
//!    - Bonds: Accrued interest, coupon payments
//!    - Swaps: Net interest payments
//!    - Options: Usually zero (no interim cashflows)
//!
//! ## Sign Convention
//!
//! - **Negative theta**: Instrument loses value over time (e.g., long options)
//! - **Positive theta**: Instrument gains value over time (e.g., short options, carry trades)
//! - **Zero theta**: No time-dependent value change (rare)

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
/// # use finstack_valuations::metrics::parse_period_days;
/// assert_eq!(parse_period_days("1D").expect("should succeed"), 1);
/// assert_eq!(parse_period_days("1W").expect("should succeed"), 7);
/// assert_eq!(parse_period_days("1M").expect("should succeed"), 30);
/// assert_eq!(parse_period_days("3M").expect("should succeed"), 90);
/// assert_eq!(parse_period_days("1Y").expect("should succeed"), 365);
/// ```
pub fn parse_period_days(period: &str) -> Result<i64> {
    let period = period.trim().to_uppercase();

    if period.is_empty() {
        return Err(finstack_core::Error::from(
            finstack_core::InputError::Invalid,
        ));
    }

    // Extract number and unit
    let (num_str, unit) = if let Some(pos) = period.find(|c: char| c.is_alphabetic()) {
        (&period[..pos], &period[pos..])
    } else {
        return Err(finstack_core::Error::from(
            finstack_core::InputError::Invalid,
        ));
    };

    let num: i64 = num_str
        .parse()
        .map_err(|_| finstack_core::Error::from(finstack_core::InputError::Invalid))?;

    let days = match unit {
        "D" => num,
        "W" => num * 7,
        "M" => num * 30,
        "Y" => num * 365,
        _ => {
            return Err(finstack_core::Error::from(
                finstack_core::InputError::Invalid,
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
        .ok_or_else(|| finstack_core::Error::from(finstack_core::InputError::Invalid))?;

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
        tracing::warn!(
            instrument_type = std::any::type_name::<I>(),
            as_of = %context.as_of,
            rolled_date = %rolled_date,
            "Theta: Instrument already expired or rolling to same date, returning 0.0"
        );
        return Ok(0.0);
    }

    // Base PV from the pre-computed valuation
    let base_pv = context.base_value.amount();

    // Reprice at rolled date with same market context
    let bumped_value = instrument.value_raw(&context.curves, rolled_date)?;
    let pv_change = bumped_value - base_pv;

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
/// use finstack_valuations::metrics::GenericTheta;
/// use finstack_valuations::instruments::Bond;
/// use finstack_valuations::metrics::{MetricRegistry, MetricId};
/// use finstack_valuations::pricer::InstrumentType;
/// use std::sync::Arc;
///
/// let mut registry = MetricRegistry::new();
/// registry.register_metric(
///     MetricId::Theta,
///     Arc::new(GenericTheta::<Bond>::default()),
///     &[InstrumentType::Bond],
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
    curves: &finstack_core::market_data::context::MarketContext,
    start_date: Date,
    end_date: Date,
) -> Result<f64>
where
    I: 'static,
{
    // Delegate to shared implementation
    collect_cashflows_impl(instrument as &dyn Any, curves, start_date, end_date)
}

/// Shared implementation for collecting cashflows from any instrument.
///
/// This is the single source of truth for cashflow collection logic,
/// used by both `collect_cashflows_in_period` and `collect_cashflows_in_period_any`.
fn collect_cashflows_impl(
    any_ref: &dyn Any,
    curves: &finstack_core::market_data::context::MarketContext,
    start_date: Date,
    end_date: Date,
) -> Result<f64> {
    use crate::cashflow::traits::CashflowProvider;
    use crate::instruments::*;

    // Try to downcast to known CashflowProvider implementors
    let cashflows: Option<Vec<(Date, finstack_core::money::Money)>> =
        // Bonds
        if let Some(bond) = any_ref.downcast_ref::<Bond>() {
            bond.build_schedule(curves, start_date).ok()
        }
        // Interest Rate Swaps
        else if let Some(irs) = any_ref.downcast_ref::<InterestRateSwap>() {
            irs.build_schedule(curves, start_date).ok()
        }
        // Deposits
        else if let Some(deposit) = any_ref.downcast_ref::<deposit::Deposit>() {
            deposit.build_schedule(curves, start_date).ok()
        }
        // FRAs
        else if let Some(fra) = any_ref.downcast_ref::<fra::ForwardRateAgreement>() {
            fra.build_schedule(curves, start_date).ok()
        }
        // IR Futures
        else if let Some(ir_fut) = any_ref.downcast_ref::<ir_future::InterestRateFuture>() {
            ir_fut.build_schedule(curves, start_date).ok()
        }
        // Equity
        else if let Some(equity) = any_ref.downcast_ref::<equity::Equity>() {
            equity.build_schedule(curves, start_date).ok()
        }
        // FX Spot
        else if let Some(fx_spot) = any_ref.downcast_ref::<fx_spot::FxSpot>() {
            fx_spot.build_schedule(curves, start_date).ok()
        }
        // Inflation-Linked Bonds
        else if let Some(inf_bond) =
            any_ref.downcast_ref::<inflation_linked_bond::InflationLinkedBond>()
        {
            inf_bond.build_schedule(curves, start_date).ok()
        }
        // Repos
        else if let Some(repo) = any_ref.downcast_ref::<repo::Repo>() {
            repo.build_schedule(curves, start_date).ok()
        }
        // Structured Credit
        else if let Some(sc) = any_ref.downcast_ref::<structured_credit::StructuredCredit>() {
            sc.build_schedule(curves, start_date).ok()
        }
        // TRS (both types)
        else if let Some(eq_trs) = any_ref.downcast_ref::<equity_trs::EquityTotalReturnSwap>() {
            eq_trs.build_schedule(curves, start_date).ok()
        } else if let Some(fi_trs) =
            any_ref.downcast_ref::<fi_trs::FIIndexTotalReturnSwap>()
        {
            fi_trs.build_schedule(curves, start_date).ok()
        }
        // Private Markets Fund
        else if let Some(pmf) =
            any_ref.downcast_ref::<private_markets_fund::PrivateMarketsFund>()
        {
            pmf.build_schedule(curves, start_date).ok()
        }
        // Variance Swap
        else if let Some(var_swap) = any_ref.downcast_ref::<variance_swap::VarianceSwap>() {
            var_swap.build_schedule(curves, start_date).ok()
        }
        // CDS - use premium schedule for cashflows
        else if let Some(cds) = any_ref.downcast_ref::<cds::CreditDefaultSwap>() {
            cds.build_premium_schedule(curves, start_date).ok()
        }
        // FX Swap - has explicit cashflows at near and far dates
        else if let Some(_fx_swap) = any_ref.downcast_ref::<fx_swap::FxSwap>() {
            // FX swaps don't have interim cashflows, only near/far settlement
            // Theta comes purely from PV change, not cashflows
            None
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
    if let Some(eq_trs) = instrument.downcast_ref::<equity_trs::EquityTotalReturnSwap>() {
        return Some(eq_trs.schedule.end);
    }
    if let Some(fi_trs) = instrument.downcast_ref::<fi_trs::FIIndexTotalReturnSwap>() {
        return Some(fi_trs.schedule.end);
    }
    if let Some(var_swap) = instrument.downcast_ref::<variance_swap::VarianceSwap>() {
        return Some(var_swap.maturity);
    }
    if let Some(ir_fut) = instrument.downcast_ref::<ir_future::InterestRateFuture>() {
        return Some(ir_fut.expiry_date);
    }
    if let Some(fx_swap) = instrument.downcast_ref::<fx_swap::FxSwap>() {
        return Some(fx_swap.far_date);
    }

    // No expiry for: equity, basket, convertible, structured_credit, private_markets_fund
    None
}

/// Universal theta calculator that works with any instrument via the Instrument trait.
///
/// Computes theta as the total carry from rolling the valuation date forward:
///   Theta = PV(end_date) - PV(start_date) + Sum(Cashflows from start to end)
///
/// This calculator works with `dyn Instrument` directly, using the trait's `value()` method,
/// and is registered as the default theta calculator for all instruments.
pub struct GenericThetaAny;

impl Default for GenericThetaAny {
    fn default() -> Self {
        Self
    }
}

impl crate::metrics::MetricCalculator for GenericThetaAny {
    fn calculate(&self, context: &mut crate::metrics::MetricContext) -> Result<f64> {
        // Get theta period from pricing overrides, default to "1D"
        let period_str = context
            .pricing_overrides
            .as_ref()
            .and_then(|po| po.theta_period.as_deref())
            .unwrap_or("1D");

        // Get expiry date if available (instrument-specific via as_any downcast)
        let expiry_date = get_instrument_expiry(context.instrument.as_any());

        // Calculate rolled date
        let rolled_date = calculate_theta_date(context.as_of, period_str, expiry_date)?;

        // If already expired or rolling to same date, theta is zero
        if rolled_date <= context.as_of {
            tracing::warn!(
                as_of = %context.as_of,
                rolled_date = %rolled_date,
                "GenericThetaAny: Instrument already expired or rolling to same date, returning 0.0"
            );
            return Ok(0.0);
        }

        // Base PV from the pre-computed valuation
        let base_pv = context.base_value.amount();

        // Reprice at rolled date with same market context using the trait method directly
        let bumped_value = context.instrument.value_raw(&context.curves, rolled_date)?;
        let pv_change = bumped_value - base_pv;

        // Collect cashflows during the period (using helper that does downcasting internally)
        let cashflows_during_period = collect_cashflows_in_period_any(
            context.instrument.as_ref(),
            &context.curves,
            context.as_of,
            rolled_date,
        )?;

        // Theta = PV change + cashflows received
        Ok(pv_change + cashflows_during_period)
    }

    fn dependencies(&self) -> &[crate::metrics::MetricId] {
        &[]
    }
}

/// Collect cashflows from any instrument during a time period.
///
/// This is a thin wrapper that delegates to `collect_cashflows_impl`.
fn collect_cashflows_in_period_any(
    instrument: &dyn crate::instruments::common::traits::Instrument,
    curves: &finstack_core::market_data::context::MarketContext,
    start_date: Date,
    end_date: Date,
) -> Result<f64> {
    // Use as_any to get &dyn Any, then delegate to shared implementation
    collect_cashflows_impl(instrument.as_any(), curves, start_date, end_date)
}

// ================================================================================================
// Unit tests (internal helpers)
// ================================================================================================

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use time::macros::date;
    use time::Month;

    fn test_date() -> Date {
        date!(2025 - 01 - 01)
    }

    // -------------------------------------------------------------------------
    // Period parsing
    // -------------------------------------------------------------------------

    #[test]
    fn parse_period_days_standard() {
        assert_eq!(parse_period_days("1D").expect("parse 1D"), 1);
        assert_eq!(parse_period_days("7D").expect("parse 7D"), 7);
        assert_eq!(parse_period_days("30D").expect("parse 30D"), 30);
    }

    #[test]
    fn parse_period_days_weeks() {
        assert_eq!(parse_period_days("1W").expect("parse 1W"), 7);
        assert_eq!(parse_period_days("2W").expect("parse 2W"), 14);
        assert_eq!(parse_period_days("4W").expect("parse 4W"), 28);
    }

    #[test]
    fn parse_period_days_months() {
        assert_eq!(parse_period_days("1M").expect("parse 1M"), 30);
        assert_eq!(parse_period_days("3M").expect("parse 3M"), 90);
        assert_eq!(parse_period_days("6M").expect("parse 6M"), 180);
        assert_eq!(parse_period_days("12M").expect("parse 12M"), 360);
    }

    #[test]
    fn parse_period_days_years() {
        assert_eq!(parse_period_days("1Y").expect("parse 1Y"), 365);
        assert_eq!(parse_period_days("2Y").expect("parse 2Y"), 730);
        assert_eq!(parse_period_days("5Y").expect("parse 5Y"), 1825);
    }

    #[test]
    fn parse_period_days_lowercase_and_whitespace() {
        assert_eq!(parse_period_days("1d").expect("parse 1d"), 1);
        assert_eq!(parse_period_days(" 1W ").expect("parse 1W"), 7);
        assert_eq!(parse_period_days(" 3m ").expect("parse 3M"), 90);
        assert_eq!(parse_period_days("  1y  ").expect("parse 1Y"), 365);
    }

    #[test]
    fn parse_period_days_invalid_format_errors() {
        assert!(parse_period_days("").is_err());
        assert!(parse_period_days("1X").is_err());
        assert!(parse_period_days("XYZ").is_err());
        assert!(parse_period_days("D").is_err());
        assert!(parse_period_days("1").is_err());
        assert!(parse_period_days("abc").is_err());
    }

    #[test]
    fn parse_period_days_edge_cases() {
        assert_eq!(parse_period_days("0D").expect("parse 0D"), 0);
        assert_eq!(parse_period_days("100D").expect("parse 100D"), 100);
        assert_eq!(parse_period_days("10Y").expect("parse 10Y"), 3650);
    }

    // -------------------------------------------------------------------------
    // Theta date calculation
    // -------------------------------------------------------------------------

    #[test]
    fn calculate_theta_date_no_expiry() {
        let base = test_date();
        let rolled = calculate_theta_date(base, "1D", None).expect("roll 1D");
        let expected = Date::from_calendar_date(2025, Month::January, 2).expect("expected date");
        assert_eq!(rolled, expected);
    }

    #[test]
    fn calculate_theta_date_one_week() {
        let base = test_date();
        let rolled = calculate_theta_date(base, "1W", None).expect("roll 1W");
        let expected = Date::from_calendar_date(2025, Month::January, 8).expect("expected date");
        assert_eq!(rolled, expected);
    }

    #[test]
    fn calculate_theta_date_one_month() {
        let base = test_date();
        let rolled = calculate_theta_date(base, "1M", None).expect("roll 1M");
        let expected = Date::from_calendar_date(2025, Month::January, 31).expect("expected date");
        assert_eq!(rolled, expected);
    }

    #[test]
    fn calculate_theta_date_with_expiry_cap() {
        let base = test_date();
        let expiry = Date::from_calendar_date(2025, Month::January, 5).expect("expiry date");

        let rolled = calculate_theta_date(base, "1W", Some(expiry)).expect("roll 1W");
        assert_eq!(rolled, expiry);
    }

    #[test]
    fn calculate_theta_date_before_expiry() {
        let base = test_date();
        let expiry = Date::from_calendar_date(2025, Month::February, 1).expect("expiry date");

        let rolled = calculate_theta_date(base, "1D", Some(expiry)).expect("roll 1D");
        let expected = Date::from_calendar_date(2025, Month::January, 2).expect("expected date");
        assert_eq!(rolled, expected);
    }

    #[test]
    fn calculate_theta_date_exactly_at_expiry() {
        let base = test_date();
        let expiry = Date::from_calendar_date(2025, Month::January, 31).expect("expiry date");

        let rolled = calculate_theta_date(base, "30D", Some(expiry)).expect("roll 30D");
        assert_eq!(rolled, expiry);
    }

    #[test]
    fn calculate_theta_date_already_past_expiry() {
        let base = Date::from_calendar_date(2025, Month::February, 1).expect("base date");
        let expiry = test_date();

        let rolled = calculate_theta_date(base, "1D", Some(expiry)).expect("roll 1D");
        assert_eq!(rolled, expiry);
    }

    #[test]
    fn calculate_theta_date_various_periods() {
        let base = test_date();

        let rolled_3m = calculate_theta_date(base, "3M", None).expect("roll 3M");
        assert_eq!(rolled_3m, base + time::Duration::days(90));

        let rolled_1y = calculate_theta_date(base, "1Y", None).expect("roll 1Y");
        assert_eq!(rolled_1y, base + time::Duration::days(365));
    }

    #[test]
    fn theta_workflow_short_dated_expiry_capped() {
        let base = test_date();
        let expiry = Date::from_calendar_date(2025, Month::January, 6).expect("expiry date");

        let theta_date_1d = calculate_theta_date(base, "1D", Some(expiry)).expect("roll 1D");
        assert_eq!(
            theta_date_1d,
            Date::from_calendar_date(2025, Month::January, 2).expect("expected date")
        );

        let theta_date_1w = calculate_theta_date(base, "1W", Some(expiry)).expect("roll 1W");
        assert_eq!(theta_date_1w, expiry);
    }
}
