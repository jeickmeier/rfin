use super::helpers::{
    wrap_base_correlation_curves, wrap_discount_curves, wrap_forward_curves, wrap_hazard_curves,
    wrap_inflation_curves,
};
use crate::core::market_data::context::PyMarketContext;
use crate::core::market_data::scalars::{PyMarketScalar, PyScalarTimeSeries};
use crate::core::market_data::surfaces::PyVolSurface;
use crate::core::market_data::term_structures::{
    PyBaseCorrelationCurve, PyDiscountCurve, PyForwardCurve, PyHazardCurve, PyInflationCurve,
};
use finstack_core::HashMap;
use finstack_valuations::attribution::{
    AttributionConfig, CurveRestoreFlags, MarketSnapshot, ModelParamsSnapshot, ScalarsSnapshot,
    TaylorAttributionConfig, TaylorAttributionResult, TaylorFactorResult, VolatilitySnapshot,
};
use pyo3::prelude::*;
use pyo3::types::PyType;

/// Python wrapper for TaylorAttributionConfig.
#[pyclass(name = "TaylorAttributionConfig", from_py_object)]
#[derive(Clone)]
pub struct PyTaylorAttributionConfig {
    pub(crate) inner: TaylorAttributionConfig,
}

#[pymethods]
impl PyTaylorAttributionConfig {
    #[new]
    #[pyo3(signature = (*, include_gamma=false, rate_bump_bp=1.0, credit_bump_bp=1.0, vol_bump=0.01))]
    fn new(include_gamma: bool, rate_bump_bp: f64, credit_bump_bp: f64, vol_bump: f64) -> Self {
        Self {
            inner: TaylorAttributionConfig {
                include_gamma,
                rate_bump_bp,
                credit_bump_bp,
                vol_bump,
            },
        }
    }

    #[getter]
    fn include_gamma(&self) -> bool {
        self.inner.include_gamma
    }

    #[getter]
    fn rate_bump_bp(&self) -> f64 {
        self.inner.rate_bump_bp
    }

    #[getter]
    fn credit_bump_bp(&self) -> f64 {
        self.inner.credit_bump_bp
    }

    #[getter]
    fn vol_bump(&self) -> f64 {
        self.inner.vol_bump
    }
}

/// Python wrapper for TaylorFactorResult.
#[pyclass(name = "TaylorFactorResult", from_py_object)]
#[derive(Clone)]
pub struct PyTaylorFactorResult {
    pub(crate) inner: TaylorFactorResult,
}

#[pymethods]
impl PyTaylorFactorResult {
    #[new]
    #[pyo3(signature = (factor_name, sensitivity, market_move, explained_pnl, *, gamma_pnl=None))]
    fn new(
        factor_name: String,
        sensitivity: f64,
        market_move: f64,
        explained_pnl: f64,
        gamma_pnl: Option<f64>,
    ) -> Self {
        Self {
            inner: TaylorFactorResult {
                factor_name,
                sensitivity,
                market_move,
                explained_pnl,
                gamma_pnl,
            },
        }
    }

    #[getter]
    fn factor_name(&self) -> &str {
        &self.inner.factor_name
    }

    #[getter]
    fn sensitivity(&self) -> f64 {
        self.inner.sensitivity
    }

    #[getter]
    fn market_move(&self) -> f64 {
        self.inner.market_move
    }

    #[getter]
    fn explained_pnl(&self) -> f64 {
        self.inner.explained_pnl
    }

    #[getter]
    fn gamma_pnl(&self) -> Option<f64> {
        self.inner.gamma_pnl
    }
}

/// Python wrapper for TaylorAttributionResult.
#[pyclass(name = "TaylorAttributionResult", from_py_object)]
#[derive(Clone)]
pub struct PyTaylorAttributionResult {
    pub(crate) inner: TaylorAttributionResult,
}

#[pymethods]
impl PyTaylorAttributionResult {
    #[new]
    #[pyo3(signature = (actual_pnl, total_explained, unexplained, unexplained_pct, factors, num_repricings, pv_t0, pv_t1))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        actual_pnl: f64,
        total_explained: f64,
        unexplained: f64,
        unexplained_pct: f64,
        factors: Vec<PyTaylorFactorResult>,
        num_repricings: usize,
        pv_t0: crate::core::money::PyMoney,
        pv_t1: crate::core::money::PyMoney,
    ) -> Self {
        Self {
            inner: TaylorAttributionResult {
                actual_pnl,
                total_explained,
                unexplained,
                unexplained_pct,
                factors: factors.into_iter().map(|value| value.inner).collect(),
                num_repricings,
                pv_t0: pv_t0.inner,
                pv_t1: pv_t1.inner,
            },
        }
    }

    #[getter]
    fn actual_pnl(&self) -> f64 {
        self.inner.actual_pnl
    }

    #[getter]
    fn total_explained(&self) -> f64 {
        self.inner.total_explained
    }

    #[getter]
    fn unexplained(&self) -> f64 {
        self.inner.unexplained
    }

    #[getter]
    fn unexplained_pct(&self) -> f64 {
        self.inner.unexplained_pct
    }

    #[getter]
    fn factors(&self) -> Vec<PyTaylorFactorResult> {
        self.inner
            .factors
            .iter()
            .cloned()
            .map(|inner| PyTaylorFactorResult { inner })
            .collect()
    }

    #[getter]
    fn num_repricings(&self) -> usize {
        self.inner.num_repricings
    }

    #[getter]
    fn pv_t0(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.pv_t0,
        }
    }

    #[getter]
    fn pv_t1(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.pv_t1,
        }
    }
}

/// Python wrapper for AttributionConfig.
#[pyclass(name = "AttributionConfig", from_py_object)]
#[derive(Clone)]
pub struct PyAttributionConfig {
    pub(crate) inner: AttributionConfig,
}

#[pymethods]
impl PyAttributionConfig {
    #[new]
    #[pyo3(signature = (*, tolerance_abs=None, tolerance_pct=None, metrics=None, strict_validation=None, rounding_scale=None, rate_bump_bp=None))]
    fn new(
        tolerance_abs: Option<f64>,
        tolerance_pct: Option<f64>,
        metrics: Option<Vec<String>>,
        strict_validation: Option<bool>,
        rounding_scale: Option<u32>,
        rate_bump_bp: Option<f64>,
    ) -> Self {
        Self {
            inner: AttributionConfig {
                tolerance_abs,
                tolerance_pct,
                metrics,
                strict_validation,
                rounding_scale,
                rate_bump_bp,
            },
        }
    }

    #[getter]
    fn tolerance_abs(&self) -> Option<f64> {
        self.inner.tolerance_abs
    }

    #[getter]
    fn tolerance_pct(&self) -> Option<f64> {
        self.inner.tolerance_pct
    }

    #[getter]
    fn metrics(&self) -> Option<Vec<String>> {
        self.inner.metrics.clone()
    }

    #[getter]
    fn strict_validation(&self) -> Option<bool> {
        self.inner.strict_validation
    }

    #[getter]
    fn rounding_scale(&self) -> Option<u32> {
        self.inner.rounding_scale
    }

    #[getter]
    fn rate_bump_bp(&self) -> Option<f64> {
        self.inner.rate_bump_bp
    }

    fn __repr__(&self) -> String {
        format!(
            "AttributionConfig(tolerance_abs={:?}, tolerance_pct={:?}, strict={:?})",
            self.inner.tolerance_abs, self.inner.tolerance_pct, self.inner.strict_validation
        )
    }
}

/// Python wrapper for ModelParamsSnapshot.
#[pyclass(name = "ModelParamsSnapshot", from_py_object)]
#[derive(Clone)]
pub struct PyModelParamsSnapshot {
    pub(crate) inner: ModelParamsSnapshot,
}

#[pymethods]
impl PyModelParamsSnapshot {
    /// Construct from a JSON string or Python dict.
    #[classmethod]
    fn from_json(_cls: &Bound<'_, PyType>, json_str: &str) -> PyResult<Self> {
        let snapshot: ModelParamsSnapshot = serde_json::from_str(json_str).map_err(|err| {
            pyo3::exceptions::PyValueError::new_err(format!(
                "Invalid ModelParamsSnapshot JSON: {err}"
            ))
        })?;
        Ok(Self { inner: snapshot })
    }

    /// Return a snapshot representing no model parameters.
    #[staticmethod]
    fn none() -> Self {
        Self {
            inner: ModelParamsSnapshot::None,
        }
    }

    /// Serialize to JSON string.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(|err| {
            pyo3::exceptions::PyRuntimeError::new_err(format!(
                "Failed to serialize ModelParamsSnapshot: {err}"
            ))
        })
    }

    /// True if this is the None variant (no model params).
    fn is_none(&self) -> bool {
        matches!(self.inner, ModelParamsSnapshot::None)
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            ModelParamsSnapshot::StructuredCredit { .. } => {
                "ModelParamsSnapshot(StructuredCredit)".to_string()
            }
            ModelParamsSnapshot::Convertible { .. } => {
                "ModelParamsSnapshot(Convertible)".to_string()
            }
            ModelParamsSnapshot::None => "ModelParamsSnapshot(None)".to_string(),
        }
    }
}

/// Python wrapper for CurveRestoreFlags.
#[pyclass(name = "CurveRestoreFlags", frozen, from_py_object)]
#[derive(Clone, Copy)]
pub struct PyCurveRestoreFlags {
    pub(crate) inner: CurveRestoreFlags,
}

#[pymethods]
impl PyCurveRestoreFlags {
    #[classattr]
    const DISCOUNT: Self = Self {
        inner: CurveRestoreFlags::DISCOUNT,
    };

    #[classattr]
    const FORWARD: Self = Self {
        inner: CurveRestoreFlags::FORWARD,
    };

    #[classattr]
    const HAZARD: Self = Self {
        inner: CurveRestoreFlags::HAZARD,
    };

    #[classattr]
    const INFLATION: Self = Self {
        inner: CurveRestoreFlags::INFLATION,
    };

    #[classattr]
    const CORRELATION: Self = Self {
        inner: CurveRestoreFlags::CORRELATION,
    };

    #[classattr]
    const RATES: Self = Self {
        inner: CurveRestoreFlags::RATES,
    };

    #[classattr]
    const CREDIT: Self = Self {
        inner: CurveRestoreFlags::CREDIT,
    };

    #[classmethod]
    fn all(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: CurveRestoreFlags::all(),
        }
    }

    #[classmethod]
    fn empty(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: CurveRestoreFlags::empty(),
        }
    }

    fn contains(&self, other: &PyCurveRestoreFlags) -> bool {
        self.inner.contains(other.inner)
    }

    fn __or__(&self, other: &PyCurveRestoreFlags) -> Self {
        Self {
            inner: self.inner | other.inner,
        }
    }

    fn __and__(&self, other: &PyCurveRestoreFlags) -> Self {
        Self {
            inner: self.inner & other.inner,
        }
    }

    fn __invert__(&self) -> Self {
        Self { inner: !self.inner }
    }
}

/// Python wrapper for MarketSnapshot.
#[pyclass(name = "MarketSnapshot", from_py_object)]
#[derive(Clone, Default)]
pub struct PyMarketSnapshot {
    pub(crate) inner: MarketSnapshot,
}

#[pymethods]
impl PyMarketSnapshot {
    #[classmethod]
    fn extract(
        _cls: &Bound<'_, PyType>,
        market: &PyMarketContext,
        flags: &PyCurveRestoreFlags,
    ) -> Self {
        Self {
            inner: MarketSnapshot::extract(&market.inner, flags.inner),
        }
    }

    #[staticmethod]
    fn restore_market(
        current_market: &PyMarketContext,
        snapshot: &PyMarketSnapshot,
        restore_flags: &PyCurveRestoreFlags,
    ) -> PyMarketContext {
        PyMarketContext {
            inner: MarketSnapshot::restore_market(
                &current_market.inner,
                &snapshot.inner,
                restore_flags.inner,
            ),
        }
    }

    fn discount_curves(&self) -> HashMap<String, PyDiscountCurve> {
        wrap_discount_curves(&self.inner.discount_curves)
    }

    fn forward_curves(&self) -> HashMap<String, PyForwardCurve> {
        wrap_forward_curves(&self.inner.forward_curves)
    }

    fn hazard_curves(&self) -> HashMap<String, PyHazardCurve> {
        wrap_hazard_curves(&self.inner.hazard_curves)
    }

    fn inflation_curves(&self) -> HashMap<String, PyInflationCurve> {
        wrap_inflation_curves(&self.inner.inflation_curves)
    }

    fn base_correlation_curves(&self) -> HashMap<String, PyBaseCorrelationCurve> {
        wrap_base_correlation_curves(&self.inner.base_correlation_curves)
    }
}

/// Python wrapper for VolatilitySnapshot.
#[pyclass(name = "VolatilitySnapshot", from_py_object)]
#[derive(Clone)]
pub struct PyVolatilitySnapshot {
    pub(crate) inner: VolatilitySnapshot,
}

#[pymethods]
impl PyVolatilitySnapshot {
    #[classmethod]
    fn extract(_cls: &Bound<'_, PyType>, market: &PyMarketContext) -> Self {
        Self {
            inner: VolatilitySnapshot::extract(&market.inner),
        }
    }

    fn surfaces(&self) -> HashMap<String, PyVolSurface> {
        self.inner
            .surfaces
            .iter()
            .map(|(key, value)| (key.to_string(), PyVolSurface::new_arc(value.clone())))
            .collect()
    }
}

/// Python wrapper for ScalarsSnapshot.
#[pyclass(name = "ScalarsSnapshot", from_py_object)]
#[derive(Clone)]
pub struct PyScalarsSnapshot {
    pub(crate) inner: ScalarsSnapshot,
}

#[pymethods]
impl PyScalarsSnapshot {
    #[classmethod]
    fn extract(_cls: &Bound<'_, PyType>, market: &PyMarketContext) -> Self {
        Self {
            inner: ScalarsSnapshot::extract(&market.inner),
        }
    }

    fn prices(&self) -> HashMap<String, PyMarketScalar> {
        self.inner
            .prices
            .iter()
            .map(|(key, value)| (key.to_string(), PyMarketScalar::new(value.clone())))
            .collect()
    }

    fn series(&self) -> HashMap<String, PyScalarTimeSeries> {
        self.inner
            .series
            .iter()
            .map(|(key, value)| (key.to_string(), PyScalarTimeSeries::new_arc(value.clone())))
            .collect()
    }
}
