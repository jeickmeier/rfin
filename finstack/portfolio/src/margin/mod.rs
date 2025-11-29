//! Portfolio-level margin aggregation.
//!
//! This module provides netting set management and portfolio-wide margin
//! aggregation, building on the instrument-level margin calculations from
//! the valuations crate.
//!
//! # Overview
//!
//! Margin requirements are typically aggregated at the netting set level,
//! where instruments under the same CSA or CCP can offset each other.
//! This module provides:
//!
//! - **Netting Set Management**: Grouping positions by CSA/CCP
//! - **Portfolio Margin Aggregation**: Combining sensitivities across positions
//! - **Margin Reporting**: Summary views of margin requirements
//!
//! # Usage
//!
//! ```rust,ignore
//! use finstack_portfolio::margin::{NettingSetManager, PortfolioMarginAggregator};
//!
//! // Create margin aggregator from portfolio
//! let aggregator = PortfolioMarginAggregator::from_portfolio(&portfolio);
//!
//! // Calculate margin requirements
//! let margin_results = aggregator.calculate(&market, as_of)?;
//!
//! // Get margin by netting set
//! for (netting_set, margin) in margin_results.by_netting_set() {
//!     println!("{}: IM={}, VM={}", netting_set, margin.im, margin.vm);
//! }
//! ```

mod aggregator;
mod netting_set;
mod results;

pub use aggregator::PortfolioMarginAggregator;
pub use netting_set::{NettingSet, NettingSetManager};
pub use results::{NettingSetMargin, PortfolioMarginResult};

