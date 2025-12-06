//! Delta calculator for equity options.
//!
//! Computes cash delta using analytical Black–Scholes formula.
//!
//! # Market Standard Formula
//!
//! **Call Delta:** Δ = e^(-qT) × N(d₁)
//!
//! **Put Delta:** Δ = -e^(-qT) × N(-d₁)
//!
//! Where:
//! - q = continuous dividend yield
//! - T = time to expiration (years)
//! - N(·) = cumulative standard normal distribution
//! - d₁ = [ln(S/K) + (r - q + σ²/2)T] / (σ√T)
//!
//! Result is scaled by contract size for cash delta.
//!
//! # Note
//!
//! Delta represents the rate of change of option value with respect to
//! the underlying price. It approximates the hedge ratio and probability
//! of finishing in-the-money under the risk-neutral measure.

use crate::define_metric_calculator;
use crate::instruments::equity_option::EquityOption;

define_metric_calculator!(
    /// Delta calculator for equity options.
    DeltaCalculator,
    instrument = EquityOption,
    calc = |option, ctx| option.delta(&ctx.curves, ctx.as_of)
);
