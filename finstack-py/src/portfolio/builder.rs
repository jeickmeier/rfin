//! Python bindings for PortfolioBuilder.

use crate::core::currency::PyCurrency;
use crate::core::dates::utils::py_to_date;
use crate::portfolio::book::{extract_book, extract_book_id};
use crate::portfolio::error::portfolio_to_py;
use crate::portfolio::portfolio::PyPortfolio;
use crate::portfolio::types::{extract_entity, extract_position};
use finstack_portfolio::PortfolioBuilder;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyList, PyModule};
use pyo3::Bound;

/// Builder for constructing a Portfolio with validation.
///
/// The builder stores all intermediate values needed to construct a portfolio and checks
/// invariants such as base currency, valuation date, and entity references before the
/// final portfolio is produced.
///
/// Examples:
///     >>> from finstack.portfolio import PortfolioBuilder, Entity
///     >>> from finstack.core import Currency
///     >>> from datetime import date
///     >>> portfolio = (PortfolioBuilder("FUND_A")
///     ...     .name("Alpha Fund")
///     ...     .base_ccy(Currency.USD)
///     ...     .as_of(date(2024, 1, 1))
///     ...     .entity(Entity("ACME"))
///     ...     .build())
#[pyclass(module = "finstack.portfolio", name = "PortfolioBuilder")]
pub struct PyPortfolioBuilder {
    inner: PortfolioBuilder,
}

#[pymethods]
impl PyPortfolioBuilder {
    #[new]
    #[pyo3(text_signature = "(id)")]
    /// Create a new portfolio builder with the given identifier.
    ///
    /// Args:
    ///     id: Unique identifier for the portfolio.
    ///
    /// Returns:
    ///     PortfolioBuilder: New builder instance.
    ///
    /// Examples:
    ///     >>> builder = PortfolioBuilder("FUND_A")
    fn new_py(id: String) -> Self {
        Self {
            inner: PortfolioBuilder::new(id),
        }
    }

    #[pyo3(text_signature = "($self, name)")]
    /// Set the portfolio's human-readable name.
    ///
    /// Args:
    ///     name: Display name stored alongside the portfolio identifier.
    ///
    /// Returns:
    ///     PortfolioBuilder: Self for chaining.
    ///
    /// Examples:
    ///     >>> builder = PortfolioBuilder("FUND_A").name("Alpha Fund")
    fn name(mut slf: PyRefMut<'_, Self>, name: String) -> PyRefMut<'_, Self> {
        let temp = std::mem::replace(&mut slf.inner, PortfolioBuilder::new(""));
        slf.inner = temp.name(name);
        slf
    }

    #[pyo3(text_signature = "($self, ccy)")]
    /// Declare the portfolio's reporting currency.
    ///
    /// Args:
    ///     ccy: Currency to use when consolidating values and metrics.
    ///
    /// Returns:
    ///     PortfolioBuilder: Self for chaining.
    ///
    /// Examples:
    ///     >>> from finstack.core import Currency
    ///     >>> builder = PortfolioBuilder("FUND_A").base_ccy(Currency.USD)
    fn base_ccy<'py>(
        mut slf: PyRefMut<'py, Self>,
        ccy: &Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let currency = if let Ok(py_ccy) = ccy.extract::<PyRef<PyCurrency>>() {
            py_ccy.inner
        } else if let Ok(s) = ccy.extract::<String>() {
            s.parse()
                .map_err(|e| PyValueError::new_err(format!("Invalid currency: {}", e)))?
        } else {
            return Err(PyTypeError::new_err("Expected Currency or string"));
        };
        let temp = std::mem::replace(&mut slf.inner, PortfolioBuilder::new(""));
        slf.inner = temp.base_ccy(currency);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, date)")]
    /// Assign the valuation date used for pricing and analytics.
    ///
    /// Args:
    ///     date: The as-of date for valuation and risk calculation.
    ///
    /// Returns:
    ///     PortfolioBuilder: Self for chaining.
    ///
    /// Examples:
    ///     >>> from datetime import date
    ///     >>> builder = PortfolioBuilder("FUND_A").as_of(date(2024, 1, 1))
    fn as_of<'py>(
        mut slf: PyRefMut<'py, Self>,
        date: &Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let dt = py_to_date(date)?;
        let temp = std::mem::replace(&mut slf.inner, PortfolioBuilder::new(""));
        slf.inner = temp.as_of(dt);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, entity_or_entities)")]
    /// Register entity or entities with the builder.
    ///
    /// Accepts either a single Entity or a list of entities.
    ///
    /// Args:
    ///     entity_or_entities: Entity or list of entities to register.
    ///
    /// Returns:
    ///     PortfolioBuilder: Self for chaining.
    ///
    /// Examples:
    ///     >>> entity = Entity("ACME")
    ///     >>> builder = PortfolioBuilder("FUND_A").entity(entity)
    ///     >>> # Or with multiple entities:
    ///     >>> builder = PortfolioBuilder("FUND_A").entity([entity1, entity2])
    fn entity<'py>(
        mut slf: PyRefMut<'py, Self>,
        entity_or_entities: &Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        // Try to extract as single entity first
        if let Ok(entity) = extract_entity(entity_or_entities) {
            let temp = std::mem::replace(&mut slf.inner, PortfolioBuilder::new(""));
            slf.inner = temp.entity(entity);
            return Ok(slf);
        }

        // Try as list of entities
        if let Ok(list) = entity_or_entities.downcast::<PyList>() {
            let mut entities = Vec::new();
            for item in list.iter() {
                entities.push(extract_entity(&item)?);
            }
            let temp = std::mem::replace(&mut slf.inner, PortfolioBuilder::new(""));
            slf.inner = temp.entities(entities);
            return Ok(slf);
        }

        Err(PyTypeError::new_err("Expected Entity or list of entities"))
    }

    #[pyo3(text_signature = "($self, position_or_positions)")]
    /// Add position or positions to the portfolio.
    ///
    /// Accepts either a single Position or a list of positions.
    ///
    /// Args:
    ///     position_or_positions: Position or list of positions to add.
    ///
    /// Returns:
    ///     PortfolioBuilder: Self for chaining.
    ///
    /// Examples:
    ///     >>> position = Position("POS_1", "ENTITY_A", "INSTR_1", instrument, 1.0, PositionUnit.UNITS)
    ///     >>> builder = PortfolioBuilder("FUND_A").position(position)
    ///     >>> # Or with multiple positions:
    ///     >>> builder = PortfolioBuilder("FUND_A").position([pos1, pos2])
    fn position<'py>(
        mut slf: PyRefMut<'py, Self>,
        position_or_positions: &Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        // Try to extract as single position first
        if let Ok(position) = extract_position(position_or_positions) {
            let temp = std::mem::replace(&mut slf.inner, PortfolioBuilder::new(""));
            slf.inner = temp.position(position);
            return Ok(slf);
        }

        // Try as list of positions
        if let Ok(list) = position_or_positions.downcast::<PyList>() {
            let mut positions = Vec::new();
            for item in list.iter() {
                positions.push(extract_position(&item)?);
            }
            let temp = std::mem::replace(&mut slf.inner, PortfolioBuilder::new(""));
            slf.inner = temp.positions(positions);
            return Ok(slf);
        }

        Err(PyTypeError::new_err(
            "Expected Position or list of positions",
        ))
    }

    #[pyo3(text_signature = "($self, book_or_books)")]
    /// Add book or books to the portfolio hierarchy.
    ///
    /// Accepts either a single Book or a list of books.
    ///
    /// Args:
    ///     book_or_books: Book or list of books to add.
    ///
    /// Returns:
    ///     PortfolioBuilder: Self for chaining.
    fn book<'py>(
        mut slf: PyRefMut<'py, Self>,
        book_or_books: &Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        // Try to extract as single book first
        if let Ok(book) = extract_book(book_or_books) {
            let temp = std::mem::replace(&mut slf.inner, PortfolioBuilder::new(""));
            slf.inner = temp.book(book);
            return Ok(slf);
        }

        // Try as list of books
        if let Ok(list) = book_or_books.downcast::<PyList>() {
            let mut books = Vec::new();
            for item in list.iter() {
                books.push(extract_book(&item)?);
            }
            let temp = std::mem::replace(&mut slf.inner, PortfolioBuilder::new(""));
            slf.inner = temp.books(books);
            return Ok(slf);
        }

        Err(PyTypeError::new_err("Expected Book or list of books"))
    }

    #[pyo3(text_signature = "($self, position_id, book_id)")]
    /// Assign a position to a book.
    ///
    /// This updates both the position's `book_id` and the book's `position_ids` list.
    ///
    /// Args:
    ///     position_id: Position identifier.
    ///     book_id: Book identifier (string or BookId).
    ///
    /// Returns:
    ///     PortfolioBuilder: Self for chaining.
    ///
    /// Raises:
    ///     ValueError: If the position or book doesn't exist.
    fn add_position_to_book<'py>(
        mut slf: PyRefMut<'py, Self>,
        position_id: String,
        book_id: &Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let book_id = extract_book_id(book_id)?;
        let temp = std::mem::replace(&mut slf.inner, PortfolioBuilder::new(""));
        slf.inner = temp
            .add_position_to_book(position_id, book_id)
            .map_err(portfolio_to_py)?;
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, key, value)")]
    /// Add a portfolio-level tag.
    ///
    /// Args:
    ///     key: Tag key.
    ///     value: Tag value.
    ///
    /// Returns:
    ///     PortfolioBuilder: Self for chaining.
    ///
    /// Examples:
    ///     >>> builder = PortfolioBuilder("FUND_A").tag("strategy", "long_only")
    fn tag(mut slf: PyRefMut<'_, Self>, key: String, value: String) -> PyRefMut<'_, Self> {
        let temp = std::mem::replace(&mut slf.inner, PortfolioBuilder::new(""));
        slf.inner = temp.tag(key, value);
        slf
    }

    #[pyo3(text_signature = "($self, key, value)")]
    /// Add portfolio-level metadata.
    ///
    /// Args:
    ///     key: Metadata key.
    ///     value: Metadata value (must be JSON-serializable).
    ///
    /// Returns:
    ///     PortfolioBuilder: Self for chaining.
    ///
    /// Examples:
    ///     >>> builder = PortfolioBuilder("FUND_A").meta("inception", "2020-01-01")
    fn meta<'py>(
        mut slf: PyRefMut<'py, Self>,
        key: String,
        value: &Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let json_value: serde_json::Value = pythonize::depythonize(value)
            .map_err(|e| PyValueError::new_err(format!("Failed to convert meta value: {}", e)))?;
        let temp = std::mem::replace(&mut slf.inner, PortfolioBuilder::new(""));
        slf.inner = temp.meta(key, json_value);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self)")]
    /// Build and validate the portfolio.
    ///
    /// Returns:
    ///     Portfolio: Validated portfolio instance.
    ///
    /// Raises:
    ///     ValueError: If validation fails (missing base_ccy, as_of, or invalid references).
    ///
    /// Examples:
    ///     >>> portfolio = builder.build()
    fn build(mut slf: PyRefMut<'_, Self>) -> PyResult<PyPortfolio> {
        let inner = std::mem::replace(&mut slf.inner, PortfolioBuilder::new(""));
        let portfolio = inner.build().map_err(portfolio_to_py)?;
        Ok(PyPortfolio::new(portfolio))
    }

    fn __repr__(&self) -> String {
        "PortfolioBuilder(...)".to_string()
    }
}

/// Register builder module exports.
pub(crate) fn register<'py>(
    _py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<String>> {
    parent.add_class::<PyPortfolioBuilder>()?;

    Ok(vec!["PortfolioBuilder".to_string()])
}
