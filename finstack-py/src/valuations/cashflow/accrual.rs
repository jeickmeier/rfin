//! Python bindings for cashflow accrual types and the accrued interest engine.

use finstack_valuations::cashflow::accrual::{AccrualConfig, AccrualMethod, ExCouponRule};
use finstack_valuations::cashflow::builder::CashFlowSchedule;
use pyo3::prelude::*;
use pyo3::types::{PyModule, PyType};

use crate::errors::core_to_py;

// ---------------------------------------------------------------------------
// AccrualMethod
// ---------------------------------------------------------------------------

/// Accrual interpolation method for interest calculations.
///
/// Linear uses simple proportion of the coupon period.
/// Compounded uses ICMA-style compounding.
#[pyclass(
    name = "AccrualMethod",
    module = "finstack.valuations.cashflow",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyAccrualMethod {
    pub(crate) inner: AccrualMethod,
}

#[pymethods]
impl PyAccrualMethod {
    /// Linear accrual (simple interest interpolation).
    #[classattr]
    const LINEAR: Self = Self {
        inner: AccrualMethod::Linear,
    };

    /// Compounded accrual (ICMA-style).
    #[classattr]
    const COMPOUNDED: Self = Self {
        inner: AccrualMethod::Compounded,
    };

    fn __repr__(&self) -> &'static str {
        match self.inner {
            AccrualMethod::Linear => "AccrualMethod.LINEAR",
            AccrualMethod::Compounded => "AccrualMethod.COMPOUNDED",
            _ => "AccrualMethod.<unknown>",
        }
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

// ---------------------------------------------------------------------------
// ExCouponRule
// ---------------------------------------------------------------------------

/// Ex-coupon convention applied to coupon flows.
///
/// When the valuation date falls inside the ex-coupon window
/// (between ex-date and payment date), accrued interest is zero.
#[pyclass(
    name = "ExCouponRule",
    module = "finstack.valuations.cashflow",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyExCouponRule {
    pub(crate) inner: ExCouponRule,
}

#[pymethods]
impl PyExCouponRule {
    #[classmethod]
    #[pyo3(
        signature = (days_before_coupon, calendar_id=None),
        text_signature = "(cls, days_before_coupon, calendar_id=None)"
    )]
    /// Create an ex-coupon rule.
    ///
    /// Args:
    ///     days_before_coupon: Number of days before coupon date that go ex.
    ///     calendar_id: Optional calendar for business-day counting.
    ///         If None, calendar days are used.
    fn new(_cls: &Bound<'_, PyType>, days_before_coupon: u32, calendar_id: Option<String>) -> Self {
        Self {
            inner: ExCouponRule {
                days_before_coupon,
                calendar_id,
            },
        }
    }

    #[getter]
    fn days_before_coupon(&self) -> u32 {
        self.inner.days_before_coupon
    }

    #[getter]
    fn calendar_id(&self) -> Option<&str> {
        self.inner.calendar_id.as_deref()
    }

    fn __repr__(&self) -> String {
        match &self.inner.calendar_id {
            Some(cal) => format!(
                "ExCouponRule(days={}, calendar='{}')",
                self.inner.days_before_coupon, cal
            ),
            None => format!(
                "ExCouponRule(days={}, calendar=None)",
                self.inner.days_before_coupon
            ),
        }
    }
}

// ---------------------------------------------------------------------------
// AccrualConfig
// ---------------------------------------------------------------------------

/// Configuration for schedule-driven interest accrual.
///
/// Bundles the accrual method, optional ex-coupon rule, and whether PIK
/// interest is included in the accrued amount.
#[pyclass(
    name = "AccrualConfig",
    module = "finstack.valuations.cashflow",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyAccrualConfig {
    pub(crate) inner: AccrualConfig,
}

#[pymethods]
impl PyAccrualConfig {
    #[classmethod]
    #[pyo3(
        signature = (method=None, ex_coupon=None, include_pik=None),
        text_signature = "(cls, method=None, ex_coupon=None, include_pik=True)"
    )]
    /// Create an accrual configuration.
    ///
    /// Args:
    ///     method: Accrual method (default: LINEAR).
    ///     ex_coupon: Optional ex-coupon rule.
    ///     include_pik: Whether to include PIK interest (default: True).
    fn new(
        _cls: &Bound<'_, PyType>,
        method: Option<PyAccrualMethod>,
        ex_coupon: Option<PyExCouponRule>,
        include_pik: Option<bool>,
    ) -> Self {
        Self {
            inner: AccrualConfig {
                method: method.map(|m| m.inner).unwrap_or(AccrualMethod::Linear),
                ex_coupon: ex_coupon.map(|e| e.inner),
                include_pik: include_pik.unwrap_or(true),
            },
        }
    }

    #[getter]
    fn method(&self) -> PyAccrualMethod {
        PyAccrualMethod {
            inner: self.inner.method.clone(),
        }
    }

    #[getter]
    fn ex_coupon(&self) -> Option<PyExCouponRule> {
        self.inner
            .ex_coupon
            .as_ref()
            .map(|e| PyExCouponRule { inner: e.clone() })
    }

    #[getter]
    fn include_pik(&self) -> bool {
        self.inner.include_pik
    }

    fn __repr__(&self) -> String {
        format!(
            "AccrualConfig(method={:?}, include_pik={})",
            self.inner.method, self.inner.include_pik
        )
    }
}

// ---------------------------------------------------------------------------
// accrued_interest_amount
// ---------------------------------------------------------------------------

/// Compute accrued interest from a cashflow schedule.
///
/// The returned value is expressed in the same currency space as the schedule's
/// coupon and notional amounts.
///
/// Args:
///     schedule: CashFlowSchedule containing coupon, PIK, and notional flows.
///     as_of: Accrual cut-off date.
///     config: Accrual configuration (method, ex-coupon, PIK inclusion).
///
/// Returns:
///     Scalar accrued interest amount. Returns 0.0 when the schedule has
///     no coupon periods, the as_of date is outside all coupon periods,
///     or the as_of date falls inside an active ex-coupon window.
///
/// Raises:
///     FinstackError: If the outstanding balance path cannot be constructed,
///         a day-count calculation fails, or a calendar ID is invalid.
#[pyfunction]
#[pyo3(
    name = "accrued_interest_amount",
    text_signature = "(schedule, as_of, config)"
)]
fn py_accrued_interest_amount(
    schedule: &super::builder::PyCashFlowSchedule,
    as_of: Bound<'_, pyo3::types::PyAny>,
    config: &PyAccrualConfig,
) -> PyResult<f64> {
    let as_of_date = crate::core::dates::utils::py_to_date(&as_of)?;
    let inner_schedule: CashFlowSchedule = schedule.inner_clone();
    finstack_valuations::cashflow::accrual::accrued_interest_amount(
        &inner_schedule,
        as_of_date,
        &config.inner,
    )
    .map_err(core_to_py)
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyAccrualMethod>()?;
    module.add_class::<PyExCouponRule>()?;
    module.add_class::<PyAccrualConfig>()?;
    module.add_function(wrap_pyfunction!(py_accrued_interest_amount, module)?)?;
    Ok(vec![
        "AccrualConfig",
        "AccrualMethod",
        "ExCouponRule",
        "accrued_interest_amount",
    ])
}
