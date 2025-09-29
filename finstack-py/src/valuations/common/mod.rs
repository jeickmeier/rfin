use crate::core::common::{labels::normalize_label, pycmp::richcmp_eq_ne};
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::pricer::{InstrumentType, ModelKey, PricerKey, PricingError};
use pyo3::basic::CompareOp;
use pyo3::exceptions::{PyKeyError, PyRuntimeError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule, PyModuleMethods, PyType};
use pyo3::Bound;
use std::fmt;

/// Wrapper around `finstack_valuations::pricer::InstrumentType` with Python helpers.
#[pyclass(module = "finstack.valuations.common", name = "InstrumentType", frozen)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyInstrumentType {
    pub(crate) inner: InstrumentType,
}

impl PyInstrumentType {
    pub(crate) const fn new(inner: InstrumentType) -> Self {
        Self { inner }
    }

    fn label(&self) -> &'static str {
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
    const CLO: Self = Self::new(InstrumentType::CLO);
    #[classattr]
    const ABS: Self = Self::new(InstrumentType::ABS);
    #[classattr]
    const RMBS: Self = Self::new(InstrumentType::RMBS);
    #[classattr]
    const CMBS: Self = Self::new(InstrumentType::CMBS);
    #[classattr]
    const PRIVATE_MARKETS_FUND: Self = Self::new(InstrumentType::PrivateMarketsFund);

    #[classmethod]
    #[pyo3(text_signature = "(cls, name)")]
    /// Parse a snake-case name into an :class:`InstrumentType`.
    fn from_name(_cls: &Bound<'_, PyType>, name: &str) -> PyResult<Self> {
        parse_instrument_type(name)
    }

    #[getter]
    /// Snake-case identifier for the instrument family.
    fn name(&self) -> &'static str {
        self.label()
    }

    fn __repr__(&self) -> String {
        format!("InstrumentType('{}')", self.label())
    }

    fn __str__(&self) -> &'static str {
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

/// Wrapper around `finstack_valuations::pricer::ModelKey` with Python helpers.
#[pyclass(module = "finstack.valuations.common", name = "ModelKey", frozen)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyModelKey {
    pub(crate) inner: ModelKey,
}

impl PyModelKey {
    pub(crate) const fn new(inner: ModelKey) -> Self {
        Self { inner }
    }

    fn label(&self) -> &'static str {
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
    /// Parse a snake-case model key label.
    fn from_name(_cls: &Bound<'_, PyType>, name: &str) -> PyResult<Self> {
        parse_model_key(name)
    }

    #[getter]
    /// Snake-case identifier for this pricing model.
    fn name(&self) -> &'static str {
        self.label()
    }

    fn __repr__(&self) -> String {
        format!("ModelKey('{}')", self.label())
    }

    fn __str__(&self) -> &'static str {
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

/// Wrapper for `PricerKey` combining instrument and model identifiers.
#[pyclass(module = "finstack.valuations.common", name = "PricerKey", frozen)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyPricerKey {
    pub(crate) inner: PricerKey,
}

#[pymethods]
impl PyPricerKey {
    #[new]
    #[pyo3(text_signature = "(instrument, model)")]
    /// Create a pricer key from instrument/model identifiers.
    fn ctor(instrument: Bound<'_, PyAny>, model: Bound<'_, PyAny>) -> PyResult<Self> {
        let InstrumentTypeArg(inst) = instrument.extract()?;
        let ModelKeyArg(model_key) = model.extract()?;
        Ok(Self {
            inner: PricerKey::new(inst, model_key),
        })
    }

    #[getter]
    /// Instrument type component of the key.
    fn instrument(&self) -> PyInstrumentType {
        PyInstrumentType::new(self.inner.instrument)
    }

    #[getter]
    /// Model key component of the key.
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
    let normalized = normalize_label(name);
    let ty = match normalized.as_str() {
        "bond" => InstrumentType::Bond,
        "loan" => InstrumentType::Loan,
        "cds" => InstrumentType::CDS,
        "cds_index" | "cdsindex" => InstrumentType::CDSIndex,
        "cds_tranche" | "cdstranche" => InstrumentType::CDSTranche,
        "cds_option" | "cdsoption" => InstrumentType::CDSOption,
        "irs" | "swap" | "interest_rate_swap" => InstrumentType::IRS,
        "cap_floor" | "capfloor" | "interest_rate_option" => InstrumentType::CapFloor,
        "swaption" => InstrumentType::Swaption,
        "trs" | "total_return_swap" => InstrumentType::TRS,
        "basis_swap" | "basisswap" => InstrumentType::BasisSwap,
        "basket" => InstrumentType::Basket,
        "convertible" | "convertible_bond" => InstrumentType::Convertible,
        "deposit" => InstrumentType::Deposit,
        "equity_option" | "equityoption" => InstrumentType::EquityOption,
        "fx_option" | "fxoption" => InstrumentType::FxOption,
        "fx_spot" | "fxspot" => InstrumentType::FxSpot,
        "fx_swap" | "fxswap" => InstrumentType::FxSwap,
        "inflation_linked_bond" | "ilb" => InstrumentType::InflationLinkedBond,
        "inflation_swap" => InstrumentType::InflationSwap,
        "interest_rate_future" | "ir_future" | "irfuture" => InstrumentType::InterestRateFuture,
        "variance_swap" | "varianceswap" => InstrumentType::VarianceSwap,
        "equity" => InstrumentType::Equity,
        "repo" => InstrumentType::Repo,
        "fra" => InstrumentType::FRA,
        "clo" => InstrumentType::CLO,
        "abs" => InstrumentType::ABS,
        "rmbs" => InstrumentType::RMBS,
        "cmbs" => InstrumentType::CMBS,
        "private_markets_fund" | "pmf" => InstrumentType::PrivateMarketsFund,
        other => {
            return Err(PyValueError::new_err(format!(
                "Unknown instrument type: {other}"
            )))
        }
    };
    Ok(PyInstrumentType::new(ty))
}

/// Parse a snake-case model label into a `ModelKey`.
fn parse_model_key(name: &str) -> PyResult<PyModelKey> {
    let normalized = normalize_label(name);
    let key = match normalized.as_str() {
        "discounting" => ModelKey::Discounting,
        "tree" | "lattice" => ModelKey::Tree,
        "black76" | "black" | "black_76" => ModelKey::Black76,
        "hull_white_1f" | "hullwhite1f" | "hw1f" => ModelKey::HullWhite1F,
        "hazard_rate" | "hazard" => ModelKey::HazardRate,
        other => return Err(PyValueError::new_err(format!("Unknown model key: {other}"))),
    };
    Ok(PyModelKey::new(key))
}

pub(crate) fn instrument_type_label(ty: InstrumentType) -> &'static str {
    match ty {
        InstrumentType::Bond => "bond",
        InstrumentType::Loan => "loan",
        InstrumentType::CDS => "cds",
        InstrumentType::CDSIndex => "cds_index",
        InstrumentType::CDSTranche => "cds_tranche",
        InstrumentType::CDSOption => "cds_option",
        InstrumentType::IRS => "irs",
        InstrumentType::CapFloor => "cap_floor",
        InstrumentType::Swaption => "swaption",
        InstrumentType::TRS => "trs",
        InstrumentType::BasisSwap => "basis_swap",
        InstrumentType::Basket => "basket",
        InstrumentType::Convertible => "convertible",
        InstrumentType::Deposit => "deposit",
        InstrumentType::EquityOption => "equity_option",
        InstrumentType::FxOption => "fx_option",
        InstrumentType::FxSpot => "fx_spot",
        InstrumentType::FxSwap => "fx_swap",
        InstrumentType::InflationLinkedBond => "inflation_linked_bond",
        InstrumentType::InflationSwap => "inflation_swap",
        InstrumentType::InterestRateFuture => "interest_rate_future",
        InstrumentType::VarianceSwap => "variance_swap",
        InstrumentType::Equity => "equity",
        InstrumentType::Repo => "repo",
        InstrumentType::FRA => "fra",
        InstrumentType::CLO => "clo",
        InstrumentType::ABS => "abs",
        InstrumentType::RMBS => "rmbs",
        InstrumentType::CMBS => "cmbs",
        InstrumentType::PrivateMarketsFund => "private_markets_fund",
    }
}

pub(crate) fn model_key_label(key: ModelKey) -> &'static str {
    match key {
        ModelKey::Discounting => "discounting",
        ModelKey::Tree => "tree",
        ModelKey::Black76 => "black76",
        ModelKey::HullWhite1F => "hull_white_1f",
        ModelKey::HazardRate => "hazard_rate",
    }
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
    let exports = ["InstrumentType", "ModelKey", "PricerKey"];
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}
