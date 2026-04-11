//! Python bindings for `finstack_core::types`.

use crate::errors::core_to_py;
use finstack_core::types::{
    Attributes, Bps, CreditRating, CurveId, InstrumentId, Percentage, Rate,
};
use finstack_core::Error;
use finstack_core::InputError;
use finstack_core::NonFiniteKind;
use pyo3::prelude::*;
use pyo3::types::{PyModule, PyType};
use std::hash::{Hash, Hasher};

/// Wrapper for [`Rate`].
#[pyclass(
    module = "finstack.core.types",
    name = "Rate",
    frozen,
    eq,
    ord,
    hash,
    skip_from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct PyRate {
    /// Underlying Rust rate.
    pub(crate) inner: Rate,
}

impl PyRate {
    /// Build a Python wrapper from a Rust [`Rate`].
    pub(crate) fn from_inner(inner: Rate) -> Self {
        Self { inner }
    }
}

impl Hash for PyRate {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.as_decimal().to_bits().hash(state);
    }
}

#[pymethods]
impl PyRate {
    /// Zero rate (0% as a decimal rate).
    #[classattr]
    const ZERO: PyRate = PyRate { inner: Rate::ZERO };

    #[new]
    #[pyo3(text_signature = "(decimal)")]
    /// Construct a rate from a decimal fraction (e.g. ``0.05`` for 5%).
    fn new(decimal: f64) -> PyResult<Self> {
        Rate::try_from_decimal(decimal)
            .map(Self::from_inner)
            .map_err(core_to_py)
    }

    /// Build from a percent value (e.g. ``5.0`` for 5%).
    #[classmethod]
    #[pyo3(text_signature = "(cls, percent)")]
    fn from_percent(_cls: &Bound<'_, PyType>, percent: f64) -> PyResult<Self> {
        if !percent.is_finite() {
            let kind = if percent.is_nan() {
                NonFiniteKind::NaN
            } else if percent.is_sign_positive() {
                NonFiniteKind::PosInfinity
            } else {
                NonFiniteKind::NegInfinity
            };
            return Err(core_to_py(InputError::NonFiniteValue { kind }.into()));
        }
        Ok(Self::from_inner(Rate::from_percent(percent)))
    }

    /// Build from an integer basis-point amount (e.g. ``500`` for 5%).
    #[classmethod]
    #[pyo3(text_signature = "(cls, bps)")]
    fn from_bps(_cls: &Bound<'_, PyType>, bps: i32) -> Self {
        Self::from_inner(Rate::from_bps(bps))
    }

    /// Rate as a decimal fraction.
    #[getter]
    fn as_decimal(&self) -> f64 {
        self.inner.as_decimal()
    }

    /// Rate as a percent value.
    #[getter]
    fn as_percent(&self) -> f64 {
        self.inner.as_percent()
    }

    /// Rate rounded to the nearest basis point.
    #[getter]
    fn as_bps(&self) -> i32 {
        self.inner.as_bps()
    }

    fn __repr__(&self) -> String {
        format!("Rate(decimal={:?})", self.inner.as_decimal())
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __add__(&self, other: PyRef<Self>) -> PyResult<Self> {
        let sum = self.inner.as_decimal() + other.inner.as_decimal();
        Rate::try_from_decimal(sum)
            .map(Self::from_inner)
            .map_err(core_to_py)
    }

    fn __sub__(&self, other: PyRef<Self>) -> PyResult<Self> {
        let diff = self.inner.as_decimal() - other.inner.as_decimal();
        Rate::try_from_decimal(diff)
            .map(Self::from_inner)
            .map_err(core_to_py)
    }

    fn __mul__(&self, rhs: f64) -> PyResult<Self> {
        let p = self.inner.as_decimal() * rhs;
        Rate::try_from_decimal(p)
            .map(Self::from_inner)
            .map_err(core_to_py)
    }

    fn __truediv__(&self, rhs: f64) -> PyResult<Self> {
        if rhs == 0.0 {
            return Err(core_to_py(Error::Validation(
                "division by zero".to_string(),
            )));
        }
        let q = self.inner.as_decimal() / rhs;
        Rate::try_from_decimal(q)
            .map(Self::from_inner)
            .map_err(core_to_py)
    }

    fn __neg__(&self) -> PyResult<Self> {
        Rate::try_from_decimal(-self.inner.as_decimal())
            .map(Self::from_inner)
            .map_err(core_to_py)
    }
}

/// Wrapper for [`Bps`].
#[pyclass(
    module = "finstack.core.types",
    name = "Bps",
    frozen,
    eq,
    ord,
    hash,
    skip_from_py_object
)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct PyBps {
    /// Underlying Rust basis-point value.
    pub(crate) inner: Bps,
}

impl PyBps {
    /// Build a Python wrapper from a Rust [`Bps`].
    pub(crate) fn from_inner(inner: Bps) -> Self {
        Self { inner }
    }
}

impl Hash for PyBps {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.as_bps().hash(state);
    }
}

#[pymethods]
impl PyBps {
    /// Zero basis points.
    #[classattr]
    const ZERO: PyBps = PyBps { inner: Bps::ZERO };

    #[new]
    #[pyo3(text_signature = "(bps)")]
    /// Construct from a floating basis-point value (rounded to the nearest integer bp).
    fn new(bps: f64) -> PyResult<Self> {
        Bps::try_new(bps).map(Self::from_inner).map_err(core_to_py)
    }

    /// Value as a decimal fraction.
    #[getter]
    fn as_decimal(&self) -> f64 {
        self.inner.as_decimal()
    }

    /// Value as whole basis points.
    #[getter]
    fn as_bps(&self) -> i32 {
        self.inner.as_bps()
    }

    fn __repr__(&self) -> String {
        format!("Bps({})", self.inner.as_bps())
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __add__(&self, other: PyRef<Self>) -> Self {
        Self::from_inner(self.inner + other.inner)
    }

    fn __sub__(&self, other: PyRef<Self>) -> Self {
        Self::from_inner(self.inner - other.inner)
    }

    fn __mul__(&self, rhs: i32) -> Self {
        Self::from_inner(self.inner * rhs)
    }

    fn __truediv__(&self, rhs: i32) -> PyResult<Self> {
        if rhs == 0 {
            return Err(core_to_py(Error::Validation(
                "division by zero".to_string(),
            )));
        }
        Ok(Self::from_inner(self.inner / rhs))
    }

    fn __neg__(&self) -> Self {
        Self::from_inner(-self.inner)
    }
}

/// Wrapper for [`Percentage`].
#[pyclass(
    module = "finstack.core.types",
    name = "Percentage",
    frozen,
    eq,
    ord,
    hash,
    skip_from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct PyPercentage {
    /// Underlying Rust percentage.
    pub(crate) inner: Percentage,
}

impl PyPercentage {
    /// Build a Python wrapper from a Rust [`Percentage`].
    pub(crate) fn from_inner(inner: Percentage) -> Self {
        Self { inner }
    }
}

impl Hash for PyPercentage {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.as_percent().to_bits().hash(state);
    }
}

#[pymethods]
impl PyPercentage {
    /// Zero percent.
    #[classattr]
    const ZERO: PyPercentage = PyPercentage {
        inner: Percentage::ZERO,
    };

    #[new]
    #[pyo3(text_signature = "(percent)")]
    /// Construct from a percent value (e.g. ``12.5`` for 12.5%).
    fn new(percent: f64) -> PyResult<Self> {
        Percentage::try_new(percent)
            .map(Self::from_inner)
            .map_err(core_to_py)
    }

    /// Value as a decimal fraction.
    #[getter]
    fn as_decimal(&self) -> f64 {
        self.inner.as_decimal()
    }

    /// Value in percent terms.
    #[getter]
    fn as_percent(&self) -> f64 {
        self.inner.as_percent()
    }

    fn __repr__(&self) -> String {
        format!("Percentage(percent={:?})", self.inner.as_percent())
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }
}

/// Wrapper for [`CreditRating`].
#[pyclass(
    module = "finstack.core.types",
    name = "CreditRating",
    frozen,
    eq,
    hash,
    skip_from_py_object
)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct PyCreditRating {
    /// Underlying Rust credit rating.
    pub(crate) inner: CreditRating,
}

impl PyCreditRating {
    /// Build a Python wrapper from a Rust [`CreditRating`].
    pub(crate) fn from_inner(inner: CreditRating) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCreditRating {
    /// Highest quality rating.
    #[classattr]
    const AAA: PyCreditRating = PyCreditRating {
        inner: CreditRating::AAA,
    };
    /// AA category.
    #[classattr]
    const AA: PyCreditRating = PyCreditRating {
        inner: CreditRating::AA,
    };
    /// Single-A category.
    #[classattr]
    const A: PyCreditRating = PyCreditRating {
        inner: CreditRating::A,
    };
    /// BBB category.
    #[classattr]
    const BBB: PyCreditRating = PyCreditRating {
        inner: CreditRating::BBB,
    };
    /// BB category.
    #[classattr]
    const BB: PyCreditRating = PyCreditRating {
        inner: CreditRating::BB,
    };
    /// B category.
    #[classattr]
    const B: PyCreditRating = PyCreditRating {
        inner: CreditRating::B,
    };
    /// CCC category.
    #[classattr]
    const CCC: PyCreditRating = PyCreditRating {
        inner: CreditRating::CCC,
    };
    /// CC category.
    #[classattr]
    const CC: PyCreditRating = PyCreditRating {
        inner: CreditRating::CC,
    };
    /// C category.
    #[classattr]
    const C: PyCreditRating = PyCreditRating {
        inner: CreditRating::C,
    };
    /// Default rating.
    #[classattr]
    const D: PyCreditRating = PyCreditRating {
        inner: CreditRating::D,
    };
    /// Not rated.
    #[classattr]
    const NR: PyCreditRating = PyCreditRating {
        inner: CreditRating::NR,
    };

    /// Parse a rating string (case-insensitive; notches map to the base letter).
    #[classmethod]
    #[pyo3(text_signature = "(cls, name)")]
    fn from_name(_cls: &Bound<'_, PyType>, name: &str) -> PyResult<Self> {
        name.parse::<CreditRating>()
            .map(Self::from_inner)
            .map_err(core_to_py)
    }

    /// Canonical rating name (e.g. ``"BBB"``).
    #[getter]
    fn name(&self) -> String {
        self.inner.to_string()
    }

    fn __repr__(&self) -> String {
        format!("CreditRating({})", self.inner)
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }
}

/// Wrapper for [`CurveId`].
#[pyclass(
    module = "finstack.core.types",
    name = "CurveId",
    frozen,
    eq,
    hash,
    skip_from_py_object
)]
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct PyCurveId {
    /// Underlying Rust curve identifier.
    pub(crate) inner: CurveId,
}

impl PyCurveId {
    /// Build a Python wrapper from a Rust [`CurveId`].
    pub(crate) fn from_inner(inner: CurveId) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCurveId {
    #[new]
    #[pyo3(text_signature = "(value)")]
    /// Create a curve identifier from its string value.
    fn new(value: &str) -> Self {
        Self::from_inner(CurveId::from(value))
    }

    /// Underlying string value.
    #[getter]
    fn as_str(&self) -> &str {
        self.inner.as_str()
    }

    fn __repr__(&self) -> String {
        format!("CurveId({:?})", self.inner.as_str())
    }

    fn __str__(&self) -> String {
        self.inner.as_str().to_string()
    }
}

/// Wrapper for [`InstrumentId`].
#[pyclass(
    module = "finstack.core.types",
    name = "InstrumentId",
    frozen,
    eq,
    hash,
    skip_from_py_object
)]
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct PyInstrumentId {
    /// Underlying Rust instrument identifier.
    pub(crate) inner: InstrumentId,
}

impl PyInstrumentId {
    /// Build a Python wrapper from a Rust [`InstrumentId`].
    pub(crate) fn from_inner(inner: InstrumentId) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyInstrumentId {
    #[new]
    #[pyo3(text_signature = "(value)")]
    /// Create an instrument identifier from its string value.
    fn new(value: &str) -> Self {
        Self::from_inner(InstrumentId::from(value))
    }

    /// Underlying string value.
    #[getter]
    fn as_str(&self) -> &str {
        self.inner.as_str()
    }

    fn __repr__(&self) -> String {
        format!("InstrumentId({:?})", self.inner.as_str())
    }

    fn __str__(&self) -> String {
        self.inner.as_str().to_string()
    }
}

/// Wrapper for [`Attributes`].
#[pyclass(
    module = "finstack.core.types",
    name = "Attributes",
    skip_from_py_object
)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PyAttributes {
    /// Underlying Rust attribute bag.
    pub(crate) inner: Attributes,
}

impl PyAttributes {
    /// Build a Python wrapper from Rust [`Attributes`].
    pub(crate) fn from_inner(inner: Attributes) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyAttributes {
    #[new]
    /// Create an empty attribute set.
    fn new() -> Self {
        Self::from_inner(Attributes::new())
    }

    /// Fetch metadata by key.
    #[pyo3(text_signature = "(self, key)")]
    fn get(&self, key: &str) -> Option<String> {
        self.inner.get_meta(key).map(str::to_string)
    }

    /// Insert or replace a metadata entry.
    #[pyo3(text_signature = "(self, key, value)")]
    fn set(&mut self, key: &str, value: &str) {
        self.inner.set(key, value);
    }

    /// Return whether `key` exists in metadata.
    #[pyo3(text_signature = "(self, key)")]
    fn contains(&self, key: &str) -> bool {
        self.inner.meta.contains_key(key)
    }

    /// Metadata keys in sorted order.
    fn keys(&self) -> Vec<String> {
        self.inner.meta.keys().cloned().collect()
    }

    /// Number of metadata entries.
    fn len(&self) -> usize {
        self.inner.meta.len()
    }

    fn __repr__(&self) -> String {
        format!(
            "Attributes(tags={}, meta_keys={})",
            self.inner.tags.len(),
            self.inner.meta.len()
        )
    }

    fn __len__(&self) -> usize {
        self.inner.meta.len()
    }
}

/// Register the `finstack.core.types` submodule.
pub fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "types")?;
    m.setattr(
        "__doc__",
        "Core finstack types: rates, identifiers, credit ratings, attributes.",
    )?;

    m.add_class::<PyRate>()?;
    m.add_class::<PyBps>()?;
    m.add_class::<PyPercentage>()?;
    m.add_class::<PyCreditRating>()?;
    m.add_class::<PyCurveId>()?;
    m.add_class::<PyInstrumentId>()?;
    m.add_class::<PyAttributes>()?;

    parent.add_submodule(&m)?;

    Ok(())
}
