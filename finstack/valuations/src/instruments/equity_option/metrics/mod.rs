//! Equity option metrics module.
//!
//! Splits equity option metrics into focused calculators per greek and
//! registers them with the `MetricRegistry`. Calculators reuse the pricing
//! engine helpers to ensure consistency between PV and greeks.

mod charm;
mod color;
mod delta;
mod dividend_risk;
mod gamma;
mod implied_vol;
mod rho;
mod speed;
mod theta;
mod vanna;
mod vega;
mod volga;

use crate::metrics::MetricRegistry;

/// Register equity option metrics with the registry.
pub fn register_equity_option_metrics(registry: &mut MetricRegistry) {
    use crate::metrics::MetricId;
    use std::sync::Arc;

    // Custom metric: Dividend risk (dividend yield sensitivity per 1bp)
    registry.register_metric(
        MetricId::Dividend01,
        Arc::new(dividend_risk::DividendRiskCalculator),
        &["EquityOption"],
    );

    crate::register_metrics! {
        registry: registry,
        instrument: "EquityOption",
        metrics: [
            (Delta, delta::DeltaCalculator),
            (Gamma, gamma::GammaCalculator),
            (Vega, vega::VegaCalculator),
            (BucketedVega, crate::metrics::KeyRateVega::<
                crate::instruments::EquityOption,
            >::standard()),
            (Dv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::EquityOption,
            >::new(crate::metrics::Dv01CalculatorConfig::parallel_combined())),
            (BucketedDv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::EquityOption,
            >::new(crate::metrics::Dv01CalculatorConfig::key_rate())),
            (Theta, theta::ThetaCalculator),
            (Rho, rho::RhoCalculator),
            (ImpliedVol, implied_vol::ImpliedVolCalculator),
            (Vanna, vanna::VannaCalculator),
            (Volga, volga::VolgaCalculator),
            (Charm, charm::CharmCalculator),
            (Color, color::ColorCalculator),
            (Speed, speed::SpeedCalculator),
        ]
    }
}
