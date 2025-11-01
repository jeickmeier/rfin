use crate::core::common::labels::normalize_label;
use crate::core::common::pycmp::richcmp_eq_ne;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::pricer::{InstrumentType, ModelKey, PricerKey, PricingError};
use pyo3::basic::CompareOp;
use pyo3::exceptions::{PyKeyError, PyRuntimeError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule, PyModuleMethods, PyType};
use pyo3::Bound;
use std::fmt;

/// Enumerates instrument families supported by the valuation engines.
///
/// Examples:
///     >>> InstrumentType.BOND.name
///     'bond'
#[pyclass(module = "finstack.valuations.common", name = "InstrumentType", frozen)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyInstrumentType {
    pub(crate) inner: InstrumentType,
}

impl PyInstrumentType {
    pub(crate) const fn new(inner: InstrumentType) -> Self {
        Self { inner }
    }

    fn label(&self) -> String {
        instrument_type_label(self.inner)
    }
}

#[pymethods]
impl PyInstrumentType {
    #[classattr]
    const BOND: Self = Self::new(InstrumentType::Bond);
    #[classattr]
    const LOAN: Self = Self::new(InstrumentType::Loan);
    #[classattr]
    const CDS: Self = Self::new(InstrumentType::CDS);
    #[classattr]
    const CDS_INDEX: Self = Self::new(InstrumentType::CDSIndex);
    #[classattr]
    const CDS_TRANCHE: Self = Self::new(InstrumentType::CDSTranche);
    #[classattr]
    const CDS_OPTION: Self = Self::new(InstrumentType::CDSOption);
    #[classattr]
    const IRS: Self = Self::new(InstrumentType::IRS);
    #[classattr]
    const CAP_FLOOR: Self = Self::new(InstrumentType::CapFloor);
    #[classattr]
    const SWAPTION: Self = Self::new(InstrumentType::Swaption);
    #[classattr]
    const TRS: Self = Self::new(InstrumentType::TRS);
    #[classattr]
    const BASIS_SWAP: Self = Self::new(InstrumentType::BasisSwap);
    #[classattr]
    const BASKET: Self = Self::new(InstrumentType::Basket);
    #[classattr]
    const CONVERTIBLE: Self = Self::new(InstrumentType::Convertible);
    #[classattr]
    const DEPOSIT: Self = Self::new(InstrumentType::Deposit);
    #[classattr]
    const EQUITY_OPTION: Self = Self::new(InstrumentType::EquityOption);
    #[classattr]
    const FX_OPTION: Self = Self::new(InstrumentType::FxOption);
    #[classattr]
    const FX_SPOT: Self = Self::new(InstrumentType::FxSpot);
    #[classattr]
    const FX_SWAP: Self = Self::new(InstrumentType::FxSwap);
    #[classattr]
    const INFLATION_LINKED_BOND: Self = Self::new(InstrumentType::InflationLinkedBond);
    #[classattr]
    const INFLATION_SWAP: Self = Self::new(InstrumentType::InflationSwap);
    #[classattr]
    const INTEREST_RATE_FUTURE: Self = Self::new(InstrumentType::InterestRateFuture);
    #[classattr]
    const VARIANCE_SWAP: Self = Self::new(InstrumentType::VarianceSwap);
    #[classattr]
    const EQUITY: Self = Self::new(InstrumentType::Equity);
    #[classattr]
    const REPO: Self = Self::new(InstrumentType::Repo);
    #[classattr]
    const FRA: Self = Self::new(InstrumentType::FRA);
    #[classattr]
    const STRUCTURED_CREDIT: Self = Self::new(InstrumentType::StructuredCredit);
    #[classattr]
    const PRIVATE_MARKETS_FUND: Self = Self::new(InstrumentType::PrivateMarketsFund);
    #[classattr]
    const ASIAN_OPTION: Self = Self::new(InstrumentType::AsianOption);
    #[classattr]
    const AUTOCALLABLE: Self = Self::new(InstrumentType::Autocallable);
    #[classattr]
    const BARRIER_OPTION: Self = Self::new(InstrumentType::BarrierOption);
    #[classattr]
    const CLIQUET_OPTION: Self = Self::new(InstrumentType::CliquetOption);
    #[classattr]
    const CMS_OPTION: Self = Self::new(InstrumentType::CmsOption);
    #[classattr]
    const FX_BARRIER_OPTION: Self = Self::new(InstrumentType::FxBarrierOption);
    #[classattr]
    const LOOKBACK_OPTION: Self = Self::new(InstrumentType::LookbackOption);
    #[classattr]
    const QUANTO_OPTION: Self = Self::new(InstrumentType::QuantoOption);
    #[classattr]
    const RANGE_ACCRUAL: Self = Self::new(InstrumentType::RangeAccrual);
    #[classattr]
    const REVOLVING_CREDIT: Self = Self::new(InstrumentType::RevolvingCredit);

    #[classmethod]
    #[pyo3(text_signature = "(cls, name)")]
    /// Convert a snake-case label into an instrument family.
    ///
    /// Args:
    ///     name: Instrument family label such as ``"bond"``.
    ///
    /// Returns:
    ///     InstrumentType: Enumeration value that matches ``name``.
    ///
    /// Raises:
    ///     ValueError: If the label is unknown.
    ///
    /// Examples:
    ///     >>> InstrumentType.from_name("bond")
    ///     InstrumentType.BOND
    fn from_name(_cls: &Bound<'_, PyType>, name: &str) -> PyResult<Self> {
        parse_instrument_type(name)
    }

    #[getter]
    /// Snake-case identifier for the instrument family.
    ///
    /// Returns:
    ///     str: Normalized instrument label such as ``"bond"``.
    fn name(&self) -> String {
        self.label()
    }

    fn __repr__(&self) -> String {
        format!("InstrumentType('{}')", self.label())
    }

    fn __str__(&self) -> String {
        self.label()
    }

    fn __hash__(&self) -> isize {
        self.inner as isize
    }

    fn __richcmp__(
        &self,
        other: Bound<'_, PyAny>,
        op: CompareOp,
        py: Python<'_>,
    ) -> PyResult<PyObject> {
        let rhs = match extract_instrument_type(&other) {
            Ok(value) => Some(value),
            Err(_) => None,
        };
        richcmp_eq_ne(py, &(self.inner as u16), rhs.map(|v| v as u16), op)
    }
}

impl fmt::Display for PyInstrumentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Enumerates pricing model categories recognized by the registry.
///
/// Examples:
///     >>> ModelKey.DISCOUNTING.name
///     'discounting'
#[pyclass(module = "finstack.valuations.common", name = "ModelKey", frozen)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyModelKey {
    pub(crate) inner: ModelKey,
}

impl PyModelKey {
    pub(crate) const fn new(inner: ModelKey) -> Self {
        Self { inner }
    }

    fn label(&self) -> String {
        model_key_label(self.inner)
    }
}

#[pymethods]
impl PyModelKey {
    #[classattr]
    const DISCOUNTING: Self = Self::new(ModelKey::Discounting);
    #[classattr]
    const TREE: Self = Self::new(ModelKey::Tree);
    #[classattr]
    const BLACK76: Self = Self::new(ModelKey::Black76);
    #[classattr]
    const HULL_WHITE_1F: Self = Self::new(ModelKey::HullWhite1F);
    #[classattr]
    const HAZARD_RATE: Self = Self::new(ModelKey::HazardRate);

    #[classmethod]
    #[pyo3(text_signature = "(cls, name)")]
    /// Convert a snake-case label into a pricing model key.
    ///
    /// Args:
    ///     name: Pricing model label such as ``"discounting"``.
    ///
    /// Returns:
    ///     ModelKey: Enumeration value that corresponds to ``name``.
    ///
    /// Raises:
    ///     ValueError: If the label is not supported.
    ///
    /// Examples:
    ///     >>> ModelKey.from_name("discounting")
    ///     ModelKey.DISCOUNTING
    fn from_name(_cls: &Bound<'_, PyType>, name: &str) -> PyResult<Self> {
        parse_model_key(name)
    }

    #[getter]
    /// Snake-case identifier for this pricing model.
    ///
    /// Returns:
    ///     str: Normalized model label such as ``"discounting"``.
    fn name(&self) -> String {
        self.label()
    }

    fn __repr__(&self) -> String {
        format!("ModelKey('{}')", self.label())
    }

    fn __str__(&self) -> String {
        self.label()
    }

    fn __hash__(&self) -> isize {
        self.inner as isize
    }

    fn __richcmp__(
        &self,
        other: Bound<'_, PyAny>,
        op: CompareOp,
        py: Python<'_>,
    ) -> PyResult<PyObject> {
        let rhs = match extract_model_key(&other) {
            Ok(value) => Some(value),
            Err(_) => None,
        };
        richcmp_eq_ne(py, &(self.inner as u16), rhs.map(|v| v as u16), op)
    }
}

impl fmt::Display for PyModelKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Composite key identifying a specific instrument/model pairing.
///
/// Examples:
///     >>> PricerKey(InstrumentType.BOND, ModelKey.DISCOUNTING)
///     PricerKey(instrument='bond', model='discounting')
#[pyclass(module = "finstack.valuations.common", name = "PricerKey", frozen)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyPricerKey {
    pub(crate) inner: PricerKey,
}

#[pymethods]
impl PyPricerKey {
    #[new]
    #[pyo3(text_signature = "(instrument, model)")]
    /// Build a key that refers to a (instrument, model) pair.
    ///
    /// Args:
    ///     instrument: Instrument type or snake-case label.
    ///     model: Model key or snake-case label.
    ///
    /// Returns:
    ///     PricerKey: Identifier usable with :class:`PricerRegistry`.
    ///
    /// Raises:
    ///     ValueError: If either identifier is not recognized.
    fn ctor(instrument: Bound<'_, PyAny>, model: Bound<'_, PyAny>) -> PyResult<Self> {
        let InstrumentTypeArg(inst) = instrument.extract()?;
        let ModelKeyArg(model_key) = model.extract()?;
        Ok(Self {
            inner: PricerKey::new(inst, model_key),
        })
    }

    #[getter]
    /// Instrument type component of the key.
    ///
    /// Returns:
    ///     InstrumentType: Instrument portion of the key.
    fn instrument(&self) -> PyInstrumentType {
        PyInstrumentType::new(self.inner.instrument)
    }

    #[getter]
    /// Model key component of the key.
    ///
    /// Returns:
    ///     ModelKey: Model portion of the key.
    fn model(&self) -> PyModelKey {
        PyModelKey::new(self.inner.model)
    }

    fn __repr__(&self) -> String {
        format!(
            "PricerKey(instrument='{}', model='{}')",
            instrument_type_label(self.inner.instrument),
            model_key_label(self.inner.model)
        )
    }

    fn __hash__(&self) -> isize {
        ((self.inner.instrument as isize) << 16) | (self.inner.model as isize)
    }

    fn __richcmp__(
        &self,
        other: Bound<'_, PyAny>,
        op: CompareOp,
        py: Python<'_>,
    ) -> PyResult<PyObject> {
        if let Ok(rhs) = other.extract::<PyRef<Self>>() {
            let lhs_key = (self.inner.instrument as u32, self.inner.model as u32);
            let rhs_key = (rhs.inner.instrument as u32, rhs.inner.model as u32);
            return richcmp_eq_ne(py, &lhs_key, Some(rhs_key), op);
        }
        richcmp_eq_ne(
            py,
            &(self.inner.instrument as u32, self.inner.model as u32),
            None::<(u32, u32)>,
            op,
        )
    }
}

impl fmt::Display for PyPricerKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "({}, {})",
            instrument_type_label(self.inner.instrument),
            model_key_label(self.inner.model)
        )
    }
}

/// Parse a snake-case instrument label into an `InstrumentType`.
fn parse_instrument_type(name: &str) -> PyResult<PyInstrumentType> {
    name.parse::<InstrumentType>()
        .map(PyInstrumentType::new)
        .map_err(|e| PyValueError::new_err(e))
}

/// Parse a snake-case model label into a `ModelKey`.
fn parse_model_key(name: &str) -> PyResult<PyModelKey> {
    name.parse::<ModelKey>()
        .map(PyModelKey::new)
        .map_err(|e| PyValueError::new_err(e))
}

pub(crate) fn instrument_type_label(ty: InstrumentType) -> String {
    ty.to_string()
}

pub(crate) fn model_key_label(key: ModelKey) -> String {
    key.to_string()
}

/// Convert a Python object into an `InstrumentType`.
pub(crate) fn extract_instrument_type(value: &Bound<'_, PyAny>) -> PyResult<InstrumentType> {
    if let Ok(wrapper) = value.extract::<PyRef<PyInstrumentType>>() {
        return Ok(wrapper.inner);
    }
    if let Ok(name) = value.extract::<&str>() {
        return parse_instrument_type(name).map(|wrapper| wrapper.inner);
    }
    Err(PyTypeError::new_err(
        "Expected InstrumentType or string identifier",
    ))
}

/// Convert a Python object into a `ModelKey`.
pub(crate) fn extract_model_key(value: &Bound<'_, PyAny>) -> PyResult<ModelKey> {
    if let Ok(wrapper) = value.extract::<PyRef<PyModelKey>>() {
        return Ok(wrapper.inner);
    }
    if let Ok(name) = value.extract::<&str>() {
        return parse_model_key(name).map(|wrapper| wrapper.inner);
    }
    Err(PyTypeError::new_err(
        "Expected ModelKey or string identifier",
    ))
}

/// Helper for argument parsing: instrument type wrapper.
#[derive(Debug, Clone, Copy)]
pub(crate) struct InstrumentTypeArg(pub InstrumentType);

impl<'py> FromPyObject<'py> for InstrumentTypeArg {
    fn extract_bound(ob: &Bound<'py, PyAny>) -> PyResult<Self> {
        extract_instrument_type(ob).map(InstrumentTypeArg)
    }
}

/// Helper for argument parsing: model key wrapper.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ModelKeyArg(pub ModelKey);

impl<'py> FromPyObject<'py> for ModelKeyArg {
    fn extract_bound(ob: &Bound<'py, PyAny>) -> PyResult<Self> {
        extract_model_key(ob).map(ModelKeyArg)
    }
}

/// Map valuation pricing errors into Python exceptions.
pub(crate) fn pricing_error_to_py(err: PricingError) -> PyErr {
    match err {
        PricingError::UnknownPricer(key) => PyKeyError::new_err(format!(
            "No pricer registered for instrument='{}', model='{}'",
            instrument_type_label(key.instrument),
            model_key_label(key.model)
        )),
        PricingError::TypeMismatch { expected, got } => PyTypeError::new_err(format!(
            "Instrument type mismatch: expected '{}', got '{}'",
            instrument_type_label(expected),
            instrument_type_label(got)
        )),
        PricingError::ModelFailure(message) => PyRuntimeError::new_err(message),
        _ => PyRuntimeError::new_err("Unknown pricing error"),
    }
}

/// Parse a curve identifier from a Python object (string expected).
pub(crate) fn extract_curve_id(value: &Bound<'_, PyAny>) -> PyResult<CurveId> {
    if let Ok(id) = value.extract::<&str>() {
        return Ok(CurveId::new(id));
    }
    Err(PyTypeError::new_err(
        "Expected curve identifier string (e.g. 'USD-OIS')",
    ))
}

/// Parse an instrument identifier from a Python object (string expected).
pub(crate) fn extract_instrument_id(value: &Bound<'_, PyAny>) -> PyResult<InstrumentId> {
    if let Ok(id) = value.extract::<&str>() {
        return Ok(InstrumentId::new(id));
    }
    Err(PyTypeError::new_err(
        "Expected instrument identifier string",
    ))
}

/// Convert an optional string to owned String.
pub(crate) fn to_optional_string(value: Option<&str>) -> Option<String> {
    value.map(|s| s.to_string())
}

/// Parse an optional payments-per-year value into a Frequency.
///
/// Defaults to 4 (quarterly) if None is provided.
pub(crate) fn frequency_from_payments_per_year(
    payments_per_year: Option<u32>,
) -> PyResult<finstack_core::dates::Frequency> {
    use finstack_core::dates::Frequency;
    let payments = payments_per_year.unwrap_or(4);
    Frequency::from_payments_per_year(payments).map_err(|e| PyValueError::new_err(e))
}

/// Parse a frequency label with broad market-friendly synonyms.
/// Examples: "quarterly", "q", "3m"; "semi_annual", "semiannual", "6m"; "annual", "yearly", "12m"; "monthly", "1m"; "bimonthly", "2m".
pub(crate) fn parse_frequency_label(
    label: Option<&str>,
) -> PyResult<finstack_core::dates::Frequency> {
    use finstack_core::dates::Frequency;
    match label.map(normalize_label).as_deref() {
        None => Ok(Frequency::quarterly()),
        Some("quarterly") | Some("q") | Some("3m") => Ok(Frequency::quarterly()),
        Some("semi_annual") | Some("semiannual") | Some("6m") | Some("sa") => {
            Ok(Frequency::semi_annual())
        }
        Some("annual") | Some("yearly") | Some("12m") | Some("1y") => Ok(Frequency::annual()),
        Some("monthly") | Some("1m") | Some("m") => Ok(Frequency::monthly()),
        Some("bimonthly") | Some("2m") => Ok(Frequency::bimonthly()),
        Some(other) => Err(PyValueError::new_err(format!(
            "Unsupported frequency label: {}",
            other
        ))),
    }
}

/// Parse a stub label into StubKind, defaulting to None.
pub(crate) fn parse_stub_kind(label: Option<&str>) -> PyResult<finstack_core::dates::StubKind> {
    match label {
        None => Ok(finstack_core::dates::StubKind::None),
        Some(s) => s.parse().map_err(|e: String| PyValueError::new_err(e)),
    }
}

pub(crate) fn intern_calendar_id_opt(value: Option<&str>) -> Option<&'static str> {
    value.map(|s| Box::leak(s.to_ascii_lowercase().into_boxed_str()) as &'static str)
}
#[allow(dead_code)]
pub(crate) fn intern_calendar_id(value: &str) -> &'static str {
    Box::leak(value.to_ascii_lowercase().into_boxed_str())
}

pub(crate) mod parameters;

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "common")?;
    module.setattr(
        "__doc__",
        "Shared enums, keys, and error helpers used across finstack valuations bindings.",
    )?;
    module.add_class::<PyInstrumentType>()?;
    module.add_class::<PyModelKey>()?;
    module.add_class::<PyPricerKey>()?;

    // Register parameter types submodule
    let _param_exports = parameters::register(py, &module)?;

    let exports = ["InstrumentType", "ModelKey", "PricerKey"];
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}
