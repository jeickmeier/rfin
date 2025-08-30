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
//! use finstack_valuations::instruments::{Instrument, Bond, InterestRateSwap, Deposit};
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
//!     attributes: finstack_valuations::traits::Attributes::new(),
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
//!     attributes: finstack_valuations::traits::Attributes::new(),
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
//!     attributes: finstack_valuations::traits::Attributes::new(),
//! };
//!
//! // Use unified interface
//! let instruments: Vec<Instrument> = vec![
//!     Instrument::Bond(bond),
//!     Instrument::IRS(irs),
//!     Instrument::Deposit(deposit),
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

// Unified instrument handling with common operations
pub mod unified;

// Grouped instrument implementations
pub mod equity;
pub mod fixed_income;
pub mod fx_spot;
pub mod options;

// Re-export unified types as the canonical Instrument
pub use unified::{Instrument, InstrumentPortfolio};

// Re-export individual instrument types
pub use equity::Equity;
pub use fixed_income::{
    Bond, CreditDefaultSwap, Deposit, InflationLinkedBond, InterestRateSwap, Loan,
};
pub use fx_spot::FxSpot;
pub use options::{CreditOption, EquityOption, FxOption, InterestRateOption, Swaption};
// The canonical Instrument enum and helpers now live in `unified` and are re-exported above.
