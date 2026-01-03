//! Python bindings for VaR (Value-at-Risk) functionality.
//!
//! Thin wrappers around Rust VaR types with type conversion only.
//! All calculation logic remains in Rust.

use crate::core::dates::utils::py_to_date;
use crate::core::market_data::PyMarketContext;
use crate::valuations::instruments::{extract_instrument, InstrumentHandle};
use finstack_core::dates::Date;
use finstack_valuations::instruments::Instrument;
use finstack_valuations::metrics::risk::{
    calculate_var, MarketHistory, MarketScenario, RiskFactorShift, RiskFactorType, VarConfig,
    VarMethod, VarResult,
};
use finstack_valuations::metrics::{standard_registry, MetricContext, MetricId};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyModule, PySequence};
use pyo3::Bound;
use std::sync::Arc;

// =============================================================================
// VarMethod Enum
// =============================================================================

/// VaR calculation method.
///
/// Examples:
///     >>> config = VarConfig(confidence_level=0.95, method=VarMethod.FULL_REVALUATION)
#[pyclass(module = "finstack.valuations", name = "VarMethod", frozen)]
#[derive(Clone, Copy)]
pub struct PyVarMethod {
    inner: VarMethod,
}

#[pymethods]
impl PyVarMethod {
    #[classattr]
    const FULL_REVALUATION: Self = Self {
        inner: VarMethod::FullRevaluation,
    };

    #[classattr]
    const TAYLOR_APPROXIMATION: Self = Self {
        inner: VarMethod::TaylorApproximation,
    };

    fn __repr__(&self) -> &str {
        match self.inner {
            VarMethod::FullRevaluation => "VarMethod.FULL_REVALUATION",
            VarMethod::TaylorApproximation => "VarMethod.TAYLOR_APPROXIMATION",
        }
    }
}

// =============================================================================
// VarConfig
// =============================================================================

/// Configuration for VaR calculation.
///
/// Args:
///     confidence_level: Confidence level (0.0 to 1.0), e.g., 0.95 for 95% VaR
///     method: Calculation method (FULL_REVALUATION or TAYLOR_APPROXIMATION)
///
/// Examples:
///     >>> config = VarConfig(confidence_level=0.99)
///     >>> config.confidence_level
///     0.99
#[pyclass(module = "finstack.valuations", name = "VarConfig", frozen)]
#[derive(Clone)]
pub struct PyVarConfig {
    pub(crate) inner: VarConfig,
}

#[pymethods]
impl PyVarConfig {
    #[new]
    #[pyo3(signature = (confidence_level=0.95, method=None))]
    fn new(confidence_level: f64, method: Option<PyVarMethod>) -> PyResult<Self> {
        if !(0.0..=1.0).contains(&confidence_level) {
            return Err(PyValueError::new_err(
                "confidence_level must be between 0.0 and 1.0",
            ));
        }

        let var_method = method
            .map(|m| m.inner)
            .unwrap_or(VarMethod::FullRevaluation);

        Ok(Self {
            inner: VarConfig::new(confidence_level).with_method(var_method),
        })
    }

    #[staticmethod]
    fn var_95() -> Self {
        Self {
            inner: VarConfig::var_95(),
        }
    }

    #[staticmethod]
    fn var_99() -> Self {
        Self {
            inner: VarConfig::var_99(),
        }
    }

    #[getter]
    fn confidence_level(&self) -> f64 {
        self.inner.confidence_level
    }

    #[getter]
    fn method(&self) -> PyVarMethod {
        PyVarMethod {
            inner: self.inner.method,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "VarConfig(confidence_level={}, method={:?})",
            self.inner.confidence_level, self.inner.method
        )
    }
}

// =============================================================================
// VarResult
// =============================================================================

/// Result of VaR calculation.
///
/// Attributes:
///     var: Value-at-Risk at specified confidence level
///     expected_shortfall: Expected Shortfall (CVaR)
///     pnl_distribution: Full P&L distribution from historical simulation (sorted)
///     confidence_level: Confidence level used
///     num_scenarios: Number of scenarios simulated
///
/// Examples:
///     >>> result.var
///     -5432.10
///     >>> result.expected_shortfall
///     -6891.50
#[pyclass(module = "finstack.valuations", name = "VarResult", frozen)]
#[derive(Clone)]
pub struct PyVarResult {
    inner: VarResult,
}

#[pymethods]
impl PyVarResult {
    #[getter]
    fn var(&self) -> f64 {
        self.inner.var
    }

    #[getter]
    fn expected_shortfall(&self) -> f64 {
        self.inner.expected_shortfall
    }

    #[getter]
    fn pnl_distribution<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        PyList::new(py, &self.inner.pnl_distribution)
    }

    #[getter]
    fn confidence_level(&self) -> f64 {
        self.inner.confidence_level
    }

    #[getter]
    fn num_scenarios(&self) -> usize {
        self.inner.num_scenarios
    }

    fn __repr__(&self) -> String {
        format!(
            "VarResult(var={:.2}, expected_shortfall={:.2}, num_scenarios={})",
            self.inner.var, self.inner.expected_shortfall, self.inner.num_scenarios
        )
    }
}

// =============================================================================
// RiskFactorType
// =============================================================================

/// Risk factor type for VaR scenarios.
///
/// Examples:
///     >>> shift = RiskFactorShift(
///     ...     factor=RiskFactorType.discount_rate("USD-OIS", 5.0),
///     ...     shift=0.01  # 1% increase
///     ... )
#[pyclass(module = "finstack.valuations", name = "RiskFactorType", frozen)]
#[derive(Clone)]
pub struct PyRiskFactorType {
    pub(crate) inner: RiskFactorType,
}

#[pymethods]
impl PyRiskFactorType {
    #[staticmethod]
    fn discount_rate(curve_id: String, tenor_years: f64) -> Self {
        Self {
            inner: RiskFactorType::DiscountRate {
                curve_id: curve_id.into(),
                tenor_years,
            },
        }
    }

    #[staticmethod]
    fn forward_rate(curve_id: String, tenor_years: f64) -> Self {
        Self {
            inner: RiskFactorType::ForwardRate {
                curve_id: curve_id.into(),
                tenor_years,
            },
        }
    }

    #[staticmethod]
    fn credit_spread(curve_id: String, tenor_years: f64) -> Self {
        Self {
            inner: RiskFactorType::CreditSpread {
                curve_id: curve_id.into(),
                tenor_years,
            },
        }
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }
}

// =============================================================================
// Market History Types
// =============================================================================

/// Single risk factor shift for a scenario.
///
/// Args:
///     factor: Risk factor being shifted
///     shift: Absolute change in the factor
#[pyclass(module = "finstack.valuations", name = "RiskFactorShift", frozen)]
#[derive(Clone)]
pub struct PyRiskFactorShift {
    pub(crate) inner: RiskFactorShift,
}

#[pymethods]
impl PyRiskFactorShift {
    #[new]
    fn new(factor: PyRiskFactorType, shift: f64) -> Self {
        Self {
            inner: RiskFactorShift {
                factor: factor.inner,
                shift,
            },
        }
    }

    #[getter]
    fn shift(&self) -> f64 {
        self.inner.shift
    }
}

/// Historical market scenario for a single date.
///
/// Args:
///     date: Scenario date
///     shifts: List of risk factor shifts for this date
#[pyclass(module = "finstack.valuations", name = "MarketScenario", frozen)]
#[derive(Clone)]
pub struct PyMarketScenario {
    pub(crate) inner: MarketScenario,
}

#[pymethods]
impl PyMarketScenario {
    #[new]
    fn new(date: (i32, u8, u8), shifts: Vec<PyRiskFactorShift>) -> PyResult<Self> {
        let date = Date::from_calendar_date(
            date.0,
            time::Month::try_from(date.1).map_err(|_| PyValueError::new_err("Invalid month"))?,
            date.2,
        )
        .map_err(|e| PyValueError::new_err(format!("Invalid date: {}", e)))?;

        let shifts = shifts.into_iter().map(|s| s.inner).collect();

        Ok(Self {
            inner: MarketScenario::new(date, shifts),
        })
    }
}

/// Historical market data for VaR calculation.
///
/// Args:
///     base_date: Base date (current market state)
///     window_days: Historical window size in days
///     scenarios: List of historical scenarios
#[pyclass(module = "finstack.valuations", name = "MarketHistory", frozen)]
#[derive(Clone)]
pub struct PyMarketHistory {
    pub(crate) inner: Arc<MarketHistory>,
}

#[pymethods]
impl PyMarketHistory {
    #[new]
    fn new(
        base_date: (i32, u8, u8),
        window_days: u32,
        scenarios: Vec<PyMarketScenario>,
    ) -> PyResult<Self> {
        let date = Date::from_calendar_date(
            base_date.0,
            time::Month::try_from(base_date.1)
                .map_err(|_| PyValueError::new_err("Invalid month"))?,
            base_date.2,
        )
        .map_err(|e| PyValueError::new_err(format!("Invalid date: {}", e)))?;

        let scenarios = scenarios.into_iter().map(|s| s.inner).collect();

        Ok(Self {
            inner: Arc::new(MarketHistory::new(date, window_days, scenarios)),
        })
    }

    #[getter]
    fn num_scenarios(&self) -> usize {
        self.inner.len()
    }
}

// =============================================================================
// VaR Calculation Functions
// =============================================================================

/// Calculate Historical VaR for one or more instruments.
///
/// Args:
///     instruments: Instrument or list of instruments to calculate VaR for
///     market: Market data (curves, surfaces, etc.)
///     history: Historical market scenarios
///     as_of: Valuation date as (year, month, day)
///     config: VaR configuration
///
/// Returns:
///     VarResult: VaR and Expected Shortfall
///
/// Examples:
///     >>> result = calculate_var([bond1, bond2], market, history, (2024, 1, 1), config)
///     >>> print(f"95% VaR: ${result.var:.2f}")
#[pyfunction(name = "calculate_var")]
#[pyo3(signature = (instruments, market, history, as_of, config))]
fn py_calculate_var(
    instruments: Bound<'_, PyAny>,
    market: &PyMarketContext,
    history: &PyMarketHistory,
    as_of: (i32, u8, u8),
    config: &PyVarConfig,
) -> PyResult<PyVarResult> {
    let as_of_date = Date::from_calendar_date(
        as_of.0,
        time::Month::try_from(as_of.1).map_err(|_| PyValueError::new_err("Invalid month"))?,
        as_of.2,
    )
    .map_err(|e| PyValueError::new_err(format!("Invalid date: {}", e)))?;

    let mut handles = Vec::new();
    if let Ok(handle) = extract_instrument(&instruments) {
        handles.push(handle);
    } else if let Ok(seq) = instruments.downcast::<PySequence>() {
        let length = seq.len().map_err(|e| {
            PyValueError::new_err(format!("Failed to inspect instruments sequence: {}", e))
        })?;
        for idx in 0..length {
            let item = seq.get_item(idx).map_err(|e| {
                PyValueError::new_err(format!("Failed to read instrument at index {}: {}", idx, e))
            })?;
            handles.push(extract_instrument(&item)?);
        }
    } else {
        return Err(PyValueError::new_err(
            "Expected an instrument or a sequence of instruments",
        ));
    }

    let inst_refs: Vec<&dyn Instrument> = handles
        .iter()
        .map(|handle| handle.instrument.as_ref() as &dyn Instrument)
        .collect();

    let result = calculate_var(
        &inst_refs,
        &market.inner,
        &history.inner,
        as_of_date,
        &config.inner,
    )
    .map_err(|e| PyValueError::new_err(format!("VaR calculation failed: {}", e)))?;

    Ok(PyVarResult { inner: result })
}

// =============================================================================
// Ladder Helpers (KRD DV01 / CS01)
// =============================================================================

fn bucketed_metric(
    py: Python<'_>,
    instrument: Bound<'_, PyAny>,
    market: &PyMarketContext,
    as_of: Bound<'_, PyAny>,
    metric_id: MetricId,
    value_key: &str,
) -> PyResult<Py<PyAny>> {
    let InstrumentHandle {
        instrument: inst, ..
    } = extract_instrument(&instrument)?;
    let as_of_date = py_to_date(&as_of)?;

    let base_value = inst
        .value(&market.inner, as_of_date)
        .map_err(|e| PyValueError::new_err(format!("Pricing failed: {}", e)))?;

    let mut context = MetricContext::new(
        inst.clone(),
        Arc::new(market.inner.clone()),
        as_of_date,
        base_value,
        MetricContext::default_config(),
    );

    let registry = standard_registry();
    registry
        .compute(&[metric_id.clone()], &mut context)
        .map_err(|e| PyValueError::new_err(format!("Metric computation failed: {}", e)))?;

    let series = context
        .computed_series
        .get(&metric_id)
        .ok_or_else(|| PyValueError::new_err(format!("{} series not available", value_key)))?;

    let bucket_labels: Vec<String> = series.iter().map(|(b, _)| b.clone()).collect();
    let values: Vec<f64> = series.iter().map(|(_, v)| *v).collect();

    let result = PyDict::new(py);
    let bucket_list = PyList::new(py, &bucket_labels)?;
    let value_list = PyList::new(py, &values)?;
    result.set_item("bucket", bucket_list)?;
    result.set_item(value_key, value_list)?;
    Ok(result.into())
}

/// Compute key-rate DV01 ladder using core metric calculators.
#[pyfunction(name = "krd_dv01_ladder")]
#[pyo3(
    signature = (instrument, market, as_of),
    text_signature = "(instrument, market, as_of)"
)]
fn py_krd_dv01_ladder(
    py: Python<'_>,
    instrument: Bound<'_, PyAny>,
    market: &PyMarketContext,
    as_of: Bound<'_, PyAny>,
) -> PyResult<Py<PyAny>> {
    bucketed_metric(
        py,
        instrument,
        market,
        as_of,
        MetricId::BucketedDv01,
        "dv01",
    )
}

/// Compute key-rate CS01 ladder using core metric calculators.
#[pyfunction(name = "cs01_ladder")]
#[pyo3(
    signature = (instrument, market, as_of),
    text_signature = "(instrument, market, as_of)"
)]
fn py_cs01_ladder(
    py: Python<'_>,
    instrument: Bound<'_, PyAny>,
    market: &PyMarketContext,
    as_of: Bound<'_, PyAny>,
) -> PyResult<Py<PyAny>> {
    bucketed_metric(
        py,
        instrument,
        market,
        as_of,
        MetricId::BucketedCs01,
        "cs01",
    )
}

// =============================================================================
// Module Registration
// =============================================================================

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "risk")?;
    module.setattr(
        "__doc__",
        "Value-at-Risk (VaR) calculation and risk metrics.",
    )?;

    // Add classes
    module.add_class::<PyVarMethod>()?;
    module.add_class::<PyVarConfig>()?;
    module.add_class::<PyVarResult>()?;
    module.add_class::<PyRiskFactorType>()?;
    module.add_class::<PyRiskFactorShift>()?;
    module.add_class::<PyMarketScenario>()?;
    module.add_class::<PyMarketHistory>()?;

    // Add functions
    module.add_function(wrap_pyfunction!(py_calculate_var, &module)?)?;
    module.add_function(wrap_pyfunction!(py_krd_dv01_ladder, &module)?)?;
    module.add_function(wrap_pyfunction!(py_cs01_ladder, &module)?)?;

    let exports = vec![
        "VarMethod",
        "VarConfig",
        "VarResult",
        "RiskFactorType",
        "RiskFactorShift",
        "MarketScenario",
        "MarketHistory",
        "calculate_var",
        "krd_dv01_ladder",
        "cs01_ladder",
    ];

    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports)
}
