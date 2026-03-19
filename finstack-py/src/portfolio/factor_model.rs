//! Python bindings for portfolio factor-model configuration and analysis.

use crate::core::dates::utils::py_to_date;
use crate::core::market_data::context::PyMarketContext;
use crate::errors::core_to_py;
use crate::portfolio::error::portfolio_to_py;
use crate::portfolio::positions::PyPortfolio;
use crate::portfolio::types::PyPosition;
use finstack_core::currency::Currency;
use finstack_core::factor_model::{
    AttributeFilter, BumpSizeConfig, CurveType, DependencyFilter, DependencyType,
    FactorCovarianceMatrix, FactorDefinition, FactorId, FactorModelConfig, FactorNode, FactorType,
    HierarchicalConfig, MappingRule, MarketDependency, MarketMapping, MatchingConfig, PricingMode,
    RiskMeasure, UnmatchedPolicy,
};
use finstack_core::market_data::bumps::BumpUnits;
use finstack_core::types::CurveId;
use finstack_portfolio::factor_model::{
    FactorAssignmentReport, FactorConstraint, FactorContribution, FactorContributionDelta,
    FactorModel, FactorModelBuilder, FactorOptimizationResult, PositionAssignment, PositionChange,
    PositionFactorContribution, RiskDecomposition, StressResult, UnmatchedEntry, WhatIfEngine,
    WhatIfResult,
};
use finstack_valuations::factor_model::sensitivity::SensitivityMatrix;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyModule, PyString};
use pyo3::Bound;
use pythonize::depythonize;
use std::sync::Arc;

fn normalized_name(value: &str) -> String {
    value
        .chars()
        .filter(|ch| !matches!(ch, '_' | '-' | ' '))
        .flat_map(char::to_lowercase)
        .collect()
}

fn parse_factor_type(value: &str) -> PyResult<FactorType> {
    let lower = normalized_name(value);
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
        _ => Err(PyValueError::new_err(format!(
            "Unsupported factor_type '{value}'. Expected Rates, Credit, Equity, FX, Volatility, Commodity, Inflation, or custom:<name>"
        ))),
    }
}

fn factor_type_to_string(value: &FactorType) -> String {
    match value {
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

fn parse_pricing_mode(value: &str) -> PyResult<PricingMode> {
    match normalized_name(value).as_str() {
        "deltabased" => Ok(PricingMode::DeltaBased),
        "fullrepricing" => Ok(PricingMode::FullRepricing),
        _ => Err(PyValueError::new_err(format!(
            "Unsupported pricing_mode '{value}'. Expected DeltaBased or FullRepricing"
        ))),
    }
}

fn pricing_mode_to_string(value: PricingMode) -> String {
    match value {
        PricingMode::DeltaBased => "DeltaBased".to_string(),
        PricingMode::FullRepricing => "FullRepricing".to_string(),
    }
}

fn parse_unmatched_policy(value: &str) -> PyResult<UnmatchedPolicy> {
    match normalized_name(value).as_str() {
        "strict" => Ok(UnmatchedPolicy::Strict),
        "residual" => Ok(UnmatchedPolicy::Residual),
        "warn" => Ok(UnmatchedPolicy::Warn),
        _ => Err(PyValueError::new_err(format!(
            "Unsupported unmatched_policy '{value}'. Expected Strict, Residual, or Warn"
        ))),
    }
}

fn unmatched_policy_to_string(value: UnmatchedPolicy) -> String {
    match value {
        UnmatchedPolicy::Strict => "Strict".to_string(),
        UnmatchedPolicy::Residual => "Residual".to_string(),
        UnmatchedPolicy::Warn => "Warn".to_string(),
    }
}

fn parse_dependency_type(value: &str) -> PyResult<DependencyType> {
    match normalized_name(value).as_str() {
        "discount" => Ok(DependencyType::Discount),
        "forward" => Ok(DependencyType::Forward),
        "credit" => Ok(DependencyType::Credit),
        "spot" => Ok(DependencyType::Spot),
        "vol" | "volsurface" | "volatility" => Ok(DependencyType::Vol),
        "fx" => Ok(DependencyType::Fx),
        "series" => Ok(DependencyType::Series),
        "hazard" => Err(PyValueError::new_err(
            "Hazard is a CurveType, not a DependencyType. Use dependency_type='Credit' and curve_type='Hazard'",
        )),
        _ => Err(PyValueError::new_err(format!(
            "Unsupported dependency_type '{value}'"
        ))),
    }
}

fn dependency_type_to_string(value: DependencyType) -> String {
    match value {
        DependencyType::Discount => "Discount".to_string(),
        DependencyType::Forward => "Forward".to_string(),
        DependencyType::Credit => "Credit".to_string(),
        DependencyType::Spot => "Spot".to_string(),
        DependencyType::Vol => "Vol".to_string(),
        DependencyType::Fx => "Fx".to_string(),
        DependencyType::Series => "Series".to_string(),
    }
}

fn parse_curve_type(value: &str) -> PyResult<CurveType> {
    match normalized_name(value).as_str() {
        "discount" => Ok(CurveType::Discount),
        "forward" => Ok(CurveType::Forward),
        "hazard" | "credit" => Ok(CurveType::Hazard),
        "inflation" => Ok(CurveType::Inflation),
        "basecorrelation" => Ok(CurveType::BaseCorrelation),
        _ => Err(PyValueError::new_err(format!(
            "Unsupported curve_type '{value}'"
        ))),
    }
}

fn curve_type_to_string(value: CurveType) -> String {
    match value {
        CurveType::Discount => "Discount".to_string(),
        CurveType::Forward => "Forward".to_string(),
        CurveType::Hazard => "Hazard".to_string(),
        CurveType::Inflation => "Inflation".to_string(),
        CurveType::BaseCorrelation => "BaseCorrelation".to_string(),
    }
}

fn parse_bump_units(value: &str) -> PyResult<BumpUnits> {
    match normalized_name(value).as_str() {
        "bp" | "bps" | "ratebp" => Ok(BumpUnits::RateBp),
        "percent" | "pct" => Ok(BumpUnits::Percent),
        "fraction" => Ok(BumpUnits::Fraction),
        "factor" => Ok(BumpUnits::Factor),
        _ => Err(PyValueError::new_err(format!(
            "Unsupported bump units '{value}'. Expected bp, percent, fraction, or factor"
        ))),
    }
}

fn bump_units_to_string(value: BumpUnits) -> String {
    match value {
        BumpUnits::RateBp => "bp".to_string(),
        BumpUnits::Percent => "percent".to_string(),
        BumpUnits::Fraction => "fraction".to_string(),
        BumpUnits::Factor => "factor".to_string(),
        _ => "unknown".to_string(),
    }
}

fn parse_risk_measure(value: Option<&Bound<'_, PyAny>>) -> PyResult<RiskMeasure> {
    let Some(value) = value else {
        return Ok(RiskMeasure::Variance);
    };

    if let Ok(text) = value.extract::<String>() {
        return match normalized_name(&text).as_str() {
            "variance" => Ok(RiskMeasure::Variance),
            "volatility" => Ok(RiskMeasure::Volatility),
            "var" => Err(PyValueError::new_err(
                "VaR requires a confidence payload, for example {'var': {'confidence': 0.99}}",
            )),
            "expectedshortfall" => Err(PyValueError::new_err(
                "ExpectedShortfall requires a confidence payload, for example {'expected_shortfall': {'confidence': 0.975}}",
            )),
            _ => Err(PyValueError::new_err(format!(
                "Unsupported risk_measure '{text}'"
            ))),
        };
    }

    let json_value: serde_json::Value = depythonize(value)
        .map_err(|err| PyValueError::new_err(format!("Failed to parse risk_measure: {err}")))?;
    serde_json::from_value(json_value).map_err(|err| PyValueError::new_err(err.to_string()))
}

fn risk_measure_to_py(py: Python<'_>, value: &RiskMeasure) -> PyResult<Py<PyAny>> {
    match value {
        RiskMeasure::Variance => Ok(PyString::new(py, "Variance").into_any().unbind()),
        RiskMeasure::Volatility => Ok(PyString::new(py, "Volatility").into_any().unbind()),
        RiskMeasure::VaR { confidence } => {
            let dict = PyDict::new(py);
            let nested = PyDict::new(py);
            nested.set_item("confidence", *confidence)?;
            dict.set_item("var", nested)?;
            Ok(dict.into())
        }
        RiskMeasure::ExpectedShortfall { confidence } => {
            let dict = PyDict::new(py);
            let nested = PyDict::new(py);
            nested.set_item("confidence", *confidence)?;
            dict.set_item("expected_shortfall", nested)?;
            Ok(dict.into())
        }
    }
}

fn build_validated_config(config: FactorModelConfig) -> PyResult<FactorModelConfig> {
    config.risk_measure.validate().map_err(core_to_py)?;
    let factor_ids = config.covariance.factor_ids().to_vec();
    let data = config.covariance.as_slice().to_vec();
    let covariance = FactorCovarianceMatrix::new(factor_ids, data).map_err(core_to_py)?;
    Ok(FactorModelConfig {
        covariance,
        ..config
    })
}

fn mapping_to_json<T: serde::Serialize>(value: &T) -> PyResult<String> {
    serde_json::to_string_pretty(value).map_err(|err| PyValueError::new_err(err.to_string()))
}

fn currency_pair_string(base: Currency, quote: Currency) -> String {
    format!("{base}/{quote}")
}

#[pyclass(
    module = "finstack.portfolio.factor_model",
    name = "MarketDependency",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyMarketDependency {
    pub(crate) inner: MarketDependency,
}

impl PyMarketDependency {
    fn from_inner(inner: MarketDependency) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyMarketDependency {
    #[getter]
    fn kind(&self) -> String {
        match self.inner {
            MarketDependency::Curve { .. } => "Curve".to_string(),
            MarketDependency::CreditCurve { .. } => "CreditCurve".to_string(),
            MarketDependency::Spot { .. } => "Spot".to_string(),
            MarketDependency::VolSurface { .. } => "VolSurface".to_string(),
            MarketDependency::FxPair { .. } => "FxPair".to_string(),
            MarketDependency::Series { .. } => "Series".to_string(),
        }
    }

    #[getter]
    fn id(&self) -> Option<String> {
        match &self.inner {
            MarketDependency::Curve { id, .. } | MarketDependency::CreditCurve { id } => {
                Some(id.as_ref().to_string())
            }
            MarketDependency::Spot { id }
            | MarketDependency::VolSurface { id }
            | MarketDependency::Series { id } => Some(id.clone()),
            MarketDependency::FxPair { base, quote } => Some(currency_pair_string(*base, *quote)),
        }
    }

    #[getter]
    fn dependency_type(&self) -> String {
        match self.inner {
            MarketDependency::Curve { curve_type, .. } => match curve_type {
                CurveType::Discount => "Discount".to_string(),
                CurveType::Forward => "Forward".to_string(),
                CurveType::Hazard => "Credit".to_string(),
                CurveType::Inflation => "Forward".to_string(),
                CurveType::BaseCorrelation => "Forward".to_string(),
            },
            MarketDependency::CreditCurve { .. } => "Credit".to_string(),
            MarketDependency::Spot { .. } => "Spot".to_string(),
            MarketDependency::VolSurface { .. } => "Vol".to_string(),
            MarketDependency::FxPair { .. } => "Fx".to_string(),
            MarketDependency::Series { .. } => "Series".to_string(),
        }
    }

    fn to_json(&self) -> PyResult<String> {
        mapping_to_json(&self.inner)
    }
}

#[pyclass(
    module = "finstack.portfolio.factor_model",
    name = "BumpSizeConfig",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyBumpSizeConfig {
    pub(crate) inner: BumpSizeConfig,
}

impl PyBumpSizeConfig {
    fn from_inner(inner: BumpSizeConfig) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyBumpSizeConfig {
    #[new]
    #[pyo3(signature = (rates_bp=1.0, credit_bp=1.0, equity_pct=1.0, fx_pct=1.0, vol_points=1.0, overrides=None))]
    fn new(
        rates_bp: f64,
        credit_bp: f64,
        equity_pct: f64,
        fx_pct: f64,
        vol_points: f64,
        overrides: Option<Vec<(String, f64)>>,
    ) -> Self {
        Self {
            inner: BumpSizeConfig {
                rates_bp,
                credit_bp,
                equity_pct,
                fx_pct,
                vol_points,
                overrides: overrides
                    .unwrap_or_default()
                    .into_iter()
                    .map(|(factor_id, value)| (FactorId::new(factor_id), value))
                    .collect(),
            },
        }
    }

    #[getter]
    fn rates_bp(&self) -> f64 {
        self.inner.rates_bp
    }

    #[getter]
    fn credit_bp(&self) -> f64 {
        self.inner.credit_bp
    }

    #[getter]
    fn equity_pct(&self) -> f64 {
        self.inner.equity_pct
    }

    #[getter]
    fn fx_pct(&self) -> f64 {
        self.inner.fx_pct
    }

    #[getter]
    fn vol_points(&self) -> f64 {
        self.inner.vol_points
    }

    #[getter]
    fn overrides(&self) -> Vec<(String, f64)> {
        self.inner
            .overrides
            .iter()
            .map(|(factor_id, value)| (factor_id.as_str().to_string(), *value))
            .collect()
    }
}

#[pyclass(
    module = "finstack.portfolio.factor_model",
    name = "MarketMapping",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyMarketMapping {
    pub(crate) inner: MarketMapping,
}

impl PyMarketMapping {
    fn from_inner(inner: MarketMapping) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyMarketMapping {
    #[staticmethod]
    fn curve_parallel(curve_ids: Vec<String>, units: String) -> PyResult<Self> {
        Ok(Self::from_inner(MarketMapping::CurveParallel {
            curve_ids: curve_ids.into_iter().map(CurveId::new).collect(),
            units: parse_bump_units(&units)?,
        }))
    }

    #[staticmethod]
    fn curve_bucketed(curve_id: String, tenor_weights: Vec<(f64, f64)>) -> Self {
        Self::from_inner(MarketMapping::CurveBucketed {
            curve_id: CurveId::new(curve_id),
            tenor_weights,
        })
    }

    #[staticmethod]
    fn equity_spot(tickers: Vec<String>) -> Self {
        Self::from_inner(MarketMapping::EquitySpot { tickers })
    }

    #[staticmethod]
    fn fx_rate(base: &Bound<'_, PyAny>, quote: &Bound<'_, PyAny>) -> PyResult<Self> {
        let base = if let Ok(text) = base.extract::<String>() {
            text.parse()
                .map_err(|err| PyValueError::new_err(format!("Invalid base currency: {err}")))?
        } else {
            return Err(PyTypeError::new_err("base must be a currency code string"));
        };
        let quote = if let Ok(text) = quote.extract::<String>() {
            text.parse()
                .map_err(|err| PyValueError::new_err(format!("Invalid quote currency: {err}")))?
        } else {
            return Err(PyTypeError::new_err("quote must be a currency code string"));
        };
        Ok(Self::from_inner(MarketMapping::FxRate {
            pair: (base, quote),
        }))
    }

    #[staticmethod]
    fn vol_shift(surface_ids: Vec<String>, units: String) -> PyResult<Self> {
        Ok(Self::from_inner(MarketMapping::VolShift {
            surface_ids,
            units: parse_bump_units(&units)?,
        }))
    }

    #[getter]
    fn kind(&self) -> String {
        match self.inner {
            MarketMapping::CurveParallel { .. } => "CurveParallel".to_string(),
            MarketMapping::CurveBucketed { .. } => "CurveBucketed".to_string(),
            MarketMapping::EquitySpot { .. } => "EquitySpot".to_string(),
            MarketMapping::FxRate { .. } => "FxRate".to_string(),
            MarketMapping::VolShift { .. } => "VolShift".to_string(),
            MarketMapping::Custom(_) => "Custom".to_string(),
        }
    }

    #[getter]
    fn curve_ids(&self) -> Vec<String> {
        match &self.inner {
            MarketMapping::CurveParallel { curve_ids, .. } => {
                curve_ids.iter().map(|id| id.as_ref().to_string()).collect()
            }
            _ => Vec::new(),
        }
    }

    #[getter]
    fn units(&self) -> Option<String> {
        match self.inner {
            MarketMapping::CurveParallel { units, .. } | MarketMapping::VolShift { units, .. } => {
                Some(bump_units_to_string(units))
            }
            _ => None,
        }
    }

    #[getter]
    fn tenor_weights(&self) -> Vec<(f64, f64)> {
        match &self.inner {
            MarketMapping::CurveBucketed { tenor_weights, .. } => tenor_weights.clone(),
            _ => Vec::new(),
        }
    }

    fn to_json(&self) -> PyResult<String> {
        mapping_to_json(&self.inner)
    }
}

#[pyclass(
    module = "finstack.portfolio.factor_model",
    name = "FactorDefinition",
    from_py_object
)]
#[derive(Clone)]
pub struct PyFactorDefinition {
    pub(crate) inner: FactorDefinition,
}

impl PyFactorDefinition {
    fn from_inner(inner: FactorDefinition) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyFactorDefinition {
    #[new]
    #[pyo3(signature = (id, factor_type, market_mapping, description=None))]
    fn new(
        id: String,
        factor_type: String,
        market_mapping: PyRef<'_, PyMarketMapping>,
        description: Option<String>,
    ) -> PyResult<Self> {
        Ok(Self::from_inner(FactorDefinition {
            id: FactorId::new(id),
            factor_type: parse_factor_type(&factor_type)?,
            market_mapping: market_mapping.inner.clone(),
            description,
        }))
    }

    #[getter]
    fn id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    #[getter]
    fn factor_type(&self) -> String {
        factor_type_to_string(&self.inner.factor_type)
    }

    #[getter]
    fn market_mapping(&self) -> PyMarketMapping {
        PyMarketMapping::from_inner(self.inner.market_mapping.clone())
    }

    #[getter]
    fn description(&self) -> Option<String> {
        self.inner.description.clone()
    }
}

#[pyclass(
    module = "finstack.portfolio.factor_model",
    name = "FactorCovarianceMatrix",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyFactorCovarianceMatrix {
    pub(crate) inner: FactorCovarianceMatrix,
}

impl PyFactorCovarianceMatrix {
    fn from_inner(inner: FactorCovarianceMatrix) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyFactorCovarianceMatrix {
    #[new]
    fn new(factor_ids: Vec<String>, matrix: Vec<Vec<f64>>) -> PyResult<Self> {
        let n = factor_ids.len();
        if matrix.len() != n || matrix.iter().any(|row| row.len() != n) {
            return Err(PyValueError::new_err(format!(
                "matrix must be square with shape {n}x{n}"
            )));
        }
        let data = matrix.into_iter().flatten().collect();
        let factor_ids = factor_ids.into_iter().map(FactorId::new).collect();
        let inner = FactorCovarianceMatrix::new(factor_ids, data).map_err(core_to_py)?;
        Ok(Self::from_inner(inner))
    }

    #[getter]
    fn factor_ids(&self) -> Vec<String> {
        self.inner
            .factor_ids()
            .iter()
            .map(|factor_id| factor_id.as_str().to_string())
            .collect()
    }

    fn matrix(&self) -> Vec<Vec<f64>> {
        let n = self.inner.n_factors();
        self.inner
            .as_slice()
            .chunks(n)
            .map(|row| row.to_vec())
            .collect()
    }

    fn n_factors(&self) -> usize {
        self.inner.n_factors()
    }

    fn variance(&self, factor_id: &str) -> f64 {
        self.inner.variance(&FactorId::new(factor_id))
    }

    fn covariance(&self, lhs: &str, rhs: &str) -> f64 {
        self.inner
            .covariance(&FactorId::new(lhs), &FactorId::new(rhs))
    }

    fn correlation(&self, lhs: &str, rhs: &str) -> f64 {
        self.inner
            .correlation(&FactorId::new(lhs), &FactorId::new(rhs))
    }

    fn to_json(&self) -> PyResult<String> {
        mapping_to_json(&self.inner)
    }
}

#[pyclass(
    module = "finstack.portfolio.factor_model",
    name = "AttributeFilter",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyAttributeFilter {
    pub(crate) inner: AttributeFilter,
}

impl PyAttributeFilter {
    fn from_inner(inner: AttributeFilter) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyAttributeFilter {
    #[new]
    #[pyo3(signature = (tags=None, meta=None))]
    fn new(tags: Option<Vec<String>>, meta: Option<Vec<(String, String)>>) -> Self {
        Self::from_inner(AttributeFilter {
            tags: tags.unwrap_or_default(),
            meta: meta.unwrap_or_default(),
        })
    }

    #[getter]
    fn tags(&self) -> Vec<String> {
        self.inner.tags.clone()
    }

    #[getter]
    fn meta(&self) -> Vec<(String, String)> {
        self.inner.meta.clone()
    }
}

#[pyclass(
    module = "finstack.portfolio.factor_model",
    name = "DependencyFilter",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyDependencyFilter {
    pub(crate) inner: DependencyFilter,
}

impl PyDependencyFilter {
    fn from_inner(inner: DependencyFilter) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyDependencyFilter {
    #[new]
    #[pyo3(signature = (dependency_type=None, curve_type=None, id=None))]
    fn new(
        dependency_type: Option<String>,
        curve_type: Option<String>,
        id: Option<String>,
    ) -> PyResult<Self> {
        Ok(Self::from_inner(DependencyFilter {
            dependency_type: dependency_type
                .as_deref()
                .map(parse_dependency_type)
                .transpose()?,
            curve_type: curve_type.as_deref().map(parse_curve_type).transpose()?,
            id,
        }))
    }

    #[getter]
    fn dependency_type(&self) -> Option<String> {
        self.inner.dependency_type.map(dependency_type_to_string)
    }

    #[getter]
    fn curve_type(&self) -> Option<String> {
        self.inner.curve_type.map(curve_type_to_string)
    }

    #[getter]
    fn id(&self) -> Option<String> {
        self.inner.id.clone()
    }
}

#[pyclass(
    module = "finstack.portfolio.factor_model",
    name = "MappingRule",
    from_py_object
)]
#[derive(Clone)]
pub struct PyMappingRule {
    pub(crate) inner: MappingRule,
}

impl PyMappingRule {
    fn from_inner(inner: MappingRule) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyMappingRule {
    #[new]
    fn new(
        dependency_filter: PyRef<'_, PyDependencyFilter>,
        attribute_filter: PyRef<'_, PyAttributeFilter>,
        factor_id: String,
    ) -> Self {
        Self::from_inner(MappingRule {
            dependency_filter: dependency_filter.inner.clone(),
            attribute_filter: attribute_filter.inner.clone(),
            factor_id: FactorId::new(factor_id),
        })
    }

    #[getter]
    fn dependency_filter(&self) -> PyDependencyFilter {
        PyDependencyFilter::from_inner(self.inner.dependency_filter.clone())
    }

    #[getter]
    fn attribute_filter(&self) -> PyAttributeFilter {
        PyAttributeFilter::from_inner(self.inner.attribute_filter.clone())
    }

    #[getter]
    fn factor_id(&self) -> String {
        self.inner.factor_id.as_str().to_string()
    }
}

#[pyclass(
    module = "finstack.portfolio.factor_model",
    name = "FactorNode",
    from_py_object
)]
#[derive(Clone)]
pub struct PyFactorNode {
    pub(crate) inner: FactorNode,
}

impl PyFactorNode {
    fn from_inner(inner: FactorNode) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyFactorNode {
    #[new]
    #[pyo3(signature = (factor_id=None, filter=None, children=None))]
    fn new(
        factor_id: Option<String>,
        filter: Option<PyRef<'_, PyAttributeFilter>>,
        children: Option<Vec<PyFactorNode>>,
    ) -> Self {
        Self::from_inner(FactorNode {
            factor_id: factor_id.map(FactorId::new),
            filter: filter
                .map(|filter| filter.inner.clone())
                .unwrap_or_default(),
            children: children
                .unwrap_or_default()
                .into_iter()
                .map(|node| node.inner)
                .collect(),
        })
    }

    #[getter]
    fn factor_id(&self) -> Option<String> {
        self.inner
            .factor_id
            .as_ref()
            .map(|factor_id| factor_id.as_str().to_string())
    }

    #[getter]
    fn filter(&self) -> PyAttributeFilter {
        PyAttributeFilter::from_inner(self.inner.filter.clone())
    }

    #[getter]
    fn children(&self) -> Vec<PyFactorNode> {
        self.inner
            .children
            .iter()
            .cloned()
            .map(PyFactorNode::from_inner)
            .collect()
    }
}

#[pyclass(
    module = "finstack.portfolio.factor_model",
    name = "HierarchicalConfig",
    from_py_object
)]
#[derive(Clone)]
pub struct PyHierarchicalConfig {
    pub(crate) inner: HierarchicalConfig,
}

impl PyHierarchicalConfig {
    fn from_inner(inner: HierarchicalConfig) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyHierarchicalConfig {
    #[new]
    #[pyo3(signature = (root, dependency_filter=None))]
    fn new(root: PyFactorNode, dependency_filter: Option<PyRef<'_, PyDependencyFilter>>) -> Self {
        Self::from_inner(HierarchicalConfig {
            dependency_filter: dependency_filter
                .map(|filter| filter.inner.clone())
                .unwrap_or_default(),
            root: root.inner,
        })
    }

    #[getter]
    fn dependency_filter(&self) -> PyDependencyFilter {
        PyDependencyFilter::from_inner(self.inner.dependency_filter.clone())
    }

    #[getter]
    fn root(&self) -> PyFactorNode {
        PyFactorNode::from_inner(self.inner.root.clone())
    }
}

#[pyclass(
    module = "finstack.portfolio.factor_model",
    name = "MatchingConfig",
    from_py_object
)]
#[derive(Clone)]
pub struct PyMatchingConfig {
    pub(crate) inner: MatchingConfig,
}

impl PyMatchingConfig {
    fn from_inner(inner: MatchingConfig) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyMatchingConfig {
    #[staticmethod]
    fn mapping_table(rules: Vec<PyMappingRule>) -> Self {
        Self::from_inner(MatchingConfig::MappingTable(
            rules.into_iter().map(|rule| rule.inner).collect(),
        ))
    }

    #[staticmethod]
    fn cascade(configs: Vec<PyMatchingConfig>) -> Self {
        Self::from_inner(MatchingConfig::Cascade(
            configs.into_iter().map(|config| config.inner).collect(),
        ))
    }

    #[staticmethod]
    fn hierarchical(config: PyHierarchicalConfig) -> Self {
        Self::from_inner(MatchingConfig::Hierarchical(config.inner))
    }

    #[getter]
    fn kind(&self) -> String {
        match self.inner {
            MatchingConfig::MappingTable(_) => "MappingTable".to_string(),
            MatchingConfig::Cascade(_) => "Cascade".to_string(),
            MatchingConfig::Hierarchical(_) => "Hierarchical".to_string(),
        }
    }

    fn to_json(&self) -> PyResult<String> {
        let json = serde_json::to_value(&self.inner)
            .map_err(|err| PyValueError::new_err(err.to_string()))?;
        let normalized = match self.inner {
            MatchingConfig::MappingTable(_) => serde_json::json!({ "mapping_table": json }),
            MatchingConfig::Cascade(_) => serde_json::json!({ "cascade": json }),
            MatchingConfig::Hierarchical(_) => serde_json::json!({ "hierarchical": json }),
        };
        serde_json::to_string_pretty(&normalized)
            .map_err(|err| PyValueError::new_err(err.to_string()))
    }

    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let value: serde_json::Value =
            serde_json::from_str(json).map_err(|err| PyValueError::new_err(err.to_string()))?;
        let inner = if let Some(inner) = value.get("mapping_table") {
            MatchingConfig::MappingTable(
                serde_json::from_value(inner.clone())
                    .map_err(|err| PyValueError::new_err(err.to_string()))?,
            )
        } else if let Some(inner) = value.get("cascade") {
            MatchingConfig::Cascade(
                serde_json::from_value(inner.clone())
                    .map_err(|err| PyValueError::new_err(err.to_string()))?,
            )
        } else if let Some(inner) = value.get("hierarchical") {
            MatchingConfig::Hierarchical(
                serde_json::from_value(inner.clone())
                    .map_err(|err| PyValueError::new_err(err.to_string()))?,
            )
        } else {
            serde_json::from_value(value).map_err(|err| PyValueError::new_err(err.to_string()))?
        };
        Ok(Self::from_inner(inner))
    }
}

#[pyclass(
    module = "finstack.portfolio.factor_model",
    name = "FactorModelConfig",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyFactorModelConfig {
    pub(crate) inner: FactorModelConfig,
}

impl PyFactorModelConfig {
    fn from_inner(inner: FactorModelConfig) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyFactorModelConfig {
    #[new]
    #[pyo3(signature = (factors, covariance, matching, pricing_mode, risk_measure=None, bump_size=None, unmatched_policy=None))]
    fn new(
        factors: Vec<PyFactorDefinition>,
        covariance: PyRef<'_, PyFactorCovarianceMatrix>,
        matching: PyRef<'_, PyMatchingConfig>,
        pricing_mode: String,
        risk_measure: Option<&Bound<'_, PyAny>>,
        bump_size: Option<PyRef<'_, PyBumpSizeConfig>>,
        unmatched_policy: Option<String>,
    ) -> PyResult<Self> {
        let config = FactorModelConfig {
            factors: factors.into_iter().map(|factor| factor.inner).collect(),
            covariance: covariance.inner.clone(),
            matching: matching.inner.clone(),
            pricing_mode: parse_pricing_mode(&pricing_mode)?,
            risk_measure: parse_risk_measure(risk_measure)?,
            bump_size: bump_size.map(|config| config.inner.clone()),
            unmatched_policy: unmatched_policy
                .as_deref()
                .map(parse_unmatched_policy)
                .transpose()?,
        };
        Ok(Self::from_inner(build_validated_config(config)?))
    }

    #[getter]
    fn factors(&self) -> Vec<PyFactorDefinition> {
        self.inner
            .factors
            .iter()
            .cloned()
            .map(PyFactorDefinition::from_inner)
            .collect()
    }

    #[getter]
    fn covariance(&self) -> PyFactorCovarianceMatrix {
        PyFactorCovarianceMatrix::from_inner(self.inner.covariance.clone())
    }

    #[getter]
    fn matching(&self) -> PyMatchingConfig {
        PyMatchingConfig::from_inner(self.inner.matching.clone())
    }

    #[getter]
    fn pricing_mode(&self) -> String {
        pricing_mode_to_string(self.inner.pricing_mode)
    }

    #[getter]
    fn risk_measure(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        risk_measure_to_py(py, &self.inner.risk_measure)
    }

    #[getter]
    fn bump_size(&self) -> Option<PyBumpSizeConfig> {
        self.inner
            .bump_size
            .clone()
            .map(PyBumpSizeConfig::from_inner)
    }

    #[getter]
    fn unmatched_policy(&self) -> Option<String> {
        self.inner.unmatched_policy.map(unmatched_policy_to_string)
    }

    fn to_json(&self) -> PyResult<String> {
        mapping_to_json(&self.inner)
    }

    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let config: FactorModelConfig =
            serde_json::from_str(json).map_err(|err| PyValueError::new_err(err.to_string()))?;
        Ok(Self::from_inner(build_validated_config(config)?))
    }
}

#[pyclass(
    module = "finstack.portfolio.factor_model",
    name = "PositionAssignment",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyPositionAssignment {
    pub(crate) inner: PositionAssignment,
}

impl PyPositionAssignment {
    fn from_inner(inner: PositionAssignment) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyPositionAssignment {
    #[getter]
    fn position_id(&self) -> String {
        self.inner.position_id.as_str().to_string()
    }

    #[getter]
    fn mappings(&self) -> Vec<(PyMarketDependency, String)> {
        self.inner
            .mappings
            .iter()
            .cloned()
            .map(|(dependency, factor_id)| {
                (
                    PyMarketDependency::from_inner(dependency),
                    factor_id.as_str().to_string(),
                )
            })
            .collect()
    }
}

#[pyclass(
    module = "finstack.portfolio.factor_model",
    name = "UnmatchedEntry",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyUnmatchedEntry {
    pub(crate) inner: UnmatchedEntry,
}

impl PyUnmatchedEntry {
    fn from_inner(inner: UnmatchedEntry) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyUnmatchedEntry {
    #[getter]
    fn position_id(&self) -> String {
        self.inner.position_id.as_str().to_string()
    }

    #[getter]
    fn dependency(&self) -> PyMarketDependency {
        PyMarketDependency::from_inner(self.inner.dependency.clone())
    }
}

#[pyclass(
    module = "finstack.portfolio.factor_model",
    name = "FactorAssignmentReport",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyFactorAssignmentReport {
    pub(crate) inner: FactorAssignmentReport,
}

impl PyFactorAssignmentReport {
    fn from_inner(inner: FactorAssignmentReport) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyFactorAssignmentReport {
    #[getter]
    fn assignments(&self) -> Vec<PyPositionAssignment> {
        self.inner
            .assignments
            .iter()
            .cloned()
            .map(PyPositionAssignment::from_inner)
            .collect()
    }

    #[getter]
    fn unmatched(&self) -> Vec<PyUnmatchedEntry> {
        self.inner
            .unmatched
            .iter()
            .cloned()
            .map(PyUnmatchedEntry::from_inner)
            .collect()
    }
}

#[pyclass(
    module = "finstack.portfolio.factor_model",
    name = "SensitivityMatrix",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PySensitivityMatrix {
    pub(crate) inner: SensitivityMatrix,
}

impl PySensitivityMatrix {
    fn from_inner(inner: SensitivityMatrix) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PySensitivityMatrix {
    fn n_positions(&self) -> usize {
        self.inner.n_positions()
    }

    fn n_factors(&self) -> usize {
        self.inner.n_factors()
    }

    fn position_ids(&self) -> Vec<String> {
        self.inner.position_ids().to_vec()
    }

    fn factor_ids(&self) -> Vec<String> {
        self.inner
            .factor_ids()
            .iter()
            .map(|factor_id| factor_id.as_str().to_string())
            .collect()
    }

    fn delta(&self, position_idx: usize, factor_idx: usize) -> f64 {
        self.inner.delta(position_idx, factor_idx)
    }

    fn position_deltas(&self, position_idx: usize) -> Vec<f64> {
        self.inner.position_deltas(position_idx).to_vec()
    }

    fn factor_deltas(&self, factor_idx: usize) -> Vec<f64> {
        self.inner.factor_deltas(factor_idx)
    }
}

#[pyclass(
    module = "finstack.portfolio.factor_model",
    name = "FactorContribution",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyFactorContribution {
    pub(crate) inner: FactorContribution,
}

impl PyFactorContribution {
    fn from_inner(inner: FactorContribution) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyFactorContribution {
    #[getter]
    fn factor_id(&self) -> String {
        self.inner.factor_id.as_str().to_string()
    }

    #[getter]
    fn absolute_risk(&self) -> f64 {
        self.inner.absolute_risk
    }

    #[getter]
    fn relative_risk(&self) -> f64 {
        self.inner.relative_risk
    }

    #[getter]
    fn marginal_risk(&self) -> f64 {
        self.inner.marginal_risk
    }
}

#[pyclass(
    module = "finstack.portfolio.factor_model",
    name = "PositionFactorContribution",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyPositionFactorContribution {
    pub(crate) inner: PositionFactorContribution,
}

impl PyPositionFactorContribution {
    fn from_inner(inner: PositionFactorContribution) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyPositionFactorContribution {
    #[getter]
    fn position_id(&self) -> String {
        self.inner.position_id.as_str().to_string()
    }

    #[getter]
    fn factor_id(&self) -> String {
        self.inner.factor_id.as_str().to_string()
    }

    #[getter]
    fn risk_contribution(&self) -> f64 {
        self.inner.risk_contribution
    }
}

#[pyclass(
    module = "finstack.portfolio.factor_model",
    name = "RiskDecomposition",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyRiskDecomposition {
    pub(crate) inner: RiskDecomposition,
}

impl PyRiskDecomposition {
    fn from_inner(inner: RiskDecomposition) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyRiskDecomposition {
    #[getter]
    fn total_risk(&self) -> f64 {
        self.inner.total_risk
    }

    #[getter]
    fn measure(&self) -> String {
        match self.inner.measure {
            RiskMeasure::Variance => "Variance".to_string(),
            RiskMeasure::Volatility => "Volatility".to_string(),
            RiskMeasure::VaR { .. } => "VaR".to_string(),
            RiskMeasure::ExpectedShortfall { .. } => "ExpectedShortfall".to_string(),
        }
    }

    #[getter]
    fn factor_contributions(&self) -> Vec<PyFactorContribution> {
        self.inner
            .factor_contributions
            .iter()
            .cloned()
            .map(PyFactorContribution::from_inner)
            .collect()
    }

    #[getter]
    fn residual_risk(&self) -> f64 {
        self.inner.residual_risk
    }

    #[getter]
    fn position_factor_contributions(&self) -> Vec<PyPositionFactorContribution> {
        self.inner
            .position_factor_contributions
            .iter()
            .cloned()
            .map(PyPositionFactorContribution::from_inner)
            .collect()
    }
}

#[pyclass(
    module = "finstack.portfolio.factor_model",
    name = "FactorContributionDelta",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyFactorContributionDelta {
    pub(crate) inner: FactorContributionDelta,
}

impl PyFactorContributionDelta {
    fn from_inner(inner: FactorContributionDelta) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyFactorContributionDelta {
    #[getter]
    fn factor_id(&self) -> String {
        self.inner.factor_id.as_str().to_string()
    }

    #[getter]
    fn absolute_change(&self) -> f64 {
        self.inner.absolute_change
    }

    #[getter]
    fn relative_change(&self) -> f64 {
        self.inner.relative_change
    }
}

#[pyclass(
    module = "finstack.portfolio.factor_model",
    name = "WhatIfResult",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyWhatIfResult {
    pub(crate) inner: WhatIfResult,
}

impl PyWhatIfResult {
    fn from_inner(inner: WhatIfResult) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyWhatIfResult {
    #[getter]
    fn before(&self) -> PyRiskDecomposition {
        PyRiskDecomposition::from_inner(self.inner.before.clone())
    }

    #[getter]
    fn after(&self) -> PyRiskDecomposition {
        PyRiskDecomposition::from_inner(self.inner.after.clone())
    }

    #[getter]
    fn delta(&self) -> Vec<PyFactorContributionDelta> {
        self.inner
            .delta
            .iter()
            .cloned()
            .map(PyFactorContributionDelta::from_inner)
            .collect()
    }
}

#[pyclass(
    module = "finstack.portfolio.factor_model",
    name = "StressResult",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyStressResult {
    pub(crate) inner: StressResult,
}

impl PyStressResult {
    fn from_inner(inner: StressResult) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyStressResult {
    #[getter]
    fn total_pnl(&self) -> f64 {
        self.inner.total_pnl
    }

    #[getter]
    fn position_pnl(&self) -> Vec<(String, f64)> {
        self.inner
            .position_pnl
            .iter()
            .map(|(position_id, pnl)| (position_id.as_str().to_string(), *pnl))
            .collect()
    }

    #[getter]
    fn stressed_decomposition(&self) -> PyRiskDecomposition {
        PyRiskDecomposition::from_inner(self.inner.stressed_decomposition.clone())
    }
}

#[pyclass(
    module = "finstack.portfolio.factor_model",
    name = "PositionChange",
    from_py_object
)]
#[derive(Clone)]
pub struct PyPositionChange {
    pub(crate) inner: PositionChange,
}

impl PyPositionChange {
    fn from_inner(inner: PositionChange) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyPositionChange {
    #[staticmethod]
    fn add(position: PyRef<'_, PyPosition>) -> Self {
        Self::from_inner(PositionChange::Add {
            position: Box::new(position.inner.clone()),
        })
    }

    #[staticmethod]
    fn remove(position_id: String) -> Self {
        Self::from_inner(PositionChange::Remove {
            position_id: finstack_portfolio::PositionId::new(position_id),
        })
    }

    #[staticmethod]
    fn resize(position_id: String, new_quantity: f64) -> Self {
        Self::from_inner(PositionChange::Resize {
            position_id: finstack_portfolio::PositionId::new(position_id),
            new_quantity,
        })
    }
}

#[pyclass(
    module = "finstack.portfolio.factor_model",
    name = "FactorConstraint",
    from_py_object
)]
#[derive(Clone)]
pub struct PyFactorConstraint {
    pub(crate) inner: FactorConstraint,
}

impl PyFactorConstraint {
    fn from_inner(inner: FactorConstraint) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyFactorConstraint {
    #[staticmethod]
    fn max_factor_risk(factor_id: String, max_risk: f64) -> Self {
        Self::from_inner(FactorConstraint::MaxFactorRisk {
            factor_id: FactorId::new(factor_id),
            max_risk,
        })
    }

    #[staticmethod]
    fn max_factor_concentration(factor_id: String, max_fraction: f64) -> Self {
        Self::from_inner(FactorConstraint::MaxFactorConcentration {
            factor_id: FactorId::new(factor_id),
            max_fraction,
        })
    }

    #[staticmethod]
    fn factor_neutral(factor_id: String) -> Self {
        Self::from_inner(FactorConstraint::FactorNeutral {
            factor_id: FactorId::new(factor_id),
        })
    }
}

#[pyclass(
    module = "finstack.portfolio.factor_model",
    name = "FactorOptimizationResult",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyFactorOptimizationResult {
    pub(crate) inner: FactorOptimizationResult,
}

impl PyFactorOptimizationResult {
    fn from_inner(inner: FactorOptimizationResult) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyFactorOptimizationResult {
    #[getter]
    fn optimized_quantities(&self) -> Vec<(String, f64)> {
        self.inner
            .optimized_quantities
            .iter()
            .map(|(position_id, quantity)| (position_id.as_str().to_string(), *quantity))
            .collect()
    }
}

#[pyclass(
    module = "finstack.portfolio.factor_model",
    name = "FactorModelBuilder",
    skip_from_py_object
)]
pub struct PyFactorModelBuilder {
    inner: FactorModelBuilder,
}

#[pymethods]
impl PyFactorModelBuilder {
    #[new]
    fn new() -> Self {
        Self {
            inner: FactorModelBuilder::new(),
        }
    }

    fn config<'py>(
        mut slf: PyRefMut<'py, Self>,
        config: PyRef<'py, PyFactorModelConfig>,
    ) -> PyRefMut<'py, Self> {
        let builder = std::mem::replace(&mut slf.inner, FactorModelBuilder::new());
        slf.inner = builder.config(config.inner.clone());
        slf
    }

    fn build(mut slf: PyRefMut<'_, Self>) -> PyResult<PyFactorModel> {
        let builder = std::mem::replace(&mut slf.inner, FactorModelBuilder::new());
        let model = builder.build().map_err(portfolio_to_py)?;
        Ok(PyFactorModel {
            inner: Arc::new(model),
        })
    }
}

#[pyclass(
    module = "finstack.portfolio.factor_model",
    name = "FactorModel",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyFactorModel {
    inner: Arc<FactorModel>,
}

#[pymethods]
impl PyFactorModel {
    fn factors(&self) -> Vec<PyFactorDefinition> {
        self.inner
            .factors()
            .iter()
            .cloned()
            .map(PyFactorDefinition::from_inner)
            .collect()
    }

    fn assign_factors(
        &self,
        portfolio: PyRef<'_, PyPortfolio>,
    ) -> PyResult<PyFactorAssignmentReport> {
        self.inner
            .assign_factors(&portfolio.inner)
            .map(PyFactorAssignmentReport::from_inner)
            .map_err(portfolio_to_py)
    }

    fn compute_sensitivities(
        &self,
        portfolio: PyRef<'_, PyPortfolio>,
        market: PyRef<'_, PyMarketContext>,
        as_of: &Bound<'_, PyAny>,
    ) -> PyResult<PySensitivityMatrix> {
        let as_of = py_to_date(as_of)?;
        self.inner
            .compute_sensitivities(&portfolio.inner, &market.inner, as_of)
            .map(PySensitivityMatrix::from_inner)
            .map_err(portfolio_to_py)
    }

    fn analyze(
        &self,
        portfolio: PyRef<'_, PyPortfolio>,
        market: PyRef<'_, PyMarketContext>,
        as_of: &Bound<'_, PyAny>,
    ) -> PyResult<PyRiskDecomposition> {
        let as_of = py_to_date(as_of)?;
        self.inner
            .analyze(&portfolio.inner, &market.inner, as_of)
            .map(PyRiskDecomposition::from_inner)
            .map_err(portfolio_to_py)
    }

    fn what_if(
        &self,
        base: PyRef<'_, PyRiskDecomposition>,
        sensitivities: PyRef<'_, PySensitivityMatrix>,
        portfolio: PyRef<'_, PyPortfolio>,
        market: PyRef<'_, PyMarketContext>,
        as_of: &Bound<'_, PyAny>,
    ) -> PyResult<PyWhatIfEngine> {
        Ok(PyWhatIfEngine {
            model: self.inner.clone(),
            base: base.inner.clone(),
            sensitivities: sensitivities.inner.clone(),
            portfolio: portfolio.inner.clone(),
            market: market.inner.clone(),
            as_of: py_to_date(as_of)?,
        })
    }
}

#[pyclass(
    module = "finstack.portfolio.factor_model",
    name = "WhatIfEngine",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyWhatIfEngine {
    model: Arc<FactorModel>,
    base: RiskDecomposition,
    sensitivities: SensitivityMatrix,
    portfolio: finstack_portfolio::Portfolio,
    market: finstack_core::market_data::context::MarketContext,
    as_of: time::Date,
}

impl PyWhatIfEngine {
    fn with_inner<T>(
        &self,
        f: impl FnOnce(WhatIfEngine<'_>) -> finstack_portfolio::Result<T>,
    ) -> finstack_portfolio::Result<T> {
        let inner = WhatIfEngine::new(
            self.model.as_ref(),
            &self.base,
            &self.sensitivities,
            &self.portfolio,
            &self.market,
            self.as_of,
        );
        f(inner)
    }
}

#[pymethods]
impl PyWhatIfEngine {
    fn position_what_if(&self, changes: Vec<PyPositionChange>) -> PyResult<PyWhatIfResult> {
        let changes: Vec<PositionChange> = changes.into_iter().map(|change| change.inner).collect();
        self.with_inner(|inner| inner.position_what_if(&changes))
            .map(PyWhatIfResult::from_inner)
            .map_err(portfolio_to_py)
    }

    fn factor_stress(&self, stresses: Vec<(String, f64)>) -> PyResult<PyStressResult> {
        let stresses: Vec<(FactorId, f64)> = stresses
            .into_iter()
            .map(|(factor_id, shift)| (FactorId::new(factor_id), shift))
            .collect();
        self.with_inner(|inner| inner.factor_stress(&stresses))
            .map(PyStressResult::from_inner)
            .map_err(portfolio_to_py)
    }

    fn optimize(
        &self,
        constraints: Vec<PyFactorConstraint>,
    ) -> PyResult<PyFactorOptimizationResult> {
        let constraints: Vec<FactorConstraint> = constraints
            .into_iter()
            .map(|constraint| constraint.inner)
            .collect();
        self.with_inner(|inner| inner.optimize(&constraints))
            .map(PyFactorOptimizationResult::from_inner)
            .map_err(portfolio_to_py)
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<String>> {
    parent.add_class::<PyMarketDependency>()?;
    parent.add_class::<PyBumpSizeConfig>()?;
    parent.add_class::<PyMarketMapping>()?;
    parent.add_class::<PyFactorDefinition>()?;
    parent.add_class::<PyFactorCovarianceMatrix>()?;
    parent.add_class::<PyAttributeFilter>()?;
    parent.add_class::<PyDependencyFilter>()?;
    parent.add_class::<PyMappingRule>()?;
    parent.add_class::<PyFactorNode>()?;
    parent.add_class::<PyHierarchicalConfig>()?;
    parent.add_class::<PyMatchingConfig>()?;
    parent.add_class::<PyFactorModelConfig>()?;
    parent.add_class::<PyPositionAssignment>()?;
    parent.add_class::<PyUnmatchedEntry>()?;
    parent.add_class::<PyFactorAssignmentReport>()?;
    parent.add_class::<PySensitivityMatrix>()?;
    parent.add_class::<PyFactorContribution>()?;
    parent.add_class::<PyPositionFactorContribution>()?;
    parent.add_class::<PyRiskDecomposition>()?;
    parent.add_class::<PyFactorContributionDelta>()?;
    parent.add_class::<PyWhatIfResult>()?;
    parent.add_class::<PyStressResult>()?;
    parent.add_class::<PyPositionChange>()?;
    parent.add_class::<PyFactorConstraint>()?;
    parent.add_class::<PyFactorOptimizationResult>()?;
    parent.add_class::<PyFactorModelBuilder>()?;
    parent.add_class::<PyFactorModel>()?;
    parent.add_class::<PyWhatIfEngine>()?;

    Ok(vec![
        "MarketDependency".to_string(),
        "BumpSizeConfig".to_string(),
        "MarketMapping".to_string(),
        "FactorDefinition".to_string(),
        "FactorCovarianceMatrix".to_string(),
        "AttributeFilter".to_string(),
        "DependencyFilter".to_string(),
        "MappingRule".to_string(),
        "FactorNode".to_string(),
        "HierarchicalConfig".to_string(),
        "MatchingConfig".to_string(),
        "FactorModelConfig".to_string(),
        "PositionAssignment".to_string(),
        "UnmatchedEntry".to_string(),
        "FactorAssignmentReport".to_string(),
        "SensitivityMatrix".to_string(),
        "FactorContribution".to_string(),
        "PositionFactorContribution".to_string(),
        "RiskDecomposition".to_string(),
        "FactorContributionDelta".to_string(),
        "WhatIfResult".to_string(),
        "StressResult".to_string(),
        "PositionChange".to_string(),
        "FactorConstraint".to_string(),
        "FactorOptimizationResult".to_string(),
        "FactorModelBuilder".to_string(),
        "FactorModel".to_string(),
        "WhatIfEngine".to_string(),
    ])
}
