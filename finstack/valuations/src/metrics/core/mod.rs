//! Core metrics framework infrastructure.
//!
//! This module provides the foundational types for the metrics system:
//! - `MetricId`: Strongly-typed metric identifiers
//! - `MetricCalculator`: Trait for implementing metric calculations
//! - `MetricContext`: Context containing instrument and market data
//! - `MetricRegistry`: Registry managing calculators with dependency resolution
//! - Finite difference utilities for numerical derivatives
//! - Trait helpers for instrument capabilities

pub mod finite_difference;
pub mod has_equity_underlying;
pub mod has_pricing_overrides;
pub mod ids;
pub mod registration_macro;
pub mod registry;
pub mod traits;
