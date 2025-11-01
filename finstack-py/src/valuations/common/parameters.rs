//! Python bindings for common parameter types (OptionType, ExerciseStyle, etc.)

use finstack_valuations::instruments::common::mc::payoff::barrier::BarrierType;
use finstack_valuations::instruments::common::parameters::{
    legs::PayReceive,
    market::{ExerciseStyle, OptionType, SettlementType},
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule, PyType};
use pyo3::Bound;

/// Option type for pricing (Call or Put).
#[pyclass(
    module = "finstack.valuations.common.parameters",
    name = "OptionType",
    frozen
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyOptionType {
    inner: OptionType,
}

impl PyOptionType {
    pub(crate) const fn new(inner: OptionType) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyOptionType {
    #[classattr]
    const CALL: Self = Self::new(OptionType::Call);
    #[classattr]
    const PUT: Self = Self::new(OptionType::Put);

    #[classmethod]
    #[pyo3(text_signature = "(cls, name)")]
    /// Convert a string into an OptionType.
    ///
    /// Args:
    ///     name: Option type label such as "call" or "put".
    ///
    /// Returns:
    ///     OptionType: Enumeration value that matches name.
    fn from_name(_cls: &Bound<'_, PyType>, name: &str) -> PyResult<Self> {
        name.parse()
            .map(Self::new)
            .map_err(|e: String| PyValueError::new_err(e))
    }

    #[getter]
    fn name(&self) -> String {
        self.inner.to_string()
    }

    fn __repr__(&self) -> String {
        format!("OptionType('{}')", self.inner)
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __hash__(&self) -> isize {
        self.inner as isize
    }
}

impl From<PyOptionType> for OptionType {
    fn from(value: PyOptionType) -> Self {
        value.inner
    }
}

impl From<OptionType> for PyOptionType {
    fn from(value: OptionType) -> Self {
        Self::new(value)
    }
}

/// Exercise style for options (European, American, Bermudan).
#[pyclass(
    module = "finstack.valuations.common.parameters",
    name = "ExerciseStyle",
    frozen
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyExerciseStyle {
    inner: ExerciseStyle,
}

impl PyExerciseStyle {
    pub(crate) const fn new(inner: ExerciseStyle) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyExerciseStyle {
    #[classattr]
    const EUROPEAN: Self = Self::new(ExerciseStyle::European);
    #[classattr]
    const AMERICAN: Self = Self::new(ExerciseStyle::American);
    #[classattr]
    const BERMUDAN: Self = Self::new(ExerciseStyle::Bermudan);

    #[classmethod]
    #[pyo3(text_signature = "(cls, name)")]
    /// Convert a string into an ExerciseStyle.
    fn from_name(_cls: &Bound<'_, PyType>, name: &str) -> PyResult<Self> {
        name.parse()
            .map(Self::new)
            .map_err(|e: String| PyValueError::new_err(e))
    }

    #[getter]
    fn name(&self) -> String {
        self.inner.to_string()
    }

    fn __repr__(&self) -> String {
        format!("ExerciseStyle('{}')", self.inner)
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __hash__(&self) -> isize {
        self.inner as isize
    }
}

impl From<PyExerciseStyle> for ExerciseStyle {
    fn from(value: PyExerciseStyle) -> Self {
        value.inner
    }
}

/// Settlement type for options (Physical or Cash).
#[pyclass(
    module = "finstack.valuations.common.parameters",
    name = "SettlementType",
    frozen
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PySettlementType {
    inner: SettlementType,
}

impl PySettlementType {
    pub(crate) const fn new(inner: SettlementType) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PySettlementType {
    #[classattr]
    const PHYSICAL: Self = Self::new(SettlementType::Physical);
    #[classattr]
    const CASH: Self = Self::new(SettlementType::Cash);

    #[classmethod]
    #[pyo3(text_signature = "(cls, name)")]
    /// Convert a string into a SettlementType.
    fn from_name(_cls: &Bound<'_, PyType>, name: &str) -> PyResult<Self> {
        name.parse()
            .map(Self::new)
            .map_err(|e: String| PyValueError::new_err(e))
    }

    #[getter]
    fn name(&self) -> String {
        self.inner.to_string()
    }

    fn __repr__(&self) -> String {
        format!("SettlementType('{}')", self.inner)
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __hash__(&self) -> isize {
        self.inner as isize
    }
}

/// Pay/Receive direction for instrument legs.
#[pyclass(
    module = "finstack.valuations.common.parameters",
    name = "PayReceive",
    frozen
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyPayReceive {
    inner: PayReceive,
}

impl PyPayReceive {
    pub(crate) const fn new(inner: PayReceive) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyPayReceive {
    #[classattr]
    const PAY_FIXED: Self = Self::new(PayReceive::PayFixed);
    #[classattr]
    const RECEIVE_FIXED: Self = Self::new(PayReceive::ReceiveFixed);

    #[classmethod]
    #[pyo3(text_signature = "(cls, name)")]
    /// Convert a string into a PayReceive.
    fn from_name(_cls: &Bound<'_, PyType>, name: &str) -> PyResult<Self> {
        name.parse()
            .map(Self::new)
            .map_err(|e: String| PyValueError::new_err(e))
    }

    #[getter]
    fn name(&self) -> String {
        self.inner.to_string()
    }

    /// Check if this is the payer side.
    fn is_payer(&self) -> bool {
        self.inner.is_payer()
    }

    /// Check if this is the receiver side.
    fn is_receiver(&self) -> bool {
        self.inner.is_receiver()
    }

    fn __repr__(&self) -> String {
        format!("PayReceive('{}')", self.inner)
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __hash__(&self) -> isize {
        self.inner as isize
    }
}

impl From<PyPayReceive> for PayReceive {
    fn from(value: PyPayReceive) -> Self {
        value.inner
    }
}

/// Barrier type for barrier options.
#[pyclass(
    module = "finstack.valuations.common.parameters",
    name = "BarrierType",
    frozen
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyBarrierType {
    inner: BarrierType,
}

impl PyBarrierType {
    pub(crate) const fn new(inner: BarrierType) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyBarrierType {
    #[classattr]
    const UP_AND_OUT: Self = Self::new(BarrierType::UpAndOut);
    #[classattr]
    const UP_AND_IN: Self = Self::new(BarrierType::UpAndIn);
    #[classattr]
    const DOWN_AND_OUT: Self = Self::new(BarrierType::DownAndOut);
    #[classattr]
    const DOWN_AND_IN: Self = Self::new(BarrierType::DownAndIn);

    /// Check if this is a knock-out barrier.
    fn is_knock_out(&self) -> bool {
        self.inner.is_knock_out()
    }

    /// Check if this is a knock-in barrier.
    fn is_knock_in(&self) -> bool {
        self.inner.is_knock_in()
    }

    /// Check if this is an up barrier.
    fn is_up(&self) -> bool {
        self.inner.is_up()
    }

    fn __repr__(&self) -> String {
        format!("BarrierType({:?})", self.inner)
    }

    fn __str__(&self) -> String {
        format!("{:?}", self.inner)
    }

    fn __hash__(&self) -> isize {
        self.inner as isize
    }
}

impl From<PyBarrierType> for BarrierType {
    fn from(value: PyBarrierType) -> Self {
        value.inner
    }
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "parameters")?;
    module.setattr(
        "__doc__",
        "Common parameter types: OptionType, ExerciseStyle, SettlementType, PayReceive, BarrierType",
    )?;

    module.add_class::<PyOptionType>()?;
    module.add_class::<PyExerciseStyle>()?;
    module.add_class::<PySettlementType>()?;
    module.add_class::<PyPayReceive>()?;
    module.add_class::<PyBarrierType>()?;

    let exports = vec![
        "OptionType",
        "ExerciseStyle",
        "SettlementType",
        "PayReceive",
        "BarrierType",
    ];

    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;

    Ok(exports)
}
