//! Multi-curve framework for interest rate derivatives.
//!
//! Post-2008 crisis, interest rate markets require separate curves for:
//! - **OIS (Overnight Index Swap)**: Risk-free discounting
//! - **IBOR/SOFR**: Forward rate projection for different tenors
//! - **Tenor Basis**: Spreads between different IBOR tenors (e.g., 3M vs 6M)
//!
//! This module provides the infrastructure for multi-curve Monte Carlo pricing.

pub mod context;

pub use context::{MultiCurveContext, Tenor, TenorBasis};

