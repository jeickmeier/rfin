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
//!     side: finstack_valuations::instruments::irs::PayReceive::PayFixed,
//!     fixed: finstack_valuations::instruments::irs::FixedLegSpec {
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
//!     float: finstack_valuations::instruments::irs::FloatLegSpec {
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

// Macros are available via #[macro_use]

// Unified instrument handling with common operations
pub mod unified;

// Original instrument implementations
pub mod irs;
pub mod bond;
pub mod deposit;
pub mod equity;
pub mod fx_spot;
pub mod loan;
pub mod cds;
pub mod options;
pub mod ilb;
pub mod swaption;

// Re-export unified types
pub use unified::{Instrument as UnifiedInstrument, InstrumentPortfolio};

// Re-export individual instrument types
pub use bond::Bond;
pub use deposit::Deposit;
pub use irs::InterestRateSwap;
pub use equity::Equity;
pub use fx_spot::FxSpot;
pub use loan::Loan;
pub use cds::CreditDefaultSwap;
pub use ilb::InflationLinkedBond;
pub use options::{EquityOption, FxOption, InterestRateOption, CreditOption};
pub use swaption::Swaption;

/// A concrete enum for all supported instrument types.
/// 
/// Provides a unified interface for different instrument types while
/// maintaining type safety and enabling pattern matching. This enum allows
/// you to work with heterogeneous collections of instruments while preserving
/// their specific functionality.
/// 
/// # Examples
/// 
/// ```rust
/// use finstack_valuations::instruments::Instrument;
/// use finstack_valuations::instruments::Bond;
/// use finstack_core::dates::{Date, Frequency, DayCount, BusinessDayConvention, StubKind};
/// use finstack_core::money::Money;
/// use finstack_core::currency::Currency;
/// use time::Month;
/// 
/// // Create a sample bond for pattern matching
/// let bond = Bond {
///     id: "BOND001".to_string(),
///     notional: Money::new(1000.0, Currency::USD),
///     coupon: 0.05,
///     freq: Frequency::semi_annual(),
///     dc: DayCount::Act365F,
///     issue: Date::from_calendar_date(2025, Month::January, 15).unwrap(),
///     maturity: Date::from_calendar_date(2026, Month::January, 15).unwrap(),
///     disc_id: "USD-OIS",
///     quoted_clean: None,
///     call_put: None,
///     amortization: None,
///     custom_cashflows: None,
///     attributes: finstack_valuations::traits::Attributes::new(),
/// };
/// 
/// let instrument = Instrument::Bond(bond);
/// 
/// // Pattern matching
/// match instrument {
///     Instrument::Bond(bond) => println!("Bond with maturity: {:?}", bond.maturity),
///     Instrument::IRS(irs) => println!("IRS with notional: {:?}", irs.notional),
///     Instrument::Deposit(dep) => println!("Deposit with end date: {:?}", dep.end),
///     Instrument::Equity(eq) => println!("Equity with ticker: {:?}", eq.ticker),
///     Instrument::FxSpot(fx) => println!("FX spot for pair: {:?}/{:?}", fx.base, fx.quote),
///     Instrument::Loan(loan) => println!("Loan with maturity: {:?}", loan.maturity_date),
///     Instrument::CDS(cds) => println!("CDS with reference entity: {:?}", cds.reference_entity),
///     Instrument::ILB(ilb) => println!("ILB with real coupon: {:?}", ilb.real_coupon),
///     Instrument::EquityOption(opt) => println!("Equity option with strike: {:?}", opt.strike),
///     Instrument::FxOption(opt) => println!("FX option with strike: {:?}", opt.strike),
///     Instrument::InterestRateOption(opt) => println!("IR option with strike: {:?}", opt.strike_rate),
///     Instrument::CreditOption(opt) => println!("Credit option with strike: {:?}", opt.strike_spread_bp),
/// }
/// 
/// // Collection handling
/// let instruments: Vec<Instrument> = vec![];
/// let bond_count = instruments.iter()
///     .filter(|i| matches!(i, Instrument::Bond(_)))
///     .count();
/// ```
#[derive(Clone, Debug)]
pub enum Instrument {
    /// Fixed-rate bond instrument
    Bond(Bond),
    /// Interest rate swap instrument
    IRS(InterestRateSwap),
    /// Deposit instrument
    Deposit(Deposit),
    /// Equity spot instrument
    Equity(Equity),
    /// FX spot instrument
    FxSpot(FxSpot),
    /// Private credit loan instrument
    Loan(Loan),
    /// Credit Default Swap instrument
    CDS(CreditDefaultSwap),
    /// Inflation-Linked Bond instrument
    ILB(InflationLinkedBond),
    /// Equity Option instrument
    EquityOption(EquityOption),
    /// FX Option instrument
    FxOption(FxOption),
    /// Interest Rate Option instrument
    InterestRateOption(InterestRateOption),
    /// Credit Option instrument
    CreditOption(CreditOption),
}

impl Instrument {
    /// Returns the instrument type as a string identifier.
    /// 
    /// Centralizes instrument type detection logic and eliminates
    /// repeated match statements throughout the codebase. This method
    /// is useful for logging, serialization, and dynamic dispatch.
    /// 
    /// # Returns
    /// Static string identifier for the instrument type
    /// 
    /// # Example
    /// ```rust
    /// use finstack_valuations::instruments::Instrument;
    /// use finstack_valuations::instruments::Bond;
    /// use finstack_core::currency::Currency;
    /// use finstack_core::money::Money;
    /// use finstack_core::dates::{Date, Frequency, DayCount, BusinessDayConvention};
    /// use finstack_core::dates::StubKind;
    /// use time::Month;
    /// 
    /// let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    /// let maturity = Date::from_calendar_date(2026, Month::January, 15).unwrap();
    /// 
    /// // Create a bond instrument
    /// let bond = Bond {
    ///     id: "BOND001".to_string(),
    ///     notional: Money::new(1000.0, Currency::USD),
    ///     coupon: 0.05,
    ///     freq: Frequency::semi_annual(),
    ///     dc: DayCount::Act365F,
    ///     issue,
    ///     maturity,
    ///     disc_id: "USD-OIS",
    ///     quoted_clean: None,
    ///     call_put: None,
    ///     amortization: None,
    ///     custom_cashflows: None,
    ///     attributes: finstack_valuations::traits::Attributes::new(),
    /// };
    /// let instrument = Instrument::Bond(bond);
    /// 
    /// // Check instrument type
    /// assert_eq!(instrument.instrument_type(), "Bond");
    /// 
    /// // Use in conditional logic
    /// if instrument.instrument_type() == "Bond" {
    ///     println!("Processing bond instrument");
    /// }
    /// ```
    pub fn instrument_type(&self) -> &'static str {
        match self {
            Instrument::Bond(_) => "Bond",
            Instrument::IRS(_) => "IRS",
            Instrument::Deposit(_) => "Deposit",
            Instrument::Equity(_) => "Equity",
            Instrument::FxSpot(_) => "FxSpot",
            Instrument::Loan(_) => "Loan",
            Instrument::CDS(_) => "CDS",
            Instrument::ILB(_) => "ILB",
            Instrument::EquityOption(_) => "EquityOption",
            Instrument::FxOption(_) => "FxOption",
            Instrument::InterestRateOption(_) => "InterestRateOption",
            Instrument::CreditOption(_) => "CreditOption",
        }
    }
}


