//! Calibration adapters for different instrument types.
//!
//! This module provides adapters that bridge between the plan-driven calibration API
//! and the domain-specific optimization logic for various financial instruments.
//!
//! # Features
//! - **Standardized Interface**: All adapters implement common traits like [`BootstrapTarget`]
//!   or [`GlobalSolveTarget`].
//! - **Broad Asset Coverage**: Support for Interest Rates (IR), Credit, Inflation, and Volatility.
//! - **Automatic Convention Resolution**: Adapters handle the mapping from high-level
//!   market quotes to concrete pricing inputs.
//!
//! # See Also
//! - [`handlers`] for the orchestration of these adapters during calibration steps.
//! - [`crate::calibration::api`] for the schema that drives these adapters.

/// Base correlation curve bootstrapping from CDS tranche quotes.
pub mod base_correlation;
/// Discount curve bootstrapping from rate quotes.
pub mod discount;
/// Forward curve bootstrapping from rate quotes.
pub mod forward;
/// Calibration step execution handlers.
pub mod handlers;
/// Hazard curve bootstrapping from CDS quotes.
pub mod hazard;
/// Inflation curve bootstrapping from inflation swap quotes.
pub mod inflation;
/// Swaption volatility surface calibration.
pub mod swaption;
/// Option volatility surface calibration.
pub mod vol;
