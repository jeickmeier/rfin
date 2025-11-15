//! Sensitivity metrics and risk calculators.
//!
//! This module provides sensitivity metrics for interest rate risk (DV01),
//! credit spread risk (CS01), volatility risk (Vega), time decay (Theta),
//! and option Greeks (Delta, Gamma, Vanna, Volga).
//!
//! All bucketed sensitivities support parallel and key-rate analysis.

pub mod cs01;
pub mod dv01;
pub mod dv01_unified;
pub mod fd_greeks;
pub mod shock_mode;
pub mod theta;
pub mod utils;
pub mod vega;
pub mod vol;

#[cfg(test)]
mod tests;
