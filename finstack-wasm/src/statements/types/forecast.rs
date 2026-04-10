//! Forecast type bindings for statements.

use finstack_statements::types::{ForecastMethod, ForecastSpec, SeasonalMode};
use wasm_bindgen::prelude::*;

/// Seasonal forecast mode.
///
/// Determines how seasonal patterns combine with base values.
#[wasm_bindgen]
#[derive(Clone, Copy, Debug)]
pub struct JsSeasonalMode {
    inner: SeasonalMode,
}

#[wasm_bindgen]
#[allow(non_snake_case)]
impl JsSeasonalMode {
    /// Additive mode: value = base + seasonal_adjustment
    #[wasm_bindgen(getter)]
    pub fn ADDITIVE() -> JsSeasonalMode {
        JsSeasonalMode {
            inner: SeasonalMode::Additive,
        }
    }

    /// Multiplicative mode: value = base * (1 + seasonal_adjustment)
    #[wasm_bindgen(getter)]
    pub fn MULTIPLICATIVE() -> JsSeasonalMode {
        JsSeasonalMode {
            inner: SeasonalMode::Multiplicative,
        }
    }

    /// Convert to string representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!("{:?}", self.inner)
    }
}

impl From<SeasonalMode> for JsSeasonalMode {
    fn from(inner: SeasonalMode) -> Self {
        Self { inner }
    }
}

impl From<JsSeasonalMode> for SeasonalMode {
    fn from(js: JsSeasonalMode) -> Self {
        js.inner
    }
}

/// Forecast method enumeration.
///
/// Defines the available forecasting methods for statement nodes.
#[wasm_bindgen]
#[derive(Clone, Copy, Debug)]
pub struct JsForecastMethod {
    inner: ForecastMethod,
}

#[wasm_bindgen]
#[allow(non_snake_case)]
impl JsForecastMethod {
    /// Forward fill - carry forward the last known value.
    #[wasm_bindgen(getter)]
    pub fn FORWARD_FILL() -> JsForecastMethod {
        JsForecastMethod {
            inner: ForecastMethod::ForwardFill,
        }
    }

    /// Growth percentage - apply constant compound growth rate.
    #[wasm_bindgen(getter)]
    pub fn GROWTH_PCT() -> JsForecastMethod {
        JsForecastMethod {
            inner: ForecastMethod::GrowthPct,
        }
    }

    /// Curve percentage - apply period-specific growth rates.
    #[wasm_bindgen(getter)]
    pub fn CURVE_PCT() -> JsForecastMethod {
        JsForecastMethod {
            inner: ForecastMethod::CurvePct,
        }
    }

    /// Override - sparse period values for specific overrides.
    #[wasm_bindgen(getter)]
    pub fn OVERRIDE() -> JsForecastMethod {
        JsForecastMethod {
            inner: ForecastMethod::Override,
        }
    }

    /// Normal distribution - deterministic sampling with seed.
    #[wasm_bindgen(getter)]
    pub fn NORMAL() -> JsForecastMethod {
        JsForecastMethod {
            inner: ForecastMethod::Normal,
        }
    }

    /// Log-normal distribution - always positive values.
    #[wasm_bindgen(getter)]
    pub fn LOG_NORMAL() -> JsForecastMethod {
        JsForecastMethod {
            inner: ForecastMethod::LogNormal,
        }
    }

    /// Time series - external data reference.
    #[wasm_bindgen(getter)]
    pub fn TIME_SERIES() -> JsForecastMethod {
        JsForecastMethod {
            inner: ForecastMethod::TimeSeries,
        }
    }

    /// Seasonal - patterns with optional growth.
    #[wasm_bindgen(getter)]
    pub fn SEASONAL() -> JsForecastMethod {
        JsForecastMethod {
            inner: ForecastMethod::Seasonal,
        }
    }

    /// Convert to string representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!("{:?}", self.inner)
    }
}

impl From<ForecastMethod> for JsForecastMethod {
    fn from(inner: ForecastMethod) -> Self {
        Self { inner }
    }
}

impl From<JsForecastMethod> for ForecastMethod {
    fn from(js: JsForecastMethod) -> Self {
        js.inner
    }
}

/// Forecast specification.
///
/// Defines how to forecast values for future periods using various methods.
#[wasm_bindgen]
pub struct JsForecastSpec {
    pub(crate) inner: ForecastSpec,
}

#[wasm_bindgen]
impl JsForecastSpec {
    /// Create a new forecast specification from JSON.
    ///
    /// # Arguments
    /// * `value` - JavaScript object representing the forecast spec
    ///
    /// # Returns
    /// Forecast specification instance
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> Result<JsForecastSpec, JsValue> {
        serde_wasm_bindgen::from_value(value)
            .map(|inner| JsForecastSpec { inner })
            .map_err(|e| JsValue::from_str(&format!("Failed to deserialize ForecastSpec: {}", e)))
    }

    /// Convert to JSON representation.
    ///
    /// # Returns
    /// JavaScript object
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize ForecastSpec: {}", e)))
    }

    /// Create a forward fill forecast.
    ///
    /// Carries forward the last known value into future periods.
    ///
    /// # Returns
    /// Forecast specification
    #[wasm_bindgen(js_name = forwardFill)]
    pub fn forward_fill() -> JsForecastSpec {
        JsForecastSpec {
            inner: ForecastSpec::forward_fill(),
        }
    }

    /// Create a growth percentage forecast.
    ///
    /// # Arguments
    /// * `growth_rate` - Annual compound growth rate (e.g., 0.05 for 5%)
    ///
    /// # Returns
    /// Forecast specification
    #[wasm_bindgen(js_name = growth)]
    pub fn growth(growth_rate: f64) -> JsForecastSpec {
        JsForecastSpec {
            inner: ForecastSpec::growth(growth_rate),
        }
    }

    /// Create a curve forecast with period-specific growth rates.
    ///
    /// # Arguments
    /// * `curve` - Array of growth rates for each period
    ///
    /// # Returns
    /// Forecast specification
    #[wasm_bindgen(js_name = curve)]
    pub fn curve(curve: Vec<f64>) -> JsForecastSpec {
        JsForecastSpec {
            inner: ForecastSpec::curve(curve),
        }
    }

    /// Create a normal distribution forecast.
    ///
    /// # Arguments
    /// * `mean` - Mean value
    /// * `std_dev` - Standard deviation
    /// * `seed` - Random seed for deterministic results
    ///
    /// # Returns
    /// Forecast specification
    #[wasm_bindgen(js_name = normal)]
    pub fn normal(mean: f64, std_dev: f64, seed: u64) -> JsForecastSpec {
        JsForecastSpec {
            inner: ForecastSpec::normal(mean, std_dev, seed),
        }
    }

    /// Create a log-normal distribution forecast.
    ///
    /// # Arguments
    /// * `mean` - Mean of underlying normal distribution
    /// * `std_dev` - Standard deviation of underlying normal distribution
    /// * `seed` - Random seed for deterministic results
    ///
    /// # Returns
    /// Forecast specification
    #[wasm_bindgen(js_name = lognormal)]
    pub fn lognormal(mean: f64, std_dev: f64, seed: u64) -> JsForecastSpec {
        JsForecastSpec {
            inner: ForecastSpec::lognormal(mean, std_dev, seed),
        }
    }

    /// Get the forecast method.
    #[wasm_bindgen(getter)]
    pub fn method(&self) -> JsForecastMethod {
        JsForecastMethod::from(self.inner.method)
    }
}
