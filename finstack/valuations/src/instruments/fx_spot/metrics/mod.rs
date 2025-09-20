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
pub mod inverse_rate;
pub mod quote_amount;
pub mod spot_rate;

use crate::metrics::MetricRegistry;

/// Register all FX Spot metrics with the registry
pub fn register_fx_spot_metrics(registry: &mut MetricRegistry) {
    use crate::metrics::MetricId;
    use std::sync::Arc;

    registry
        .register_metric(
            MetricId::custom("spot_rate"),
            Arc::new(spot_rate::SpotRateCalculator),
            &["FxSpot"],
        )
        .register_metric(
            MetricId::custom("base_amount"),
            Arc::new(base_amount::BaseAmountCalculator),
            &["FxSpot"],
        )
        .register_metric(
            MetricId::custom("quote_amount"),
            Arc::new(quote_amount::QuoteAmountCalculator),
            &["FxSpot"],
        )
        .register_metric(
            MetricId::custom("inverse_rate"),
            Arc::new(inverse_rate::InverseRateCalculator),
            &["FxSpot"],
        );
}
