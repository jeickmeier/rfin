//! Vega calculator for equity options.
//!
//! Computes vega using analytical Black–Scholes formula.
//!
//! # Market Standard Formula
//!
//! ν = (S × e^(-qT) × N'(d₁) × √T) / 100
//!
//! Where:
//! - S = spot price of underlying
//! - T = time to expiration (years)
//! - q = continuous dividend yield
//! - N'(·) = standard normal probability density function
//! - d₁ = [ln(S/K) + (r - q + σ²/2)T] / (σ√T)
//!
//! Result is per 1% (0.01) volatility move and scaled by contract size.
//!
//! # Note
//!
//! Vega represents the rate of change of option value with respect to
//! volatility. It is highest for at-the-money options with longer time
//! to expiration. Vega is the same for calls and puts with the same
//! strike and expiration.

use crate::define_metric_calculator;
use crate::instruments::equity_option::EquityOption;

define_metric_calculator!(
    /// Vega calculator for equity options.
    VegaCalculator,
    instrument = EquityOption,
    calc = |option, ctx| option.vega(&ctx.curves, ctx.as_of)
);
