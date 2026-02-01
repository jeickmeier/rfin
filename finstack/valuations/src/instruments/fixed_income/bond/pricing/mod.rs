//! Bond pricing entrypoints and pricers.
//!
//! This module provides bond pricing engines and pricers for various pricing models:
//!
//! # Pricing Engines
//!
//! - **Discount Engine**: Standard present value calculation using discount curves
//! - **Hazard Engine**: Credit-adjusted pricing using hazard curves with fractional recovery of par
//! - **Tree Engine**: Tree-based pricing for callable/putable bonds and option-adjusted spread (OAS)
//! - **Quote Engine**: Conversion between price, yield, and spread metrics
//! - **YTM Solver**: Robust yield-to-maturity calculation using hybrid Newton-Brent method
//!
//! # Examples
//!
//! Price a bond using the [`Instrument`] trait:
//!
//! ```rust,ignore
//! use finstack_valuations::instruments::Bond;
//! use finstack_valuations::instruments::Instrument;
//! use finstack_core::market_data::context::MarketContext;
//! use time::macros::date;
//!
//! let bond = Bond::example();
//! let market = MarketContext::new();
//! let as_of = date!(2024-01-15);
//!
//! // Use Instrument trait for pricing
//! let pv = bond.value(&market, as_of)?;
//! ```
//!
//! [`Instrument`]: crate::instruments::common_impl::traits::Instrument
//!
//! # See Also
//!
//! - [`Bond`] for bond construction
//! - [`discount_engine::BondEngine`] for standard discounting
//! - [`hazard_engine::HazardBondEngine`] for credit-adjusted pricing
//! - [`tree_engine::TreePricer`] for OAS and embedded options

/// Bond pricing engine (discount curve-based valuation logic)
pub mod discount_engine;
/// Hazard-rate FRP bond pricing engine (HazardCurve + recovery)
pub mod hazard_engine;
/// Bond pricer implementation (registry integration)
pub(crate) mod pricer;
/// Quote engine for mapping between price, yields, and spreads
pub mod quote_engine;
/// Tree-based pricing for callable/putable bonds and OAS
pub mod tree_engine;
pub mod ytm_solver;

/// Settlement date and quote-date utilities for bond pricing.
///
/// This module provides the `QuoteDateContext` struct which computes:
/// - `quote_date`: Settlement date (as_of + settlement_days) or as_of if no settlement convention
/// - `accrued_at_quote_date`: Accrued interest at the quote date
///
/// These are used by yield/spread metrics to properly interpret market quotes.
pub(crate) mod settlement;
