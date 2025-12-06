//! Rho calculators for FX options (per 1bp).

use crate::define_metric_calculator;
use crate::instruments::fx_option::FxOption;

define_metric_calculator!(
    /// Domestic rho calculator for FX options (per bp).
    RhoDomesticCalculator,
    instrument = FxOption,
    calc = |option, ctx| {
        let greeks = option.compute_greeks(&ctx.curves, ctx.as_of)?;
        Ok(greeks.rho_domestic / 100.0)
    }
);

define_metric_calculator!(
    /// Foreign rho calculator for FX options (per bp).
    RhoForeignCalculator,
    instrument = FxOption,
    calc = |option, ctx| {
        let greeks = option.compute_greeks(&ctx.curves, ctx.as_of)?;
        Ok(greeks.rho_foreign / 100.0)
    }
);
