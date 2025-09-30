use crate::core::error::core_to_py;
use crate::core::money::PyMoney;
use crate::core::utils::py_to_date;
use finstack_core::money::Money;
use finstack_valuations::cashflow::builder as val_builder;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyList, PyModule, PyType};
use pyo3::Bound;

/// Coupon split type (cash, PIK, split) mirroring valuations builder.
#[pyclass(
    module = "finstack.valuations.cashflow.builder",
    name = "CouponType",
    frozen
)]
#[derive(Clone, Copy, Debug)]
pub struct PyCouponType {
    inner: val_builder::CouponType,
}

impl PyCouponType {
    fn new(inner: val_builder::CouponType) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCouponType {
    #[classattr]
    const CASH: Self = Self {
        inner: val_builder::CouponType::Cash,
    };
    #[classattr]
    const PIK: Self = Self {
        inner: val_builder::CouponType::PIK,
    };

    #[classmethod]
    #[pyo3(text_signature = "(cls, cash_pct, pik_pct)")]
    /// Create a split coupon type with percentage weights summing to ~1.0.
    fn split(_cls: &Bound<'_, PyType>, cash_pct: f64, pik_pct: f64) -> Self {
        Self::new(val_builder::CouponType::Split { cash_pct, pik_pct })
    }
}

/// Schedule parameter bundle.
#[pyclass(
    module = "finstack.valuations.cashflow.builder",
    name = "ScheduleParams",
    frozen
)]
#[derive(Clone, Copy, Debug)]
pub struct PyScheduleParams {
    pub(crate) inner: val_builder::ScheduleParams,
}

#[pymethods]
impl PyScheduleParams {
    #[classmethod]
    #[pyo3(text_signature = "(cls, freq, day_count, bdc, calendar_id=None, stub=None)")]
    fn new(
        _cls: &Bound<'_, PyType>,
        freq: crate::core::dates::schedule::PyFrequency,
        day_count: crate::core::dates::daycount::PyDayCount,
        bdc: crate::core::dates::calendar::PyBusinessDayConvention,
        calendar_id: Option<&str>,
        stub: Option<crate::core::dates::schedule::PyStubKind>,
    ) -> Self {
        let s_owned = calendar_id.map(|s| s.to_string());
        let cal_static: Option<&'static str> = s_owned.map(|owned| {
            let leaked: &'static mut str = Box::leak(owned.into_boxed_str());
            &*leaked
        });
        Self {
            inner: val_builder::ScheduleParams {
                freq: freq.inner,
                dc: day_count.inner,
                bdc: bdc.inner,
                calendar_id: cal_static,
                stub: stub
                    .map(|s| s.inner)
                    .unwrap_or(finstack_core::dates::StubKind::None),
            },
        }
    }

    #[classmethod]
    fn quarterly_act360(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: val_builder::ScheduleParams::quarterly_act360(),
        }
    }
    #[classmethod]
    fn semiannual_30360(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: val_builder::ScheduleParams::semiannual_30360(),
        }
    }
    #[classmethod]
    fn annual_actact(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: val_builder::ScheduleParams::annual_actact(),
        }
    }
}

/// Fixed coupon specification.
#[pyclass(
    module = "finstack.valuations.cashflow.builder",
    name = "FixedCouponSpec",
    frozen
)]
#[derive(Clone, Copy, Debug)]
pub struct PyFixedCouponSpec {
    pub(crate) inner: val_builder::FixedCouponSpec,
}

#[pymethods]
impl PyFixedCouponSpec {
    #[classmethod]
    #[pyo3(text_signature = "(cls, rate, schedule, coupon_type=None)")]
    fn new(
        _cls: &Bound<'_, PyType>,
        rate: f64,
        schedule: PyScheduleParams,
        coupon_type: Option<PyCouponType>,
    ) -> Self {
        Self {
            inner: val_builder::FixedCouponSpec {
                coupon_type: coupon_type
                    .map(|c| c.inner)
                    .unwrap_or(val_builder::CouponType::Cash),
                rate,
                freq: schedule.inner.freq,
                dc: schedule.inner.dc,
                bdc: schedule.inner.bdc,
                calendar_id: schedule.inner.calendar_id,
                stub: schedule.inner.stub,
            },
        }
    }
}

/// Floating coupon parameters and spec.
#[pyclass(
    module = "finstack.valuations.cashflow.builder",
    name = "FloatCouponParams",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyFloatCouponParams {
    pub(crate) inner: val_builder::FloatCouponParams,
}

#[pymethods]
impl PyFloatCouponParams {
    #[classmethod]
    #[pyo3(text_signature = "(cls, index_id, margin_bp, *, gearing=1.0, reset_lag_days=2)")]
    fn new(
        _cls: &Bound<'_, PyType>,
        index_id: &str,
        margin_bp: f64,
        gearing: Option<f64>,
        reset_lag_days: Option<i32>,
    ) -> Self {
        Self {
            inner: val_builder::FloatCouponParams {
                index_id: finstack_core::types::CurveId::new(index_id),
                margin_bp,
                gearing: gearing.unwrap_or(1.0),
                reset_lag_days: reset_lag_days.unwrap_or(2),
            },
        }
    }
}

#[pyclass(
    module = "finstack.valuations.cashflow.builder",
    name = "FloatingCouponSpec",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyFloatingCouponSpec {
    pub(crate) inner: val_builder::FloatingCouponSpec,
}

#[pymethods]
impl PyFloatingCouponSpec {
    #[classmethod]
    #[pyo3(text_signature = "(cls, params, schedule, coupon_type=None)")]
    fn new(
        _cls: &Bound<'_, PyType>,
        params: PyFloatCouponParams,
        schedule: PyScheduleParams,
        coupon_type: Option<PyCouponType>,
    ) -> Self {
        Self {
            inner: val_builder::FloatingCouponSpec {
                index_id: params.inner.index_id.clone(),
                margin_bp: params.inner.margin_bp,
                gearing: params.inner.gearing,
                coupon_type: coupon_type
                    .map(|c| c.inner)
                    .unwrap_or(val_builder::CouponType::Cash),
                freq: schedule.inner.freq,
                dc: schedule.inner.dc,
                bdc: schedule.inner.bdc,
                calendar_id: schedule.inner.calendar_id,
                stub: schedule.inner.stub,
                reset_lag_days: params.inner.reset_lag_days,
            },
        }
    }
}

/// Python wrapper for the composable valuations CashflowBuilder.
#[pyclass(
    module = "finstack.valuations.cashflow.builder",
    name = "CashflowBuilder",
    unsendable
)]
pub struct PyCashflowBuilder {
    inner: val_builder::CashflowBuilder,
}

#[pymethods]
impl PyCashflowBuilder {
    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    fn new(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: val_builder::CashflowBuilder::new(),
        }
    }

    #[pyo3(text_signature = "(self, amount, currency, issue, maturity)")]
    fn principal(
        &mut self,
        amount: f64,
        currency: &crate::core::currency::PyCurrency,
        issue: Bound<'_, PyAny>,
        maturity: Bound<'_, PyAny>,
    ) -> Self {
        let issue_date = py_to_date(&issue).expect("valid date");
        let maturity_date = py_to_date(&maturity).expect("valid date");
        let money = Money::new(amount, currency.inner);
        self.inner.principal(money, issue_date, maturity_date);
        Self {
            inner: self.inner.clone(),
        }
    }

    #[pyo3(text_signature = "(self, amortization)")]
    fn amortization(
        &mut self,
        amortization: Option<crate::core::cashflow::primitives::PyAmortizationSpec>,
    ) -> Self {
        if let Some(spec) = amortization {
            self.inner.amortization(spec.inner);
        }
        Self {
            inner: self.inner.clone(),
        }
    }

    #[pyo3(text_signature = "(self, spec)")]
    fn fixed_cf(&mut self, spec: PyFixedCouponSpec) -> Self {
        self.inner.fixed_cf(spec.inner);
        Self {
            inner: self.inner.clone(),
        }
    }

    #[pyo3(text_signature = "(self, spec)")]
    fn floating_cf(&mut self, spec: PyFloatingCouponSpec) -> Self {
        self.inner.floating_cf(spec.inner);
        Self {
            inner: self.inner.clone(),
        }
    }

    #[pyo3(text_signature = "(self, steps, schedule, default_split)")]
    /// Fixed step-up program with boundaries `steps=[(end_date, rate), ...]`.
    fn fixed_stepup(
        &mut self,
        steps: Vec<(Bound<'_, PyAny>, f64)>,
        schedule: PyScheduleParams,
        default_split: PyCouponType,
    ) -> Self {
        let mut rust_steps: Vec<(time::Date, f64)> = Vec::with_capacity(steps.len());
        for (d, r) in steps {
            rust_steps.push((py_to_date(&d).expect("valid date"), r));
        }
        self.inner
            .fixed_stepup(&rust_steps, schedule.inner, default_split.inner);
        Self {
            inner: self.inner.clone(),
        }
    }

    #[pyo3(text_signature = "(self, steps)")]
    /// Payment split program `(end_date, split)` where `split` is CouponType.
    fn payment_split_program(&mut self, steps: Vec<(Bound<'_, PyAny>, PyCouponType)>) -> Self {
        let mut rust_steps: Vec<(time::Date, val_builder::CouponType)> =
            Vec::with_capacity(steps.len());
        for (d, split) in steps {
            rust_steps.push((py_to_date(&d).expect("valid date"), split.inner));
        }
        self.inner.payment_split_program(&rust_steps);
        Self {
            inner: self.inner.clone(),
        }
    }

    #[pyo3(text_signature = "(self)")]
    fn build(&self) -> PyResult<PyCashFlowSchedule> {
        self.inner
            .build()
            .map(PyCashFlowSchedule::new)
            .map_err(core_to_py)
    }
}

/// CashflowSchedule wrapper exposing holder-side flows and metadata.
#[pyclass(
    module = "finstack.valuations.cashflow.builder",
    name = "CashFlowSchedule",
    frozen
)]
#[derive(Clone)]
pub struct PyCashFlowSchedule {
    inner: val_builder::CashFlowSchedule,
}

impl PyCashFlowSchedule {
    fn new(inner: val_builder::CashFlowSchedule) -> Self {
        Self { inner }
    }
    pub(crate) fn inner_clone(&self) -> val_builder::CashFlowSchedule {
        self.inner.clone()
    }
}

#[pymethods]
impl PyCashFlowSchedule {
    #[getter]
    fn day_count(&self) -> crate::core::dates::PyDayCount {
        crate::core::dates::PyDayCount::new(self.inner.day_count)
    }

    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional.initial)
    }

    #[pyo3(text_signature = "(self)")]
    fn flows(&self, py: Python<'_>) -> PyResult<PyObject> {
        let items: Vec<crate::core::cashflow::primitives::PyCashFlow> = self
            .inner
            .flows
            .iter()
            .copied()
            .map(crate::core::cashflow::primitives::PyCashFlow::new)
            .collect();
        Ok(PyList::new(py, items)?.into())
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyCouponType>()?;
    module.add_class::<PyScheduleParams>()?;
    module.add_class::<PyFixedCouponSpec>()?;
    module.add_class::<PyFloatCouponParams>()?;
    module.add_class::<PyFloatingCouponSpec>()?;
    module.add_class::<PyCashflowBuilder>()?;
    module.add_class::<PyCashFlowSchedule>()?;
    Ok(vec![
        "CouponType",
        "ScheduleParams",
        "FixedCouponSpec",
        "FloatCouponParams",
        "FloatingCouponSpec",
        "CashflowBuilder",
        "CashFlowSchedule",
    ])
}
