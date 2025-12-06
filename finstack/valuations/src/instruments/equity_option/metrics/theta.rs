//! Analytical theta calculator for equity options.
//!
//! Uses the closed-form Black-Scholes theta formula for European options.

use crate::define_metric_calculator;
use crate::instruments::equity_option::EquityOption;

define_metric_calculator!(
    /// Analytical theta calculator using Black-Scholes formula.
    ThetaCalculator,
    instrument = EquityOption,
    calc = |option, ctx| option.theta(&ctx.curves, ctx.as_of)
);
