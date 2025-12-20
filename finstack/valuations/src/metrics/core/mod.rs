//! Core metrics framework infrastructure.
//!
//! This module provides the foundational types for the metrics system:
//! - `MetricId`: Strongly-typed metric identifiers
//! - `MetricCalculator`: Trait for implementing metric calculations
//! - `MetricContext`: Context containing instrument and market data
//! - `MetricRegistry`: Registry managing calculators with dependency resolution
//! - `StrictMode`: Control over metric computation error handling (in `registry` module)
//! - Finite difference utilities for numerical derivatives

pub mod finite_difference;
pub mod ids;
pub mod registration_macro;
pub mod registry;
pub mod traits;
