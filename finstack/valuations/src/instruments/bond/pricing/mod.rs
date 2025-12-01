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
//! ```rust,no_run
//! use finstack_valuations::instruments::bond::Bond;
//! use finstack_valuations::instruments::bond::pricing::discount_engine::BondEngine;
//! use finstack_core::market_data::context::MarketContext;
//! use finstack_core::dates::Date;
//!
//! # let bond = Bond::example();
//! # let market = MarketContext::new();
//! # let as_of = Date::from_calendar_date(2024, time::Month::January, 15).unwrap();
//! let pv = BondEngine::price(&bond, &market, as_of)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
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
pub mod pricer;
/// Quote engine for mapping between price, yields, and spreads
pub mod quote_engine;
/// Tree-based pricing for callable/putable bonds and OAS
pub mod tree_engine;
pub mod ytm_solver;
