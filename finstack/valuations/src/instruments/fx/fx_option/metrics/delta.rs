//! Delta calculator for FX options.
//!
//! Computes cash delta using Garman–Kohlhagen greeks from the pricing engine.

use crate::define_metric_calculator;
use crate::instruments::fx_option::FxOption;

define_metric_calculator!(
    /// Delta calculator for FX options.
    DeltaCalculator,
    instrument = FxOption,
    calc = |option, ctx| {
        let greeks = option.compute_greeks(&ctx.curves, ctx.as_of)?;
        Ok(greeks.delta)
    }
);
