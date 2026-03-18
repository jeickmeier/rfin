//! Equity option metrics module.
//!
//! Splits equity option metrics into focused calculators per greek and
//! registers them with the `MetricRegistry`. Calculators reuse the pricing
//! engine helpers to ensure consistency between PV and greeks.

mod charm;
mod color;
mod dividend_risk;
mod implied_vol;
mod speed;

use crate::metrics::MetricRegistry;

/// Register equity option metrics with the registry.
pub fn register_equity_option_metrics(registry: &mut MetricRegistry) {
    use crate::metrics::sensitivities::cross_factor::{
        CrossFactorCalculator, CrossFactorPair, SpotBumperFactory, VolBumperFactory,
    };
    use crate::metrics::MetricId;
    use crate::pricer::InstrumentType;
    use std::sync::Arc;

    // Custom metric: Dividend risk (dividend yield sensitivity per 1bp)
    registry.register_metric(
        MetricId::Dividend01,
        Arc::new(dividend_risk::DividendRiskCalculator),
        &[InstrumentType::EquityOption],
    );
    registry.register_metric(
        MetricId::CrossGammaSpotVol,
        Arc::new(CrossFactorCalculator::new(
            CrossFactorPair::SpotVol,
            Arc::new(SpotBumperFactory),
            Arc::new(VolBumperFactory),
        )),
        &[InstrumentType::EquityOption],
    );

    crate::register_metrics! {
        registry: registry,
        instrument: InstrumentType::EquityOption,
        metrics: [
            (Delta, crate::metrics::OptionDeltaCalculator::<crate::instruments::EquityOption>::default()),
            (Gamma, crate::metrics::OptionGammaCalculator::<crate::instruments::EquityOption>::default()),
            (Vega, crate::metrics::OptionVegaCalculator::<crate::instruments::EquityOption>::default()),
            (BucketedVega, crate::metrics::KeyRateVega::<
                crate::instruments::EquityOption,
            >::standard()),
            (Dv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::EquityOption,
            >::new(crate::metrics::Dv01CalculatorConfig::parallel_combined())),
            (BucketedDv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::EquityOption,
            >::new(crate::metrics::Dv01CalculatorConfig::triangular_key_rate())),
            (Theta, crate::metrics::OptionThetaCalculator::<crate::instruments::EquityOption>::default()),
            (Rho, crate::metrics::OptionRhoCalculator::<crate::instruments::EquityOption>::default()),
            (ImpliedVol, implied_vol::ImpliedVolCalculator),
            (Vanna, crate::metrics::OptionVannaCalculator::<crate::instruments::EquityOption>::default()),
            (Volga, crate::metrics::OptionVolgaCalculator::<crate::instruments::EquityOption>::default()),
            (Charm, charm::CharmCalculator),
            (Color, color::ColorCalculator),
            (Speed, speed::SpeedCalculator),
        ]
    }
}
