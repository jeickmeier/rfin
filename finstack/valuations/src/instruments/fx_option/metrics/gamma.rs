//! Gamma calculator for FX options.

use crate::define_metric_calculator;
use crate::instruments::fx_option::FxOption;

define_metric_calculator!(
    /// Gamma calculator for FX options.
    GammaCalculator,
    instrument = FxOption,
    calc = |option, ctx| {
        let greeks = option.compute_greeks(&ctx.curves, ctx.as_of)?;
        Ok(greeks.gamma)
    }
);
