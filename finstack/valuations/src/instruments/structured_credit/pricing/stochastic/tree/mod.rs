//! Scenario tree infrastructure for stochastic structured credit.
//!
//! This module provides:
//! - **ScenarioNode**: Node state with prepayment, default, and recovery
//! - **ScenarioTree**: Recombining lattice representation of scenario paths
//! - **ScenarioTreeConfig**: Tree generation configuration
//!
//! # Design Philosophy
//!
//! The implementation uses a recombining lattice geometry (O(n²) nodes)
//! while still storing path-dependent statistics (burnout, cumulative
//! prepayments/defaults). This provides sufficient accuracy for structured
//! products without the exponential blow-up of a full non-recombining tree.
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
