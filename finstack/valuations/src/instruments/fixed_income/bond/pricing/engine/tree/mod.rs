//! Tree-based pricing engine for bonds with embedded options and OAS calculations.
//!
//! This module provides tree-based pricing for callable/putable bonds and option-adjusted
//! spread (OAS) calculations using either:
//! - **Short-rate tree**: For bonds without credit risk
//! - **Rates+credit tree**: For bonds with credit risk (two-factor model)
//!
//! # Pricing Models
//!
//! ## Short-Rate Tree
//! Used for bonds without embedded credit risk. The tree models interest rate evolution
//! and applies call/put constraints via backward induction.
//!
//! ## Rates+Credit Tree
//! Used when a hazard curve is present in the market context. Models both interest rate
//! and credit risk evolution, with default events and recovery payments.
//!
//! # Examples
//!
//! ```rust,no_run
//! use finstack_valuations::instruments::fixed_income::bond::Bond;
//! use finstack_valuations::instruments::fixed_income::bond::pricing::engine::tree::TreePricer;
//! use finstack_core::market_data::context::MarketContext;
//! use finstack_core::dates::Date;
//!
//! # let bond = Bond::example().unwrap();
//! # let market = MarketContext::new();
//! # let as_of = Date::from_calendar_date(2024, time::Month::January, 15).unwrap();
//! let pricer = TreePricer::new();
//! let oas_bp = pricer.calculate_oas(&bond, &market, as_of, 98.5)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # See Also
//!
//! - `TreePricer` for OAS calculation
//! - tree-valuator implementation details in this module
//! - `TreePricerConfig` for configuration options

mod bond_valuator;
mod config;
#[cfg(test)]
mod tests;
mod tree_pricer;

pub use bond_valuator::BondValuator;
pub use config::{bond_tree_config, TreeModelChoice, TreePricerConfig};
pub use tree_pricer::{calculate_oas, TreePricer};
