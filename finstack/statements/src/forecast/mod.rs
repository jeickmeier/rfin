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
