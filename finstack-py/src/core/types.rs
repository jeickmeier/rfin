//! Type bindings: phantom-typed IDs and rate helpers.
//!
//! This module exposes newtype identifiers (CurveId, InstrumentId, etc.) and
//! rate conversion types (Rate, Bps, Percentage) from finstack_core::types
//! to Python. These types prevent accidental ID mismatches and provide
//! clear conversion semantics for financial rates.

use crate::core::common::pycmp::richcmp_eq_ne;
use finstack_core::types::{Bps, CurveId, IndexId, InstrumentId, PriceId, Rate, UnderlyingId};
use pyo3::basic::CompareOp;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::{Bound, IntoPyObjectExt};
use std::hash::{Hash, Hasher};

// ============================================================================
// ID Types
// ============================================================================

/// Type-safe identifier for market data curves.
///
/// Parameters
/// ----------
/// id : str
///     String identifier for the curve.
///
/// Returns
/// -------
/// CurveId
///     Curve identifier instance.
#[pyclass(name = "CurveId", module = "finstack.core.types", frozen)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PyCurveId {
    pub(crate) inner: CurveId,
}

impl PyCurveId {
    pub(crate) fn new(inner: CurveId) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCurveId {
    #[new]
    #[pyo3(text_signature = "(id)")]
    fn ctor(id: &str) -> Self {
        Self::new(CurveId::from(id))
    }

    #[pyo3(text_signature = "(self)")]
    /// Get the string representation of this ID.
    fn as_str(&self) -> &str {
        self.inner.as_str()
    }

    fn __repr__(&self) -> String {
        format!("CurveId('{}')", self.inner.as_str())
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __hash__(&self) -> isize {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.inner.hash(&mut hasher);
        (hasher.finish() & isize::MAX as u64) as isize
    }

    fn __richcmp__(
        &self,
        other: Bound<'_, PyAny>,
        op: CompareOp,
        py: Python<'_>,
    ) -> PyResult<PyObject> {
        let rhs = match other.extract::<PyRef<PyCurveId>>() {
            Ok(id) => Some(id.inner.clone()),
            Err(_) => None,
        };
        richcmp_eq_ne(py, &self.inner, rhs, op)
    }
}

/// Type-safe identifier for financial instruments.
#[pyclass(name = "InstrumentId", module = "finstack.core.types", frozen)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PyInstrumentId {
    pub(crate) inner: InstrumentId,
}

impl PyInstrumentId {
    pub(crate) fn new(inner: InstrumentId) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyInstrumentId {
    #[new]
    #[pyo3(text_signature = "(id)")]
    fn ctor(id: &str) -> Self {
        Self::new(InstrumentId::from(id))
    }

    #[pyo3(text_signature = "(self)")]
    fn as_str(&self) -> &str {
        self.inner.as_str()
    }

    fn __repr__(&self) -> String {
        format!("InstrumentId('{}')", self.inner.as_str())
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __hash__(&self) -> isize {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.inner.hash(&mut hasher);
        (hasher.finish() & isize::MAX as u64) as isize
    }

    fn __richcmp__(
        &self,
        other: Bound<'_, PyAny>,
        op: CompareOp,
        py: Python<'_>,
    ) -> PyResult<PyObject> {
        let rhs = match other.extract::<PyRef<PyInstrumentId>>() {
            Ok(id) => Some(id.inner.clone()),
            Err(_) => None,
        };
        richcmp_eq_ne(py, &self.inner, rhs, op)
    }
}

/// Type-safe identifier for market indices.
#[pyclass(name = "IndexId", module = "finstack.core.types", frozen)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PyIndexId {
    pub(crate) inner: IndexId,
}

impl PyIndexId {
    pub(crate) fn new(inner: IndexId) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyIndexId {
    #[new]
    #[pyo3(text_signature = "(id)")]
    fn ctor(id: &str) -> Self {
        Self::new(IndexId::from(id))
    }

    #[pyo3(text_signature = "(self)")]
    fn as_str(&self) -> &str {
        self.inner.as_str()
    }

    fn __repr__(&self) -> String {
        format!("IndexId('{}')", self.inner.as_str())
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __hash__(&self) -> isize {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.inner.hash(&mut hasher);
        (hasher.finish() & isize::MAX as u64) as isize
    }

    fn __richcmp__(
        &self,
        other: Bound<'_, PyAny>,
        op: CompareOp,
        py: Python<'_>,
    ) -> PyResult<PyObject> {
        let rhs = match other.extract::<PyRef<PyIndexId>>() {
            Ok(id) => Some(id.inner.clone()),
            Err(_) => None,
        };
        richcmp_eq_ne(py, &self.inner, rhs, op)
    }
}

/// Type-safe identifier for market prices/scalars.
#[pyclass(name = "PriceId", module = "finstack.core.types", frozen)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PyPriceId {
    pub(crate) inner: PriceId,
}

impl PyPriceId {
    pub(crate) fn new(inner: PriceId) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyPriceId {
    #[new]
    #[pyo3(text_signature = "(id)")]
    fn ctor(id: &str) -> Self {
        Self::new(PriceId::from(id))
    }

    #[pyo3(text_signature = "(self)")]
    fn as_str(&self) -> &str {
        self.inner.as_str()
    }

    fn __repr__(&self) -> String {
        format!("PriceId('{}')", self.inner.as_str())
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __hash__(&self) -> isize {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.inner.hash(&mut hasher);
        (hasher.finish() & isize::MAX as u64) as isize
    }

    fn __richcmp__(
        &self,
        other: Bound<'_, PyAny>,
        op: CompareOp,
        py: Python<'_>,
    ) -> PyResult<PyObject> {
        let rhs = match other.extract::<PyRef<PyPriceId>>() {
            Ok(id) => Some(id.inner.clone()),
            Err(_) => None,
        };
        richcmp_eq_ne(py, &self.inner, rhs, op)
    }
}

/// Type-safe identifier for underlying assets.
#[pyclass(name = "UnderlyingId", module = "finstack.core.types", frozen)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PyUnderlyingId {
    pub(crate) inner: UnderlyingId,
}

impl PyUnderlyingId {
    pub(crate) fn new(inner: UnderlyingId) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyUnderlyingId {
    #[new]
    #[pyo3(text_signature = "(id)")]
    fn ctor(id: &str) -> Self {
        Self::new(UnderlyingId::from(id))
    }

    #[pyo3(text_signature = "(self)")]
    fn as_str(&self) -> &str {
        self.inner.as_str()
    }

    fn __repr__(&self) -> String {
        format!("UnderlyingId('{}')", self.inner.as_str())
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __hash__(&self) -> isize {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.inner.hash(&mut hasher);
        (hasher.finish() & isize::MAX as u64) as isize
    }

    fn __richcmp__(
        &self,
        other: Bound<'_, PyAny>,
        op: CompareOp,
        py: Python<'_>,
    ) -> PyResult<PyObject> {
        let rhs = match other.extract::<PyRef<PyUnderlyingId>>() {
            Ok(id) => Some(id.inner.clone()),
            Err(_) => None,
        };
        richcmp_eq_ne(py, &self.inner, rhs, op)
    }
}

// ============================================================================
// Rate Types
// ============================================================================

/// Financial rate stored as decimal (0.05 = 5%).
///
/// Parameters
/// ----------
/// None
///     Use class methods like :meth:`from_percent`, :meth:`from_decimal`, or :meth:`from_bps`.
///
/// Returns
/// -------
/// Rate
///     Rate instance.
#[pyclass(name = "Rate", module = "finstack.core.types", frozen)]
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct PyRate {
    pub(crate) inner: Rate,
}

impl PyRate {
    pub(crate) fn new(inner: Rate) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyRate {
    #[staticmethod]
    #[pyo3(text_signature = "(decimal)")]
    /// Create a rate from a decimal value (0.05 = 5%).
    fn from_decimal(decimal: f64) -> Self {
        Self::new(Rate::from_decimal(decimal))
    }

    #[staticmethod]
    #[pyo3(text_signature = "(percent)")]
    /// Create a rate from a percentage value (5.0 = 5%).
    fn from_percent(percent: f64) -> Self {
        Self::new(Rate::from_percent(percent))
    }

    #[staticmethod]
    #[pyo3(text_signature = "(bps)")]
    /// Create a rate from basis points (500 bps = 5%).
    fn from_bps(bps: i32) -> Self {
        Self::new(Rate::from_bps(bps))
    }

    #[pyo3(text_signature = "(self)")]
    /// Get the rate as a decimal value.
    fn as_decimal(&self) -> f64 {
        self.inner.as_decimal()
    }

    #[pyo3(text_signature = "(self)")]
    /// Get the rate as a percentage value.
    fn as_percent(&self) -> f64 {
        self.inner.as_percent()
    }

    #[pyo3(text_signature = "(self)")]
    /// Get the rate as basis points (rounded to nearest integer).
    fn as_bps(&self) -> i32 {
        self.inner.as_bps()
    }

    fn __repr__(&self) -> String {
        format!("Rate({})", self.inner.as_decimal())
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __hash__(&self) -> isize {
        let bits = self.inner.as_decimal().to_bits();
        (bits & isize::MAX as u64) as isize
    }

    fn __richcmp__(
        &self,
        other: Bound<'_, PyAny>,
        op: CompareOp,
        py: Python<'_>,
    ) -> PyResult<PyObject> {
        if let Ok(rhs) = other.extract::<PyRef<PyRate>>() {
            let result = match op {
                CompareOp::Lt => self.inner < rhs.inner,
                CompareOp::Le => self.inner <= rhs.inner,
                CompareOp::Eq => self.inner == rhs.inner,
                CompareOp::Ne => self.inner != rhs.inner,
                CompareOp::Gt => self.inner > rhs.inner,
                CompareOp::Ge => self.inner >= rhs.inner,
            };
            let py_bool = result.into_bound_py_any(py)?;
            return Ok(py_bool.into());
        }
        Ok(py.NotImplemented())
    }

    fn __add__(&self, other: &Self) -> Self {
        Self::new(self.inner + other.inner)
    }

    fn __sub__(&self, other: &Self) -> Self {
        Self::new(self.inner - other.inner)
    }

    fn __mul__(&self, scalar: f64) -> Self {
        Self::new(self.inner * scalar)
    }

    fn __truediv__(&self, scalar: f64) -> PyResult<Self> {
        if scalar == 0.0 {
            return Err(PyValueError::new_err("Cannot divide by zero"));
        }
        Ok(Self::new(self.inner / scalar))
    }

    fn __neg__(&self) -> Self {
        Self::new(-self.inner)
    }
}

/// Basis points - a unit of measure for rates (1 bp = 0.01%).
///
/// Parameters
/// ----------
/// bps : int
///     Basis points value.
///
/// Returns
/// -------
/// Bps
///     Basis points instance.
#[pyclass(name = "Bps", module = "finstack.core.types", frozen)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PyBps {
    pub(crate) inner: Bps,
}

impl PyBps {
    pub(crate) fn new(inner: Bps) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyBps {
    #[new]
    #[pyo3(text_signature = "(bps)")]
    fn ctor(bps: i32) -> Self {
        Self::new(Bps::new(bps))
    }

    #[pyo3(text_signature = "(self)")]
    /// Get the basis points as an integer.
    fn as_bps(&self) -> i32 {
        self.inner.as_bps()
    }

    #[pyo3(text_signature = "(self)")]
    /// Convert to decimal representation.
    fn as_decimal(&self) -> f64 {
        self.inner.as_decimal()
    }

    #[pyo3(text_signature = "(self)")]
    /// Convert to percentage representation.
    fn as_percent(&self) -> f64 {
        self.inner.as_percent()
    }

    #[pyo3(text_signature = "(self)")]
    /// Convert to Rate.
    fn as_rate(&self) -> PyRate {
        PyRate::new(self.inner.as_rate())
    }

    fn __repr__(&self) -> String {
        format!("Bps({})", self.inner.as_bps())
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __hash__(&self) -> isize {
        self.inner.as_bps() as isize
    }

    fn __richcmp__(
        &self,
        other: Bound<'_, PyAny>,
        op: CompareOp,
        py: Python<'_>,
    ) -> PyResult<PyObject> {
        if let Ok(rhs) = other.extract::<PyRef<PyBps>>() {
            let result = match op {
                CompareOp::Lt => self.inner < rhs.inner,
                CompareOp::Le => self.inner <= rhs.inner,
                CompareOp::Eq => self.inner == rhs.inner,
                CompareOp::Ne => self.inner != rhs.inner,
                CompareOp::Gt => self.inner > rhs.inner,
                CompareOp::Ge => self.inner >= rhs.inner,
            };
            let py_bool = result.into_bound_py_any(py)?;
            return Ok(py_bool.into());
        }
        Ok(py.NotImplemented())
    }

    fn __add__(&self, other: &Self) -> Self {
        Self::new(self.inner + other.inner)
    }

    fn __sub__(&self, other: &Self) -> Self {
        Self::new(self.inner - other.inner)
    }

    fn __mul__(&self, scalar: i32) -> Self {
        Self::new(self.inner * scalar)
    }

    fn __truediv__(&self, scalar: i32) -> PyResult<Self> {
        if scalar == 0 {
            return Err(PyValueError::new_err("Cannot divide by zero"));
        }
        Ok(Self::new(self.inner / scalar))
    }

    fn __neg__(&self) -> Self {
        Self::new(-self.inner)
    }
}

/// A percentage value (5.0 = 5%).
///
/// Parameters
/// ----------
/// percent : float
///     Percentage value.
///
/// Returns
/// -------
/// Percentage
///     Percentage instance.
#[pyclass(name = "Percentage", module = "finstack.core.types", frozen)]
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct PyPercentage {
    pub(crate) inner: finstack_core::types::Percentage,
}

impl PyPercentage {
    pub(crate) fn new(inner: finstack_core::types::Percentage) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyPercentage {
    #[new]
    #[pyo3(text_signature = "(percent)")]
    fn ctor(percent: f64) -> Self {
        Self::new(finstack_core::types::Percentage::new(percent))
    }

    #[pyo3(text_signature = "(self)")]
    /// Get the percentage value.
    fn as_percent(&self) -> f64 {
        self.inner.as_percent()
    }

    #[pyo3(text_signature = "(self)")]
    /// Convert to decimal representation.
    fn as_decimal(&self) -> f64 {
        self.inner.as_decimal()
    }

    #[pyo3(text_signature = "(self)")]
    /// Convert to Rate.
    fn as_rate(&self) -> PyRate {
        PyRate::new(self.inner.as_rate())
    }

    #[pyo3(text_signature = "(self)")]
    /// Convert to basis points.
    fn as_bps(&self) -> i32 {
        self.inner.as_bps()
    }

    fn __repr__(&self) -> String {
        format!("Percentage({})", self.inner.as_percent())
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __hash__(&self) -> isize {
        let bits = self.inner.as_percent().to_bits();
        (bits & isize::MAX as u64) as isize
    }

    fn __richcmp__(
        &self,
        other: Bound<'_, PyAny>,
        op: CompareOp,
        py: Python<'_>,
    ) -> PyResult<PyObject> {
        if let Ok(rhs) = other.extract::<PyRef<PyPercentage>>() {
            let result = match op {
                CompareOp::Lt => self.inner < rhs.inner,
                CompareOp::Le => self.inner <= rhs.inner,
                CompareOp::Eq => self.inner == rhs.inner,
                CompareOp::Ne => self.inner != rhs.inner,
                CompareOp::Gt => self.inner > rhs.inner,
                CompareOp::Ge => self.inner >= rhs.inner,
            };
            let py_bool = result.into_bound_py_any(py)?;
            return Ok(py_bool.into());
        }
        Ok(py.NotImplemented())
    }

    fn __add__(&self, other: &Self) -> Self {
        Self::new(self.inner + other.inner)
    }

    fn __sub__(&self, other: &Self) -> Self {
        Self::new(self.inner - other.inner)
    }

    fn __mul__(&self, scalar: f64) -> Self {
        Self::new(self.inner * scalar)
    }

    fn __truediv__(&self, scalar: f64) -> PyResult<Self> {
        if scalar == 0.0 {
            return Err(PyValueError::new_err("Cannot divide by zero"));
        }
        Ok(Self::new(self.inner / scalar))
    }

    fn __neg__(&self) -> Self {
        Self::new(-self.inner)
    }
}

pub(crate) fn register<'py>(py: Python<'py>, parent: &Bound<'py, PyModule>) -> PyResult<()> {
    let module = PyModule::new(py, "types")?;
    module.setattr(
        "__doc__",
        concat!(
            "Type-safe identifiers and rate helpers mirroring finstack_core::types.\n\n",
            "This module provides:\n",
            "- ID newtypes: CurveId, InstrumentId, IndexId, PriceId, UnderlyingId\n",
            "- Rate helpers: Rate, Bps, Percentage\n\n",
            "These types prevent accidental mismatches and provide clear conversion semantics."
        ),
    )?;

    // Register ID types
    module.add_class::<PyCurveId>()?;
    module.add_class::<PyInstrumentId>()?;
    module.add_class::<PyIndexId>()?;
    module.add_class::<PyPriceId>()?;
    module.add_class::<PyUnderlyingId>()?;

    // Register rate types
    module.add_class::<PyRate>()?;
    module.add_class::<PyBps>()?;
    module.add_class::<PyPercentage>()?;

    let exports = [
        "CurveId",
        "InstrumentId",
        "IndexId",
        "PriceId",
        "UnderlyingId",
        "Rate",
        "Bps",
        "Percentage",
    ];
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    Ok(())
}
