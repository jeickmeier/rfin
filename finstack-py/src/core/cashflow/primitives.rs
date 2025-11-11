use crate::core::common::{labels::normalize_label, pycmp::richcmp_eq_ne};
use crate::core::error::core_to_py;
use crate::core::money::{extract_money, PyMoney};
use crate::core::utils::{date_to_py, py_to_date};
use finstack_core::cashflow::primitives::{CFKind, CashFlow};
use pyo3::basic::CompareOp;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAnyMethods, PyModule, PyType};
use pyo3::Bound;

/// Enumeration of cash-flow categories used across finstack-core.
///
/// Parameters
/// ----------
/// None
///     Use class attributes such as :attr:`CFKind.FIXED` instead of instantiating directly.
///
/// Returns
/// -------
/// CFKind
///     Enum value describing the cash-flow classification.
#[pyclass(name = "CFKind", module = "finstack.core.cashflow", frozen)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PyCFKind {
    pub(crate) inner: CFKind,
}

impl PyCFKind {
    pub(crate) const fn new(inner: CFKind) -> Self {
        Self { inner }
    }

    fn label(&self) -> &'static str {
        match self.inner {
            CFKind::Fixed => "fixed",
            CFKind::FloatReset => "float_reset",
            CFKind::CommitmentFee => "commitment_fee",
            CFKind::UsageFee => "usage_fee",
            CFKind::FacilityFee => "facility_fee",
            CFKind::Notional => "notional",
            CFKind::PIK => "pik",
            CFKind::Amortization => "amortization",
            CFKind::Fee => "fee",
            CFKind::Stub => "stub",
            _ => "unknown",
        }
    }

    fn parse(name: &str) -> Option<CFKind> {
        let normalized = normalize_label(name);
        match normalized.as_str() {
            "fixed" => Some(CFKind::Fixed),
            "float_reset" => Some(CFKind::FloatReset),
            "commitment_fee" => Some(CFKind::CommitmentFee),
            "usage_fee" => Some(CFKind::UsageFee),
            "facility_fee" => Some(CFKind::FacilityFee),
            "notional" => Some(CFKind::Notional),
            "pik" => Some(CFKind::PIK),
            "amortization" | "amort" => Some(CFKind::Amortization),
            "fee" => Some(CFKind::Fee),
            "stub" => Some(CFKind::Stub),
            _ => None,
        }
    }
}

#[pymethods]
impl PyCFKind {
    #[classattr]
    const FIXED: Self = Self::new(CFKind::Fixed);
    #[classattr]
    const FLOAT_RESET: Self = Self::new(CFKind::FloatReset);
    #[classattr]
    const COMMITMENT_FEE: Self = Self::new(CFKind::CommitmentFee);
    #[classattr]
    const USAGE_FEE: Self = Self::new(CFKind::UsageFee);
    #[classattr]
    const FACILITY_FEE: Self = Self::new(CFKind::FacilityFee);
    #[classattr]
    const NOTIONAL: Self = Self::new(CFKind::Notional);
    #[classattr]
    const PIK: Self = Self::new(CFKind::PIK);
    #[classattr]
    const AMORTIZATION: Self = Self::new(CFKind::Amortization);
    #[classattr]
    const FEE: Self = Self::new(CFKind::Fee);
    #[classattr]
    const STUB: Self = Self::new(CFKind::Stub);

    #[classmethod]
    #[pyo3(text_signature = "(cls, name)")]
    /// Parse a cash-flow kind from its snake-case name.
    ///
    /// Parameters
    /// ----------
    /// name : str
    ///     Snake-case identifier such as ``"fixed"``.
    ///
    /// Returns
    /// -------
    /// CFKind
    ///     Parsed enumeration value.
    pub fn from_name(_cls: &Bound<'_, PyType>, name: &str) -> PyResult<Self> {
        match Self::parse(name) {
            Some(kind) => Ok(Self::new(kind)),
            None => Err(PyValueError::new_err(format!(
                "Unknown cash-flow kind: {name}"
            ))),
        }
    }

    /// Snake-case name of the enumeration value.
    #[getter]
    pub fn name(&self) -> &'static str {
        self.label()
    }

    fn __repr__(&self) -> String {
        format!("CFKind('{}')", self.label())
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
        let rhs = match extract_cf_kind(&other) {
            Ok(value) => Some(value),
            Err(_) => None,
        };
        richcmp_eq_ne(py, &self.inner, rhs, op)
    }
}

#[pyclass(name = "CashFlow", module = "finstack.core.cashflow")]
#[derive(Clone, Debug)]
pub struct PyCashFlow {
    pub(crate) inner: CashFlow,
}

impl PyCashFlow {
    pub(crate) fn new(inner: CashFlow) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCashFlow {
    #[new]
    #[pyo3(signature = (date, amount, kind, accrual_factor=0.0, reset_date=None))]
    /// Create a new cashflow.
    ///
    /// Parameters
    /// ----------
    /// date : date or str
    ///     Cashflow payment date.
    /// amount : Money
    ///     Cashflow amount.
    /// kind : CFKind
    ///     Cashflow kind.
    /// accrual_factor : float, optional
    ///     Accrual factor (default: 0.0).
    /// reset_date : date or str, optional
    ///     Reset date for floating cashflows (default: None).
    ///
    /// Returns
    /// -------
    /// CashFlow
    ///     A new cashflow instance.
    ///
    /// Examples
    /// --------
    /// >>> from finstack import Money
    /// >>> from finstack.core.cashflow import CashFlow, CFKind
    /// >>> from datetime import date
    /// >>> cf = CashFlow(
    /// ...     date=date(2025, 6, 15),
    /// ...     amount=Money(2500, "USD"),
    /// ...     kind=CFKind.from_name("Fixed"),
    /// ...     accrual_factor=0.25
    /// ... )
    pub fn __new__(
        date: &Bound<'_, PyAny>,
        amount: &Bound<'_, PyAny>,
        kind: &Bound<'_, PyAny>,
        accrual_factor: f64,
        reset_date: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        let date = py_to_date(date)?;
        let amount = extract_money(amount)?;
        let kind = extract_cf_kind(kind)?;
        let reset_date = reset_date.map(py_to_date).transpose()?;

        Ok(Self {
            inner: CashFlow {
                date,
                amount,
                kind,
                accrual_factor,
                reset_date,
                rate: None,
            },
        })
    }

    /// Validate cashflow amount and fields.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the cashflow amount is zero.
    ///
    /// Examples
    /// --------
    /// >>> from finstack import Money
    /// >>> from finstack.core.cashflow import CashFlow, CFKind
    /// >>> from datetime import date
    /// >>> cf = CashFlow(
    /// ...     date=date(2025, 6, 15),
    /// ...     amount=Money(2500, "USD"),
    /// ...     kind=CFKind.FIXED,
    /// ...     accrual_factor=0.25
    /// ... )
    /// >>> cf.validate()  # Should pass
    pub fn validate(&self) -> PyResult<()> {
        self.inner.validate().map_err(core_to_py)
    }

    #[getter]
    /// Cash-flow kind (classification helper).
    ///
    /// Returns
    /// -------
    /// CFKind
    ///     Enum identifying the cash-flow type.
    pub fn kind(&self) -> PyCFKind {
        PyCFKind::new(self.inner.kind)
    }

    #[getter]
    /// Payment or event date.
    ///
    /// Returns
    /// -------
    /// datetime.date
    ///     Cash-flow date.
    pub fn date(&self, py: Python<'_>) -> PyResult<PyObject> {
        date_to_py(py, self.inner.date)
    }

    #[getter]
    /// Index reset date for floating coupons (if present).
    ///
    /// Returns
    /// -------
    /// datetime.date or None
    ///     Reset date when available.
    pub fn reset_date(&self, py: Python<'_>) -> PyResult<Option<PyObject>> {
        match self.inner.reset_date {
            Some(value) => Ok(Some(date_to_py(py, value)?)),
            None => Ok(None),
        }
    }

    #[getter]
    /// Monetary amount attached to the cash-flow.
    ///
    /// Returns
    /// -------
    /// Money
    ///     Underlying money amount.
    pub fn amount(&self) -> PyMoney {
        PyMoney::new(self.inner.amount)
    }

    #[getter]
    /// Accrual factor associated with the cash-flow.
    ///
    /// Returns
    /// -------
    /// float
    ///     Stored accrual fraction.
    pub fn accrual_factor(&self) -> f64 {
        self.inner.accrual_factor
    }

    #[setter]
    /// Update the stored accrual factor.
    ///
    /// Parameters
    /// ----------
    /// value : float
    ///     New accrual fraction.
    pub fn set_accrual_factor(&mut self, value: f64) {
        self.inner.accrual_factor = value;
    }

    #[pyo3(text_signature = "(self)")]
    /// Convert to a tuple of ``(date, amount, kind, accrual_factor, reset_date)``.
    ///
    /// Returns
    /// -------
    /// tuple
    ///     Tuple containing date, :class:`Money`, :class:`CFKind`, accrual factor, and optional reset date.
    pub fn to_tuple(
        &self,
        py: Python<'_>,
    ) -> PyResult<(PyObject, PyMoney, PyCFKind, f64, Option<PyObject>)> {
        let date = date_to_py(py, self.inner.date)?;
        let reset = match self.inner.reset_date {
            Some(value) => Some(date_to_py(py, value)?),
            None => None,
        };
        Ok((
            date,
            PyMoney::new(self.inner.amount),
            PyCFKind::new(self.inner.kind),
            self.inner.accrual_factor,
            reset,
        ))
    }

    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        let date = date_to_py(py, self.inner.date)?;
        let reset = match self.inner.reset_date {
            Some(value) => Some(date_to_py(py, value)?),
            None => None,
        };
        let reset_repr = match reset {
            Some(obj) => {
                let py_ref = obj.bind(py);
                format!("reset_date={}", py_ref.str()?.to_str()?)
            }
            None => "reset_date=None".to_string(),
        };
        Ok(format!(
            "CashFlow(kind={}, date={}, amount={}, {}, accrual_factor={})",
            self.kind().name(),
            date.bind(py).str()?.to_str()?,
            self.inner.amount,
            reset_repr,
            self.inner.accrual_factor
        ))
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyCFKind>()?;
    module.add_class::<PyCashFlow>()?;
    Ok(vec!["CFKind", "CashFlow"])
}

pub(crate) fn extract_cf_kind(value: &Bound<'_, PyAny>) -> PyResult<CFKind> {
    if let Ok(kind) = value.extract::<PyRef<PyCFKind>>() {
        return Ok(kind.inner);
    }

    if let Ok(name) = value.extract::<&str>() {
        return PyCFKind::parse(name)
            .ok_or_else(|| PyValueError::new_err(format!("Unknown cash-flow kind: {name}")));
    }

    Err(PyTypeError::new_err("Expected CFKind or string identifier"))
}
