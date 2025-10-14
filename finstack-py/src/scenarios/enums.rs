//! Enum bindings for scenarios Python module.

use finstack_scenarios::{CurveKind, TenorMatchMode, VolSurfaceKind};
use pyo3::basic::CompareOp;
use pyo3::prelude::*;
use pyo3::types::PyModule;

/// Identifies which family of curve an operation targets.
///
/// Parameters
/// ----------
/// None
///     Use class attributes: ``CurveKind.Discount``, ``CurveKind.Forecast``, etc.
///
/// Examples
/// --------
/// >>> from finstack.scenarios import CurveKind
/// >>> kind = CurveKind.Discount
#[pyclass(module = "finstack.scenarios", name = "CurveKind", frozen)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PyCurveKind {
    pub(crate) inner: CurveKind,
}

impl PyCurveKind {
    pub fn new(inner: CurveKind) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCurveKind {
    #[classattr]
    #[allow(non_snake_case)]
    fn Discount() -> Self {
        Self::new(CurveKind::Discount)
    }

    #[classattr]
    #[allow(non_snake_case)]
    fn Forecast() -> Self {
        Self::new(CurveKind::Forecast)
    }

    #[classattr]
    #[allow(non_snake_case)]
    fn Hazard() -> Self {
        Self::new(CurveKind::Hazard)
    }

    #[classattr]
    #[allow(non_snake_case)]
    fn Inflation() -> Self {
        Self::new(CurveKind::Inflation)
    }

    fn __repr__(&self) -> String {
        match self.inner {
            CurveKind::Discount => "CurveKind.Discount".to_string(),
            CurveKind::Forecast => "CurveKind.Forecast".to_string(),
            CurveKind::Hazard => "CurveKind.Hazard".to_string(),
            CurveKind::Inflation => "CurveKind.Inflation".to_string(),
        }
    }

    fn __str__(&self) -> String {
        match self.inner {
            CurveKind::Discount => "Discount".to_string(),
            CurveKind::Forecast => "Forecast".to_string(),
            CurveKind::Hazard => "Hazard".to_string(),
            CurveKind::Inflation => "Inflation".to_string(),
        }
    }

    fn __richcmp__(&self, other: &Self, op: CompareOp) -> bool {
        match op {
            CompareOp::Eq => self.inner == other.inner,
            CompareOp::Ne => self.inner != other.inner,
            _ => false,
        }
    }

    fn __hash__(&self) -> u64 {
        self.inner as u64
    }
}

/// Identifies which category of volatility surface an operation targets.
///
/// Parameters
/// ----------
/// None
///     Use class attributes: ``VolSurfaceKind.Equity``, ``VolSurfaceKind.Credit``, etc.
///
/// Examples
/// --------
/// >>> from finstack.scenarios import VolSurfaceKind
/// >>> kind = VolSurfaceKind.Equity
#[pyclass(module = "finstack.scenarios", name = "VolSurfaceKind", frozen)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PyVolSurfaceKind {
    pub(crate) inner: VolSurfaceKind,
}

impl PyVolSurfaceKind {
    pub fn new(inner: VolSurfaceKind) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyVolSurfaceKind {
    #[classattr]
    #[allow(non_snake_case)]
    fn Equity() -> Self {
        Self::new(VolSurfaceKind::Equity)
    }

    #[classattr]
    #[allow(non_snake_case)]
    fn Credit() -> Self {
        Self::new(VolSurfaceKind::Credit)
    }

    #[classattr]
    #[allow(non_snake_case)]
    fn Swaption() -> Self {
        Self::new(VolSurfaceKind::Swaption)
    }

    fn __repr__(&self) -> String {
        match self.inner {
            VolSurfaceKind::Equity => "VolSurfaceKind.Equity".to_string(),
            VolSurfaceKind::Credit => "VolSurfaceKind.Credit".to_string(),
            VolSurfaceKind::Swaption => "VolSurfaceKind.Swaption".to_string(),
        }
    }

    fn __str__(&self) -> String {
        match self.inner {
            VolSurfaceKind::Equity => "Equity".to_string(),
            VolSurfaceKind::Credit => "Credit".to_string(),
            VolSurfaceKind::Swaption => "Swaption".to_string(),
        }
    }

    fn __richcmp__(&self, other: &Self, op: CompareOp) -> bool {
        match op {
            CompareOp::Eq => self.inner == other.inner,
            CompareOp::Ne => self.inner != other.inner,
            _ => false,
        }
    }

    fn __hash__(&self) -> u64 {
        self.inner as u64
    }
}

/// Strategy for aligning requested tenor bumps with curve pillars.
///
/// Parameters
/// ----------
/// None
///     Use class attributes: ``TenorMatchMode.Exact``, ``TenorMatchMode.Interpolate``
///
/// Examples
/// --------
/// >>> from finstack.scenarios import TenorMatchMode
/// >>> mode = TenorMatchMode.Interpolate
#[pyclass(module = "finstack.scenarios", name = "TenorMatchMode", frozen)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PyTenorMatchMode {
    pub(crate) inner: TenorMatchMode,
}

impl PyTenorMatchMode {
    pub fn new(inner: TenorMatchMode) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyTenorMatchMode {
    #[classattr]
    #[allow(non_snake_case)]
    fn Exact() -> Self {
        Self::new(TenorMatchMode::Exact)
    }

    #[classattr]
    #[allow(non_snake_case)]
    fn Interpolate() -> Self {
        Self::new(TenorMatchMode::Interpolate)
    }

    fn __repr__(&self) -> String {
        match self.inner {
            TenorMatchMode::Exact => "TenorMatchMode.Exact".to_string(),
            TenorMatchMode::Interpolate => "TenorMatchMode.Interpolate".to_string(),
        }
    }

    fn __str__(&self) -> String {
        match self.inner {
            TenorMatchMode::Exact => "Exact".to_string(),
            TenorMatchMode::Interpolate => "Interpolate".to_string(),
        }
    }

    fn __richcmp__(&self, other: &Self, op: CompareOp) -> bool {
        match op {
            CompareOp::Eq => self.inner == other.inner,
            CompareOp::Ne => self.inner != other.inner,
            _ => false,
        }
    }

    fn __hash__(&self) -> u64 {
        self.inner as u64
    }
}

/// Register enum types with the scenarios module.
pub(crate) fn register(_py: Python<'_>, module: &Bound<'_, PyModule>) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyCurveKind>()?;
    module.add_class::<PyVolSurfaceKind>()?;
    module.add_class::<PyTenorMatchMode>()?;

    Ok(vec!["CurveKind", "VolSurfaceKind", "TenorMatchMode"])
}

