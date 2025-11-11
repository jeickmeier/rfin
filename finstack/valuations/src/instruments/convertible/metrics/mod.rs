//! Convertible Bond metrics module.
//!
//! Provides metric calculators specific to `ConvertibleBond`, split into
//! focused files. The calculators compose with the shared metrics framework
//! and are registered via `register_convertible_metrics`.
//!
//! Exposed metrics:
//! - Parity
//! - Conversion premium
//! - Greeks: Delta, Gamma, Vega, Rho, Theta

mod conversion01;
mod conversion_premium;
mod cs01;
mod dividend_risk;
mod greeks;
mod parity;
// risk_bucketed_dv01 and theta now using generic implementations

use crate::metrics::MetricRegistry;

/// Register convertible bond metrics into the registry.
pub fn register_convertible_metrics(registry: &mut MetricRegistry) {
    use crate::metrics::MetricId;
    use std::sync::Arc;

    // Custom metrics (not in standard MetricId enum)
    registry.register_metric(
        MetricId::custom("parity"),
        Arc::new(parity::ParityCalculator),
        &["ConvertibleBond"],
    );
    registry.register_metric(
        MetricId::custom("conversion_premium"),
        Arc::new(conversion_premium::ConversionPremiumCalculator),
        &["ConvertibleBond"],
    );
    registry.register_metric(
        MetricId::Dividend01,
        Arc::new(dividend_risk::DividendRiskCalculator),
        &["ConvertibleBond"],
    );
    registry.register_metric(
        MetricId::Conversion01,
        Arc::new(conversion01::Conversion01Calculator),
        &["ConvertibleBond"],
    );

    // Standard metrics using macro
    crate::register_metrics! {
        registry: registry,
        instrument: "ConvertibleBond",
        metrics: [
            (Delta, greeks::DeltaCalculator),
            (Gamma, greeks::GammaCalculator),
            (Vega, greeks::VegaCalculator),
            (Rho, greeks::RhoCalculator),
            (Cs01, cs01::Cs01Calculator),
            (Dv01, crate::metrics::GenericParallelDv01::<
                crate::instruments::ConvertibleBond,
            >::default()),
            // Theta is now registered universally in metrics::standard_registry()
            (BucketedDv01, crate::metrics::GenericBucketedDv01WithContext::<
                crate::instruments::ConvertibleBond,
            >::default()),
        ]
    }
}
