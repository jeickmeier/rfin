//! Sensitivity metrics and risk calculators.
//!
//! This module provides sensitivity metrics for interest rate risk (DV01),
//! credit spread risk (CS01), volatility risk (Vega), time decay (Theta),
//! and option Greeks (Delta, Gamma, Vanna, Volga).
//!
//! All bucketed sensitivities support parallel and key-rate analysis.

pub mod cs01;
pub mod dv01;
pub mod fd_greeks;
pub mod theta;
pub mod vega;

#[cfg(test)]
mod tests;
