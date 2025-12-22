//! Delta calculator for volatility index options.
//!
//! Computes cash delta using analytical Black (1976) formula.
//!
//! # Market Standard Formula
//!
//! **Call Delta:** Δ = DF × N(d₁)
//!
//! **Put Delta:** Δ = DF × (N(d₁) - 1) = -DF × N(-d₁)
//!
//! Where:
//! - DF = discount factor to expiry
//! - N(·) = cumulative standard normal distribution
//! - d₁ = [ln(F/K) + σ²T/2] / (σ√T)
//! - F = forward volatility index level
//!
//! Result is scaled by contract size for cash delta.
//!
//! # Note
//!
//! Delta represents the rate of change of option value with respect to
//! the forward volatility index level. It approximates the hedge ratio and
//! probability of finishing in-the-money under the risk-neutral measure.

use crate::define_metric_calculator;
use crate::instruments::vol_index_option::VolatilityIndexOption;

define_metric_calculator!(
    /// Delta calculator for volatility index options.
    DeltaCalculator,
    instrument = VolatilityIndexOption,
    calc = |option, ctx| option.delta(&ctx.curves)
);
