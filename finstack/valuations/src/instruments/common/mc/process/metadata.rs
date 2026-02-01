//! Process metadata extraction for Monte Carlo simulations.
//!
//! This module provides a trait for extracting process parameters and metadata
//! that can be included in captured path datasets for visualization and analysis.

use crate::instruments::common_impl::mc::paths::ProcessParams;

/// Trait for extracting metadata from stochastic processes.
///
/// Implementations should provide a snapshot of the process configuration
/// including parameters, correlation matrices, and factor names.
pub trait ProcessMetadata {
    /// Extract process parameters and metadata.
    fn metadata(&self) -> ProcessParams;
}
