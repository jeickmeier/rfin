//! Finstack Valuations module.
//!
//! Provides pricing, risk metrics, and cashflow generation for financial instruments.
//! Built on a simplified registry system that replaces complex macro-driven pricing.
//!
//! # Quick Start (New Simplified API)
//! ```rust,no_run
//! use finstack_valuations::instruments::Bond;
//! use finstack_valuations::pricer::{create_standard_registry, ModelKey};
//! use finstack_core::currency::Currency;
//! use finstack_core::money::Money;
//! use finstack_core::dates::Date;
//! use time::Month;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Create simplified pricer registry (no macros!)
//! let registry = create_standard_registry();
//!
//! // Create a bond
//! let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
//! let maturity = Date::from_calendar_date(2026, Month::January, 15).unwrap();
//! let bond = Bond::fixed(
//!     "BOND-001",
//!     Money::new(1_000_000.0, Currency::USD),
//!     0.05,
//!     issue,
//!     maturity,
//!     "USD-OIS"
//! );
//!
//! # let market_context = finstack_core::market_data::MarketContext::new();
//! // Price using simplified registry system
//! let result = registry.price_with_registry(
//!     &bond,
//!     ModelKey::Discounting,
//!     &market_context
//! )?;
//!
//! println!("Bond PV: {}", result.value);
//! # Ok(())
//! # }
//! ```
//!
//! # Key Improvements
//! - ✅ **No macro complexity** - direct function calls
//! - ✅ **Type-safe** - compile-time instrument type checking
//! - ✅ **IDE-friendly** - full code completion and debugging
//! - ✅ **Explicit** - clear pricer registration and lookup

pub mod calibration;
pub mod cashflow;
pub mod constants;
pub mod pricer;
pub mod results;

// Export macros before instruments module
#[macro_use]
pub mod instruments;
pub mod covenants;
pub mod metrics;
pub mod performance;
