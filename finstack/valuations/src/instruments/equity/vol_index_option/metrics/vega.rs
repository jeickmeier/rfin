//! Vega calculator for volatility index options.
//!
//! Computes vega (sensitivity to volatility of volatility) using
//! analytical Black (1976) formula.
//!
//! # Market Standard Formula
//!
//! **Vega:** ν = DF × F × √T × n(d₁)
//!
//! Where:
//! - DF = discount factor to expiry
//! - F = forward volatility index level
//! - T = time to expiration (years)
//! - n(·) = standard normal probability density function
//! - d₁ = [ln(F/K) + σ²T/2] / (σ√T)
//!
//! Result is scaled by contract size.
//!
//! # Note
//!
//! Vega represents the sensitivity of option value to changes in the
//! volatility of the underlying volatility index (volatility of volatility).
//! This is a critical risk factor for volatility index options.

use crate::define_metric_calculator;
use crate::instruments::vol_index_option::VolatilityIndexOption;

define_metric_calculator!(
    /// Vega calculator for volatility index options.
    VegaCalculator,
    instrument = VolatilityIndexOption,
    calc = |option, ctx| option.vega(&ctx.curves, ctx.as_of)
);
