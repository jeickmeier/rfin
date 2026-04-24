//! Bond pricing engines.
//!
//! Each engine implements the core pricing math for a specific model:
//!
//! - [`self::discount`]: PV = sum(CF_i * DF_i) using discount curves
//! - [`self::hazard`]: Survival-weighted PV + fractional recovery of par (FRP)
//! - [`self::tree`]: Binomial/trinomial tree for callable/putable bonds and OAS
//! - `merton_mc`: Merton structural credit Monte Carlo for PIK bonds (when `mc` feature is enabled)

/// Discount curve-based bond pricing (PV = sum CF_i * DF_i).
pub mod discount;
/// Hazard-rate pricing with fractional recovery of par (FRP).
pub mod hazard;
/// Merton structural credit Monte Carlo for PIK bonds.
pub mod merton_mc;
/// Binomial tree pricing for callable/putable bonds and OAS.
pub mod tree;
