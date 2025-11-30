//! Scenario tree infrastructure for stochastic structured credit.
//!
//! This module provides:
//! - **ScenarioNode**: Node state with prepayment, default, and recovery
//! - **ScenarioTree**: Non-recombining tree data structure
//! - **ScenarioTreeConfig**: Tree generation configuration
//!
//! # Design Philosophy
//!
//! We use **non-recombining trees** (accuracy over speed) because:
//! - Path-dependent state (burnout, cumulative prepayments)
//! - Complex correlation structures between prepay and default
//! - Need to preserve full scenario information for risk analysis
//!
//! # Usage
//!
//! ```ignore
//! let config = ScenarioTreeConfig::new(
//!     60,                          // 60 monthly periods
//!     5.0,                         // 5 year horizon
//!     BranchingSpec::Fixed { branches: 3 },
//! );
//!
//! let tree = ScenarioTree::build(&config, prepay_model, default_model, seed)?;
//!
//! // Compute expected value
//! let ev = tree.expected_value(|node| node.pool_balance);
//! ```

mod config;
mod node;
#[allow(clippy::module_inception)]
mod tree;

pub use config::{BranchingSpec, ScenarioTreeConfig};
pub use node::{ScenarioNode, ScenarioNodeId, ScenarioPath};
pub use tree::ScenarioTree;
