//! Analytical pricing formulas for validation and benchmarking.
//!
//! This module re-exports analytical formulas from the common models module.
//! The implementation has been moved to `instruments::common::models::closed_form` for broader reuse.

// Re-export from common models closed_form module
pub use crate::instruments::common::models::closed_form::{
    barrier_call_continuous, barrier_put_continuous, bs_call_delta, bs_call_greeks, bs_call_rho,
    bs_call_theta, bs_gamma, bs_put_delta, bs_put_greeks, bs_put_rho, bs_put_theta, bs_vega,
    heston_call_price_fourier, heston_put_price_fourier, up_in_call, up_out_call, BarrierType,
};
