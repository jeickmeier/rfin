//! Python wrappers for statement model types and enums.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

// ---------------------------------------------------------------------------
// ForecastMethod
// ---------------------------------------------------------------------------

/// Available forecast methods for projecting node values.
#[pyclass(
    name = "ForecastMethod",
    module = "finstack.statements",
    eq,
    skip_from_py_object
)]
#[derive(Clone, PartialEq)]
pub struct PyForecastMethod {
    pub(super) inner: finstack_statements::types::ForecastMethod,
}

#[pymethods]
impl PyForecastMethod {
    #[staticmethod]
    fn forward_fill() -> Self {
        Self {
            inner: finstack_statements::types::ForecastMethod::ForwardFill,
        }
    }

    #[staticmethod]
    fn growth_pct() -> Self {
        Self {
            inner: finstack_statements::types::ForecastMethod::GrowthPct,
        }
    }

    #[staticmethod]
    fn curve_pct() -> Self {
        Self {
            inner: finstack_statements::types::ForecastMethod::CurvePct,
        }
    }

    #[staticmethod]
    fn normal() -> Self {
        Self {
            inner: finstack_statements::types::ForecastMethod::Normal,
        }
    }

    #[staticmethod]
    fn log_normal() -> Self {
        Self {
            inner: finstack_statements::types::ForecastMethod::LogNormal,
        }
    }

    #[staticmethod]
    fn override_method() -> Self {
        Self {
            inner: finstack_statements::types::ForecastMethod::Override,
        }
    }

    #[staticmethod]
    fn time_series() -> Self {
        Self {
            inner: finstack_statements::types::ForecastMethod::TimeSeries,
        }
    }

    #[staticmethod]
    fn seasonal() -> Self {
        Self {
            inner: finstack_statements::types::ForecastMethod::Seasonal,
        }
    }

    fn __repr__(&self) -> String {
        format!("ForecastMethod({:?})", self.inner)
    }
}

// ---------------------------------------------------------------------------
// NodeType
// ---------------------------------------------------------------------------

/// Node computation type.
#[pyclass(
    name = "NodeType",
    module = "finstack.statements",
    eq,
    skip_from_py_object
)]
#[derive(Clone, PartialEq)]
pub struct PyNodeType {
    pub(super) inner: finstack_statements::types::NodeType,
}

#[pymethods]
impl PyNodeType {
    #[staticmethod]
    fn value() -> Self {
        Self {
            inner: finstack_statements::types::NodeType::Value,
        }
    }

    #[staticmethod]
    fn calculated() -> Self {
        Self {
            inner: finstack_statements::types::NodeType::Calculated,
        }
    }

    #[staticmethod]
    fn mixed() -> Self {
        Self {
            inner: finstack_statements::types::NodeType::Mixed,
        }
    }

    fn __repr__(&self) -> String {
        format!("NodeType({:?})", self.inner)
    }
}

// ---------------------------------------------------------------------------
// NodeId
// ---------------------------------------------------------------------------

/// Type-safe identifier for a node in a financial model.
#[pyclass(name = "NodeId", module = "finstack.statements", skip_from_py_object)]
#[derive(Clone)]
pub struct PyNodeId {
    pub(super) inner: finstack_statements::types::NodeId,
}

#[pymethods]
impl PyNodeId {
    #[new]
    fn new(id: &str) -> Self {
        Self {
            inner: finstack_statements::types::NodeId::new(id),
        }
    }

    fn as_str(&self) -> &str {
        self.inner.as_str()
    }

    fn __repr__(&self) -> String {
        format!("NodeId({:?})", self.inner.as_str())
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }
}

// ---------------------------------------------------------------------------
// NumericMode
// ---------------------------------------------------------------------------

/// Numeric evaluation mode.
#[pyclass(
    name = "NumericMode",
    module = "finstack.statements",
    eq,
    skip_from_py_object
)]
#[derive(Clone, PartialEq)]
pub struct PyNumericMode {
    pub(super) inner: finstack_statements::evaluator::NumericMode,
}

#[pymethods]
impl PyNumericMode {
    #[staticmethod]
    fn float64() -> Self {
        Self {
            inner: finstack_statements::evaluator::NumericMode::Float64,
        }
    }

    #[staticmethod]
    fn decimal() -> Self {
        Self {
            inner: finstack_statements::evaluator::NumericMode::Decimal,
        }
    }

    fn __repr__(&self) -> String {
        format!("NumericMode({:?})", self.inner)
    }
}

// ---------------------------------------------------------------------------
// FinancialModelSpec — JSON round-trip
// ---------------------------------------------------------------------------

/// Top-level financial model specification.
#[pyclass(
    name = "FinancialModelSpec",
    module = "finstack.statements",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyFinancialModelSpec {
    pub(crate) inner: finstack_statements::FinancialModelSpec,
}

#[pymethods]
impl PyFinancialModelSpec {
    /// Deserialize from a JSON string.
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: finstack_statements::FinancialModelSpec =
            serde_json::from_str(json).map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Serialize to a JSON string.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Model identifier.
    #[getter]
    fn id(&self) -> &str {
        &self.inner.id
    }

    /// Number of periods.
    #[getter]
    fn period_count(&self) -> usize {
        self.inner.periods.len()
    }

    /// Number of nodes.
    #[getter]
    fn node_count(&self) -> usize {
        self.inner.nodes.len()
    }

    /// Node identifiers in declaration order.
    fn node_ids(&self) -> Vec<String> {
        self.inner.nodes.keys().map(|k| k.to_string()).collect()
    }

    /// Whether the model has a node with the given ID.
    fn has_node(&self, node_id: &str) -> bool {
        self.inner.has_node(node_id)
    }

    /// Schema version.
    #[getter]
    fn schema_version(&self) -> u32 {
        self.inner.schema_version
    }

    fn __repr__(&self) -> String {
        format!(
            "FinancialModelSpec(id={:?}, periods={}, nodes={})",
            self.inner.id,
            self.inner.periods.len(),
            self.inner.nodes.len()
        )
    }
}

/// Register type classes.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyForecastMethod>()?;
    m.add_class::<PyNodeType>()?;
    m.add_class::<PyNodeId>()?;
    m.add_class::<PyNumericMode>()?;
    m.add_class::<PyFinancialModelSpec>()?;
    Ok(())
}
