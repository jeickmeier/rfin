//! Financial model specification bindings.

use super::node::PyNodeSpec;
use super::waterfall::PyWaterfallSpec;
use crate::core::dates::periods::PyPeriod;
use crate::errors::ParameterError;
use crate::statements::error::stmt_to_py;
use crate::statements::utils::json_to_py;
use finstack_statements::types::{CapitalStructureSpec, DebtInstrumentSpec, FinancialModelSpec};
use indexmap::IndexMap;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAnyMethods, PyDict, PyModule, PyType};
use pyo3::Bound;

/// Capital structure specification.
///
/// Defines debt and equity instruments in a model.
#[pyclass(
    module = "finstack.statements.types",
    name = "CapitalStructureSpec",
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyCapitalStructureSpec {
    pub(crate) inner: CapitalStructureSpec,
}

impl PyCapitalStructureSpec {
    pub(crate) fn new(inner: CapitalStructureSpec) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCapitalStructureSpec {
    #[new]
    #[pyo3(signature = (debt_instruments=None, equity_instruments=None, waterfall=None))]
    #[pyo3(text_signature = "(debt_instruments=None, equity_instruments=None, waterfall=None)")]
    /// Create a capital structure specification.
    ///
    /// Parameters
    /// ----------
    /// debt_instruments : list[DebtInstrumentSpec], optional
    ///     Debt instruments
    /// equity_instruments : list, optional
    ///     Reserved for future expansion and currently unsupported
    /// waterfall : WaterfallSpec, optional
    ///     Waterfall configuration for dynamic cash flow allocation
    ///
    /// Returns
    /// -------
    /// CapitalStructureSpec
    ///     Capital structure spec
    fn new_py(
        debt_instruments: Option<Vec<PyDebtInstrumentSpec>>,
        equity_instruments: Option<Vec<Py<PyAny>>>,
        waterfall: Option<PyWaterfallSpec>,
    ) -> PyResult<Self> {
        let debt_instruments = debt_instruments
            .map(|v| v.into_iter().map(|d| d.inner).collect())
            .unwrap_or_default();

        if equity_instruments.is_some_and(|items| !items.is_empty()) {
            return Err(ParameterError::new_err(
                "CapitalStructureSpec.equity_instruments is not supported yet",
            ));
        }

        Ok(Self::new(CapitalStructureSpec {
            debt_instruments,
            equity_instruments: Vec::new(),
            meta: IndexMap::new(),
            reporting_currency: None,
            fx_policy: None,
            waterfall: waterfall.map(|w| w.inner),
        }))
    }

    #[getter]
    /// Get debt instruments.
    ///
    /// Returns
    /// -------
    /// list[DebtInstrumentSpec]
    ///     Debt instruments
    fn debt_instruments(&self) -> Vec<PyDebtInstrumentSpec> {
        self.inner
            .debt_instruments
            .iter()
            .map(|d| PyDebtInstrumentSpec::new(d.clone()))
            .collect()
    }

    #[getter]
    /// Get waterfall spec.
    ///
    /// Returns
    /// -------
    /// WaterfallSpec | None
    ///     Waterfall configuration if set
    fn waterfall(&self) -> Option<PyWaterfallSpec> {
        self.inner
            .waterfall
            .as_ref()
            .map(|w| PyWaterfallSpec { inner: w.clone() })
    }

    /// Convert to JSON string.
    ///
    /// Returns
    /// -------
    /// str
    ///     JSON representation
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner)
            .map_err(|e| PyValueError::new_err(format!("Serialization error: {}", e)))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, json_str)")]
    /// Create from JSON string.
    ///
    /// Parameters
    /// ----------
    /// json_str : str
    ///     JSON string
    ///
    /// Returns
    /// -------
    /// CapitalStructureSpec
    ///     Deserialized spec
    fn from_json(_cls: &Bound<'_, PyType>, json_str: &str) -> PyResult<Self> {
        serde_json::from_str(json_str)
            .map(Self::new)
            .map_err(|e| PyValueError::new_err(format!("Deserialization error: {}", e)))
    }

    fn __repr__(&self) -> String {
        format!(
            "CapitalStructureSpec(debt_instruments={})",
            self.inner.debt_instruments.len()
        )
    }
}

/// Debt instrument specification.
///
/// Represents a debt instrument in a capital structure.
#[pyclass(
    module = "finstack.statements.types",
    name = "DebtInstrumentSpec",
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyDebtInstrumentSpec {
    pub(crate) inner: DebtInstrumentSpec,
}

impl PyDebtInstrumentSpec {
    pub(crate) fn new(inner: DebtInstrumentSpec) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyDebtInstrumentSpec {
    #[staticmethod]
    #[pyo3(text_signature = "(id, spec)")]
    /// Create a bond instrument.
    ///
    /// Parameters
    /// ----------
    /// id : str
    ///     Instrument identifier
    /// spec : dict
    ///     Instrument specification
    ///
    /// Returns
    /// -------
    /// DebtInstrumentSpec
    ///     Bond instrument spec
    fn bond(id: String, spec: &Bound<'_, PyDict>) -> PyResult<Self> {
        let spec_value = dict_to_json(spec)?;
        Ok(Self::new(DebtInstrumentSpec::Bond {
            id,
            spec: spec_value,
        }))
    }

    #[staticmethod]
    #[pyo3(text_signature = "(id, spec)")]
    /// Create a swap instrument.
    ///
    /// Parameters
    /// ----------
    /// id : str
    ///     Instrument identifier
    /// spec : dict
    ///     Instrument specification
    ///
    /// Returns
    /// -------
    /// DebtInstrumentSpec
    ///     Swap instrument spec
    fn swap(id: String, spec: &Bound<'_, PyDict>) -> PyResult<Self> {
        let spec_value = dict_to_json(spec)?;
        Ok(Self::new(DebtInstrumentSpec::Swap {
            id,
            spec: spec_value,
        }))
    }

    #[staticmethod]
    #[pyo3(text_signature = "(id, spec)")]
    /// Create a term loan instrument.
    ///
    /// Parameters
    /// ----------
    /// id : str
    ///     Instrument identifier
    /// spec : dict
    ///     Instrument specification (e.g. notional, spread, base_rate,
    ///     amortization_schedule, prepayment_penalty)
    ///
    /// Returns
    /// -------
    /// DebtInstrumentSpec
    ///     Term loan instrument spec
    fn term_loan(id: String, spec: &Bound<'_, PyDict>) -> PyResult<Self> {
        let spec_value = dict_to_json(spec)?;
        Ok(Self::new(DebtInstrumentSpec::TermLoan {
            id,
            spec: spec_value,
        }))
    }

    #[staticmethod]
    #[pyo3(text_signature = "(id, spec)")]
    /// Create a generic debt instrument.
    ///
    /// Parameters
    /// ----------
    /// id : str
    ///     Instrument identifier
    /// spec : dict
    ///     Instrument specification
    ///
    /// Returns
    /// -------
    /// DebtInstrumentSpec
    ///     Generic instrument spec
    fn generic(id: String, spec: &Bound<'_, PyDict>) -> PyResult<Self> {
        let spec_value = dict_to_json(spec)?;
        Ok(Self::new(DebtInstrumentSpec::Generic {
            id,
            spec: spec_value,
        }))
    }

    /// Convert to JSON string.
    ///
    /// Returns
    /// -------
    /// str
    ///     JSON representation
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner)
            .map_err(|e| PyValueError::new_err(format!("Serialization error: {}", e)))
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            DebtInstrumentSpec::Bond { id, .. } => format!("DebtInstrumentSpec.bond('{}')", id),
            DebtInstrumentSpec::Swap { id, .. } => format!("DebtInstrumentSpec.swap('{}')", id),
            DebtInstrumentSpec::TermLoan { id, .. } => {
                format!("DebtInstrumentSpec.term_loan('{}')", id)
            }
            DebtInstrumentSpec::Generic { id, .. } => {
                format!("DebtInstrumentSpec.generic('{}')", id)
            }
        }
    }
}

/// Financial model specification.
///
/// Top-level specification for a complete financial statement model.
#[pyclass(
    module = "finstack.statements.types",
    name = "FinancialModelSpec",
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyFinancialModelSpec {
    pub(crate) inner: FinancialModelSpec,
}

impl PyFinancialModelSpec {
    pub(crate) fn new(inner: FinancialModelSpec) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyFinancialModelSpec {
    #[new]
    #[pyo3(text_signature = "(id, periods)")]
    /// Create a new financial model specification.
    ///
    /// Parameters
    /// ----------
    /// id : str
    ///     Unique model identifier
    /// periods : list[Period]
    ///     Ordered list of periods
    ///
    /// Returns
    /// -------
    /// FinancialModelSpec
    ///     Model specification
    fn new_py(id: String, periods: Vec<PyPeriod>) -> Self {
        let periods = periods.into_iter().map(|p| p.inner).collect();
        Self::new(FinancialModelSpec::new(id, periods))
    }

    #[pyo3(text_signature = "(self, node)")]
    /// Add a node to the model.
    ///
    /// Parameters
    /// ----------
    /// node : NodeSpec
    ///     Node specification to add
    fn add_node(&mut self, node: &PyNodeSpec) {
        self.inner.add_node(node.inner.clone());
    }

    #[pyo3(text_signature = "(self, node_id)")]
    /// Get a node by ID.
    ///
    /// Parameters
    /// ----------
    /// node_id : str
    ///     Node identifier
    ///
    /// Returns
    /// -------
    /// NodeSpec | None
    ///     Node spec if found
    fn get_node(&self, node_id: &str) -> Option<PyNodeSpec> {
        self.inner
            .get_node(node_id)
            .map(|n| PyNodeSpec::new(n.clone()))
    }

    #[pyo3(text_signature = "(self, node_id)")]
    /// Check if a node exists.
    ///
    /// Parameters
    /// ----------
    /// node_id : str
    ///     Node identifier
    ///
    /// Returns
    /// -------
    /// bool
    ///     True if node exists
    fn has_node(&self, node_id: &str) -> bool {
        self.inner.has_node(node_id)
    }

    #[getter]
    /// Get model ID.
    ///
    /// Returns
    /// -------
    /// str
    ///     Model ID
    fn id(&self) -> String {
        self.inner.id.clone()
    }

    #[getter]
    /// Get periods.
    ///
    /// Returns
    /// -------
    /// list[Period]
    ///     Ordered periods
    fn periods(&self) -> Vec<PyPeriod> {
        self.inner
            .periods
            .iter()
            .map(|p| PyPeriod::new(p.clone()))
            .collect()
    }

    #[getter]
    /// Get all nodes.
    ///
    /// Returns
    /// -------
    /// dict[str, NodeSpec]
    ///     Map of node_id to NodeSpec
    fn nodes(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new(py);
        for (node_id, node_spec) in &self.inner.nodes {
            dict.set_item(node_id.as_str(), PyNodeSpec::new(node_spec.clone()))?;
        }
        Ok(dict.into())
    }

    #[getter]
    /// Get capital structure.
    ///
    /// Returns
    /// -------
    /// CapitalStructureSpec | None
    ///     Capital structure if set
    fn capital_structure(&self) -> Option<PyCapitalStructureSpec> {
        self.inner
            .capital_structure
            .as_ref()
            .map(|cs| PyCapitalStructureSpec::new(cs.clone()))
    }

    #[getter]
    /// Get metadata.
    ///
    /// Returns
    /// -------
    /// dict
    ///     Metadata dictionary
    fn meta(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new(py);
        for (key, value) in &self.inner.meta {
            dict.set_item(key, json_to_py(py, value)?)?;
        }
        Ok(dict.into())
    }

    #[getter]
    /// Get schema version.
    ///
    /// Returns
    /// -------
    /// int
    ///     Schema version
    fn schema_version(&self) -> u32 {
        self.inner.schema_version
    }

    /// Convert to JSON string.
    ///
    /// Returns
    /// -------
    /// str
    ///     JSON representation
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner)
            .map_err(|e| PyValueError::new_err(format!("Serialization error: {}", e)))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, json_str)")]
    /// Create from JSON string.
    ///
    /// Parameters
    /// ----------
    /// json_str : str
    ///     JSON string
    ///
    /// Returns
    /// -------
    /// FinancialModelSpec
    ///     Deserialized model spec
    fn from_json(_cls: &Bound<'_, PyType>, json_str: &str) -> PyResult<Self> {
        serde_json::from_str(json_str)
            .map(Self::new)
            .map_err(|e| PyValueError::new_err(format!("Deserialization error: {}", e)))
    }

    fn __repr__(&self) -> String {
        format!(
            "FinancialModelSpec(id='{}', periods={}, nodes={})",
            self.inner.id,
            self.inner.periods.len(),
            self.inner.nodes.len()
        )
    }

    #[pyo3(
        signature = (target_node, target_period, target_value, driver_node, driver_period=None, update_model=true, bounds=None),
        text_signature = "(self, target_node, target_period, target_value, driver_node, driver_period=None, update_model=True, bounds=None)"
    )]
    /// Perform goal seek to find the driver value that achieves a target metric.
    ///
    /// Solves for the driver node value that achieves a target metric value in a specific period.
    /// Uses Brent's method for robust root-finding (tolerance ~1e-9, max 128 iterations).
    ///
    /// Parameters
    /// ----------
    /// target_node : str
    ///     Node identifier for the target metric (e.g., "interest_coverage")
    /// target_period : str
    ///     Period in which to evaluate the target (e.g., "2025Q4")
    /// target_value : float
    ///     Desired value for the target metric
    /// driver_node : str
    ///     Node identifier for the driver input to vary (e.g., "revenue")
    /// driver_period : str, optional
    ///     Period in which to vary the driver. Defaults to target_period if None.
    /// update_model : bool, optional
    ///     If True (default), update the model with the solved driver value
    /// bounds : tuple[float, float], optional
    ///     Explicit (lower, upper) search bounds for the driver value.
    ///     If None, bounds are inferred automatically from the model.
    ///
    /// Returns
    /// -------
    /// float
    ///     The solved driver value that achieves the target
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the target or driver node doesn't exist, periods are invalid,
    ///     or no solution can be found
    ///
    /// Examples
    /// --------
    /// >>> model = ModelBuilder("test") \\
    /// ...     .periods("2025Q1..Q4", None) \\
    /// ...     .value("revenue", [(PeriodId.quarter(2025, 1), 100_000.0)]) \\
    /// ...     .forecast("revenue", ForecastSpec.growth(0.05)) \\
    /// ...     .compute("interest_expense", "10000.0") \\
    /// ...     .compute("ebitda", "revenue * 0.3") \\
    /// ...     .compute("interest_coverage", "ebitda / interest_expense") \\
    /// ...     .build()
    /// >>> # Solve for Q4 revenue that achieves 2.0x interest coverage
    /// >>> solved = model.goal_seek(
    /// ...     target_node="interest_coverage",
    /// ...     target_period="2025Q4",
    /// ...     target_value=2.0,
    /// ...     driver_node="revenue",
    /// ... )
    /// >>> print(f"Revenue needed: ${solved:,.2f}")
    #[allow(clippy::too_many_arguments)]
    fn goal_seek(
        &mut self,
        target_node: &str,
        target_period: &str,
        target_value: f64,
        driver_node: &str,
        driver_period: Option<&str>,
        update_model: bool,
        bounds: Option<(f64, f64)>,
    ) -> PyResult<f64> {
        let target_period_id: finstack_core::dates::PeriodId =
            target_period.parse().map_err(|e| {
                PyValueError::new_err(format!("Invalid target period '{}': {}", target_period, e))
            })?;

        let driver_period_str = driver_period.unwrap_or(target_period);
        let driver_period_id: finstack_core::dates::PeriodId =
            driver_period_str.parse().map_err(|e| {
                PyValueError::new_err(format!(
                    "Invalid driver period '{}': {}",
                    driver_period_str, e
                ))
            })?;

        finstack_statements_analytics::analysis::goal_seek(
            &mut self.inner,
            target_node,
            target_period_id,
            target_value,
            driver_node,
            driver_period_id,
            update_model,
            bounds,
        )
        .map_err(stmt_to_py)
    }
}

/// Helper to convert PyDict to serde_json::Value
fn dict_to_json(dict: &Bound<'_, PyDict>) -> PyResult<serde_json::Value> {
    let mut map = serde_json::Map::new();
    for (key, value) in dict.iter() {
        let key_str: String = key.extract()?;
        let json_value = crate::statements::utils::py_to_json(&value)?;
        map.insert(key_str, json_value);
    }
    Ok(serde_json::Value::Object(map))
}

pub(crate) fn register<'py>(_py: Python<'py>, module: &Bound<'py, PyModule>) -> PyResult<()> {
    module.add_class::<PyCapitalStructureSpec>()?;
    module.add_class::<PyDebtInstrumentSpec>()?;
    module.add_class::<PyFinancialModelSpec>()?;
    Ok(())
}
