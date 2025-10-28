//! Analytical pricing formulas for validation and benchmarking.
//!
//! This module provides closed-form and semi-analytical pricing formulas
//! used to validate Monte Carlo implementations.

pub mod heston_fourier;
pub mod black_scholes_greeks;
pub mod barrier_continuous;

pub use heston_fourier::{heston_call_price_fourier, heston_put_price_fourier};
pub use black_scholes_greeks::{
    bs_call_delta, bs_put_delta, bs_gamma, bs_vega, 
    bs_call_theta, bs_put_theta, bs_call_rho, bs_put_rho,
    bs_call_greeks, bs_put_greeks,
};
pub use barrier_continuous::{
    barrier_call_continuous, barrier_put_continuous,
    up_in_call, up_out_call,
};

