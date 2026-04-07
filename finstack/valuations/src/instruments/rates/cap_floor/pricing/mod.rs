//! Interest rate option pricing components.
//!
//! Exposes the pricing entrypoints for `InterestRateOption`. Core pricing
//! logic is now in the instrument struct itself. This module provides
//! Black model helpers and registry integration.

pub(crate) mod black;
pub(crate) mod normal;
/// Cap/floor pricer implementation
pub(crate) mod pricer;
