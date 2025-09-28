use crate::core::error::core_to_py;
use crate::core::money::{extract_money, PyMoney};
use crate::core::utils::{date_to_py, py_to_date};
use finstack_core::cashflow::primitives::{CFKind, CashFlow};
use pyo3::basic::CompareOp;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAnyMethods, PyModule, PyType};
use pyo3::{Bound, IntoPyObjectExt};

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
            CFKind::Notional => "notional",
            CFKind::PIK => "pik",
            CFKind::Amortization => "amortization",
            CFKind::Fee => "fee",
            CFKind::Stub => "stub",
            _ => "unknown",
        }
    }

    fn parse(name: &str) -> Option<CFKind> {
        let normalized = name.trim().to_ascii_lowercase().replace('-', "_");
        match normalized.as_str() {
            "fixed" => Some(CFKind::Fixed),
            "float_reset" => Some(CFKind::FloatReset),
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

        let result = match op {
            CompareOp::Eq => rhs.map(|v| v == self.inner).unwrap_or(false),
            CompareOp::Ne => rhs.map(|v| v != self.inner).unwrap_or(true),
            _ => return Err(PyValueError::new_err("Unsupported comparison")),
        };

        let py_bool = result.into_bound_py_any(py)?;
        Ok(py_bool.into())
    }
}

#[pyclass(name = "CashFlow", module = "finstack.core.cashflow")]
#[derive(Clone, Debug)]
pub struct PyCashFlow {
    pub(crate) inner: CashFlow,
}

impl PyCashFlow {
    fn new(inner: CashFlow) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCashFlow {
    #[classmethod]
    #[pyo3(
        text_signature = "(cls, date, amount, *, accrual_factor=0.0)",
        signature = (date, amount, *, accrual_factor=0.0)
    )]
    /// Create a fixed coupon cash-flow.
    ///
    /// Parameters
    /// ----------
    /// date : datetime.date
    ///     Payment date for the fixed coupon.
    /// amount : Money or tuple[float, Currency]
    ///     Coupon amount with currency.
    /// accrual_factor : float, default 0.0
    ///     Accrual fraction associated with the coupon for analytics.
    ///
    /// Returns
    /// -------
    /// CashFlow
    ///     Cash-flow tagged as :attr:`CFKind.FIXED`.
    ///
    /// Examples
    /// --------
    /// >>> from finstack import Money
    /// >>> from finstack.core.cashflow import CashFlow
    /// >>> cf = CashFlow.fixed(date(2025, 6, 15), Money(2500, "USD"), accrual_factor=0.25)
    pub fn fixed(
        _cls: &Bound<'_, PyType>,
        date: Bound<'_, PyAny>,
        amount: Bound<'_, PyAny>,
        accrual_factor: f64,
    ) -> PyResult<Self> {
        let pay_date = py_to_date(&date)?;
        let money = extract_money(&amount)?;
        let mut inner = CashFlow::fixed_cf(pay_date, money).map_err(core_to_py)?;
        inner.accrual_factor = accrual_factor;
        Ok(Self::new(inner))
    }

    #[classmethod]
    #[pyo3(
        text_signature = "(cls, date, amount, *, reset_date=None, accrual_factor=0.0)",
        signature = (date, amount, *, reset_date=None, accrual_factor=0.0)
    )]
    /// Create a floating-rate cash-flow with optional reset date.
    ///
    /// Parameters
    /// ----------
    /// date : datetime.date
    ///     Coupon payment date.
    /// amount : Money or tuple[float, Currency]
    ///     Floating coupon amount.
    /// reset_date : datetime.date, optional
    ///     Index reset date; defaults to the payment date when omitted.
    /// accrual_factor : float, default 0.0
    ///     Accrual fraction associated with the coupon.
    ///
    /// Returns
    /// -------
    /// CashFlow
    ///     Cash-flow tagged as :attr:`CFKind.FLOAT_RESET`.
    pub fn floating(
        _cls: &Bound<'_, PyType>,
        date: Bound<'_, PyAny>,
        amount: Bound<'_, PyAny>,
        reset_date: Option<Bound<'_, PyAny>>,
        accrual_factor: f64,
    ) -> PyResult<Self> {
        let pay_date = py_to_date(&date)?;
        let money = extract_money(&amount)?;
        let reset = match reset_date {
            Some(value) => Some(py_to_date(&value)?),
            None => None,
        };
        let mut inner = CashFlow::floating_cf(pay_date, money, reset).map_err(core_to_py)?;
        inner.accrual_factor = accrual_factor;
        Ok(Self::new(inner))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, date, amount)")]
    /// Create a payment-in-kind (PIK) cash-flow.
    ///
    /// Parameters
    /// ----------
    /// date : datetime.date
    ///     Date on which the PIK amount capitalizes.
    /// amount : Money or tuple[float, Currency]
    ///     Amount added to principal.
    ///
    /// Returns
    /// -------
    /// CashFlow
    ///     Cash-flow tagged as :attr:`CFKind.PIK`.
    pub fn pik(
        _cls: &Bound<'_, PyType>,
        date: Bound<'_, PyAny>,
        amount: Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        let pay_date = py_to_date(&date)?;
        let money = extract_money(&amount)?;
        let inner = CashFlow::pik_cf(pay_date, money).map_err(core_to_py)?;
        Ok(Self::new(inner))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, date, amount)")]
    /// Create an amortization principal cash-flow.
    ///
    /// Parameters
    /// ----------
    /// date : datetime.date
    ///     Date of the principal reduction.
    /// amount : Money or tuple[float, Currency]
    ///     Principal amount paid.
    ///
    /// Returns
    /// -------
    /// CashFlow
    ///     Cash-flow tagged as :attr:`CFKind.AMORTIZATION`.
    pub fn amortization(
        _cls: &Bound<'_, PyType>,
        date: Bound<'_, PyAny>,
        amount: Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        let pay_date = py_to_date(&date)?;
        let money = extract_money(&amount)?;
        let inner = CashFlow::amort_cf(pay_date, money).map_err(core_to_py)?;
        Ok(Self::new(inner))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, date, amount)")]
    /// Create a principal exchange cash-flow.
    ///
    /// Parameters
    /// ----------
    /// date : datetime.date
    ///     Settlement date for the notional exchange.
    /// amount : Money or tuple[float, Currency]
    ///     Principal amount exchanged.
    ///
    /// Returns
    /// -------
    /// CashFlow
    ///     Cash-flow tagged as :attr:`CFKind.NOTIONAL`.
    pub fn principal_exchange(
        _cls: &Bound<'_, PyType>,
        date: Bound<'_, PyAny>,
        amount: Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        let pay_date = py_to_date(&date)?;
        let money = extract_money(&amount)?;
        let inner = CashFlow::principal_exchange(pay_date, money).map_err(core_to_py)?;
        Ok(Self::new(inner))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, date, amount)")]
    /// Create a fee cash-flow.
    ///
    /// Parameters
    /// ----------
    /// date : datetime.date
    ///     Fee payment date.
    /// amount : Money or tuple[float, Currency]
    ///     Fee amount.
    ///
    /// Returns
    /// -------
    /// CashFlow
    ///     Cash-flow tagged as :attr:`CFKind.FEE`.
    pub fn fee(
        _cls: &Bound<'_, PyType>,
        date: Bound<'_, PyAny>,
        amount: Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        let pay_date = py_to_date(&date)?;
        let money = extract_money(&amount)?;
        let inner = CashFlow::fee(pay_date, money).map_err(core_to_py)?;
        Ok(Self::new(inner))
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
