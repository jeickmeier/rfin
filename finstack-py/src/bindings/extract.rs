//! Shared polymorphic extraction helpers for PyO3 bindings.
//!
//! Each helper accepts a `&Bound<'_, PyAny>` and tries two paths:
//!
//! 1. **Typed fast path** — cast to the corresponding `#[pyclass]` wrapper
//!    and clone the inner Rust type.  No JSON round-trip, no allocation beyond
//!    the clone itself.
//! 2. **JSON fallback** — extract a Python `str`, then `serde_json::from_str`.
//!    This keeps backward compatibility with callers that pass pre-serialized
//!    JSON strings.
//!
//! Using these helpers lets every public function transparently accept either
//! form, giving callers a zero-parse fast path when they already hold a typed
//! Python object.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use crate::bindings::core::market_data::context::PyMarketContext;
use crate::bindings::statements::evaluator::PyStatementResult;
use crate::bindings::statements::types::PyFinancialModelSpec;

fn to_py(e: impl std::fmt::Display) -> PyErr {
    PyValueError::new_err(e.to_string())
}

// ---------------------------------------------------------------------------
// Zero-clone access types (available for callers that only need &T)
// ---------------------------------------------------------------------------

/// Access to a [`FinancialModelSpec`] without cloning on the typed fast path.
///
/// When the caller passes a `FinancialModelSpec` Python object, the
/// `Borrowed` variant holds a `PyRef` guard — no clone occurs.  When the
/// caller passes a JSON string, the `Owned` variant holds the parsed value.
///
/// Use `Deref` (i.e. `&model`) for read-only access.  Call `.into_owned()`
/// only when ownership is truly needed (e.g. `goal_seek` which mutates).
pub enum ModelAccess<'py> {
    Borrowed(PyRef<'py, PyFinancialModelSpec>),
    Owned(finstack_statements::FinancialModelSpec),
}

impl std::ops::Deref for ModelAccess<'_> {
    type Target = finstack_statements::FinancialModelSpec;
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Borrowed(r) => &r.inner,
            Self::Owned(m) => m,
        }
    }
}

impl ModelAccess<'_> {
    /// Consume this access and produce an owned value, cloning only if
    /// the data was borrowed from a Python object.
    #[allow(dead_code)]
    pub fn into_owned(self) -> finstack_statements::FinancialModelSpec {
        match self {
            Self::Borrowed(r) => r.inner.clone(),
            Self::Owned(m) => m,
        }
    }
}

/// Access to a [`StatementResult`] without cloning on the typed fast path.
pub enum ResultAccess<'py> {
    Borrowed(PyRef<'py, PyStatementResult>),
    Owned(finstack_statements::evaluator::StatementResult),
}

impl std::ops::Deref for ResultAccess<'_> {
    type Target = finstack_statements::evaluator::StatementResult;
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Borrowed(r) => &r.inner,
            Self::Owned(r) => r,
        }
    }
}

impl ResultAccess<'_> {
    #[allow(dead_code)]
    pub fn into_owned(self) -> finstack_statements::evaluator::StatementResult {
        match self {
            Self::Borrowed(r) => r.inner.clone(),
            Self::Owned(r) => r,
        }
    }
}

/// Extract a [`FinancialModelSpec`] without cloning when a typed Python
/// object is passed.  Returns [`ModelAccess`] which dereferences to
/// `&FinancialModelSpec`.
#[allow(dead_code)]
pub fn extract_model_ref<'py>(obj: &Bound<'py, PyAny>) -> PyResult<ModelAccess<'py>> {
    if let Ok(spec) = obj.cast::<PyFinancialModelSpec>() {
        return Ok(ModelAccess::Borrowed(spec.borrow()));
    }
    let json: String = obj.extract()?;
    let inner: finstack_statements::FinancialModelSpec =
        serde_json::from_str(&json).map_err(to_py)?;
    Ok(ModelAccess::Owned(inner))
}

/// Extract a [`StatementResult`] without cloning when a typed Python
/// object is passed.
#[allow(dead_code)]
pub fn extract_results_ref<'py>(obj: &Bound<'py, PyAny>) -> PyResult<ResultAccess<'py>> {
    if let Ok(result) = obj.cast::<PyStatementResult>() {
        return Ok(ResultAccess::Borrowed(result.borrow()));
    }
    let json: String = obj.extract()?;
    let inner: finstack_statements::evaluator::StatementResult =
        serde_json::from_str(&json).map_err(to_py)?;
    Ok(ResultAccess::Owned(inner))
}

// ---------------------------------------------------------------------------
// Owned extraction (for callers that need mutable or owned values)
// ---------------------------------------------------------------------------

/// Extract a [`FinancialModelSpec`] — always produces an owned value.
///
/// Prefer [`extract_model_ref`] when only a reference is needed.
pub fn extract_model(obj: &Bound<'_, PyAny>) -> PyResult<finstack_statements::FinancialModelSpec> {
    if let Ok(spec) = obj.cast::<PyFinancialModelSpec>() {
        return Ok(spec.borrow().inner.clone());
    }
    let json: String = obj.extract()?;
    serde_json::from_str(&json).map_err(to_py)
}

/// Extract a [`MarketContext`] from a `MarketContext` Python object
/// (fast path) or a JSON string (fallback).
pub fn extract_market(
    obj: &Bound<'_, PyAny>,
) -> PyResult<finstack_core::market_data::context::MarketContext> {
    if let Ok(ctx) = obj.cast::<PyMarketContext>() {
        return Ok(ctx.borrow().inner.clone());
    }
    let json: String = obj.extract()?;
    serde_json::from_str(&json).map_err(to_py)
}

/// Extract an optional [`MarketContext`] from `Option<&Bound<'_, PyAny>>`.
///
/// Returns `Ok(None)` when `obj` is `None`.
pub fn extract_market_opt(
    obj: Option<&Bound<'_, PyAny>>,
) -> PyResult<Option<finstack_core::market_data::context::MarketContext>> {
    match obj {
        Some(o) => extract_market(o).map(Some),
        None => Ok(None),
    }
}
