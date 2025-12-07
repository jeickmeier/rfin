//! Type bindings: phantom-typed IDs and rate helpers.
//!
//! This module exposes newtype identifiers (CurveId, InstrumentId, etc.) and
//! rate conversion types (Rate, Bps, Percentage) from finstack_core::types
//! to Python. These types prevent accidental ID mismatches and provide
//! clear conversion semantics for financial rates.

use crate::core::common::pycmp::richcmp_eq_ne;
use crate::errors::core_to_py;
use finstack_core::types::moodys_warf_factor;
use finstack_core::types::ratings::{
    CreditRating, NotchedRating, RatingFactorTable, RatingLabel, RatingNotch,
};
use finstack_core::types::{Bps, CurveId, IndexId, InstrumentId, PriceId, Rate, UnderlyingId};
use pyo3::basic::CompareOp;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule, PyType};
use pyo3::{Bound, IntoPyObjectExt};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::str::FromStr;

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
    ) -> PyResult<Py<PyAny>> {
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
    ) -> PyResult<Py<PyAny>> {
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
    ) -> PyResult<Py<PyAny>> {
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
    ) -> PyResult<Py<PyAny>> {
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
    ) -> PyResult<Py<PyAny>> {
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
    ) -> PyResult<Py<PyAny>> {
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
    ) -> PyResult<Py<PyAny>> {
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
    ) -> PyResult<Py<PyAny>> {
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

// ============================================================================
// Credit ratings
// ============================================================================

#[pyclass(name = "RatingNotch", module = "finstack.core.types", frozen)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PyRatingNotch {
    pub(crate) inner: RatingNotch,
}

impl PyRatingNotch {
    pub(crate) const fn new(inner: RatingNotch) -> Self {
        Self { inner }
    }

    fn label(&self) -> &'static str {
        match self.inner {
            RatingNotch::Plus => "plus",
            RatingNotch::Flat => "flat",
            RatingNotch::Minus => "minus",
        }
    }
}

#[pymethods]
impl PyRatingNotch {
    #[classattr]
    const PLUS: Self = Self {
        inner: RatingNotch::Plus,
    };
    #[classattr]
    const FLAT: Self = Self {
        inner: RatingNotch::Flat,
    };
    #[classattr]
    const MINUS: Self = Self {
        inner: RatingNotch::Minus,
    };

    #[getter]
    fn name(&self) -> &'static str {
        self.label()
    }

    #[getter]
    fn symbol(&self) -> &'static str {
        self.inner.symbol()
    }

    fn __repr__(&self) -> String {
        format!("RatingNotch('{}')", self.label())
    }

    fn __str__(&self) -> &'static str {
        self.inner.symbol()
    }

    fn __hash__(&self) -> isize {
        self.inner as isize
    }

    fn __richcmp__(
        &self,
        other: Bound<'_, PyAny>,
        op: CompareOp,
        py: Python<'_>,
    ) -> PyResult<Py<PyAny>> {
        let rhs = match other.extract::<PyRef<PyRatingNotch>>() {
            Ok(v) => Some(v.inner),
            Err(_) => None,
        };
        let result = match op {
            CompareOp::Eq => rhs.map(|v| v == self.inner).unwrap_or(false),
            CompareOp::Ne => rhs.map(|v| v != self.inner).unwrap_or(true),
            _ => return Err(PyTypeError::new_err("Unsupported comparison")),
        };
        Ok(result.into_bound_py_any(py)?.into())
    }
}

impl fmt::Display for PyRatingNotch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner.symbol())
    }
}

#[pyclass(name = "CreditRating", module = "finstack.core.types", frozen)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PyCreditRating {
    pub(crate) inner: CreditRating,
}

impl PyCreditRating {
    pub(crate) const fn new(inner: CreditRating) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCreditRating {
    #[classattr]
    const AAA: Self = Self::new(CreditRating::AAA);
    #[classattr]
    const AA: Self = Self::new(CreditRating::AA);
    #[classattr]
    const A: Self = Self::new(CreditRating::A);
    #[classattr]
    const BBB: Self = Self::new(CreditRating::BBB);
    #[classattr]
    const BB: Self = Self::new(CreditRating::BB);
    #[classattr]
    const B: Self = Self::new(CreditRating::B);
    #[classattr]
    const CCC: Self = Self::new(CreditRating::CCC);
    #[classattr]
    const CC: Self = Self::new(CreditRating::CC);
    #[classattr]
    const C: Self = Self::new(CreditRating::C);
    #[classattr]
    const D: Self = Self::new(CreditRating::D);
    #[classattr]
    const NR: Self = Self::new(CreditRating::NR);

    #[new]
    #[pyo3(text_signature = "(value)")]
    fn ctor(value: &str) -> PyResult<Self> {
        CreditRating::from_str(value)
            .map(Self::new)
            .map_err(core_to_py)
    }

    #[getter]
    fn name(&self) -> String {
        self.inner.to_string()
    }

    #[pyo3(text_signature = "(self, notch)")]
    fn with_notch(&self, notch: PyRef<PyRatingNotch>) -> PyNotchedRating {
        PyNotchedRating::new(self.inner.with_notch(notch.inner))
    }

    fn is_investment_grade(&self) -> bool {
        self.inner.is_investment_grade()
    }

    fn is_speculative_grade(&self) -> bool {
        self.inner.is_speculative_grade()
    }

    fn is_default(&self) -> bool {
        self.inner.is_default()
    }

    fn __repr__(&self) -> String {
        format!("CreditRating('{}')", self.inner)
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __hash__(&self) -> isize {
        self.inner as isize
    }

    fn __richcmp__(
        &self,
        other: Bound<'_, PyAny>,
        op: CompareOp,
        py: Python<'_>,
    ) -> PyResult<Py<PyAny>> {
        let rhs = match other.extract::<PyRef<PyCreditRating>>() {
            Ok(v) => Some(v.inner),
            Err(_) => None,
        };
        let result = match op {
            CompareOp::Eq => rhs.map(|v| v == self.inner).unwrap_or(false),
            CompareOp::Ne => rhs.map(|v| v != self.inner).unwrap_or(true),
            _ => return Err(PyTypeError::new_err("Unsupported comparison")),
        };
        Ok(result.into_bound_py_any(py)?.into())
    }
}

impl fmt::Display for PyCreditRating {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner)
    }
}

#[pyclass(name = "NotchedRating", module = "finstack.core.types", frozen)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PyNotchedRating {
    pub(crate) inner: NotchedRating,
}

impl PyNotchedRating {
    pub(crate) const fn new(inner: NotchedRating) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyNotchedRating {
    #[new]
    #[pyo3(signature = (rating, notch=None), text_signature = "(rating, notch=None)")]
    fn ctor(rating: PyRef<PyCreditRating>, notch: Option<PyRef<PyRatingNotch>>) -> PyResult<Self> {
        let notch_val = notch.map_or(RatingNotch::Flat, |n| n.inner);
        Ok(Self::new(rating.inner.with_notch(notch_val)))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, value)")]
    fn parse(_cls: &Bound<'_, PyType>, value: &str) -> PyResult<Self> {
        value
            .parse::<NotchedRating>()
            .map(Self::new)
            .map_err(core_to_py)
    }

    #[getter]
    fn base(&self) -> PyCreditRating {
        PyCreditRating::new(self.inner.base())
    }

    #[getter]
    fn notch(&self) -> PyRatingNotch {
        PyRatingNotch::new(self.inner.notch())
    }

    fn is_investment_grade(&self) -> bool {
        self.inner.is_investment_grade()
    }

    fn is_speculative_grade(&self) -> bool {
        self.inner.is_speculative_grade()
    }

    fn is_default(&self) -> bool {
        self.inner.is_default()
    }

    #[pyo3(text_signature = "(self)")]
    fn without_notch(&self) -> Self {
        Self::new(self.inner.without_notch())
    }

    #[pyo3(text_signature = "(self)")]
    fn moodys(&self) -> String {
        self.inner.to_moodys_string()
    }

    fn __repr__(&self) -> String {
        format!("NotchedRating('{}')", self.inner)
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __hash__(&self) -> isize {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.inner.base().hash(&mut hasher);
        self.inner.notch().hash(&mut hasher);
        (hasher.finish() & isize::MAX as u64) as isize
    }

    fn __richcmp__(
        &self,
        other: Bound<'_, PyAny>,
        op: CompareOp,
        py: Python<'_>,
    ) -> PyResult<Py<PyAny>> {
        let rhs = match other.extract::<PyRef<PyNotchedRating>>() {
            Ok(v) => Some(v.inner),
            Err(_) => None,
        };
        let result = match op {
            CompareOp::Eq => rhs.map(|v| v == self.inner).unwrap_or(false),
            CompareOp::Ne => rhs.map(|v| v != self.inner).unwrap_or(true),
            _ => return Err(PyTypeError::new_err("Unsupported comparison")),
        };
        Ok(result.into_bound_py_any(py)?.into())
    }
}

impl fmt::Display for PyNotchedRating {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner)
    }
}

#[pyclass(name = "RatingLabel", module = "finstack.core.types", frozen)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PyRatingLabel {
    pub(crate) inner: RatingLabel,
}

impl PyRatingLabel {
    pub(crate) fn new(inner: RatingLabel) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyRatingLabel {
    #[classmethod]
    #[pyo3(text_signature = "(cls, rating)")]
    fn generic(_cls: &Bound<'_, PyType>, rating: PyRef<PyNotchedRating>) -> Self {
        Self::new(RatingLabel::generic(rating.inner))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, rating)")]
    fn moodys(_cls: &Bound<'_, PyType>, rating: PyRef<PyNotchedRating>) -> Self {
        Self::new(RatingLabel::moodys(rating.inner))
    }

    #[getter]
    fn value(&self) -> &str {
        self.inner.as_str()
    }

    fn __repr__(&self) -> String {
        format!("RatingLabel('{}')", self.inner.as_str())
    }

    fn __str__(&self) -> &str {
        self.inner.as_str()
    }

    fn __hash__(&self) -> isize {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.inner.as_str().hash(&mut hasher);
        (hasher.finish() & isize::MAX as u64) as isize
    }

    fn __richcmp__(
        &self,
        other: Bound<'_, PyAny>,
        op: CompareOp,
        py: Python<'_>,
    ) -> PyResult<Py<PyAny>> {
        let rhs = match other.extract::<PyRef<PyRatingLabel>>() {
            Ok(v) => Some(v.inner.as_str().to_string()),
            Err(_) => None,
        };
        let result = match op {
            CompareOp::Eq => rhs
                .as_ref()
                .map(|v| v == self.inner.as_str())
                .unwrap_or(false),
            CompareOp::Ne => rhs
                .as_ref()
                .map(|v| v != self.inner.as_str())
                .unwrap_or(true),
            _ => return Err(PyTypeError::new_err("Unsupported comparison")),
        };
        Ok(result.into_bound_py_any(py)?.into())
    }
}

#[pyclass(name = "RatingFactorTable", module = "finstack.core.types")]
#[derive(Clone, Debug)]
pub struct PyRatingFactorTable {
    pub(crate) inner: RatingFactorTable,
}

impl PyRatingFactorTable {
    pub(crate) fn new(inner: RatingFactorTable) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyRatingFactorTable {
    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    fn moodys_standard(_cls: &Bound<'_, PyType>) -> Self {
        Self::new(RatingFactorTable::moodys_standard())
    }

    #[pyo3(text_signature = "(self, rating)")]
    fn get_factor(&self, rating: Bound<'_, PyAny>) -> PyResult<f64> {
        let rating = extract_notched_rating(&rating)?;
        Ok(self.inner.get_factor(rating))
    }

    #[getter]
    fn agency(&self) -> &str {
        self.inner.agency()
    }

    #[getter]
    fn methodology(&self) -> &str {
        self.inner.methodology()
    }

    #[getter]
    fn default_factor(&self) -> f64 {
        self.inner.default_factor()
    }

    fn __repr__(&self) -> String {
        format!(
            "RatingFactorTable(agency='{}', methodology='{}')",
            self.inner.agency(),
            self.inner.methodology()
        )
    }
}

#[pyfunction(name = "moodys_warf_factor")]
fn moodys_warf_factor_py(rating: Bound<'_, PyAny>) -> PyResult<f64> {
    let rating = extract_notched_rating(&rating)?;
    Ok(moodys_warf_factor(rating))
}

fn extract_notched_rating(value: &Bound<'_, PyAny>) -> PyResult<NotchedRating> {
    if let Ok(r) = value.extract::<PyRef<PyNotchedRating>>() {
        return Ok(r.inner);
    }
    if let Ok(r) = value.extract::<PyRef<PyCreditRating>>() {
        return Ok(r.inner.with_notch(RatingNotch::Flat));
    }
    if let Ok(text) = value.extract::<&str>() {
        return NotchedRating::from_str(text).map_err(core_to_py);
    }

    Err(PyTypeError::new_err(
        "Expected NotchedRating, CreditRating, or rating string",
    ))
}

pub(crate) fn register<'py>(py: Python<'py>, parent: &Bound<'py, PyModule>) -> PyResult<()> {
    let module = PyModule::new(py, "types")?;
    module.setattr(
        "__doc__",
        concat!(
            "Type-safe identifiers and rate helpers mirroring finstack_core::types.\n\n",
            "This module provides:\n",
            "- ID newtypes: CurveId, InstrumentId, IndexId, PriceId, UnderlyingId\n",
            "- Rate helpers: Rate, Bps, Percentage\n",
            "- Credit ratings: CreditRating, RatingNotch, NotchedRating, RatingFactorTable\n\n",
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
    module.add_class::<PyRatingNotch>()?;
    module.add_class::<PyCreditRating>()?;
    module.add_class::<PyNotchedRating>()?;
    module.add_class::<PyRatingLabel>()?;
    module.add_class::<PyRatingFactorTable>()?;
    module.add_function(wrap_pyfunction!(moodys_warf_factor_py, &module)?)?;

    let exports = [
        "CurveId",
        "InstrumentId",
        "IndexId",
        "PriceId",
        "UnderlyingId",
        "Rate",
        "Bps",
        "Percentage",
        "RatingNotch",
        "CreditRating",
        "NotchedRating",
        "RatingLabel",
        "RatingFactorTable",
        "moodys_warf_factor",
    ];
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    Ok(())
}
