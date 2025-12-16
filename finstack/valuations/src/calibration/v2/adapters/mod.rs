//! Calibration adapters for different instrument types.
//!
//! This module provides adapters that bridge between the calibration API
//! and domain-specific bootstrapping logic for various financial instruments.
//! Each adapter implements the [`BootstrapTarget`] trait to enable sequential
//! bootstrapping of term structures.

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
