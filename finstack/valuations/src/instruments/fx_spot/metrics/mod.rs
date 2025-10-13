//! FX Spot metrics module.
//!
//! Provides metric calculators specific to `FxSpot`, split into focused files
//! to mirror the repository-wide metrics organization used by more complex
//! instruments (e.g., `cds`).
//!
//! Exposed metrics via `MetricId::custom("...")` under the instrument type
//! "FxSpot":
//! - `spot_rate`
//! - `base_amount`
//! - `quote_amount`
//! - `inverse_rate`

pub mod base_amount;
pub mod dv01;
pub mod inverse_rate;
pub mod quote_amount;
pub mod spot_rate;
pub mod theta;

use crate::metrics::MetricRegistry;

/// Register all FX Spot metrics with the registry
pub fn register_fx_spot_metrics(registry: &mut MetricRegistry) {
    crate::register_metrics! {
        registry: registry,
        instrument: "FxSpot",
        metrics: [
            (SpotRate, spot_rate::SpotRateCalculator),
            (BaseAmount, base_amount::BaseAmountCalculator),
            (QuoteAmount, quote_amount::QuoteAmountCalculator),
            (InverseRate, inverse_rate::InverseRateCalculator),
            (Dv01, dv01::FxSpotDv01Calculator),
            (Theta, theta::ThetaCalculator),
        ]
    };
}
