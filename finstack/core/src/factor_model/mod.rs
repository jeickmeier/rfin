//! Factor-model primitives for portfolio risk decomposition.
//!
//! **Naming note:** This module defines *market-risk factor models* used for
//! sensitivity-based portfolio risk decomposition. The latent-variable factor
//! models used for correlated default/prepayment simulation live in the
//! `finstack_correlation::factor_model` module (sibling crate).
//!
//! This module provides the lightweight building blocks needed to map market
//! objects onto risk factors, attach covariance data, and choose how a pricing
//! engine turns factor shocks into portfolio risk measures.
//!
//! # Workflow
//!
//! A typical factor-model workflow is:
//!
//! 1. Describe factors with [`crate::factor_model::FactorDefinition`] and
//!    [`crate::factor_model::FactorType`].
//! 2. Express dependencies such as curves, surfaces, or scalars with
//!    [`crate::factor_model::MarketDependency`].
//! 3. Map dependencies to factors with one of the matcher implementations in
//!    [`crate::factor_model::matching`].
//! 4. Supply a validated covariance matrix via
//!    [`crate::factor_model::FactorCovarianceMatrix`].
//! 5. Choose risk interpretation and repricing behavior with
//!    [`crate::factor_model::FactorModelConfig`],
//!    [`crate::factor_model::RiskMeasure`], and
//!    [`crate::factor_model::PricingMode`].
//!
//! # Design Notes
//!
//! - Matching stays deterministic and configuration-driven.
//! - This module defines factor-model metadata and matching primitives; actual
//!   portfolio aggregation lives in downstream crates.
//! - The public types are serialization-friendly so risk configurations can be
//!   persisted and reused across pricing runs.
//!
//! # References
//!
//! - Factor-model and covariance conventions:
//!   `docs/REFERENCES.md#meucci-risk-and-asset-allocation`
//! - VaR / Expected Shortfall interpretation:
//!   `docs/REFERENCES.md#mcneil-frey-embrechts-qrm`

mod config;
mod covariance;
mod definition;
mod dependency;
mod error;
/// Matching primitives and built-in matcher components.
pub mod matching;
mod types;

pub use config::{BumpSizeConfig, FactorModelConfig, PricingMode, RiskMeasure};
pub use covariance::FactorCovarianceMatrix;
pub use definition::{FactorDefinition, MarketMapping};
pub use dependency::{CurveType, DependencyType, MarketDependency};
pub use error::{FactorModelError, UnmatchedPolicy};
pub use matching::{
    AttributeFilter, CascadeMatcher, DependencyFilter, FactorMatcher, FactorNode,
    HierarchicalConfig, HierarchicalMatcher, MappingRule, MappingTableMatcher, MatchingConfig,
};
pub use types::{FactorId, FactorType};
