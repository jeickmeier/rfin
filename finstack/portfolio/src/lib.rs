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
//! use finstack_portfolio::{PortfolioBuilder, Entity};
//! use finstack_core::prelude::*;
//! use time::macros::date;
//!
//! // Create a simple portfolio
//! let portfolio = PortfolioBuilder::new("MY_FUND")
//!     .base_ccy(Currency::USD)
//!     .as_of(date!(2024-01-01))
//!     .entity(Entity::new("ACME_CORP"))
//!     .build()
//!     .unwrap();
//!
//! assert_eq!(portfolio.id, "MY_FUND");
//! assert_eq!(portfolio.base_ccy, Currency::USD);
//! ```

#![deny(unsafe_code)]

pub mod builder;
#[cfg(feature = "dataframe")]
pub mod dataframe;
pub mod error;
pub mod grouping;
pub mod metrics;
pub mod portfolio;
pub mod position;
pub mod results;
pub mod types;
pub mod valuation;

#[cfg(feature = "scenarios")]
pub mod scenarios;

// Re-export key types
pub use builder::PortfolioBuilder;
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
