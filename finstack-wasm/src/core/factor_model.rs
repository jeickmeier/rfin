//! Factor-model primitive types exposed to JS/TS.
//!
//! Wraps `finstack_core::factor_model` building blocks: factor definitions,
//! covariance matrices, bump-size configuration, and the model config bundle.
//! These are the data-plane types consumed by the portfolio factor-model engine.

use crate::core::error::{core_to_js, js_error};
use crate::utils::json::{from_js_value, to_js_value};
use finstack_core::factor_model::{
    BumpSizeConfig, FactorCovarianceMatrix, FactorDefinition, FactorId, FactorModelConfig,
    FactorType, MarketDependency, MarketMapping, PricingMode, RiskMeasure,
};
use finstack_core::market_data::bumps::BumpUnits;
use finstack_core::types::CurveId;
use js_sys::Array;
use wasm_bindgen::prelude::*;

// ---------------------------------------------------------------------------
// FactorId
// ---------------------------------------------------------------------------

/// Unique identifier for a risk factor.
///
/// @example
/// ```javascript
/// const id = new FactorId("USD-Rates");
/// console.log(id.value);  // "USD-Rates"
/// ```
#[wasm_bindgen(js_name = FactorId)]
#[derive(Clone)]
pub struct JsFactorId {
    inner: FactorId,
}

impl JsFactorId {
    #[allow(dead_code)]
    pub(crate) fn from_inner(inner: FactorId) -> Self {
        Self { inner }
    }

    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> &FactorId {
        &self.inner
    }
}

#[wasm_bindgen(js_class = FactorId)]
impl JsFactorId {
    /// Create a new factor identifier.
    #[wasm_bindgen(constructor)]
    pub fn new(id: &str) -> Self {
        Self {
            inner: FactorId::new(id),
        }
    }

    /// Get the identifier string value.
    #[wasm_bindgen(getter)]
    pub fn value(&self) -> String {
        self.inner.as_str().to_string()
    }

    /// String representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_js_string(&self) -> String {
        self.inner.as_str().to_string()
    }
}

// ---------------------------------------------------------------------------
// MarketDependency
// ---------------------------------------------------------------------------

/// A single market dependency extracted from an instrument.
///
/// Represents what market data an instrument needs: curves, spots, vol surfaces,
/// FX pairs, or time series.
///
/// @example
/// ```javascript
/// const dep = MarketDependency.fromJSON({ Curve: { id: "USD-OIS", curve_type: "Discount" } });
/// console.log(dep.kind);  // "Curve"
/// ```
#[wasm_bindgen(js_name = MarketDependency)]
#[derive(Clone)]
pub struct JsMarketDependency {
    inner: MarketDependency,
}

impl JsMarketDependency {
    #[allow(dead_code)]
    pub(crate) fn from_inner(inner: MarketDependency) -> Self {
        Self { inner }
    }

    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> &MarketDependency {
        &self.inner
    }
}

#[wasm_bindgen(js_class = MarketDependency)]
impl JsMarketDependency {
    /// Get the dependency kind (Curve, CreditCurve, Spot, VolSurface, FxPair, Series).
    #[wasm_bindgen(getter)]
    pub fn kind(&self) -> String {
        match &self.inner {
            MarketDependency::Curve { .. } => "Curve".to_string(),
            MarketDependency::CreditCurve { .. } => "CreditCurve".to_string(),
            MarketDependency::Spot { .. } => "Spot".to_string(),
            MarketDependency::VolSurface { .. } => "VolSurface".to_string(),
            MarketDependency::FxPair { .. } => "FxPair".to_string(),
            MarketDependency::Series { .. } => "Series".to_string(),
        }
    }

    /// Get the identifier of the dependency.
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> Option<String> {
        match &self.inner {
            MarketDependency::Curve { id, .. } | MarketDependency::CreditCurve { id } => {
                Some(id.as_ref().to_string())
            }
            MarketDependency::Spot { id }
            | MarketDependency::VolSurface { id }
            | MarketDependency::Series { id } => Some(id.clone()),
            MarketDependency::FxPair { base, quote } => Some(format!("{base}/{quote}")),
        }
    }

    /// Deserialize from a JSON object.
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> Result<JsMarketDependency, JsValue> {
        from_js_value(value).map(|inner| Self { inner })
    }

    /// Serialize to a JSON object.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }
}

// ---------------------------------------------------------------------------
// FactorDefinition
// ---------------------------------------------------------------------------

/// Complete definition of a risk factor including its market mapping.
///
/// @example
/// ```javascript
/// const def = FactorDefinition.fromJSON({
///   id: "USD-Rates",
///   factor_type: "Rates",
///   market_mapping: { CurveParallel: { curve_ids: ["USD-OIS"], units: "RateBp" } }
/// });
/// ```
#[wasm_bindgen(js_name = FactorDefinition)]
#[derive(Clone)]
pub struct JsFactorDefinition {
    inner: FactorDefinition,
}

impl JsFactorDefinition {
    #[allow(dead_code)]
    pub(crate) fn from_inner(inner: FactorDefinition) -> Self {
        Self { inner }
    }

    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> &FactorDefinition {
        &self.inner
    }
}

#[wasm_bindgen(js_class = FactorDefinition)]
impl JsFactorDefinition {
    /// Get the factor identifier.
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    /// Get the factor type classification.
    #[wasm_bindgen(getter, js_name = factorType)]
    pub fn factor_type(&self) -> String {
        factor_type_to_string(&self.inner.factor_type)
    }

    /// Get the optional description.
    #[wasm_bindgen(getter)]
    pub fn description(&self) -> Option<String> {
        self.inner.description.clone()
    }

    /// Create a factor definition for a parallel curve shift.
    #[wasm_bindgen(js_name = curveParallel)]
    pub fn curve_parallel(
        id: &str,
        factor_type: &str,
        curve_ids: Vec<String>,
        units: &str,
        description: Option<String>,
    ) -> Result<JsFactorDefinition, JsValue> {
        Ok(Self {
            inner: FactorDefinition {
                id: FactorId::new(id),
                factor_type: parse_factor_type(factor_type)?,
                market_mapping: MarketMapping::CurveParallel {
                    curve_ids: curve_ids.into_iter().map(CurveId::new).collect(),
                    units: parse_bump_units(units)?,
                },
                description,
            },
        })
    }

    /// Create a factor definition for an equity spot shift.
    #[wasm_bindgen(js_name = equitySpot)]
    pub fn equity_spot(
        id: &str,
        tickers: Vec<String>,
        description: Option<String>,
    ) -> JsFactorDefinition {
        Self {
            inner: FactorDefinition {
                id: FactorId::new(id),
                factor_type: FactorType::Equity,
                market_mapping: MarketMapping::EquitySpot { tickers },
                description,
            },
        }
    }

    /// Deserialize from a JSON object.
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> Result<JsFactorDefinition, JsValue> {
        from_js_value(value).map(|inner| Self { inner })
    }

    /// Serialize to a JSON object.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }
}

// ---------------------------------------------------------------------------
// FactorCovarianceMatrix
// ---------------------------------------------------------------------------

/// Factor covariance matrix for portfolio risk decomposition.
///
/// Stores a validated, symmetric, positive semi-definite covariance matrix
/// aligned to factor identifiers.
///
/// @example
/// ```javascript
/// const cov = new FactorCovarianceMatrix(["Rates", "Credit"], [0.04, 0.01, 0.01, 0.09]);
/// console.log(cov.nFactors);  // 2
/// console.log(cov.variance("Rates"));  // 0.04
/// ```
#[wasm_bindgen(js_name = FactorCovarianceMatrix)]
#[derive(Clone)]
pub struct JsFactorCovarianceMatrix {
    inner: FactorCovarianceMatrix,
}

impl JsFactorCovarianceMatrix {
    #[allow(dead_code)]
    pub(crate) fn from_inner(inner: FactorCovarianceMatrix) -> Self {
        Self { inner }
    }

    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> &FactorCovarianceMatrix {
        &self.inner
    }
}

#[wasm_bindgen(js_class = FactorCovarianceMatrix)]
impl JsFactorCovarianceMatrix {
    /// Create a validated covariance matrix.
    ///
    /// @param factorIds - Ordered factor identifier strings
    /// @param data - Row-major covariance data (length = n*n)
    #[wasm_bindgen(constructor)]
    pub fn new(
        factor_ids: Vec<String>,
        data: Vec<f64>,
    ) -> Result<JsFactorCovarianceMatrix, JsValue> {
        let ids: Vec<FactorId> = factor_ids.into_iter().map(FactorId::new).collect();
        let inner = FactorCovarianceMatrix::new(ids, data).map_err(core_to_js)?;
        Ok(Self { inner })
    }

    /// Number of factors in the matrix.
    #[wasm_bindgen(getter, js_name = nFactors)]
    pub fn n_factors(&self) -> usize {
        self.inner.n_factors()
    }

    /// Ordered factor identifier strings.
    #[wasm_bindgen(getter, js_name = factorIds)]
    pub fn factor_ids(&self) -> Vec<String> {
        self.inner
            .factor_ids()
            .iter()
            .map(|id| id.as_str().to_string())
            .collect()
    }

    /// Get the variance for a factor.
    pub fn variance(&self, factor_id: &str) -> f64 {
        self.inner.variance(&FactorId::new(factor_id))
    }

    /// Get the covariance between two factors.
    pub fn covariance(&self, lhs: &str, rhs: &str) -> f64 {
        self.inner
            .covariance(&FactorId::new(lhs), &FactorId::new(rhs))
    }

    /// Get the correlation between two factors.
    pub fn correlation(&self, lhs: &str, rhs: &str) -> f64 {
        self.inner
            .correlation(&FactorId::new(lhs), &FactorId::new(rhs))
    }

    /// Deserialize from a JSON object.
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> Result<JsFactorCovarianceMatrix, JsValue> {
        from_js_value(value).map(|inner| Self { inner })
    }

    /// Serialize to a JSON object.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }
}

// ---------------------------------------------------------------------------
// BumpSizeConfig
// ---------------------------------------------------------------------------

/// Per-factor-type bump magnitudes for finite-difference sensitivity engines.
///
/// @example
/// ```javascript
/// const bumps = new BumpSizeConfig();
/// console.log(bumps.ratesBp);  // 1.0
/// ```
#[wasm_bindgen(js_name = BumpSizeConfig)]
#[derive(Clone)]
pub struct JsBumpSizeConfig {
    inner: BumpSizeConfig,
}

impl JsBumpSizeConfig {
    #[allow(dead_code)]
    pub(crate) fn from_inner(inner: BumpSizeConfig) -> Self {
        Self { inner }
    }

    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> &BumpSizeConfig {
        &self.inner
    }
}

impl Default for JsBumpSizeConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen(js_class = BumpSizeConfig)]
impl JsBumpSizeConfig {
    /// Create a bump size configuration with defaults (all 1.0).
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: BumpSizeConfig::default(),
        }
    }

    /// Default rates bump in basis points.
    #[wasm_bindgen(getter, js_name = ratesBp)]
    pub fn rates_bp(&self) -> f64 {
        self.inner.rates_bp
    }

    /// Set rates bump in basis points.
    #[wasm_bindgen(setter, js_name = ratesBp)]
    pub fn set_rates_bp(&mut self, value: f64) {
        self.inner.rates_bp = value;
    }

    /// Default credit bump in basis points.
    #[wasm_bindgen(getter, js_name = creditBp)]
    pub fn credit_bp(&self) -> f64 {
        self.inner.credit_bp
    }

    /// Set credit bump in basis points.
    #[wasm_bindgen(setter, js_name = creditBp)]
    pub fn set_credit_bp(&mut self, value: f64) {
        self.inner.credit_bp = value;
    }

    /// Default equity spot bump in percent.
    #[wasm_bindgen(getter, js_name = equityPct)]
    pub fn equity_pct(&self) -> f64 {
        self.inner.equity_pct
    }

    /// Set equity spot bump in percent.
    #[wasm_bindgen(setter, js_name = equityPct)]
    pub fn set_equity_pct(&mut self, value: f64) {
        self.inner.equity_pct = value;
    }

    /// Default FX spot bump in percent.
    #[wasm_bindgen(getter, js_name = fxPct)]
    pub fn fx_pct(&self) -> f64 {
        self.inner.fx_pct
    }

    /// Set FX spot bump in percent.
    #[wasm_bindgen(setter, js_name = fxPct)]
    pub fn set_fx_pct(&mut self, value: f64) {
        self.inner.fx_pct = value;
    }

    /// Default volatility bump in absolute vol points.
    #[wasm_bindgen(getter, js_name = volPoints)]
    pub fn vol_points(&self) -> f64 {
        self.inner.vol_points
    }

    /// Set volatility bump in vol points.
    #[wasm_bindgen(setter, js_name = volPoints)]
    pub fn set_vol_points(&mut self, value: f64) {
        self.inner.vol_points = value;
    }

    /// Deserialize from a JSON object.
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> Result<JsBumpSizeConfig, JsValue> {
        from_js_value(value).map(|inner| Self { inner })
    }

    /// Serialize to a JSON object.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }
}

// ---------------------------------------------------------------------------
// FactorModelConfig
// ---------------------------------------------------------------------------

/// Serializable configuration bundle for constructing a factor-model workflow.
///
/// Bundles factor definitions, covariance matrix, matching configuration,
/// pricing mode, and risk measure into a single validated configuration.
///
/// @example
/// ```javascript
/// const config = FactorModelConfig.fromJSON({
///   factors: [...],
///   covariance: {...},
///   matching: { MappingTable: [] },
///   pricing_mode: "delta_based",
///   risk_measure: "variance"
/// });
/// ```
#[wasm_bindgen(js_name = FactorModelConfig)]
#[derive(Clone)]
pub struct JsFactorModelConfig {
    inner: FactorModelConfig,
}

impl JsFactorModelConfig {
    #[allow(dead_code)]
    pub(crate) fn from_inner(inner: FactorModelConfig) -> Self {
        Self { inner }
    }

    pub(crate) fn inner(&self) -> &FactorModelConfig {
        &self.inner
    }
}

#[wasm_bindgen(js_class = FactorModelConfig)]
impl JsFactorModelConfig {
    /// Get the factor definitions.
    #[wasm_bindgen(getter)]
    pub fn factors(&self) -> Array {
        self.inner
            .factors
            .iter()
            .cloned()
            .map(|f| JsValue::from(JsFactorDefinition::from_inner(f)))
            .collect()
    }

    /// Get the covariance matrix.
    #[wasm_bindgen(getter)]
    pub fn covariance(&self) -> JsFactorCovarianceMatrix {
        JsFactorCovarianceMatrix::from_inner(self.inner.covariance.clone())
    }

    /// Get the pricing mode ("DeltaBased" or "FullRepricing").
    #[wasm_bindgen(getter, js_name = pricingMode)]
    pub fn pricing_mode(&self) -> String {
        match self.inner.pricing_mode {
            PricingMode::DeltaBased => "DeltaBased".to_string(),
            PricingMode::FullRepricing => "FullRepricing".to_string(),
        }
    }

    /// Get the risk measure name.
    #[wasm_bindgen(getter, js_name = riskMeasure)]
    pub fn risk_measure(&self) -> String {
        match self.inner.risk_measure {
            RiskMeasure::Variance => "Variance".to_string(),
            RiskMeasure::Volatility => "Volatility".to_string(),
            RiskMeasure::VaR { .. } => "VaR".to_string(),
            RiskMeasure::ExpectedShortfall { .. } => "ExpectedShortfall".to_string(),
        }
    }

    /// Get the bump size config (if any).
    #[wasm_bindgen(getter, js_name = bumpSize)]
    pub fn bump_size(&self) -> Option<JsBumpSizeConfig> {
        self.inner
            .bump_size
            .clone()
            .map(JsBumpSizeConfig::from_inner)
    }

    /// Deserialize from a JSON object.
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> Result<JsFactorModelConfig, JsValue> {
        let inner: FactorModelConfig = from_js_value(value)?;
        inner.risk_measure.validate().map_err(core_to_js)?;
        Ok(Self { inner })
    }

    /// Serialize to a JSON object.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn factor_type_to_string(ft: &FactorType) -> String {
    match ft {
        FactorType::Rates => "Rates".to_string(),
        FactorType::Credit => "Credit".to_string(),
        FactorType::Equity => "Equity".to_string(),
        FactorType::FX => "FX".to_string(),
        FactorType::Volatility => "Volatility".to_string(),
        FactorType::Commodity => "Commodity".to_string(),
        FactorType::Inflation => "Inflation".to_string(),
        FactorType::Custom(name) => format!("Custom:{name}"),
    }
}

fn parse_factor_type(value: &str) -> Result<FactorType, JsValue> {
    let lower: String = value
        .chars()
        .filter(|ch| !matches!(ch, '_' | '-' | ' '))
        .flat_map(char::to_lowercase)
        .collect();
    match lower.as_str() {
        "rates" => Ok(FactorType::Rates),
        "credit" => Ok(FactorType::Credit),
        "equity" => Ok(FactorType::Equity),
        "fx" => Ok(FactorType::FX),
        "volatility" | "vol" => Ok(FactorType::Volatility),
        "commodity" => Ok(FactorType::Commodity),
        "inflation" => Ok(FactorType::Inflation),
        _ if lower.starts_with("custom:") => Ok(FactorType::Custom(
            value
                .split_once(':')
                .map(|(_, tail)| tail.trim().to_string())
                .unwrap_or_default(),
        )),
        _ => Err(js_error(format!(
            "Unsupported factor_type '{value}'. Expected Rates, Credit, Equity, FX, Volatility, Commodity, Inflation, or custom:<name>"
        ))),
    }
}

fn parse_bump_units(value: &str) -> Result<BumpUnits, JsValue> {
    let lower: String = value
        .chars()
        .filter(|ch| !matches!(ch, '_' | '-' | ' '))
        .flat_map(char::to_lowercase)
        .collect();
    match lower.as_str() {
        "bp" | "bps" | "ratebp" => Ok(BumpUnits::RateBp),
        "percent" | "pct" => Ok(BumpUnits::Percent),
        "fraction" => Ok(BumpUnits::Fraction),
        "factor" => Ok(BumpUnits::Factor),
        _ => Err(js_error(format!(
            "Unsupported bump units '{value}'. Expected bp, percent, fraction, or factor"
        ))),
    }
}
