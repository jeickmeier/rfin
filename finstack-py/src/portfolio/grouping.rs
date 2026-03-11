//! Python bindings for portfolio grouping utilities.

use crate::core::money::PyMoney;
use crate::portfolio::error::portfolio_to_py;
use crate::portfolio::positions::extract_portfolio;
use crate::portfolio::types::PyPosition;
use crate::portfolio::valuation::extract_portfolio_valuation;
use finstack_portfolio::grouping::{
    aggregate_by_attribute, aggregate_by_book, aggregate_by_multiple_attributes, group_by_attribute,
};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyList, PyModule, PyTuple};
use pyo3::Bound;

/// Group portfolio positions by an attribute.
///
/// Returns a dictionary mapping attribute values to lists of positions.
/// Positions missing the requested attribute are placed into the `_untagged` bucket.
///
/// Args:
///     portfolio: Portfolio to group.
///     attribute_key: Tag key to group by (e.g., "sector", "rating").
///
/// Returns:
///     dict[str, list[Position]]: Mapping of attribute values to position lists.
///
/// Raises:
///     RuntimeError: If grouping fails.
///
/// Examples:
///     >>> from finstack.portfolio import group_by_attribute
///     >>> by_sector = group_by_attribute(portfolio, "sector")
///     >>> by_sector["Technology"]
///     [Position(...), Position(...)]
#[pyfunction]
fn py_group_by_attribute(
    portfolio: &Bound<'_, PyAny>,
    attribute_key: &str,
    py: Python<'_>,
) -> PyResult<Py<PyAny>> {
    let portfolio_inner = extract_portfolio(portfolio)?;
    let grouped = group_by_attribute(&portfolio_inner.positions, attribute_key);

    let dict = PyDict::new(py);
    for (key, positions) in grouped {
        let py_positions: Vec<PyPosition> = positions
            .into_iter()
            .map(|p| PyPosition::new(p.clone()))
            .collect();
        dict.set_item(key, PyList::new(py, py_positions)?)?;
    }

    Ok(dict.into())
}

/// Aggregate portfolio valuation by an attribute.
///
/// Sums position values within each attribute group. Only positions with the
/// specified attribute key in their tags are included (otherwise `_untagged`). Values are converted
/// to the portfolio base currency before aggregation.
///
/// Args:
///     valuation: Portfolio valuation results.
///     portfolio: Portfolio containing positions.
///     attribute_key: Tag key to group by (e.g., "sector", "rating").
///
/// Returns:
///     dict[str, Money]: Mapping of attribute values to aggregated amounts.
///
/// Raises:
///     RuntimeError: If aggregation fails.
///
/// Examples:
///     >>> from finstack.portfolio import aggregate_by_attribute
///     >>> by_sector = aggregate_by_attribute(valuation, portfolio, "sector")
///     >>> by_sector["Technology"]
///     Money(USD, 5000000.0)
#[pyfunction]
fn py_aggregate_by_attribute(
    valuation: &Bound<'_, PyAny>,
    portfolio: &Bound<'_, PyAny>,
    attribute_key: &str,
    py: Python<'_>,
) -> PyResult<Py<PyAny>> {
    let valuation_inner = extract_portfolio_valuation(valuation)?;
    let portfolio_inner = extract_portfolio(portfolio)?;

    let aggregated = aggregate_by_attribute(
        &valuation_inner,
        &portfolio_inner.positions,
        attribute_key,
        portfolio_inner.base_ccy,
    )
    .map_err(portfolio_to_py)?;

    let dict = PyDict::new(py);
    for (key, money) in aggregated {
        dict.set_item(key, PyMoney::new(money))?;
    }

    Ok(dict.into())
}

/// Aggregate portfolio valuation by book hierarchy.
///
/// Computes total value for each book by summing direct position values plus
/// recursively aggregated values from child books.
///
/// Args:
///     valuation: Portfolio valuation results.
///     portfolio: Portfolio containing books and positions.
///
/// Returns:
///     dict[str, Money]: Mapping of book IDs to aggregated amounts (base currency).
///
/// Raises:
///     RuntimeError: If aggregation fails.
#[pyfunction]
fn py_aggregate_by_book(
    valuation: &Bound<'_, PyAny>,
    portfolio: &Bound<'_, PyAny>,
    py: Python<'_>,
) -> PyResult<Py<PyAny>> {
    let valuation_inner = extract_portfolio_valuation(valuation)?;
    let portfolio_inner = extract_portfolio(portfolio)?;

    let aggregated = aggregate_by_book(
        &valuation_inner,
        &portfolio_inner.books,
        portfolio_inner.base_ccy,
    )
    .map_err(portfolio_to_py)?;

    let dict = PyDict::new(py);
    for (book_id, money) in aggregated {
        dict.set_item(book_id.as_str(), PyMoney::new(money))?;
    }
    Ok(dict.into())
}

/// Aggregate portfolio valuation by multiple attributes simultaneously.
///
/// Groups positions by the combination of multiple tag keys and sums their
/// values. Returns a dictionary mapping attribute value tuples to aggregated
/// amounts in the portfolio base currency.
///
/// Args:
///     valuation: Portfolio valuation results.
///     portfolio: Portfolio containing positions.
///     attribute_keys: List of tag keys to group by (e.g., ["sector", "rating"]).
///
/// Returns:
///     dict[tuple[str, ...], Money]: Mapping of attribute value tuples to aggregated amounts.
///
/// Raises:
///     RuntimeError: If aggregation fails.
#[pyfunction]
fn py_aggregate_by_multiple_attributes(
    valuation: &Bound<'_, PyAny>,
    portfolio: &Bound<'_, PyAny>,
    attribute_keys: Vec<String>,
    py: Python<'_>,
) -> PyResult<Py<PyAny>> {
    let valuation_inner = extract_portfolio_valuation(valuation)?;
    let portfolio_inner = extract_portfolio(portfolio)?;
    let keys: Vec<&str> = attribute_keys.iter().map(|s| s.as_str()).collect();

    let aggregated = aggregate_by_multiple_attributes(
        &valuation_inner,
        &portfolio_inner.positions,
        &keys,
        portfolio_inner.base_ccy,
    )
    .map_err(portfolio_to_py)?;

    let dict = PyDict::new(py);
    for (key_vec, money) in aggregated {
        let tuple = PyTuple::new(py, &key_vec)?;
        dict.set_item(tuple, PyMoney::new(money))?;
    }
    Ok(dict.into())
}

/// Register grouping module exports.
pub(crate) fn register<'py>(
    _py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<String>> {
    let wrapped_group = wrap_pyfunction!(py_group_by_attribute, parent)?;
    parent.add("group_by_attribute", wrapped_group)?;

    let wrapped_agg = wrap_pyfunction!(py_aggregate_by_attribute, parent)?;
    parent.add("aggregate_by_attribute", wrapped_agg)?;

    let wrapped_book = wrap_pyfunction!(py_aggregate_by_book, parent)?;
    parent.add("aggregate_by_book", wrapped_book)?;

    let wrapped_multi = wrap_pyfunction!(py_aggregate_by_multiple_attributes, parent)?;
    parent.add("aggregate_by_multiple_attributes", wrapped_multi)?;

    Ok(vec![
        "group_by_attribute".to_string(),
        "aggregate_by_attribute".to_string(),
        "aggregate_by_book".to_string(),
        "aggregate_by_multiple_attributes".to_string(),
    ])
}
