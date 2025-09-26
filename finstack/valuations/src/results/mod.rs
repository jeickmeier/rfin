//! Result types and helpers for valuations.
//!
//! This module is intentionally minimal and only declares submodules and
//! re-exports their public items for a clean surface.

mod valuation_result;

pub use valuation_result::ValuationResult;
pub use finstack_core::config::ResultsMeta;
