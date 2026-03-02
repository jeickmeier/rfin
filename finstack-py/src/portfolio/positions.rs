//! Python bindings for Portfolio.

use crate::core::currency::PyCurrency;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::portfolio::book::PyBook;
use crate::portfolio::error::portfolio_to_py;
use crate::portfolio::types::{PyEntity, PyPosition, PyPositionSpec};
use finstack_portfolio::portfolio::PortfolioSpec;
use finstack_portfolio::Portfolio;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyList, PyModule};
use pyo3::Bound;

/// A portfolio of positions across multiple entities.
///
/// The portfolio holds a flat list of positions, each referencing an entity and instrument.
/// Positions can be grouped and aggregated by entity or by arbitrary attributes (tags).
///
/// Examples:
///     >>> from finstack.portfolio import Portfolio, Entity
///     >>> from finstack.core import Currency
///     >>> from datetime import date
///     >>> portfolio = Portfolio("FUND_A", Currency.USD, date(2024, 1, 1))
///     >>> portfolio.entities["ACME"] = Entity("ACME")
///     >>> len(portfolio.positions)
///     0
#[pyclass(module = "finstack.portfolio", name = "Portfolio")]
pub struct PyPortfolio {
    pub(crate) inner: Portfolio,
}

impl PyPortfolio {
    pub(crate) fn new(inner: Portfolio) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyPortfolio {
    #[new]
    #[pyo3(signature = (id, base_ccy, as_of))]
    #[pyo3(text_signature = "(id, base_ccy, as_of)")]
    /// Create a new empty portfolio.
    ///
    /// Args:
    ///     id: Unique portfolio identifier.
    ///     base_ccy: Reporting currency.
    ///     as_of: Valuation date.
    ///
    /// Returns:
    ///     Portfolio: New portfolio instance.
    ///
    /// Examples:
    ///     >>> from finstack.core import Currency
    ///     >>> from datetime import date
    ///     >>> portfolio = Portfolio("FUND_A", Currency.USD, date(2024, 1, 1))
    ///     >>> portfolio.id
    ///     'FUND_A'
    fn new_py(id: String, base_ccy: &Bound<'_, PyAny>, as_of: &Bound<'_, PyAny>) -> PyResult<Self> {
        let ccy = if let Ok(py_ccy) = base_ccy.extract::<PyRef<PyCurrency>>() {
            py_ccy.inner
        } else if let Ok(s) = base_ccy.extract::<String>() {
            s.parse()
                .map_err(|e| PyValueError::new_err(format!("Invalid currency: {}", e)))?
        } else {
            return Err(PyTypeError::new_err("Expected Currency or string"));
        };

        let date = py_to_date(as_of)?;
        Ok(Self::new(Portfolio::new(id, ccy, date)))
    }

    #[pyo3(text_signature = "($self, position_id)")]
    /// Get a position by identifier.
    ///
    /// Args:
    ///     position_id: Identifier of the position to locate.
    ///
    /// Returns:
    ///     Position or None: The position if found.
    ///
    /// Examples:
    ///     >>> position = portfolio.get_position("POS_1")
    fn get_position(&self, position_id: &str) -> Option<PyPosition> {
        self.inner
            .get_position(position_id)
            .map(|p| PyPosition::new(p.clone()))
    }

    #[pyo3(text_signature = "($self, entity_id)")]
    /// Get all positions for a given entity.
    ///
    /// Args:
    ///     entity_id: Entity identifier used for filtering.
    ///
    /// Returns:
    ///     list[Position]: List of positions for the entity.
    ///
    /// Examples:
    ///     >>> positions = portfolio.positions_for_entity("ENTITY_A")
    ///     >>> len(positions)
    ///     1
    fn positions_for_entity(&self, entity_id: &str, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let entity_id_string = entity_id.to_string();
        let positions = self.inner.positions_for_entity(&entity_id_string);
        let py_positions: Vec<PyPosition> = positions
            .into_iter()
            .map(|p| PyPosition::new(p.clone()))
            .collect();
        Ok(PyList::new(py, py_positions)?.into())
    }

    #[pyo3(text_signature = "($self, key, value)")]
    /// Get all positions with a specific tag value.
    ///
    /// Args:
    ///     key: Tag key to filter by.
    ///     value: Tag value to match.
    ///
    /// Returns:
    ///     list[Position]: List of positions with matching tag.
    ///
    /// Examples:
    ///     >>> positions = portfolio.positions_with_tag("sector", "Technology")
    fn positions_with_tag(&self, key: &str, value: &str, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let positions = self.inner.positions_with_tag(key, value);
        let py_positions: Vec<PyPosition> = positions
            .into_iter()
            .map(|p| PyPosition::new(p.clone()))
            .collect();
        Ok(PyList::new(py, py_positions)?.into())
    }

    #[pyo3(text_signature = "($self)")]
    /// Validate the portfolio structure and references.
    ///
    /// Checks that all positions reference valid entities and that structural
    /// invariants are maintained.
    ///
    /// Raises:
    ///     ValueError: If validation fails.
    ///
    /// Examples:
    ///     >>> portfolio.validate()
    fn validate(&self) -> PyResult<()> {
        self.inner.validate().map_err(portfolio_to_py)
    }

    #[pyo3(text_signature = "($self)")]
    /// Check if the portfolio contains the dummy entity.
    fn has_dummy_entity(&self) -> bool {
        self.inner.has_dummy_entity()
    }

    #[getter]
    /// Get the portfolio identifier.
    fn id(&self) -> String {
        self.inner.id.clone()
    }

    #[getter]
    /// Get the portfolio name.
    fn name(&self) -> Option<String> {
        self.inner.name.clone()
    }

    #[setter]
    /// Set the portfolio name.
    fn set_name(&mut self, name: Option<String>) {
        self.inner.name = name;
    }

    #[getter]
    /// Get the base currency.
    fn base_ccy(&self) -> PyCurrency {
        PyCurrency::new(self.inner.base_ccy)
    }

    #[getter]
    /// Get the valuation date.
    fn as_of(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.as_of)
    }

    #[getter]
    /// Get the portfolio entities.
    fn entities(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new(py);
        for (id, entity) in &self.inner.entities {
            dict.set_item(id.as_str(), PyEntity::new(entity.clone()))?;
        }
        Ok(dict.into())
    }

    #[getter]
    /// Get the portfolio positions.
    fn positions(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let positions: Vec<PyPosition> = self
            .inner
            .positions
            .iter()
            .map(|p| PyPosition::new(p.clone()))
            .collect();
        Ok(PyList::new(py, positions)?.into())
    }

    #[getter]
    /// Get the portfolio books (hierarchical organization).
    fn books(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new(py);
        for (id, book) in &self.inner.books {
            dict.set_item(id.as_str(), PyBook::new(book.clone()))?;
        }
        Ok(dict.into())
    }

    #[getter]
    /// Get portfolio tags.
    fn tags(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new(py);
        for (k, v) in &self.inner.tags {
            dict.set_item(k, v)?;
        }
        Ok(dict.into())
    }

    #[getter]
    /// Get portfolio metadata.
    fn meta(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let bound = pythonize::pythonize(py, &self.inner.meta)
            .map_err(|e| PyValueError::new_err(format!("Failed to convert meta: {}", e)))?;
        Ok(bound.unbind())
    }

    /// Convert to a serializable specification.
    fn to_spec(&self) -> PyPortfolioSpec {
        PyPortfolioSpec::new(self.inner.to_spec())
    }

    /// Reconstruct a Portfolio from a specification.
    #[staticmethod]
    fn from_spec(spec: &PyPortfolioSpec) -> PyResult<Self> {
        let portfolio = Portfolio::from_spec(spec.inner.clone()).map_err(portfolio_to_py)?;
        Ok(Self::new(portfolio))
    }

    fn __repr__(&self) -> String {
        format!(
            "Portfolio(id='{}', base_ccy={}, as_of={}, positions={})",
            self.inner.id,
            self.inner.base_ccy,
            self.inner.as_of,
            self.inner.positions.len()
        )
    }

    fn __str__(&self) -> String {
        self.inner
            .name
            .clone()
            .unwrap_or_else(|| self.inner.id.clone())
    }

    /// Return the number of positions in the portfolio.
    fn __len__(&self) -> usize {
        self.inner.positions.len()
    }

    /// Check if a position with the given ID exists in the portfolio.
    fn __contains__(&self, position_id: &str) -> bool {
        self.inner.get_position(position_id).is_some()
    }

    /// Get a position by index or ID.
    fn __getitem__(&self, key: &Bound<'_, PyAny>) -> PyResult<PyPosition> {
        if let Ok(idx) = key.extract::<isize>() {
            let len = self.inner.positions.len() as isize;
            let actual = if idx < 0 { len + idx } else { idx };
            if actual < 0 || actual >= len {
                return Err(pyo3::exceptions::PyIndexError::new_err(format!(
                    "position index out of range: {}",
                    idx
                )));
            }
            Ok(PyPosition::new(
                self.inner.positions[actual as usize].clone(),
            ))
        } else if let Ok(id) = key.extract::<String>() {
            self.inner
                .get_position(&id)
                .map(|p| PyPosition::new(p.clone()))
                .ok_or_else(|| {
                    pyo3::exceptions::PyKeyError::new_err(format!("Position '{}' not found", id))
                })
        } else {
            Err(PyTypeError::new_err("Position index must be int or str"))
        }
    }

    /// Return an iterator over the positions in the portfolio.
    fn __iter__(slf: PyRef<'_, Self>) -> PyResult<Py<PyPositionIterator>> {
        let positions: Vec<finstack_portfolio::Position> = slf.inner.positions.clone();
        Py::new(
            slf.py(),
            PyPositionIterator {
                positions,
                index: 0,
            },
        )
    }
}

/// Iterator over positions in a portfolio.
#[pyclass(module = "finstack.portfolio", name = "PositionIterator")]
pub struct PyPositionIterator {
    positions: Vec<finstack_portfolio::Position>,
    index: usize,
}

#[pymethods]
impl PyPositionIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<PyPosition> {
        if slf.index < slf.positions.len() {
            let pos = PyPosition::new(slf.positions[slf.index].clone());
            slf.index += 1;
            Some(pos)
        } else {
            None
        }
    }
}

/// A serializable portfolio specification.
#[pyclass(module = "finstack.portfolio", name = "PortfolioSpec")]
pub struct PyPortfolioSpec {
    pub(crate) inner: PortfolioSpec,
}

impl PyPortfolioSpec {
    pub(crate) fn new(inner: PortfolioSpec) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyPortfolioSpec {
    #[getter]
    fn id(&self) -> String {
        self.inner.id.clone()
    }

    #[getter]
    fn name(&self) -> Option<String> {
        self.inner.name.clone()
    }

    #[getter]
    fn base_ccy(&self) -> PyCurrency {
        PyCurrency::new(self.inner.base_ccy)
    }

    #[getter]
    fn as_of(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.as_of)
    }

    #[getter]
    fn positions(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let specs: Vec<PyPositionSpec> = self
            .inner
            .positions
            .iter()
            .map(|s| PyPositionSpec::new(s.clone()))
            .collect();
        Ok(PyList::new(py, specs)?.into())
    }

    /// Serialize to JSON string.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string_pretty(&self.inner)
            .map_err(|e| PyValueError::new_err(format!("JSON serialization failed: {}", e)))
    }

    /// Deserialize from JSON string.
    #[staticmethod]
    fn from_json(json_str: &str) -> PyResult<Self> {
        let spec: PortfolioSpec = serde_json::from_str(json_str)
            .map_err(|e| PyValueError::new_err(format!("JSON deserialization failed: {}", e)))?;
        Ok(Self::new(spec))
    }

    fn __repr__(&self) -> String {
        format!(
            "PortfolioSpec(id='{}', positions={})",
            self.inner.id,
            self.inner.positions.len()
        )
    }
}

/// Extract a Portfolio from Python object.
pub(crate) fn extract_portfolio(value: &Bound<'_, PyAny>) -> PyResult<Portfolio> {
    if let Ok(py_portfolio) = value.extract::<PyRef<PyPortfolio>>() {
        Ok(py_portfolio.inner.clone())
    } else {
        Err(PyTypeError::new_err("Expected Portfolio"))
    }
}

/// Register portfolio module exports.
pub(crate) fn register<'py>(
    _py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<String>> {
    parent.add_class::<PyPortfolio>()?;
    parent.add_class::<PyPositionIterator>()?;
    parent.add_class::<PyPortfolioSpec>()?;

    Ok(vec![
        "Portfolio".to_string(),
        "PositionIterator".to_string(),
        "PortfolioSpec".to_string(),
    ])
}
