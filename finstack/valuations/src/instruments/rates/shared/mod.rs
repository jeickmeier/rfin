//! Shared rates pricing utilities.

/// Bermudan call provision shared across callable exotic rate products.
pub mod bermudan_call;
/// Cumulative coupon tracker for path-dependent products (TARN, Snowball).
pub mod cumulative_coupon;
/// Forward swap rate and annuity helpers shared by CMS instruments.
pub mod forward_swap_rate;
/// Monte Carlo configuration shared across rate exotic pricers.
pub mod mc_config;
pub use mc_config::RateExoticMcConfig;

/// HW1F parameter resolution with overrides/surface/default precedence.
#[cfg(feature = "mc")]
pub mod hw1f_calibration;
#[cfg(feature = "mc")]
pub use hw1f_calibration::{resolve_hw1f_params, Hw1fResolveRequest};
