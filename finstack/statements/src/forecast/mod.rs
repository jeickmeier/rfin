//! Forecast methods for time-series projection.
//!
//! This module provides various forecast methods for projecting values into
//! future periods, including:
//! - **Deterministic**: ForwardFill, GrowthPct, CurvePct, Override
//! - **Statistical**: Normal, LogNormal (with deterministic seeding)
//! - **TimeSeries**: Seasonal patterns
//!
//! All forecast methods operate on a base value (typically the last actual value)
//! and project forward for a specified number of periods.
//!
//! # Random Number Generation
//!
//! Statistical forecast methods (Normal, LogNormal) require a `seed` parameter
//! for deterministic random number generation. This ensures reproducibility:
//! - Same seed → identical forecast values across runs
//! - Different seeds → different (but still deterministic) values
//!
//! The RNG uses the Box-Muller transform for normal distribution sampling,
//! with guards against edge cases (e.g., ln(0)).
//!
//! # Parameter Validation
//!
//! - **std_dev**: Must be non-negative. Zero produces a degenerate distribution.
//! - **rate** (GrowthPct): Rates > 100% per period produce warnings.
//! - **seed**: Required for statistical methods (ensures reproducibility).
//!
//! # Overflow Protection
//!
//! Compound growth methods (GrowthPct, CurvePct) detect and error on overflow
//! conditions to prevent silent numerical failures.
//!
//! # Warnings
//!
//! The following conditions produce log warnings (but not errors):
//! - Growth rates exceeding 100% per period
//! - std_dev = 0.0 in LogNormal (degenerate distribution)
//!
//! For forecast analysis tools (backtesting, covenant breach detection), see
//! [`crate::analysis::backtesting`] and [`crate::analysis::covenants`].

mod deterministic;
mod override_method;
mod statistical;
mod timeseries;

pub use deterministic::{curve_pct, forward_fill, growth_pct};
pub use override_method::apply_override;
pub use statistical::{lognormal_forecast, normal_forecast};
pub use timeseries::{seasonal_forecast, timeseries_forecast};

use crate::error::Result;
use crate::types::ForecastSpec;
use finstack_core::dates::PeriodId;

/// Apply a forecast method to generate values for forecast periods.
///
/// # Arguments
///
/// * `spec` - Forecast specification with method and parameters
/// * `base_value` - Starting value (typically last actual value)
/// * `forecast_periods` - List of periods to forecast
///
/// # Returns
///
/// Map of period_id → forecasted value
pub fn apply_forecast(
    spec: &ForecastSpec,
    base_value: f64,
    forecast_periods: &[PeriodId],
) -> Result<indexmap::IndexMap<PeriodId, f64>> {
    use crate::types::ForecastMethod;

    match spec.method {
        ForecastMethod::ForwardFill => forward_fill(base_value, forecast_periods),
        ForecastMethod::GrowthPct => growth_pct(base_value, forecast_periods, &spec.params),
        ForecastMethod::CurvePct => curve_pct(base_value, forecast_periods, &spec.params),
        ForecastMethod::Override => apply_override(base_value, forecast_periods, &spec.params),
        ForecastMethod::Normal => normal_forecast(base_value, forecast_periods, &spec.params),
        ForecastMethod::LogNormal => lognormal_forecast(base_value, forecast_periods, &spec.params),
        ForecastMethod::TimeSeries => {
            timeseries_forecast(base_value, forecast_periods, &spec.params)
        }
        ForecastMethod::Seasonal => seasonal_forecast(base_value, forecast_periods, &spec.params),
    }
}

/// Apply a forecast method with an additional seed offset for statistical
/// methods.
///
/// This is used by Monte Carlo evaluation to derive independent, but still
/// deterministic, per-path seeds from the base seed configured in the
/// [`ForecastSpec`].
pub(crate) fn apply_forecast_with_seed_offset(
    spec: &ForecastSpec,
    base_value: f64,
    forecast_periods: &[PeriodId],
    seed_offset: u64,
) -> Result<indexmap::IndexMap<PeriodId, f64>> {
    use crate::types::ForecastMethod;

    match spec.method {
        ForecastMethod::Normal => {
            // Clone params so we can override the seed without mutating the spec.
            let mut params = spec.params.clone();
            if let Some(seed_val) = params.get_mut("seed") {
                if let Some(seed) = seed_val.as_u64() {
                    let effective_seed = seed ^ seed_offset;
                    *seed_val = serde_json::json!(effective_seed);
                }
            }
            normal_forecast(base_value, forecast_periods, &params)
        }
        ForecastMethod::LogNormal => {
            let mut params = spec.params.clone();
            if let Some(seed_val) = params.get_mut("seed") {
                if let Some(seed) = seed_val.as_u64() {
                    let effective_seed = seed ^ seed_offset;
                    *seed_val = serde_json::json!(effective_seed);
                }
            }
            lognormal_forecast(base_value, forecast_periods, &params)
        }
        // Deterministic methods ignore the seed offset and reuse the
        // standard apply_forecast implementation.
        _ => apply_forecast(spec, base_value, forecast_periods),
    }
}

