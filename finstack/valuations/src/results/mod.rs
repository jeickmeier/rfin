//! Result types and helpers for valuations.
//!
//! This module is intentionally minimal and only declares submodules and
//! re-exports their public items for a clean surface.

pub mod dataframe;
mod valuation_result;

pub use finstack_core::config::ResultsMeta;
pub use valuation_result::ValuationResult;
