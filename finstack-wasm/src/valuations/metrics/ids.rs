//! Metric identifier bindings for WASM.
//!
//! Provides strongly-typed identifiers for financial metrics like present value,
//! duration, Greeks, and risk sensitivities.

use finstack_valuations::metrics::MetricId;
use js_sys::Array;
use wasm_bindgen::prelude::*;

/// Strongly-typed metric identifier.
///
/// Represents financial metrics like present value, DV01, duration, Greeks, etc.
/// Can be created from string names or using typed constructors.
///
/// @example
/// ```typescript
/// // From string
/// const pv = MetricId.fromName("pv");
///
/// // Using typed constructor
/// const dv01 = MetricId.DV01();
///
/// // Get all standard names
/// const metrics = MetricId.standardNames();
/// ```
#[wasm_bindgen(js_name = MetricId)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct JsMetricId {
    inner: MetricId,
}

impl JsMetricId {
    pub(crate) fn from_inner(inner: MetricId) -> Self {
        Self { inner }
    }

    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> MetricId {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = MetricId)]
impl JsMetricId {
    /// Parse a metric ID from a string name (permissive).
    ///
    /// **Warning**: This method accepts unknown metric names and creates custom metrics.
    /// For strict validation of user inputs, use `parseStrict()` instead.
    ///
    /// @param {string} name - Metric name like "pv", "dv01", "duration_modified"
    /// @returns {MetricId} Parsed metric identifier
    ///
    /// @example
    /// ```typescript
    /// // Known metrics:
    /// const pv = MetricId.fromName("pv");
    /// const dv01 = MetricId.fromName("dv01");
    ///
    /// // Unknown names create custom metrics (permissive):
    /// const custom = MetricId.fromName("my_custom_metric");
    /// console.log(custom.name); // "my_custom_metric"
    /// ```
    #[wasm_bindgen(js_name = fromName)]
    pub fn from_name(name: &str) -> JsMetricId {
        JsMetricId::from_inner(name.parse().unwrap_or_else(|_| MetricId::custom(name)))
    }

    /// Parse a metric ID from a string name strictly.
    ///
    /// This method validates that the metric name is a known standard metric.
    /// Use this for user inputs, configuration files, and external APIs where
    /// unknown metrics should be rejected with a clear error.
    ///
    /// @param {string} name - Metric name like "pv", "dv01", "duration_modified"
    /// @returns {MetricId} Parsed metric identifier
    /// @throws {Error} If the metric name is not a known standard metric.
    ///                 The error includes a list of all available metrics.
    ///
    /// @example
    /// ```typescript
    /// // Known metrics parse successfully:
    /// const dv01 = MetricId.parseStrict("dv01");
    /// console.log(dv01.name); // "dv01"
    ///
    /// // Unknown metrics throw an error:
    /// try {
    ///   MetricId.parseStrict("unknown_metric");
    /// } catch (error) {
    ///   console.error("Invalid metric:", error.message);
    ///   // Error message includes list of available metrics
    /// }
    ///
    /// // Migration from fromName:
    /// // OLD (permissive):
    /// const metric = MetricId.fromName(userInput);
    /// // NEW (strict - recommended for user inputs):
    /// const metric = MetricId.parseStrict(userInput);
    /// ```
    #[wasm_bindgen(js_name = parseStrict)]
    pub fn parse_strict(name: &str) -> Result<JsMetricId, JsValue> {
        MetricId::parse_strict(name)
            .map(JsMetricId::from_inner)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Get the string name of the metric.
    ///
    /// @returns {string} Metric name in snake_case
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        self.inner.as_str().to_string()
    }

    /// Get all standard metric names.
    ///
    /// @returns {Array<string>} Array of all built-in metric identifiers
    ///
    /// @example
    /// ```typescript
    /// const names = MetricId.standardNames();
    /// console.log(names.includes("pv")); // true
    /// ```
    #[wasm_bindgen(js_name = standardNames)]
    pub fn standard_names() -> Array {
        let names = Array::new();
        for metric in MetricId::ALL_STANDARD.iter() {
            names.push(&JsValue::from_str(metric.as_str()));
        }
        names
    }

    // ========================================================================
    // Common Metrics
    // ========================================================================

    // Note: No standalone PV/NPV - use instrument-specific PV metrics like PvFixed, PvFloat, etc.

    // ========================================================================
    // Bond Metrics
    // ========================================================================

    /// Yield to maturity metric.
    #[wasm_bindgen(js_name = YTM)]
    pub fn ytm() -> JsMetricId {
        JsMetricId::from_inner(MetricId::Ytm)
    }

    /// Yield to worst metric.
    #[wasm_bindgen(js_name = YTW)]
    pub fn ytw() -> JsMetricId {
        JsMetricId::from_inner(MetricId::Ytw)
    }

    /// Modified duration metric.
    #[wasm_bindgen(js_name = DurationModified)]
    pub fn duration_modified() -> JsMetricId {
        JsMetricId::from_inner(MetricId::DurationMod)
    }

    /// Macaulay duration metric.
    #[wasm_bindgen(js_name = DurationMacaulay)]
    pub fn duration_macaulay() -> JsMetricId {
        JsMetricId::from_inner(MetricId::DurationMac)
    }

    /// Convexity metric.
    #[wasm_bindgen(js_name = Convexity)]
    pub fn convexity() -> JsMetricId {
        JsMetricId::from_inner(MetricId::Convexity)
    }

    /// Accrued interest metric.
    #[wasm_bindgen(js_name = Accrued)]
    pub fn accrued() -> JsMetricId {
        JsMetricId::from_inner(MetricId::Accrued)
    }

    /// Clean price metric.
    #[wasm_bindgen(js_name = CleanPrice)]
    pub fn clean_price() -> JsMetricId {
        JsMetricId::from_inner(MetricId::CleanPrice)
    }

    /// Dirty price metric.
    #[wasm_bindgen(js_name = DirtyPrice)]
    pub fn dirty_price() -> JsMetricId {
        JsMetricId::from_inner(MetricId::DirtyPrice)
    }

    /// Z-spread metric (zero-volatility spread).
    #[wasm_bindgen(js_name = ZSpread)]
    pub fn z_spread() -> JsMetricId {
        JsMetricId::from_inner(MetricId::ZSpread)
    }

    /// I-spread metric (interpolated spread).
    #[wasm_bindgen(js_name = ISpread)]
    pub fn i_spread() -> JsMetricId {
        JsMetricId::from_inner(MetricId::ISpread)
    }

    /// Option-adjusted spread metric.
    #[wasm_bindgen(js_name = OAS)]
    pub fn oas() -> JsMetricId {
        JsMetricId::from_inner(MetricId::Oas)
    }

    /// Asset swap par spread metric.
    #[wasm_bindgen(js_name = ASW)]
    pub fn asw() -> JsMetricId {
        JsMetricId::from_inner(MetricId::ASWPar)
    }

    /// Discount margin metric.
    #[wasm_bindgen(js_name = DM)]
    pub fn dm() -> JsMetricId {
        JsMetricId::from_inner(MetricId::DiscountMargin)
    }

    // ========================================================================
    // Risk Metrics
    // ========================================================================

    /// DV01 (dollar value of 1 basis point) metric.
    #[wasm_bindgen(js_name = DV01)]
    pub fn dv01() -> JsMetricId {
        JsMetricId::from_inner(MetricId::Dv01)
    }

    /// Credit spread sensitivity (CS01).
    #[wasm_bindgen(js_name = CS01)]
    pub fn cs01() -> JsMetricId {
        JsMetricId::from_inner(MetricId::Cs01)
    }

    /// Bucketed DV01 (key-rate durations).
    #[wasm_bindgen(js_name = BucketedDV01)]
    pub fn bucketed_dv01() -> JsMetricId {
        JsMetricId::from_inner(MetricId::BucketedDv01)
    }

    // ========================================================================
    // Options Greeks (First Order)
    // ========================================================================

    /// Delta (sensitivity to underlying price).
    #[wasm_bindgen(js_name = Delta)]
    pub fn delta() -> JsMetricId {
        JsMetricId::from_inner(MetricId::Delta)
    }

    /// Vega (sensitivity to volatility).
    #[wasm_bindgen(js_name = Vega)]
    pub fn vega() -> JsMetricId {
        JsMetricId::from_inner(MetricId::Vega)
    }

    /// Theta (time decay).
    #[wasm_bindgen(js_name = Theta)]
    pub fn theta() -> JsMetricId {
        JsMetricId::from_inner(MetricId::Theta)
    }

    /// Rho (sensitivity to interest rates).
    #[wasm_bindgen(js_name = Rho)]
    pub fn rho() -> JsMetricId {
        JsMetricId::from_inner(MetricId::Rho)
    }

    // ========================================================================
    // Options Greeks (Second Order)
    // ========================================================================

    /// Gamma (sensitivity of delta to underlying).
    #[wasm_bindgen(js_name = Gamma)]
    pub fn gamma() -> JsMetricId {
        JsMetricId::from_inner(MetricId::Gamma)
    }

    /// Vanna (delta sensitivity to volatility).
    #[wasm_bindgen(js_name = Vanna)]
    pub fn vanna() -> JsMetricId {
        JsMetricId::from_inner(MetricId::Vanna)
    }

    /// Volga (vega sensitivity to volatility).
    #[wasm_bindgen(js_name = Volga)]
    pub fn volga() -> JsMetricId {
        JsMetricId::from_inner(MetricId::Volga)
    }

    /// Veta (vega sensitivity to time).
    #[wasm_bindgen(js_name = Veta)]
    pub fn veta() -> JsMetricId {
        JsMetricId::from_inner(MetricId::Veta)
    }

    // ========================================================================
    // Options Greeks (Third Order)
    // ========================================================================

    /// Charm (delta sensitivity to time).
    #[wasm_bindgen(js_name = Charm)]
    pub fn charm() -> JsMetricId {
        JsMetricId::from_inner(MetricId::Charm)
    }

    /// Color (gamma sensitivity to time).
    #[wasm_bindgen(js_name = Color)]
    pub fn color() -> JsMetricId {
        JsMetricId::from_inner(MetricId::Color)
    }

    /// Speed (gamma sensitivity to underlying).
    #[wasm_bindgen(js_name = Speed)]
    pub fn speed() -> JsMetricId {
        JsMetricId::from_inner(MetricId::Speed)
    }

    // ========================================================================
    // Other Options Metrics
    // ========================================================================

    /// Implied volatility metric.
    #[wasm_bindgen(js_name = ImpliedVol)]
    pub fn implied_vol() -> JsMetricId {
        JsMetricId::from_inner(MetricId::ImpliedVol)
    }

    /// Forward curve PV01.
    #[wasm_bindgen(js_name = ForwardPV01)]
    pub fn forward_pv01() -> JsMetricId {
        JsMetricId::from_inner(MetricId::ForwardPv01)
    }

    // ========================================================================
    // IRS Metrics
    // ========================================================================

    /// Fixed leg present value.
    #[wasm_bindgen(js_name = FixedLegPV)]
    pub fn fixed_leg_pv() -> JsMetricId {
        JsMetricId::from_inner(MetricId::PvFixed)
    }

    /// Floating leg present value.
    #[wasm_bindgen(js_name = FloatingLegPV)]
    pub fn floating_leg_pv() -> JsMetricId {
        JsMetricId::from_inner(MetricId::PvFloat)
    }

    /// Annuity factor.
    #[wasm_bindgen(js_name = Annuity)]
    pub fn annuity() -> JsMetricId {
        JsMetricId::from_inner(MetricId::Annuity)
    }

    /// Par rate.
    #[wasm_bindgen(js_name = ParRate)]
    pub fn par_rate() -> JsMetricId {
        JsMetricId::from_inner(MetricId::ParRate)
    }

    // ========================================================================
    // Credit Metrics
    // ========================================================================

    /// Credit spread (use ParSpread for CDS).
    #[wasm_bindgen(js_name = Spread)]
    pub fn spread() -> JsMetricId {
        JsMetricId::from_inner(MetricId::ParSpread)
    }

    /// Survival probability (available via metric name "survival_probability" - not standard enum).
    #[wasm_bindgen(js_name = SurvivalProbability)]
    pub fn survival_probability() -> JsMetricId {
        // This is computed via custom metric
        JsMetricId::from_inner(MetricId::custom("survival_probability"))
    }

    /// Default probability.
    #[wasm_bindgen(js_name = DefaultProbability)]
    pub fn default_probability() -> JsMetricId {
        JsMetricId::from_inner(MetricId::DefaultProbability)
    }

    /// Recovery rate sensitivity.
    #[wasm_bindgen(js_name = Recovery01)]
    pub fn recovery_01() -> JsMetricId {
        JsMetricId::from_inner(MetricId::Recovery01)
    }

    // ========================================================================
    // Variance Swap Metrics
    // ========================================================================

    /// Variance vega (per variance point).
    #[wasm_bindgen(js_name = VarianceVega)]
    pub fn variance_vega() -> JsMetricId {
        JsMetricId::from_inner(MetricId::VarianceVega)
    }

    /// Expected variance under pricing model.
    #[wasm_bindgen(js_name = ExpectedVariance)]
    pub fn expected_variance() -> JsMetricId {
        JsMetricId::from_inner(MetricId::ExpectedVariance)
    }

    /// Realized variance from observed paths.
    #[wasm_bindgen(js_name = RealizedVariance)]
    pub fn realized_variance() -> JsMetricId {
        JsMetricId::from_inner(MetricId::RealizedVariance)
    }

    /// Variance notional exposure.
    #[wasm_bindgen(js_name = VarianceNotional)]
    pub fn variance_notional() -> JsMetricId {
        JsMetricId::from_inner(MetricId::VarianceNotional)
    }

    /// Strike volatility (sqrt of strike variance).
    #[wasm_bindgen(js_name = VarianceStrikeVol)]
    pub fn variance_strike_vol() -> JsMetricId {
        JsMetricId::from_inner(MetricId::VarianceStrikeVol)
    }

    /// Time to maturity in variance swap conventions.
    #[wasm_bindgen(js_name = VarianceTimeToMaturity)]
    pub fn variance_time_to_maturity() -> JsMetricId {
        JsMetricId::from_inner(MetricId::VarianceTimeToMaturity)
    }

    // ========================================================================
    // Display Methods
    // ========================================================================

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        self.name()
    }

    #[wasm_bindgen(js_name = valueOf)]
    pub fn value_of(&self) -> String {
        self.name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_name_falls_back_to_custom_metric() {
        let metric = JsMetricId::from_name("my_custom_metric");
        assert_eq!(metric.name(), "my_custom_metric");
    }
}
