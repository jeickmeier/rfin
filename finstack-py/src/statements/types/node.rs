//! Node specification bindings.

use super::forecast::PyForecastSpec;
use super::value::PyAmountOrScalar;
use crate::core::dates::periods::PyPeriodId;
use crate::statements::utils::json_to_py;
use finstack_core::dates::PeriodId;
use finstack_statements::types::{NodeSpec, NodeType};
use indexmap::IndexMap;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAnyMethods, PyDict, PyList, PyModule, PyType};
use pyo3::Bound;

/// Node computation type.
///
/// Determines how a node's value is computed.
#[pyclass(
    module = "finstack.statements.types",
    name = "NodeType",
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, Debug)]
pub struct PyNodeType {
    pub(crate) inner: NodeType,
}

impl PyNodeType {
    pub(crate) fn new(inner: NodeType) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyNodeType {
    #[classattr]
    const VALUE: Self = Self {
        inner: NodeType::Value,
    };
    #[classattr]
    const CALCULATED: Self = Self {
        inner: NodeType::Calculated,
    };
    #[classattr]
    const MIXED: Self = Self {
        inner: NodeType::Mixed,
    };

    fn __repr__(&self) -> String {
        format!("NodeType.{:?}", self.inner)
    }

    fn __str__(&self) -> String {
        format!("{:?}", self.inner)
    }
}

/// Node specification.
///
/// Specifies a single node (metric/line item) in a financial model.
#[pyclass(
    module = "finstack.statements.types",
    name = "NodeSpec",
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyNodeSpec {
    pub(crate) inner: NodeSpec,
}

impl PyNodeSpec {
    pub(crate) fn new(inner: NodeSpec) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyNodeSpec {
    #[new]
    #[pyo3(text_signature = "(node_id, node_type)")]
    /// Create a new node specification.
    ///
    /// Parameters
    /// ----------
    /// node_id : str
    ///     Unique identifier for this node
    /// node_type : NodeType
    ///     Node computation type
    ///
    /// Returns
    /// -------
    /// NodeSpec
    ///     Node specification
    fn new_py(node_id: String, node_type: PyNodeType) -> Self {
        Self::new(NodeSpec::new(node_id, node_type.inner))
    }

    #[pyo3(text_signature = "(self, name)")]
    /// Set the human-readable name.
    ///
    /// Parameters
    /// ----------
    /// name : str
    ///     Human-readable name
    ///
    /// Returns
    /// -------
    /// NodeSpec
    ///     Updated node spec
    fn with_name(&self, name: String) -> Self {
        Self::new(self.inner.clone().with_name(name))
    }

    #[pyo3(text_signature = "(self, values)")]
    /// Add explicit values per period.
    ///
    /// Parameters
    /// ----------
    /// values : dict[PeriodId, AmountOrScalar] or list[tuple[PeriodId, AmountOrScalar]]
    ///     Period values
    ///
    /// Returns
    /// -------
    /// NodeSpec
    ///     Updated node spec
    fn with_values(&self, values: &Bound<'_, PyAny>) -> PyResult<Self> {
        let values_map = parse_period_values(values)?;
        Ok(Self::new(self.inner.clone().with_values(values_map)))
    }

    #[pyo3(text_signature = "(self, formula)")]
    /// Set the formula text.
    ///
    /// Parameters
    /// ----------
    /// formula : str
    ///     Formula text in statement DSL
    ///
    /// Returns
    /// -------
    /// NodeSpec
    ///     Updated node spec
    fn with_formula(&self, formula: String) -> Self {
        Self::new(self.inner.clone().with_formula(formula))
    }

    #[pyo3(text_signature = "(self, forecast_spec)")]
    /// Set the forecast specification.
    ///
    /// Parameters
    /// ----------
    /// forecast_spec : ForecastSpec
    ///     Forecast specification
    ///
    /// Returns
    /// -------
    /// NodeSpec
    ///     Updated node spec
    fn with_forecast(&self, forecast_spec: &PyForecastSpec) -> Self {
        Self::new(
            self.inner
                .clone()
                .with_forecast(forecast_spec.inner.clone()),
        )
    }

    #[pyo3(text_signature = "(self, tags)")]
    /// Add tags for grouping/filtering.
    ///
    /// Parameters
    /// ----------
    /// tags : list[str]
    ///     Tags
    ///
    /// Returns
    /// -------
    /// NodeSpec
    ///     Updated node spec
    fn with_tags(&self, tags: Vec<String>) -> Self {
        Self::new(self.inner.clone().with_tags(tags))
    }

    #[getter]
    /// Get the node ID.
    ///
    /// Returns
    /// -------
    /// str
    ///     Node ID
    fn node_id(&self) -> String {
        self.inner.node_id.clone()
    }

    #[getter]
    /// Get the human-readable name.
    ///
    /// Returns
    /// -------
    /// str | None
    ///     Name if set
    fn name(&self) -> Option<String> {
        self.inner.name.clone()
    }

    #[getter]
    /// Get the node type.
    ///
    /// Returns
    /// -------
    /// NodeType
    ///     Node computation type
    fn node_type(&self) -> PyNodeType {
        PyNodeType::new(self.inner.node_type)
    }

    #[getter]
    /// Get explicit period values.
    ///
    /// Returns
    /// -------
    /// dict[PeriodId, AmountOrScalar] | None
    ///     Period values if set
    fn values(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        self.inner
            .values
            .as_ref()
            .map(|values| -> PyResult<Py<PyAny>> {
                let dict = PyDict::new(py);
                for (period_id, amount_or_scalar) in values {
                    let py_period_id = PyPeriodId::new(*period_id);
                    let py_amount = PyAmountOrScalar::new(*amount_or_scalar);
                    dict.set_item(py_period_id, py_amount)?;
                }
                Ok(dict.into())
            })
            .transpose()
    }

    #[getter]
    /// Get the forecast specification.
    ///
    /// Returns
    /// -------
    /// ForecastSpec | None
    ///     Forecast spec if set
    fn forecast(&self) -> Option<PyForecastSpec> {
        self.inner
            .forecast
            .as_ref()
            .map(|f| PyForecastSpec::new(f.clone()))
    }

    #[getter]
    /// Get the formula text.
    ///
    /// Returns
    /// -------
    /// str | None
    ///     Formula text if set
    fn formula_text(&self) -> Option<String> {
        self.inner.formula_text.clone()
    }

    #[getter]
    /// Get the where clause.
    ///
    /// Returns
    /// -------
    /// str | None
    ///     Where clause if set
    fn where_text(&self) -> Option<String> {
        self.inner.where_text.clone()
    }

    #[getter]
    /// Get tags.
    ///
    /// Returns
    /// -------
    /// list[str]
    ///     Tags
    fn tags(&self) -> Vec<String> {
        self.inner.tags.clone()
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
    /// NodeSpec
    ///     Deserialized node spec
    fn from_json(_cls: &Bound<'_, PyType>, json_str: &str) -> PyResult<Self> {
        serde_json::from_str(json_str)
            .map(Self::new)
            .map_err(|e| PyValueError::new_err(format!("Deserialization error: {}", e)))
    }

    fn __repr__(&self) -> String {
        format!(
            "NodeSpec(node_id='{}', node_type={:?})",
            self.inner.node_id, self.inner.node_type
        )
    }
}

/// Helper to parse period values from dict or list of tuples.
fn parse_period_values(
    values: &Bound<'_, PyAny>,
) -> PyResult<IndexMap<PeriodId, finstack_statements::types::AmountOrScalar>> {
    let mut map = IndexMap::new();

    if let Ok(dict) = values.cast::<PyDict>() {
        // Dict format
        for (key, value) in dict.iter() {
            let period_id: PyPeriodId = key.extract()?;
            let amount: PyAmountOrScalar = value.extract()?;
            map.insert(period_id.inner, amount.inner);
        }
    } else if let Ok(list) = values.cast::<PyList>() {
        // List of tuples format
        for (idx, item) in list.iter().enumerate() {
            let (period, amount) =
                item.extract::<(PyPeriodId, PyAmountOrScalar)>()
                    .map_err(|err| {
                        PyValueError::new_err(format!(
                            "Invalid values[{idx}] (expected (PeriodId, AmountOrScalar)): {err}"
                        ))
                    })?;
            map.insert(period.inner, amount.inner);
        }
    } else {
        return Err(PyValueError::new_err(
            "values must be a dict or list of tuples",
        ));
    }

    Ok(map)
}

pub(crate) fn register<'py>(_py: Python<'py>, module: &Bound<'py, PyModule>) -> PyResult<()> {
    module.add_class::<PyNodeType>()?;
    module.add_class::<PyNodeSpec>()?;
    Ok(())
}
