#![deny(unsafe_code)]
#![warn(missing_docs)]
#![deny(clippy::unwrap_used)]

//! Portfolio management and aggregation for finstack.
//!
//! This crate provides portfolio-level operations including:
//! - Entity and position management
//! - Valuation aggregation across positions
//! - Metrics aggregation with cross-currency support
//! - Attribute-based grouping and analysis
//! - Scenario application
//! - DataFrame exports for analysis
//!
//! # Quick Start
//!
//! ```rust
//! use finstack_portfolio::{PortfolioBuilder, Entity, Position, PositionUnit};
//! use finstack_core::prelude::*;
//! use finstack_valuations::instruments::deposit::Deposit;
//! use std::sync::Arc;
//! use time::macros::date;
//!
//! let as_of = date!(2024-01-01);
//!
//! // Create a deposit instrument
//! let deposit = Deposit::builder()
//!     .id("DEP_1M".into())
//!     .notional(Money::new(1_000_000.0, Currency::USD))
//!     .start(as_of)
//!     .end(date!(2024-02-01))
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
//!  .with_tag("asset_class", "cash");
//!
//! // Build the portfolio with the entity and position
//! let portfolio = PortfolioBuilder::new("MY_FUND")
//!     .base_ccy(Currency::USD)
//!     .as_of(as_of)
//!     .entity(Entity::new("ACME_CORP"))
//!     .position(position)
//!     .build()
//!     .expect("test should succeed");
//! ```

pub mod attribution;
pub mod builder;
#[cfg(feature = "dataframes")]
pub mod dataframe;
pub mod error;
pub mod grouping;
pub mod metrics;
pub mod portfolio;
pub mod position;
pub mod results;
pub mod types;
pub mod valuation;

/// Cashflow ladder and schedule aggregation utilities.
pub mod cashflows;

#[cfg(test)]
mod test_utils;

#[cfg(feature = "scenarios")]
pub mod scenarios;

// Re-export key types
pub use attribution::{attribute_portfolio_pnl, PortfolioAttribution};
pub use builder::PortfolioBuilder;
pub use cashflows::{aggregate_cashflows, PortfolioCashflows};
pub use error::{PortfolioError, Result};
pub use grouping::{aggregate_by_attribute, group_by_attribute};
pub use metrics::{aggregate_metrics, AggregatedMetric, PortfolioMetrics};
pub use portfolio::Portfolio;
pub use position::{Position, PositionUnit};
pub use results::PortfolioResults;
pub use types::{Entity, EntityId, PositionId, DUMMY_ENTITY_ID};
pub use valuation::{value_portfolio, PortfolioValuation, PositionValue};

#[cfg(feature = "scenarios")]
pub use scenarios::{apply_and_revalue, apply_scenario};
