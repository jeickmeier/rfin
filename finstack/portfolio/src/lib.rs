#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(clippy::new_without_default)]
#![warn(clippy::float_cmp)]
#![cfg_attr(test, allow(clippy::float_cmp))]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
// Allow expect() in doc tests (they are test code)
#![doc(test(attr(allow(clippy::expect_used))))]

//! Portfolio management and aggregation for finstack.
//!
//! This crate provides portfolio-level operations including:
//! - Entity and position management
//! - Valuation aggregation across positions
//! - Metrics aggregation with cross-currency support
//! - Attribute-based grouping and analysis
//! - Scenario application
//! - Tabular exports for analysis
//!
//! # Quick Start
//!
//! ```rust
//! use finstack_portfolio::Portfolio;
//! use finstack_portfolio::position::{Position, PositionUnit};
//! use finstack_portfolio::types::Entity;
//! use finstack_core::currency::Currency;
//! use finstack_core::money::Money;
//! use finstack_valuations::instruments::rates::deposit::Deposit;
//! use std::sync::Arc;
//! use time::macros::date;
//!
//! let as_of = date!(2024-01-01);
//!
//! // Create a deposit instrument
//! let deposit = Deposit::builder()
//!     .id("DEP_1M".into())
//!     .notional(Money::new(1_000_000.0, Currency::USD))
//!     .start_date(as_of)
//!     .maturity(date!(2024-02-01))
//!     .day_count(finstack_core::dates::DayCount::Act360)
//!     .discount_curve_id("USD".into())
//!     .build()
//!     .expect("test should succeed");
//!
//! // Create a position holding the deposit
//! let position = Position::new(
//!     "POS_001",
//!     "ACME_CORP",
//!     "DEP_1M",
//!     Arc::new(deposit),
//!     1.0,
//!     PositionUnit::Units,
//! ).expect("test should succeed")
//!  .with_text_attribute("asset_class", "cash");
//!
//! // Build the portfolio with the entity and position
//! let portfolio = Portfolio::builder("MY_FUND")
//!     .base_ccy(Currency::USD)
//!     .as_of(as_of)
//!     .entity(Entity::new("ACME_CORP"))
//!     .position(position)
//!     .build()
//!     .expect("test should succeed");
//! ```

/// Portfolio-level PnL attribution and breakdowns.
pub mod attribution;
/// Book hierarchy and identifiers.
pub mod book;
/// Fluent portfolio construction helpers.
pub mod builder;
/// Tabular exports for portfolio results.
pub mod dataframe;
/// Error types for portfolio operations.
pub mod error;
/// Factor-model portfolio risk decomposition outputs and engines.
pub mod factor_model;
/// Grouping and aggregation by attributes or books.
pub mod grouping;
/// Liquidity risk metrics, spread estimation, and portfolio scoring.
pub mod liquidity;
/// Portfolio margin and netting set utilities.
pub mod margin;
/// Metrics aggregation and reporting.
pub mod metrics;
/// Portfolio optimization engines and constraints.
pub mod optimization;
/// Portfolio container and state management.
pub mod portfolio;
/// Position primitives and units.
pub mod position;
/// Result envelopes for portfolio operations.
pub mod results;
/// Core portfolio entity and ID types.
pub mod types;
/// Portfolio valuation APIs.
pub mod valuation;

/// Cashflow ladder and schedule aggregation utilities.
pub mod cashflows;
/// Market-factor dependency index for selective repricing.
pub mod dependencies;

#[cfg(test)]
#[allow(clippy::expect_used)]
mod test_utils;

/// Scenario application for portfolios.
pub mod scenarios;

/// Historical scenario replay for portfolios.
pub mod replay;

pub use builder::PortfolioBuilder;
pub use error::{Error, Result};
pub use portfolio::Portfolio;
