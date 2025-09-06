//! Financial instruments for valuation and risk analysis.
//!
//! This module provides concrete implementations of common financial instruments
//! including bonds, interest rate swaps, and deposits. Each instrument type
//! implements the necessary traits for pricing, cashflow generation, and
//! metric calculation.
//!
//! # Supported Instruments
//!
//! - **Bonds**: Fixed-rate bonds with configurable coupon schedules and day counts
//! - **Interest Rate Swaps**: Fixed-for-floating interest rate swaps
//! - **Deposits**: Simple interest-bearing deposits with various day count conventions
//!
//! # Quick Start
//!
//! ```rust
//! use finstack_valuations::instruments::{Bond, InterestRateSwap, Deposit};
//! use finstack_core::dates::{Date, Frequency, DayCount, BusinessDayConvention, StubKind};
//! use finstack_core::money::Money;
//! use finstack_core::currency::Currency;
//! use time::Month;
//!
//! // Create instruments with proper constructors
//! let bond = Bond {
//!     id: "BOND001".to_string(),
//!     notional: Money::new(1000.0, Currency::USD),
//!     coupon: 0.05,
//!     freq: Frequency::semi_annual(),
//!     dc: DayCount::Act365F,
//!     issue: Date::from_calendar_date(2025, Month::January, 15).unwrap(),
//!     maturity: Date::from_calendar_date(2026, Month::January, 15).unwrap(),
//!     disc_id: "USD-OIS",
//!     quoted_clean: None,
//!     call_put: None,
//!     amortization: None,
//!     custom_cashflows: None,
//!     attributes: finstack_valuations::instruments::traits::Attributes::new(),
//! };
//!
//! let irs = InterestRateSwap {
//!     id: "IRS001".to_string(),
//!     notional: Money::new(1000.0, Currency::USD),
//!     side: finstack_valuations::instruments::fixed_income::irs::PayReceive::PayFixed,
//!     fixed: finstack_valuations::instruments::fixed_income::irs::FixedLegSpec {
//!         start: Date::from_calendar_date(2025, Month::January, 15).unwrap(),
//!         end: Date::from_calendar_date(2030, Month::January, 15).unwrap(),
//!         freq: Frequency::semi_annual(),
//!         stub: StubKind::None,
//!         bdc: BusinessDayConvention::Following,
//!         calendar_id: None,
//!         dc: DayCount::Act365F,
//!         rate: 0.05,
//!         disc_id: "USD-OIS",
//!     },
//!     float: finstack_valuations::instruments::fixed_income::irs::FloatLegSpec {
//!         start: Date::from_calendar_date(2025, Month::January, 15).unwrap(),
//!         end: Date::from_calendar_date(2030, Month::January, 15).unwrap(),
//!         freq: Frequency::semi_annual(),
//!         stub: StubKind::None,
//!         bdc: BusinessDayConvention::Following,
//!         calendar_id: None,
//!         dc: DayCount::Act365F,
//!         disc_id: "USD-OIS",
//!         fwd_id: "USD-LIBOR-3M",
//!         spread_bp: 0.0,
//!     },
//!     attributes: finstack_valuations::instruments::traits::Attributes::new(),
//! };
//!
//! let deposit = Deposit {
//!     id: "DEP001".to_string(),
//!     notional: Money::new(1000.0, Currency::USD),
//!     start: Date::from_calendar_date(2025, Month::January, 15).unwrap(),
//!     end: Date::from_calendar_date(2025, Month::July, 15).unwrap(),
//!     day_count: DayCount::Act365F,
//!     disc_id: "USD-OIS",
//!     quote_rate: Some(0.05),
//!     attributes: finstack_valuations::instruments::traits::Attributes::new(),
//! };
//!
//! // Use trait objects for unified handling
//! use finstack_valuations::instruments::traits::InstrumentLike;
//! let instruments: Vec<Box<dyn InstrumentLike>> = vec![
//!     Box::new(bond),
//!     Box::new(irs),
//!     Box::new(deposit),
//! ];
//!
//! // Check instrument types
//! for instrument in &instruments {
//!     println!("Instrument type: {}", instrument.instrument_type());
//! }
//! ```

// Macro infrastructure for reducing boilerplate
#[macro_use]
pub mod macros;

// Instrument-level traits and metadata
pub mod traits;

// Grouped instrument implementations
pub mod equity;
pub mod fixed_income;
// fx_spot moved under fixed_income
pub mod options;

// Re-export individual instrument types
pub use equity::Equity;
pub use fixed_income::fx_spot::FxSpot;
pub use fixed_income::{
    Bond, ConvertibleBond, CreditDefaultSwap, Deposit, FxSwap, InflationLinkedBond,
    InterestRateSwap, Loan,
};
pub use options::{CreditOption, EquityOption, FxOption, InterestRateOption, Swaption};
// Individual instrument types can be used directly or via trait objects for unified handling.

/// Shared helper to build a ValuationResult with a set of metrics.
///
/// Centralizes the repeated pattern across instruments to compute base value,
/// build metric context, compute metrics and stamp a result.
///
/// This function uses trait objects to avoid generic monomorphization across
/// compilation units, which can cause coverage metadata mismatches.
pub fn build_with_metrics_dyn(
    instrument: &dyn traits::InstrumentLike,
    curves: &finstack_core::market_data::MarketContext,
    as_of: finstack_core::dates::Date,
    base_value: finstack_core::money::Money,
    metrics: &[crate::metrics::MetricId],
) -> finstack_core::Result<crate::results::ValuationResult> {
    use crate::metrics::{standard_registry, MetricContext};
    use indexmap::IndexMap;
    use std::sync::Arc;

    // Create an owned clone for the Arc to avoid lifetime issues
    // This approach reduces generic monomorphization across compilation units
    let instrument_clone: Box<dyn traits::InstrumentLike> = {
        use crate::instruments::*;

        // Fixed Income instruments
        if let Some(bond) = instrument.as_any().downcast_ref::<Bond>() {
            Box::new(bond.clone())
        } else if let Some(loan) = instrument.as_any().downcast_ref::<Loan>() {
            Box::new(loan.clone())
        } else if let Some(irs) = instrument.as_any().downcast_ref::<InterestRateSwap>() {
            Box::new(irs.clone())
        } else if let Some(cds) = instrument.as_any().downcast_ref::<CreditDefaultSwap>() {
            Box::new(cds.clone())
        } else if let Some(convertible) = instrument.as_any().downcast_ref::<ConvertibleBond>() {
            Box::new(convertible.clone())
        } else if let Some(deposit) = instrument.as_any().downcast_ref::<Deposit>() {
            Box::new(deposit.clone())
        } else if let Some(inflation_bond) =
            instrument.as_any().downcast_ref::<InflationLinkedBond>()
        {
            Box::new(inflation_bond.clone())
        } else if let Some(fx_spot) = instrument.as_any().downcast_ref::<FxSpot>() {
            Box::new(fx_spot.clone())
        } else if let Some(fx_swap) = instrument.as_any().downcast_ref::<FxSwap>() {
            Box::new(fx_swap.clone())

        // Equity instruments
        } else if let Some(equity) = instrument.as_any().downcast_ref::<Equity>() {
            Box::new(equity.clone())

        // Options
        } else if let Some(equity_option) = instrument.as_any().downcast_ref::<EquityOption>() {
            Box::new(equity_option.clone())
        } else if let Some(fx_option) = instrument.as_any().downcast_ref::<FxOption>() {
            Box::new(fx_option.clone())
        } else if let Some(credit_option) = instrument.as_any().downcast_ref::<CreditOption>() {
            Box::new(credit_option.clone())
        } else if let Some(ir_option) = instrument.as_any().downcast_ref::<InterestRateOption>() {
            Box::new(ir_option.clone())
        } else if let Some(swaption) = instrument.as_any().downcast_ref::<Swaption>() {
            Box::new(swaption.clone())
        } else {
            return Err(finstack_core::error::InputError::NotFound {
                id: format!(
                    "unsupported instrument type for metrics computation: {}",
                    instrument.instrument_type()
                ),
            }
            .into());
        }
    };

    let mut context = MetricContext::new(
        Arc::from(instrument_clone),
        Arc::new(curves.clone()),
        as_of,
        base_value,
    );

    let registry = standard_registry();
    let metric_measures = registry.compute(metrics, &mut context)?;

    // Deterministic insertion order: follow the requested metrics slice order
    let mut measures: IndexMap<String, finstack_core::F> = IndexMap::new();
    for metric_id in metrics {
        if let Some(value) = metric_measures.get(metric_id) {
            measures.insert(metric_id.as_str().to_string(), *value);
        }
    }

    let mut result = crate::results::ValuationResult::stamped(instrument.id(), as_of, base_value);
    result.measures = measures;
    Ok(result)
}

/// Deprecated generic version for backward compatibility.
/// Use `build_with_metrics_dyn` instead to avoid coverage metadata conflicts.
#[deprecated(
    since = "0.3.0",
    note = "Use build_with_metrics_dyn to avoid coverage metadata conflicts"
)]
pub fn build_with_metrics<I>(
    instrument: I,
    curves: &finstack_core::market_data::MarketContext,
    as_of: finstack_core::dates::Date,
    base_value: finstack_core::money::Money,
    metrics: &[crate::metrics::MetricId],
) -> finstack_core::Result<crate::results::ValuationResult>
where
    I: traits::InstrumentLike + Clone + 'static,
{
    build_with_metrics_dyn(&instrument, curves, as_of, base_value, metrics)
}
