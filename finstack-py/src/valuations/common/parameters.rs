//! Python bindings for common parameter types (OptionType, ExerciseStyle, etc.)

use finstack_valuations::instruments::{
    legs::PayReceive,
    market::{ExerciseStyle, OptionType, SettlementType},
    rates::cap_floor::CapFloorVolType,
    rates::swaption::{CashSettlementMethod, VolatilityModel},
    BarrierType,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule, PyType};
use pyo3::Bound;

/// Option type for pricing (Call or Put).
#[pyclass(
    module = "finstack.valuations.common.parameters",
    name = "OptionType",
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyOptionType {
    pub(crate) inner: OptionType,
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
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyExerciseStyle {
    pub(crate) inner: ExerciseStyle,
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
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PySettlementType {
    pub(crate) inner: SettlementType,
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
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyPayReceive {
    pub(crate) inner: PayReceive,
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

/// Volatility model for option pricing (Black or Normal).
#[pyclass(
    module = "finstack.valuations.common.parameters",
    name = "VolatilityModel",
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyVolatilityModel {
    pub(crate) inner: VolatilityModel,
}

impl PyVolatilityModel {
    pub(crate) const fn new(inner: VolatilityModel) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyVolatilityModel {
    #[classattr]
    const BLACK: Self = Self::new(VolatilityModel::Black);
    #[classattr]
    const NORMAL: Self = Self::new(VolatilityModel::Normal);

    #[classmethod]
    #[pyo3(text_signature = "(cls, name)")]
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
        format!("VolatilityModel('{}')", self.inner)
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __hash__(&self) -> isize {
        self.inner as isize
    }
}

impl From<PyVolatilityModel> for VolatilityModel {
    fn from(value: PyVolatilityModel) -> Self {
        value.inner
    }
}

impl From<VolatilityModel> for PyVolatilityModel {
    fn from(value: VolatilityModel) -> Self {
        Self::new(value)
    }
}

/// Cash settlement method for cash-settled swaptions.
#[pyclass(
    module = "finstack.valuations.common.parameters",
    name = "CashSettlementMethod",
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyCashSettlementMethod {
    pub(crate) inner: CashSettlementMethod,
}

impl PyCashSettlementMethod {
    pub(crate) const fn new(inner: CashSettlementMethod) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCashSettlementMethod {
    #[classattr]
    const PAR_YIELD: Self = Self::new(CashSettlementMethod::ParYield);
    #[classattr]
    const ISDA_PAR_PAR: Self = Self::new(CashSettlementMethod::IsdaParPar);
    #[classattr]
    const ZERO_COUPON: Self = Self::new(CashSettlementMethod::ZeroCoupon);

    #[classmethod]
    #[pyo3(text_signature = "(cls, name)")]
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
        format!("CashSettlementMethod('{}')", self.inner)
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __hash__(&self) -> isize {
        self.inner as isize
    }
}

impl From<PyCashSettlementMethod> for CashSettlementMethod {
    fn from(value: PyCashSettlementMethod) -> Self {
        value.inner
    }
}

impl From<CashSettlementMethod> for PyCashSettlementMethod {
    fn from(value: CashSettlementMethod) -> Self {
        Self::new(value)
    }
}

/// Volatility convention for cap/floor pricing.
#[pyclass(
    module = "finstack.valuations.common.parameters",
    name = "CapFloorVolType",
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyCapFloorVolType {
    pub(crate) inner: CapFloorVolType,
}

impl PyCapFloorVolType {
    pub(crate) const fn new(inner: CapFloorVolType) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCapFloorVolType {
    #[classattr]
    const LOGNORMAL: Self = Self::new(CapFloorVolType::Lognormal);
    #[classattr]
    const SHIFTED_LOGNORMAL: Self = Self::new(CapFloorVolType::ShiftedLognormal);
    #[classattr]
    const NORMAL: Self = Self::new(CapFloorVolType::Normal);

    #[classmethod]
    #[pyo3(text_signature = "(cls, name)")]
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
        format!("CapFloorVolType('{}')", self.inner)
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __hash__(&self) -> isize {
        self.inner as isize
    }
}

impl From<PyCapFloorVolType> for CapFloorVolType {
    fn from(value: PyCapFloorVolType) -> Self {
        value.inner
    }
}

impl From<CapFloorVolType> for PyCapFloorVolType {
    fn from(value: CapFloorVolType) -> Self {
        Self::new(value)
    }
}

/// Barrier type for barrier options.
#[pyclass(
    module = "finstack.valuations.common.parameters",
    name = "BarrierType",
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyBarrierType {
    pub(crate) inner: BarrierType,
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
        matches!(self.inner, BarrierType::UpAndOut | BarrierType::DownAndOut)
    }

    /// Check if this is a knock-in barrier.
    fn is_knock_in(&self) -> bool {
        matches!(self.inner, BarrierType::UpAndIn | BarrierType::DownAndIn)
    }

    /// Check if this is an up barrier.
    fn is_up(&self) -> bool {
        matches!(self.inner, BarrierType::UpAndOut | BarrierType::UpAndIn)
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
        "Common parameter types: OptionType, ExerciseStyle, SettlementType, PayReceive, BarrierType, VolatilityModel, CashSettlementMethod, CapFloorVolType",
    )?;

    module.add_class::<PyOptionType>()?;
    module.add_class::<PyExerciseStyle>()?;
    module.add_class::<PySettlementType>()?;
    module.add_class::<PyPayReceive>()?;
    module.add_class::<PyBarrierType>()?;
    module.add_class::<PyVolatilityModel>()?;
    module.add_class::<PyCashSettlementMethod>()?;
    module.add_class::<PyCapFloorVolType>()?;

    let exports = vec![
        "OptionType",
        "ExerciseStyle",
        "SettlementType",
        "PayReceive",
        "BarrierType",
        "VolatilityModel",
        "CashSettlementMethod",
        "CapFloorVolType",
    ];

    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;

    Ok(exports)
}
