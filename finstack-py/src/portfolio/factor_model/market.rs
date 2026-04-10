use crate::errors::core_to_py;
use finstack_core::currency::Currency;
use finstack_core::factor_model::{
    BumpSizeConfig, CurveType, FactorCovarianceMatrix, FactorDefinition, FactorId,
    MarketDependency, MarketMapping,
};
use finstack_core::types::CurveId;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::Bound;

use super::helpers::{
    bump_units_to_string, currency_pair_string, factor_type_to_string, mapping_to_json,
    parse_bump_units, parse_factor_type,
};

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
    pub(super) fn from_inner(inner: MarketDependency) -> Self {
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
    pub(super) fn from_inner(inner: BumpSizeConfig) -> Self {
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
        let base: Currency = if let Ok(text) = base.extract::<String>() {
            text.parse()
                .map_err(|err| PyValueError::new_err(format!("Invalid base currency: {err}")))?
        } else {
            return Err(PyTypeError::new_err("base must be a currency code string"));
        };
        let quote: Currency = if let Ok(text) = quote.extract::<String>() {
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
    pub(super) fn from_inner(inner: FactorDefinition) -> Self {
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
    pub(super) fn from_inner(inner: FactorCovarianceMatrix) -> Self {
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
