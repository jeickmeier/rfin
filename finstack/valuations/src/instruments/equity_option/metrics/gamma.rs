//! Gamma calculator for equity options.
//!
//! Computes gamma using analytical Black–Scholes formula.
//!
//! # Market Standard Formula
//!
//! Γ = [e^(-qT) × N'(d₁)] / (S × σ × √T)
//!
//! Where:
//! - S = spot price of underlying
//! - σ = volatility
//! - T = time to expiration (years)
//! - q = continuous dividend yield
//! - N'(·) = standard normal probability density function
//! - d₁ = [ln(S/K) + (r - q + σ²/2)T] / (σ√T)
//!
//! Result is scaled by contract size.
//!
//! # Note
//!
//! Gamma represents the rate of change of delta with respect to the
//! underlying price (second derivative of option value). It measures
//! the convexity of the option's value and is highest for at-the-money
//! options near expiration.

use crate::define_metric_calculator;
use crate::instruments::equity_option::EquityOption;

define_metric_calculator!(
    /// Gamma calculator for equity options.
    GammaCalculator,
    instrument = EquityOption,
    calc = |option, ctx| option.gamma(&ctx.curves, ctx.as_of)
);
