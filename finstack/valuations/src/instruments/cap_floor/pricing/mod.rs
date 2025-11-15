//! Interest rate option pricing components.
//!
//! Exposes the pricing entrypoints for `InterestRateOption`. Core pricing
//! logic is now in the instrument struct itself. This module provides
//! Black model helpers and registry integration.

pub mod black;
/// Cap/floor pricer implementation
pub mod pricer;
