//! Scenario engine and execution context.

use crate::core::dates::calendar::PyCalendar;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::market_data::context::PyMarketContext;
use crate::scenarios::error::scenario_to_py;
use crate::scenarios::reports::PyApplicationReport;
use crate::scenarios::spec::{PyRateBindingSpec, PyScenarioSpec};
use crate::statements::types::model::PyFinancialModelSpec;
use crate::valuations::instruments::extract_instrument;
use finstack_core::HashMap;
use finstack_scenarios::engine::{ExecutionContext, ScenarioEngine};
use finstack_scenarios::spec::RateBindingSpec;
use finstack_scenarios::ScenarioSpec;
use finstack_valuations::instruments::Instrument;
use indexmap::IndexMap;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyAnyMethods, PyModule};

/// Execution context for scenario application.
///
/// Holds all mutable state that a scenario can touch — market data,
/// statement models, instruments, and rate bindings — together with
/// the current valuation date.
///
/// Parameters
/// ----------
/// market : MarketContext
///     Market data context (curves, surfaces, FX, etc.).
/// model : FinancialModelSpec
///     Financial statements model.
/// as_of : date
///     Valuation date for context.
/// instruments : list, optional
///     Optional vector of instruments for price/spread shocks and carry calculations.
/// rate_bindings : dict[str, RateBindingSpec] | list[RateBindingSpec] | dict[str, str], optional
///     Optional mapping from statement node IDs to rate binding specifications. Legacy dict[str, str]
///     is supported for backwards compatibility (assumes 1Y continuous rates).
/// calendar : Calendar, optional
///     Optional holiday calendar for calendar-aware tenor calculations.
///
/// Examples
/// --------
/// >>> from finstack.scenarios import ExecutionContext
/// >>> from finstack.core import MarketContext
/// >>> from finstack.statements import FinancialModelSpec
/// >>> from datetime import date
/// >>> market = MarketContext()
/// >>> model = FinancialModelSpec("demo", [])
/// >>> ctx = ExecutionContext(market, model, date(2025, 1, 1))
#[pyclass(module = "finstack.scenarios", name = "ExecutionContext")]
pub struct PyExecutionContext {
    market: Py<PyMarketContext>,
    model: Py<PyFinancialModelSpec>,
    instruments: Option<Vec<Py<PyAny>>>,
    rust_instruments: Option<Vec<Box<dyn Instrument>>>,
    rate_bindings: Option<IndexMap<String, RateBindingSpec>>,
    calendar: Option<Py<PyCalendar>>,
    as_of: finstack_core::dates::Date,
}

impl PyExecutionContext {
    fn convert_instruments(
        py: Python<'_>,
        instruments: &Option<Vec<Py<PyAny>>>,
    ) -> PyResult<Option<Vec<Box<dyn Instrument>>>> {
        if let Some(list) = instruments {
            let mut rust_instruments = Vec::with_capacity(list.len());
            for obj in list {
                let bound = obj.bind(py);
                let handle = extract_instrument(&bound)?;
                rust_instruments.push(handle.instrument);
            }
            Ok(Some(rust_instruments))
        } else {
            Ok(None)
        }
    }

    fn convert_rate_bindings(
        py: Python<'_>,
        bindings: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Option<IndexMap<String, RateBindingSpec>>> {
        let Some(obj) = bindings else {
            return Ok(None);
        };

        if obj.is_none() {
            return Ok(None);
        }

        // Preferred: dict[str, RateBindingSpec]
        if let Ok(map) = obj.extract::<HashMap<String, Py<PyRateBindingSpec>>>() {
            let mut out: IndexMap<String, RateBindingSpec> = IndexMap::with_capacity(map.len());
            for (node_id, spec_obj) in map {
                let borrowed = spec_obj.borrow(py);
                out.insert(node_id, borrowed.inner.clone());
            }
            return Ok(Some(out));
        }

        // Fallback: list[RateBindingSpec]
        if let Ok(list) = obj.extract::<Vec<Py<PyRateBindingSpec>>>() {
            let mut out: IndexMap<String, RateBindingSpec> = IndexMap::with_capacity(list.len());
            for spec_obj in list {
                let borrowed = spec_obj.borrow(py);
                let spec = borrowed.inner.clone();
                out.insert(spec.node_id.clone(), spec);
            }
            return Ok(Some(out));
        }

        // Legacy: dict[str, str] mapping node_id -> curve_id
        if let Ok(map) = obj.extract::<HashMap<String, String>>() {
            let legacy: IndexMap<String, String> = map.into_iter().collect();
            return Ok(Some(RateBindingSpec::map_from_legacy(legacy)));
        }

        Err(PyTypeError::new_err(
            "rate_bindings must be a dict[str, RateBindingSpec], list[RateBindingSpec], dict[str, str], or None",
        ))
    }
}

#[pymethods]
impl PyExecutionContext {
    #[new]
    #[pyo3(signature = (market, model, as_of, instruments=None, rate_bindings=None, calendar=None))]
    /// Create a new execution context.
    ///
    /// Parameters
    /// ----------
    /// market : MarketContext
    ///     Market data context.
    /// model : FinancialModelSpec
    ///     Financial model.
    /// as_of : date
    ///     Valuation date.
    /// instruments : list, optional
    ///     Optional instruments.
    /// rate_bindings : dict[str, RateBindingSpec] | list[RateBindingSpec] | dict[str, str], optional
    ///     Optional rate bindings (legacy dict[str, str] uses default tenor/compounding).
    ///
    /// Returns
    /// -------
    /// ExecutionContext
    ///     New context instance.
    fn new(
        _py: Python<'_>,
        market: &Bound<'_, PyAny>,
        model: &Bound<'_, PyAny>,
        as_of: &Bound<'_, PyAny>,
        instruments: Option<Vec<Py<PyAny>>>,
        rate_bindings: Option<&Bound<'_, PyAny>>,
        calendar: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        let market_ref = market.extract::<Py<PyMarketContext>>()?;
        let model_ref = model.extract::<Py<PyFinancialModelSpec>>()?;
        let date = py_to_date(as_of)?;
        let calendar_ref = match calendar {
            Some(cal) => Some(cal.extract::<Py<PyCalendar>>()?),
            None => None,
        };
        let rust_instruments = Self::convert_instruments(_py, &instruments)?;
        let rate_bindings = Self::convert_rate_bindings(_py, rate_bindings)?;

        Ok(Self {
            market: market_ref,
            model: model_ref,
            instruments,
            rust_instruments,
            rate_bindings,
            calendar: calendar_ref,
            as_of: date,
        })
    }

    #[getter]
    /// Get the market context.
    ///
    /// Returns
    /// -------
    /// MarketContext
    ///     Market data context.
    fn market(&self, py: Python<'_>) -> PyResult<Py<PyMarketContext>> {
        Ok(self.market.clone_ref(py))
    }

    #[getter]
    /// Get the financial model.
    ///
    /// Returns
    /// -------
    /// FinancialModelSpec
    ///     Financial model.
    fn model(&self, py: Python<'_>) -> PyResult<Py<PyFinancialModelSpec>> {
        Ok(self.model.clone_ref(py))
    }

    #[getter]
    /// Get the valuation date.
    ///
    /// Returns
    /// -------
    /// date
    ///     Valuation date.
    fn as_of(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.as_of)
    }

    #[setter]
    /// Set the valuation date.
    ///
    /// Parameters
    /// ----------
    /// value : date
    ///     New valuation date.
    fn set_as_of(&mut self, value: &Bound<'_, PyAny>) -> PyResult<()> {
        self.as_of = py_to_date(value)?;
        Ok(())
    }

    #[getter]
    /// Get the instruments list.
    ///
    /// Returns
    /// -------
    /// list | None
    ///     Instruments if set.
    fn instruments(&self, py: Python<'_>) -> Option<Vec<Py<PyAny>>> {
        self.instruments
            .as_ref()
            .map(|vec| vec.iter().map(|obj| obj.clone_ref(py)).collect())
    }

    #[setter]
    /// Set the instruments list.
    ///
    /// Parameters
    /// ----------
    /// value : list | None
    ///     New instruments list.
    fn set_instruments(&mut self, py: Python<'_>, value: Option<Vec<Py<PyAny>>>) -> PyResult<()> {
        self.instruments = value;
        self.rust_instruments = Self::convert_instruments(py, &self.instruments)?;
        Ok(())
    }

    #[getter]
    /// Get the rate bindings.
    ///
    /// Returns
    /// -------
    /// dict[str, RateBindingSpec] | None
    ///     Rate bindings if set.
    fn rate_bindings(&self) -> Option<HashMap<String, PyRateBindingSpec>> {
        self.rate_bindings.as_ref().map(|bindings| {
            bindings
                .iter()
                .map(|(k, v)| (k.clone(), PyRateBindingSpec::from_inner(v.clone())))
                .collect()
        })
    }

    #[setter]
    /// Set the rate bindings.
    ///
    /// Parameters
    /// ----------
    /// value : dict[str, RateBindingSpec] | list[RateBindingSpec] | dict[str, str] | None
    ///     New rate bindings.
    fn set_rate_bindings(
        &mut self,
        py: Python<'_>,
        value: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<()> {
        self.rate_bindings = Self::convert_rate_bindings(py, value)?;
        Ok(())
    }

    #[getter]
    /// Get the holiday calendar.
    ///
    /// Returns
    /// -------
    /// Calendar | None
    ///     Calendar if set.
    fn calendar(&self, py: Python<'_>) -> Option<Py<PyCalendar>> {
        self.calendar.as_ref().map(|cal| cal.clone_ref(py))
    }

    #[setter]
    /// Set the holiday calendar.
    ///
    /// Parameters
    /// ----------
    /// value : Calendar | None
    ///     New calendar reference.
    fn set_calendar(&mut self, value: Option<&Bound<'_, PyAny>>) -> PyResult<()> {
        self.calendar = match value {
            Some(cal) => Some(cal.extract::<Py<PyCalendar>>()?),
            None => None,
        };
        Ok(())
    }

    fn __repr__(&self) -> String {
        format!(
            "ExecutionContext(as_of={:?}, has_instruments={}, has_rate_bindings={}, has_calendar={})",
            self.as_of,
            self.instruments.is_some(),
            self.rate_bindings.is_some(),
            self.calendar.is_some()
        )
    }
}

/// Orchestrates the deterministic application of a ScenarioSpec.
///
/// The engine is intentionally lightweight: it does not own any state and can
/// be cloned or reused freely. All mutable inputs are supplied via ExecutionContext.
///
/// Examples
/// --------
/// >>> from finstack.scenarios import ScenarioEngine, ScenarioSpec, OperationSpec
/// >>> engine = ScenarioEngine()
/// >>> # Create and apply scenarios...
#[pyclass(module = "finstack.scenarios", name = "ScenarioEngine")]
pub struct PyScenarioEngine {
    inner: ScenarioEngine,
}

#[pymethods]
impl PyScenarioEngine {
    #[new]
    /// Create a new scenario engine with default settings.
    ///
    /// Returns
    /// -------
    /// ScenarioEngine
    ///     New engine instance.
    fn new() -> Self {
        Self {
            inner: ScenarioEngine::new(),
        }
    }

    #[pyo3(text_signature = "(self, scenarios)")]
    /// Compose multiple scenarios into a single deterministic spec.
    ///
    /// Operations are sorted by (priority, declaration_index); conflicts use last-wins.
    ///
    /// Parameters
    /// ----------
    /// scenarios : list[ScenarioSpec]
    ///     Collection of scenario specifications to combine.
    ///
    /// Returns
    /// -------
    /// ScenarioSpec
    ///     Combined scenario containing all operations with deterministic ordering.
    fn compose(&self, scenarios: Vec<PyScenarioSpec>) -> PyScenarioSpec {
        let rust_scenarios: Vec<ScenarioSpec> = scenarios.iter().map(|s| s.inner.clone()).collect();
        let composed = self.inner.compose(rust_scenarios);
        PyScenarioSpec::from_inner(composed)
    }

    #[pyo3(text_signature = "(self, scenario, context)")]
    /// Apply a scenario specification to the execution context.
    ///
    /// Operations are applied in this order:
    /// 1. Market data (FX, equities, vol surfaces, curves, base correlation)
    /// 2. Rate bindings update (if configured)
    /// 3. Statement forecast adjustments
    /// 4. Statement re-evaluation
    ///
    /// Parameters
    /// ----------
    /// scenario : ScenarioSpec
    ///     Scenario specification to apply.
    /// context : ExecutionContext
    ///     Mutable execution context that supplies market data, statements,
    ///     instruments, and rate bindings.
    ///
    /// Returns
    /// -------
    /// ApplicationReport
    ///     Summary of how many operations were applied and any warnings.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If operation cannot be completed (e.g., missing market data,
    ///     unsupported operation, or invalid tenor strings).
    fn apply(
        &self,
        py: Python<'_>,
        scenario: &PyScenarioSpec,
        context: &mut PyExecutionContext,
    ) -> PyResult<PyApplicationReport> {
        // Extract mutable references from Python objects
        let mut market_borrow = context.market.borrow_mut(py);
        let mut model_borrow = context.model.borrow_mut(py);

        // Convert rate_bindings to IndexMap if present
        let rate_bindings = context.rate_bindings.clone();

        let instruments_option = context.rust_instruments.as_mut().map(|vec| vec.as_mut());
        let calendar = context
            .calendar
            .as_ref()
            .map(|cal| cal.borrow(py).inner as &dyn finstack_core::dates::HolidayCalendar);

        // Build temporary ExecutionContext with references
        let mut exec_ctx = ExecutionContext {
            market: &mut market_borrow.inner,
            model: &mut model_borrow.inner,
            instruments: instruments_option,
            rate_bindings,
            calendar,
            as_of: context.as_of,
        };

        // Apply scenario
        let report = self
            .inner
            .apply(&scenario.inner, &mut exec_ctx)
            .map_err(scenario_to_py)?;

        // Update the as_of date in context if it changed
        context.as_of = exec_ctx.as_of;

        Ok(PyApplicationReport::new(report))
    }

    fn __repr__(&self) -> String {
        "ScenarioEngine()".to_string()
    }
}

/// Register engine types with the scenarios module.
pub(crate) fn register(
    _py: Python<'_>,
    module: &Bound<'_, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyExecutionContext>()?;
    module.add_class::<PyScenarioEngine>()?;

    Ok(vec!["ExecutionContext", "ScenarioEngine"])
}
