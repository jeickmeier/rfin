//! Embedded JSON convention loaders.

/// Bond convention loader.
pub mod bond;
/// CDS convention loader.
pub mod cds;
/// FX convention loader.
pub mod fx;
/// FX option convention loader.
pub mod fx_option;
/// Inflation Swap convention loader.
pub mod inflation_swap;
/// Interest Rate Future convention loader.
pub mod ir_future;
/// Generic JSON loader.
pub mod json;
/// Option convention loader.
pub mod option;
/// Rate index convention loader.
pub mod rate_index;
/// Swaption convention loader.
pub mod swaption;
/// Cross-currency swap convention loader.
pub mod xccy;
