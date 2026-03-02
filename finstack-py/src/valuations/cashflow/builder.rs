use crate::core::dates::periods::{PyPeriod, PyPeriodPlan};
use crate::core::dates::utils::py_to_date;
use crate::core::market_data::context::PyMarketContext;
use crate::core::market_data::term_structures::PyDiscountCurve;
use crate::core::money::PyMoney;
use crate::errors::core_to_py;
use crate::errors::PyContext;
use finstack_core::dates::{Period, PeriodId};
use finstack_core::money::Money;
use finstack_core::types::CurveId;
use finstack_core::HashMap;
use finstack_valuations::cashflow::builder as val_builder;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyList, PyModule, PyType};
use pyo3::Bound;

/// Coupon split type (cash, PIK, split) mirroring valuations builder.
#[pyclass(
    module = "finstack.valuations.cashflow.builder",
    name = "CouponType",
    frozen,
    from_py_object
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
        Self::new(val_builder::CouponType::Split {
            cash_pct: rust_decimal::Decimal::from_f64_retain(cash_pct).unwrap_or_default(),
            pik_pct: rust_decimal::Decimal::from_f64_retain(pik_pct).unwrap_or_default(),
        })
    }
}

/// Schedule parameter bundle.
#[pyclass(
    module = "finstack.valuations.cashflow.builder",
    name = "ScheduleParams",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyScheduleParams {
    pub(crate) inner: val_builder::ScheduleParams,
}

#[pymethods]
impl PyScheduleParams {
    #[classmethod]
    #[pyo3(
        signature = (freq, day_count, bdc, calendar_id, stub=None, end_of_month=None, payment_lag_days=None),
        text_signature = "(cls, freq, day_count, bdc, calendar_id, stub=None, end_of_month=False, payment_lag_days=0)"
    )]
    #[allow(clippy::too_many_arguments)]
    fn new(
        _cls: &Bound<'_, PyType>,
        freq: crate::core::dates::schedule::PyFrequency,
        day_count: crate::core::dates::daycount::PyDayCount,
        bdc: crate::core::dates::calendar::PyBusinessDayConvention,
        calendar_id: &str,
        stub: Option<crate::core::dates::schedule::PyStubKind>,
        end_of_month: Option<bool>,
        payment_lag_days: Option<i32>,
    ) -> Self {
        Self {
            inner: val_builder::ScheduleParams {
                freq: freq.inner,
                dc: day_count.inner,
                bdc: bdc.inner,
                calendar_id: calendar_id.to_string(),
                stub: stub
                    .map(|s| s.inner)
                    .unwrap_or(finstack_core::dates::StubKind::None),
                end_of_month: end_of_month.unwrap_or(false),
                payment_lag_days: payment_lag_days.unwrap_or(0),
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

    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    /// USD market standard: quarterly, Act/360, Modified Following, USD calendar.
    ///
    /// Returns:
    ///     ScheduleParams: USD standard configuration
    fn usd_standard(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: val_builder::ScheduleParams::usd_standard(),
        }
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    /// EUR market standard: semi-annual, 30/360, Modified Following, EUR calendar.
    ///
    /// Returns:
    ///     ScheduleParams: EUR standard configuration
    fn eur_standard(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: val_builder::ScheduleParams::eur_standard(),
        }
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    /// GBP market standard: semi-annual, Act/365, Modified Following, GBP calendar.
    ///
    /// Returns:
    ///     ScheduleParams: GBP standard configuration
    fn gbp_standard(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: val_builder::ScheduleParams::gbp_standard(),
        }
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    /// JPY market standard: semi-annual, Act/365, Modified Following, JPY calendar.
    ///
    /// Returns:
    ///     ScheduleParams: JPY standard configuration
    fn jpy_standard(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: val_builder::ScheduleParams::jpy_standard(),
        }
    }
}

/// Fixed coupon specification.
#[pyclass(
    module = "finstack.valuations.cashflow.builder",
    name = "FixedCouponSpec",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyFixedCouponSpec {
    pub(crate) inner: val_builder::FixedCouponSpec,
}

#[pymethods]
impl PyFixedCouponSpec {
    #[classmethod]
    #[pyo3(
        signature = (rate, schedule, coupon_type=None),
        text_signature = "(cls, rate, schedule, coupon_type=None)"
    )]
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
                rate: rust_decimal::Decimal::from_f64_retain(rate).unwrap_or_default(),
                freq: schedule.inner.freq,
                dc: schedule.inner.dc,
                bdc: schedule.inner.bdc,
                calendar_id: schedule.inner.calendar_id,
                stub: schedule.inner.stub,
                end_of_month: schedule.inner.end_of_month,
                payment_lag_days: schedule.inner.payment_lag_days,
            },
        }
    }
}

/// Floating coupon parameters and spec.
#[pyclass(
    module = "finstack.valuations.cashflow.builder",
    name = "FloatCouponParams",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyFloatCouponParams {
    pub(crate) inner: val_builder::FloatCouponParams,
}

#[pymethods]
impl PyFloatCouponParams {
    #[classmethod]
    #[pyo3(
        signature = (index_id, margin_bp, *, gearing=None, reset_lag_days=None),
        text_signature = "(cls, index_id, margin_bp, *, gearing=1.0, reset_lag_days=2)"
    )]
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
                margin_bp: rust_decimal::Decimal::from_f64_retain(margin_bp).unwrap_or_default(),
                gearing: rust_decimal::Decimal::from_f64_retain(gearing.unwrap_or(1.0))
                    .unwrap_or(rust_decimal::Decimal::ONE),
                reset_lag_days: reset_lag_days.unwrap_or(2),
            },
        }
    }
}

#[pyclass(
    module = "finstack.valuations.cashflow.builder",
    name = "FloatingCouponSpec",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyFloatingCouponSpec {
    pub(crate) inner: val_builder::FloatingCouponSpec,
}

#[pymethods]
impl PyFloatingCouponSpec {
    #[classmethod]
    #[pyo3(
        signature = (params, schedule, coupon_type=None),
        text_signature = "(cls, params, schedule, coupon_type=None)"
    )]
    /// Create from simplified FloatCouponParams (no caps/floors/compounding).
    fn new(
        _cls: &Bound<'_, PyType>,
        params: PyFloatCouponParams,
        schedule: PyScheduleParams,
        coupon_type: Option<PyCouponType>,
    ) -> Self {
        let calendar_id = schedule.inner.calendar_id.clone();
        Self {
            inner: val_builder::FloatingCouponSpec {
                rate_spec: val_builder::FloatingRateSpec {
                    index_id: params.inner.index_id.clone(),
                    spread_bp: params.inner.margin_bp,
                    gearing: params.inner.gearing,
                    gearing_includes_spread: true,
                    floor_bp: None,
                    all_in_floor_bp: None,
                    cap_bp: None,
                    index_cap_bp: None,
                    reset_freq: schedule.inner.freq,
                    reset_lag_days: params.inner.reset_lag_days,
                    dc: schedule.inner.dc,
                    bdc: schedule.inner.bdc,
                    calendar_id: calendar_id.clone(),
                    fixing_calendar_id: Some(calendar_id),
                    end_of_month: schedule.inner.end_of_month,
                    overnight_compounding: None,
                    payment_lag_days: schedule.inner.payment_lag_days,
                },
                coupon_type: coupon_type
                    .map(|c| c.inner)
                    .unwrap_or(val_builder::CouponType::Cash),
                freq: schedule.inner.freq,
                stub: schedule.inner.stub,
            },
        }
    }

    #[classmethod]
    #[pyo3(
        signature = (rate_spec, schedule, coupon_type=None),
        text_signature = "(cls, rate_spec, schedule, coupon_type=None)"
    )]
    /// Create from full FloatingRateSpec (with caps, floors, compounding).
    fn from_rate_spec(
        _cls: &Bound<'_, PyType>,
        rate_spec: super::specs::PyFloatingRateSpec,
        schedule: PyScheduleParams,
        coupon_type: Option<PyCouponType>,
    ) -> Self {
        Self {
            inner: val_builder::FloatingCouponSpec {
                rate_spec: rate_spec.inner,
                coupon_type: coupon_type
                    .map(|c| c.inner)
                    .unwrap_or(val_builder::CouponType::Cash),
                freq: schedule.inner.freq,
                stub: schedule.inner.stub,
            },
        }
    }
}

/// Python wrapper for the composable valuations CashFlowBuilder.
#[pyclass(
    module = "finstack.valuations.cashflow.builder",
    name = "CashFlowBuilder",
    unsendable
)]
pub struct PyCashFlowBuilder {
    inner: val_builder::CashFlowBuilder,
}

#[pymethods]
impl PyCashFlowBuilder {
    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    fn new(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: val_builder::CashFlowBuilder::default(),
        }
    }

    #[pyo3(text_signature = "(self, amount, currency, issue, maturity)")]
    fn principal(
        &mut self,
        amount: f64,
        currency: &crate::core::currency::PyCurrency,
        issue: Bound<'_, PyAny>,
        maturity: Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        let issue_date = py_to_date(&issue).context("issue")?;
        let maturity_date = py_to_date(&maturity).context("maturity")?;
        let money = Money::new(amount, currency.inner);
        let _ = self.inner.principal(money, issue_date, maturity_date);
        Ok(Self {
            inner: self.inner.clone(),
        })
    }

    #[pyo3(text_signature = "(self, amortization)")]
    fn amortization(&mut self, amortization: Option<super::specs::PyAmortizationSpec>) -> Self {
        if let Some(spec) = amortization {
            let _ = self.inner.amortization(spec.inner);
        }
        Self {
            inner: self.inner.clone(),
        }
    }

    #[pyo3(text_signature = "(self, spec)")]
    fn fixed_cf(&mut self, spec: &PyFixedCouponSpec) -> Self {
        let _ = self.inner.fixed_cf(spec.inner.clone());
        Self {
            inner: self.inner.clone(),
        }
    }

    #[pyo3(text_signature = "(self, spec)")]
    fn floating_cf(&mut self, spec: PyFloatingCouponSpec) -> Self {
        let _ = self.inner.floating_cf(spec.inner);
        Self {
            inner: self.inner.clone(),
        }
    }

    #[pyo3(text_signature = "(self, steps, schedule, default_split)")]
    /// Fixed step-up program with boundaries `steps=[(end_date, rate), ...]`.
    fn fixed_stepup(
        &mut self,
        steps: Vec<(Bound<'_, PyAny>, f64)>,
        schedule: &PyScheduleParams,
        default_split: PyCouponType,
    ) -> PyResult<Self> {
        let mut rust_steps: Vec<(time::Date, f64)> = Vec::with_capacity(steps.len());
        for (d, r) in steps {
            rust_steps.push((py_to_date(&d).context("steps[].date")?, r));
        }
        let _ = self
            .inner
            .fixed_stepup(&rust_steps, schedule.inner.clone(), default_split.inner);
        Ok(Self {
            inner: self.inner.clone(),
        })
    }

    #[pyo3(text_signature = "(self, steps)")]
    /// Payment split program `(end_date, split)` where `split` is CouponType.
    fn payment_split_program(
        &mut self,
        steps: Vec<(Bound<'_, PyAny>, PyCouponType)>,
    ) -> PyResult<Self> {
        let mut rust_steps: Vec<(time::Date, val_builder::CouponType)> =
            Vec::with_capacity(steps.len());
        for (d, split) in steps {
            rust_steps.push((py_to_date(&d).context("steps[].date")?, split.inner));
        }
        let _ = self.inner.payment_split_program(&rust_steps);
        Ok(Self {
            inner: self.inner.clone(),
        })
    }

    #[pyo3(text_signature = "(self, spec)")]
    /// Add a fee specification to the schedule.
    fn fee(&mut self, spec: &PyFeeSpec) -> Self {
        let _ = self.inner.fee(spec.inner.clone());
        Self {
            inner: self.inner.clone(),
        }
    }

    #[pyo3(text_signature = "(self, events)")]
    /// Add custom principal events (draws/repays) to the schedule.
    fn principal_events(&mut self, events: Vec<PyRef<super::specs::PyPrincipalEvent>>) -> Self {
        let rust_events: Vec<val_builder::PrincipalEvent> =
            events.iter().map(|e| e.inner.clone()).collect();
        let _ = self.inner.principal_events(&rust_events);
        Self {
            inner: self.inner.clone(),
        }
    }

    #[pyo3(text_signature = "(self, date, delta, cash, kind)")]
    /// Add a single principal event (draw or repay).
    fn add_principal_event(
        &mut self,
        date: Bound<'_, PyAny>,
        delta: Bound<'_, PyAny>,
        cash: Bound<'_, PyAny>,
        kind: &crate::core::cashflow::primitives::PyCFKind,
    ) -> PyResult<Self> {
        use crate::core::money::extract_money;
        let d = py_to_date(&date).context("date")?;
        let delta_money = extract_money(&delta).context("delta")?;
        let cash_money = extract_money(&cash).context("cash")?;
        let _ = self
            .inner
            .add_principal_event(d, delta_money, Some(cash_money), kind.inner);
        Ok(Self {
            inner: self.inner.clone(),
        })
    }

    #[pyo3(text_signature = "(self, start, end, rate, schedule, split)")]
    /// Add a fixed coupon window with explicit start/end dates.
    fn add_fixed_coupon_window(
        &mut self,
        start: Bound<'_, PyAny>,
        end: Bound<'_, PyAny>,
        rate: f64,
        schedule: &PyScheduleParams,
        split: PyCouponType,
    ) -> PyResult<Self> {
        let s = py_to_date(&start).context("start")?;
        let e = py_to_date(&end).context("end")?;
        let _ = self
            .inner
            .add_fixed_coupon_window(s, e, rate, schedule.inner.clone(), split.inner);
        Ok(Self {
            inner: self.inner.clone(),
        })
    }

    #[pyo3(text_signature = "(self, start, end, params, schedule, split)")]
    /// Add a floating coupon window with explicit start/end dates.
    fn add_float_coupon_window(
        &mut self,
        start: Bound<'_, PyAny>,
        end: Bound<'_, PyAny>,
        params: PyFloatCouponParams,
        schedule: &PyScheduleParams,
        split: PyCouponType,
    ) -> PyResult<Self> {
        let s = py_to_date(&start).context("start")?;
        let e = py_to_date(&end).context("end")?;
        let _ = self.inner.add_float_coupon_window(
            s,
            e,
            params.inner,
            schedule.inner.clone(),
            split.inner,
        );
        Ok(Self {
            inner: self.inner.clone(),
        })
    }

    #[pyo3(text_signature = "(self, start, end, split)")]
    /// Add a payment window (PIK toggle) with explicit start/end dates.
    fn add_payment_window(
        &mut self,
        start: Bound<'_, PyAny>,
        end: Bound<'_, PyAny>,
        split: PyCouponType,
    ) -> PyResult<Self> {
        let s = py_to_date(&start).context("start")?;
        let e = py_to_date(&end).context("end")?;
        let _ = self.inner.add_payment_window(s, e, split.inner);
        Ok(Self {
            inner: self.inner.clone(),
        })
    }

    #[pyo3(text_signature = "(self, steps, base_params, schedule, default_split)")]
    /// Floating margin step-up program with boundaries `steps=[(end_date, margin_bp), ...]`.
    fn float_margin_stepup(
        &mut self,
        steps: Vec<(Bound<'_, PyAny>, f64)>,
        base_params: PyFloatCouponParams,
        schedule: &PyScheduleParams,
        default_split: PyCouponType,
    ) -> PyResult<Self> {
        let mut rust_steps: Vec<(time::Date, f64)> = Vec::with_capacity(steps.len());
        for (d, margin) in steps {
            rust_steps.push((py_to_date(&d).context("steps[].date")?, margin));
        }
        let _ = self.inner.float_margin_stepup(
            &rust_steps,
            base_params.inner,
            schedule.inner.clone(),
            default_split.inner,
        );
        Ok(Self {
            inner: self.inner.clone(),
        })
    }

    #[pyo3(text_signature = "(self, switch, fixed_win, float_win, default_split)")]
    /// Fixed-to-float switch at a given date.
    fn fixed_to_float(
        &mut self,
        switch: Bound<'_, PyAny>,
        fixed_win: &PyFixedWindow,
        float_win: &PyFloatWindow,
        default_split: PyCouponType,
    ) -> PyResult<Self> {
        let switch_date = py_to_date(&switch).context("switch")?;
        let _ = self.inner.fixed_to_float(
            switch_date,
            fixed_win.inner.clone(),
            float_win.inner.clone(),
            default_split.inner,
        );
        Ok(Self {
            inner: self.inner.clone(),
        })
    }

    #[pyo3(text_signature = "(self, market=None)")]
    /// Build the cashflow schedule with optional market curves for floating rate computation.
    ///
    /// When a market context is provided, floating rate coupons include the forward rate
    /// from the curve: `coupon = outstanding * (forward_rate * gearing + margin_bp * 1e-4) * year_fraction`
    ///
    /// Without curves (or using `build_with_curves(None)`), only the margin is used:
    /// `coupon = outstanding * (margin_bp * 1e-4 * gearing) * year_fraction`
    fn build_with_curves(
        &self,
        market: Option<PyRef<PyMarketContext>>,
    ) -> PyResult<PyCashFlowSchedule> {
        let schedule = match market {
            Some(market) => self.inner.build_with_curves(Some(&market.inner)),
            None => self.inner.build_with_curves(None),
        };
        schedule.map(PyCashFlowSchedule::new).map_err(core_to_py)
    }
}

/// CashflowSchedule wrapper exposing holder-side flows and metadata.
#[pyclass(
    module = "finstack.valuations.cashflow.builder",
    name = "CashFlowSchedule",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub struct PyCashFlowSchedule {
    inner: val_builder::CashFlowSchedule,
}

impl PyCashFlowSchedule {
    pub(crate) fn new(inner: val_builder::CashFlowSchedule) -> Self {
        Self { inner }
    }
    pub(crate) fn inner_clone(&self) -> val_builder::CashFlowSchedule {
        self.inner.clone()
    }
}

// Internal helpers to reduce duplication across methods
fn extract_periods(periods: &Bound<'_, PyAny>) -> PyResult<Vec<Period>> {
    if let Ok(plan) = periods.extract::<PyRef<PyPeriodPlan>>() {
        Ok(plan.periods.clone())
    } else if let Ok(periods_list) = periods.extract::<Vec<PyRef<PyPeriod>>>() {
        Ok(periods_list.iter().map(|p| p.inner.clone()).collect())
    } else {
        Err(pyo3::exceptions::PyTypeError::new_err(
            "periods must be PeriodPlan or list[Period]",
        ))
    }
}

fn extract_day_count(
    day_count: Option<Bound<'_, PyAny>>,
    default_dc: finstack_core::dates::DayCount,
) -> PyResult<finstack_core::dates::DayCount> {
    use crate::core::dates::daycount::PyDayCount;
    if let Some(dc_obj) = day_count {
        if let Ok(py_dc) = dc_obj.extract::<PyRef<PyDayCount>>() {
            Ok(py_dc.inner)
        } else {
            Err(pyo3::exceptions::PyTypeError::new_err(
                "day_count must be DayCount",
            ))
        }
    } else {
        Ok(default_dc)
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
    fn flows(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let items: Vec<crate::core::cashflow::primitives::PyCashFlow> = self
            .inner
            .flows
            .iter()
            .copied()
            .map(crate::core::cashflow::primitives::PyCashFlow::new)
            .collect();
        Ok(PyList::new(py, items)?.into())
    }

    #[pyo3(text_signature = "(self)")]
    /// Return the unique payment dates from the schedule.
    fn dates(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dates: Vec<Py<PyAny>> = self
            .inner
            .dates()
            .into_iter()
            .map(|d| {
                pyo3::types::PyDate::new(py, d.year(), d.month() as u8, d.day()).map(|dt| dt.into())
            })
            .collect::<PyResult<_>>()?;
        Ok(PyList::new(py, dates)?.into())
    }

    #[pyo3(text_signature = "(self)")]
    /// Return only the coupon cashflows from the schedule.
    fn coupons(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let items: Vec<crate::core::cashflow::primitives::PyCashFlow> = self
            .inner
            .coupons()
            .copied()
            .map(crate::core::cashflow::primitives::PyCashFlow::new)
            .collect();
        Ok(PyList::new(py, items)?.into())
    }

    #[pyo3(text_signature = "(self)")]
    /// Return the outstanding balance path per cashflow as (date, Money) pairs.
    fn outstanding_path_per_flow(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let path = self.inner.outstanding_path_per_flow().map_err(core_to_py)?;
        let items: Vec<(Py<PyAny>, PyMoney)> = path
            .into_iter()
            .map(|(d, m)| {
                let date = pyo3::types::PyDate::new(py, d.year(), d.month() as u8, d.day())?;
                Ok((date.into(), PyMoney::new(m)))
            })
            .collect::<PyResult<_>>()?;
        Ok(PyList::new(py, items)?.into())
    }

    #[pyo3(text_signature = "(self)")]
    /// Return the outstanding balance by date as (date, Money) pairs.
    fn outstanding_by_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let path = self.inner.outstanding_by_date().map_err(core_to_py)?;
        let items: Vec<(Py<PyAny>, PyMoney)> = path
            .into_iter()
            .map(|(d, m)| {
                let date = pyo3::types::PyDate::new(py, d.year(), d.month() as u8, d.day())?;
                Ok((date.into(), PyMoney::new(m)))
            })
            .collect::<PyResult<_>>()?;
        Ok(PyList::new(py, items)?.into())
    }

    #[pyo3(
        signature = (market_or_curve, *, discount_curve_id=None, as_of=None, day_count=None),
        text_signature = "(market_or_curve, /, *, discount_curve_id=None, as_of=None, day_count=None)"
    )]
    /// Compute the net present value of all cashflows.
    ///
    /// Args:
    ///     market_or_curve: MarketContext or DiscountCurve
    ///     discount_curve_id: Curve ID (required when using MarketContext)
    ///     as_of: Valuation date (defaults to curve base_date)
    ///     day_count: Day count convention (defaults to schedule day_count)
    fn npv(
        &self,
        market_or_curve: Bound<'_, PyAny>,
        discount_curve_id: Option<&str>,
        as_of: Option<Bound<'_, PyAny>>,
        day_count: Option<Bound<'_, PyAny>>,
    ) -> PyResult<f64> {
        use finstack_core::cashflow::Discountable;
        let dc = extract_day_count(day_count, self.inner.day_count)?;

        if let Ok(market) = market_or_curve.extract::<PyRef<PyMarketContext>>() {
            let disc_id = discount_curve_id.ok_or_else(|| {
                pyo3::exceptions::PyValueError::new_err(
                    "discount_curve_id required when using MarketContext",
                )
            })?;
            let disc_arc = market.inner.get_discount(disc_id).map_err(core_to_py)?;
            let base = if let Some(as_of_obj) = as_of {
                py_to_date(&as_of_obj)?
            } else {
                disc_arc.base_date()
            };
            let result = self
                .inner
                .npv(disc_arc.as_ref(), base, Some(dc))
                .map_err(core_to_py)?;
            Ok(result.amount())
        } else if let Ok(disc_curve) = market_or_curve.extract::<PyRef<PyDiscountCurve>>() {
            let base = if let Some(as_of_obj) = as_of {
                py_to_date(&as_of_obj)?
            } else {
                disc_curve.inner.base_date()
            };
            let result = self
                .inner
                .npv(disc_curve.inner.as_ref(), base, Some(dc))
                .map_err(core_to_py)?;
            Ok(result.amount())
        } else {
            Err(pyo3::exceptions::PyTypeError::new_err(
                "market_or_curve must be MarketContext or DiscountCurve",
            ))
        }
    }

    #[pyo3(
        signature = (*, market, discount_curve_id, as_of=None, credit_curve_id=None, forward_curve_id=None, include_floating_decomposition=false, day_count=None, discount_day_count=None, facility_limit=None),
        text_signature = "(*, market, discount_curve_id, as_of=None, credit_curve_id=None, forward_curve_id=None, include_floating_decomposition=False, day_count=None, discount_day_count=None, facility_limit=None)"
    )]
    #[allow(clippy::too_many_arguments)]
    /// Convert the cashflow schedule to a Polars DataFrame.
    ///
    /// Args:
    ///     market: MarketContext for discount factor and PV calculations
    ///     discount_curve_id: Curve ID for discounting
    ///     as_of: Valuation date (defaults to curve base_date)
    ///     credit_curve_id: Optional credit/hazard curve ID
    ///     forward_curve_id: Optional forward curve ID for floating rate decomposition
    ///     include_floating_decomposition: Include base_rate/spread columns (default: False)
    ///     day_count: Optional day count for year fraction calculations
    ///     discount_day_count: Optional day count override for discounting time
    ///     facility_limit: Optional facility limit for undrawn balance calculations
    ///
    /// Returns:
    ///     PyDataFrame: Polars DataFrame with cashflow data
    fn to_dataframe(
        &self,
        market: PyRef<PyMarketContext>,
        discount_curve_id: &str,
        as_of: Option<Bound<'_, PyAny>>,
        credit_curve_id: Option<&str>,
        forward_curve_id: Option<&str>,
        include_floating_decomposition: bool,
        day_count: Option<Bound<'_, PyAny>>,
        discount_day_count: Option<Bound<'_, PyAny>>,
        facility_limit: Option<Bound<'_, PyAny>>,
    ) -> PyResult<pyo3_polars::PyDataFrame> {
        let as_of_date = as_of.map(|d| py_to_date(&d)).transpose()?;
        let dc = day_count
            .map(|d| extract_day_count(Some(d), self.inner.day_count))
            .transpose()?;
        let disc_dc = discount_day_count
            .map(|d| extract_day_count(Some(d), self.inner.day_count))
            .transpose()?;
        let fac_limit = facility_limit
            .map(|f| crate::core::money::extract_money(&f))
            .transpose()?;

        let options = val_builder::PeriodDataFrameOptions {
            credit_curve_id,
            forward_curve_id,
            as_of: as_of_date,
            day_count: dc,
            discount_day_count: disc_dc,
            facility_limit: fac_limit,
            include_floating_decomposition,
            frequency: None,
            calendar_id: None,
        };

        let periods: Vec<Period> = if let (Some(first), Some(last)) =
            (self.inner.flows.first(), self.inner.flows.last())
        {
            vec![Period {
                id: PeriodId::annual(first.date.year()),
                start: first.date,
                end: last.date,
                is_actual: true,
            }]
        } else {
            Vec::new()
        };

        let frame = self
            .inner
            .to_period_dataframe(&periods, &market.inner, discount_curve_id, options)
            .map_err(core_to_py)?;

        super::dataframe::period_dataframe_to_polars(frame)
    }

    /// Compute present values aggregated by period.
    ///
    /// Groups cashflows by period and computes the present value of each cashflow
    /// discounted back to the base date. Returns a dictionary mapping period codes
    /// to PV values (for single-currency schedules).
    ///
    /// This method is kept for compatibility with period-based analysis workflows.
    /// For full cashflow details, use `to_dataframe()` and group/aggregate in Polars/pandas.
    ///
    /// Args:
    ///     periods: List of Period objects or PeriodPlan
    ///     market_or_curve: Either MarketContext (when using curve IDs) or DiscountCurve (backwards compat)
    ///     discount_curve_id: Optional curve ID when using MarketContext
    ///     hazard_curve_id: Optional hazard curve ID for credit adjustment
    ///     as_of: Optional valuation date (defaults to curve base_date)
    ///     day_count: Optional day count convention (defaults to schedule.day_count)
    ///
    /// Returns:
    ///     dict[str, float]: Mapping from PeriodId.code to PV value
    #[pyo3(
        signature = (periods, market_or_curve, *, discount_curve_id=None, hazard_curve_id=None, as_of=None, day_count=None),
        text_signature = "(periods, market_or_curve, /, *, discount_curve_id=None, hazard_curve_id=None, as_of=None, day_count=None)"
    )]
    #[allow(clippy::too_many_arguments)]
    pub fn per_period_pv(
        &self,
        _py: Python<'_>,
        periods: Bound<'_, PyAny>,
        market_or_curve: Bound<'_, PyAny>,
        discount_curve_id: Option<&str>,
        hazard_curve_id: Option<&str>,
        as_of: Option<Bound<'_, PyAny>>,
        day_count: Option<Bound<'_, PyAny>>,
    ) -> PyResult<HashMap<String, f64>> {
        // Extract periods
        let periods_vec: Vec<Period> = extract_periods(&periods)?;

        // Determine day count
        let dc = extract_day_count(day_count, self.inner.day_count)?;

        // Handle MarketContext vs DiscountCurve
        let pv_map = if let Ok(market) = market_or_curve.extract::<PyRef<PyMarketContext>>() {
            // Using MarketContext
            let disc_id = discount_curve_id.ok_or_else(|| {
                pyo3::exceptions::PyValueError::new_err(
                    "discount_curve_id required when using MarketContext",
                )
            })?;
            let disc_curve_id = CurveId::from(disc_id);
            let hazard_curve_id_opt = hazard_curve_id.map(CurveId::from);

            let base = if let Some(as_of_obj) = as_of {
                py_to_date(&as_of_obj)?
            } else {
                // Get base date from discount curve
                let disc_arc = market.inner.get_discount(disc_id).map_err(core_to_py)?;
                disc_arc.base_date()
            };

            let pv_result = self
                .inner
                .pv_by_period_with_market_and_ctx(
                    &periods_vec,
                    &market.inner,
                    &disc_curve_id,
                    hazard_curve_id_opt.as_ref(),
                    base,
                    dc,
                    finstack_core::dates::DayCountCtx::default(),
                )
                .map_err(core_to_py)?;
            pv_result
        } else if let Ok(disc_curve) = market_or_curve.extract::<PyRef<PyDiscountCurve>>() {
            // Using DiscountCurve directly (backwards compat, no hazard support)
            if hazard_curve_id.is_some() {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "hazard_curve_id not supported when using DiscountCurve directly; use MarketContext",
                ));
            }

            let base = if let Some(as_of_obj) = as_of {
                py_to_date(&as_of_obj)?
            } else {
                disc_curve.inner.base_date()
            };

            let pv_map = self
                .inner
                .pv_by_period_with_ctx(
                    &periods_vec,
                    disc_curve.inner.as_ref(),
                    base,
                    dc,
                    finstack_core::dates::DayCountCtx::default(),
                )
                .map_err(core_to_py)?;
            pv_map
        } else {
            return Err(pyo3::exceptions::PyTypeError::new_err(
                "market_or_curve must be MarketContext or DiscountCurve",
            ));
        };

        // Convert to Python dict: PeriodId.code -> PV (single currency)
        let mut result = HashMap::default();
        for (period_id, currency_map) in pv_map {
            // For single-currency schedules, take the first (and only) currency
            if let Some((_currency, pv_money)) = currency_map.first() {
                result.insert(period_id.to_string(), pv_money.amount());
            }
        }

        Ok(result)
    }

    /// Return the number of cashflows in the schedule.
    fn __len__(&self) -> usize {
        self.inner.flows.len()
    }

    /// Access a cashflow by index (supports negative indices).
    fn __getitem__(&self, index: isize) -> PyResult<crate::core::cashflow::primitives::PyCashFlow> {
        let len = self.inner.flows.len() as isize;
        let actual = if index < 0 { len + index } else { index };
        if actual < 0 || actual >= len {
            return Err(pyo3::exceptions::PyIndexError::new_err(
                "index out of range",
            ));
        }
        Ok(crate::core::cashflow::primitives::PyCashFlow::new(
            self.inner.flows[actual as usize],
        ))
    }

    /// Return an iterator over the cashflows in the schedule.
    fn __iter__(slf: PyRef<'_, Self>) -> PyResult<Py<PyCashFlowScheduleIterator>> {
        let flows: Vec<finstack_core::cashflow::CashFlow> = slf.inner.flows.clone();
        Py::new(slf.py(), PyCashFlowScheduleIterator { flows, index: 0 })
    }
}

/// Fee base for periodic basis point fees.
///
/// Determines what balance is used to calculate periodic fees.
///
/// Examples:
///     >>> # Fee on drawn balance
///     >>> fee_base = FeeBase.drawn()
///
///     >>> # Fee on undrawn (unused) facility
///     >>> from finstack.core import Money
///     >>> fee_base = FeeBase.undrawn(Money("USD", 10_000_000))
#[pyclass(
    module = "finstack.valuations.cashflow.builder",
    name = "FeeBase",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyFeeBase {
    pub(crate) inner: finstack_valuations::cashflow::builder::FeeBase,
}

impl PyFeeBase {
    pub(crate) fn new(inner: finstack_valuations::cashflow::builder::FeeBase) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyFeeBase {
    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    /// Fee calculated on drawn (outstanding) balance.
    ///
    /// Returns:
    ///     FeeBase: Drawn balance base
    fn drawn(_cls: &Bound<'_, PyType>) -> Self {
        Self::new(finstack_valuations::cashflow::builder::FeeBase::Drawn)
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, facility_limit)")]
    /// Fee calculated on undrawn (unused) facility.
    ///
    /// Args:
    ///     facility_limit: Total facility size as Money
    ///
    /// Returns:
    ///     FeeBase: Undrawn balance base (facility_limit - outstanding)
    fn undrawn(_cls: &Bound<'_, PyType>, facility_limit: Bound<'_, PyAny>) -> PyResult<Self> {
        use crate::core::money::extract_money;
        let limit = extract_money(&facility_limit)?;
        Ok(Self::new(
            finstack_valuations::cashflow::builder::FeeBase::Undrawn {
                facility_limit: limit,
            },
        ))
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            finstack_valuations::cashflow::builder::FeeBase::Drawn => "FeeBase.drawn()".to_string(),
            finstack_valuations::cashflow::builder::FeeBase::Undrawn { facility_limit } => {
                format!("FeeBase.undrawn({})", facility_limit)
            }
        }
    }
}

/// Fee specification for cashflow schedules.
///
/// Supports both fixed one-time fees and periodic fees calculated as
/// basis points on drawn or undrawn balances.
///
/// Examples:
///     >>> from finstack.core import Money
///     >>> import datetime
///     >>>
///     >>> # One-time fixed fee
///     >>> fee = FeeSpec.fixed(
///     ...     datetime.date(2025, 6, 15),
///     ...     Money("USD", 50_000)
///     ... )
///
///     >>> # Periodic commitment fee on undrawn balance
///     >>> from finstack.valuations.cashflow.builder import FeeBase, ScheduleParams
///     >>> fee = FeeSpec.periodic_bps(
///     ...     FeeBase.undrawn(Money("USD", 10_000_000)),
///     ...     25.0,  # 25 bps
///     ...     ScheduleParams.quarterly_act360()
///     ... )
#[pyclass(
    module = "finstack.valuations.cashflow.builder",
    name = "FeeSpec",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyFeeSpec {
    pub(crate) inner: finstack_valuations::cashflow::builder::FeeSpec,
}

impl PyFeeSpec {
    pub(crate) fn new(inner: finstack_valuations::cashflow::builder::FeeSpec) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyFeeSpec {
    #[classmethod]
    #[pyo3(text_signature = "(cls, date, amount)")]
    /// Create a fixed one-time fee.
    ///
    /// Args:
    ///     date: Payment date
    ///     amount: Fee amount as Money
    ///
    /// Returns:
    ///     FeeSpec: Fixed fee specification
    fn fixed(
        _cls: &Bound<'_, PyType>,
        date: Bound<'_, PyAny>,
        amount: Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        use crate::core::dates::utils::py_to_date;
        use crate::core::money::extract_money;

        let payment_date = py_to_date(&date)?;
        let fee_amount = extract_money(&amount)?;

        Ok(Self::new(
            finstack_valuations::cashflow::builder::FeeSpec::Fixed {
                date: payment_date,
                amount: fee_amount,
            },
        ))
    }

    #[classmethod]
    #[pyo3(
        signature = (base, bps, schedule, *, calendar=None, stub=None),
        text_signature = "(cls, base, bps, schedule, /, *, calendar=None, stub='none')"
    )]
    /// Create a periodic fee calculated as basis points on a balance.
    ///
    /// Args:
    ///     base: Fee base (drawn or undrawn balance)
    ///     bps: Fee rate in basis points (e.g., 25.0 for 0.25%)
    ///     schedule: Schedule parameters (frequency, day count, BDC)
    ///     calendar: Optional calendar identifier (defaults to "weekends_only")
    ///     stub: Optional stub kind (default: "none")
    ///
    /// Returns:
    ///     FeeSpec: Periodic fee specification
    fn periodic_bps(
        _cls: &Bound<'_, PyType>,
        base: &PyFeeBase,
        bps: f64,
        schedule: &PyScheduleParams,
        calendar: Option<&str>,
        stub: Option<&str>,
    ) -> PyResult<Self> {
        use finstack_core::dates::StubKind;

        let stub_kind = if let Some(s) = stub {
            s.parse::<StubKind>()
                .map_err(|e: String| pyo3::exceptions::PyValueError::new_err(e))?
        } else {
            StubKind::None
        };

        Ok(Self::new(
            finstack_valuations::cashflow::builder::FeeSpec::PeriodicBps {
                base: base.inner.clone(),
                bps: rust_decimal::Decimal::from_f64_retain(bps).unwrap_or_default(),
                freq: schedule.inner.freq,
                dc: schedule.inner.dc,
                bdc: schedule.inner.bdc,
                calendar_id: calendar
                    .unwrap_or(finstack_valuations::cashflow::builder::calendar::WEEKENDS_ONLY_ID)
                    .to_string(),
                stub: stub_kind,
            },
        ))
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            finstack_valuations::cashflow::builder::FeeSpec::Fixed { date, amount } => {
                format!("FeeSpec.fixed({}, {})", date, amount)
            }
            finstack_valuations::cashflow::builder::FeeSpec::PeriodicBps { bps, freq, .. } => {
                format!("FeeSpec.periodic_bps(bps={:.2}, freq={:?})", bps, freq)
            }
        }
    }
}

/// Fixed coupon window for rate step-up programs.
///
/// Defines a period with a specific fixed rate and schedule.
///
/// Examples:
///     >>> window = FixedWindow(
///     ...     rate=0.05,
///     ...     schedule=ScheduleParams.quarterly_act360()
///     ... )
#[pyclass(
    module = "finstack.valuations.cashflow.builder",
    name = "FixedWindow",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyFixedWindow {
    pub(crate) inner: finstack_valuations::cashflow::builder::FixedWindow,
}

impl PyFixedWindow {
    pub(crate) fn new(inner: finstack_valuations::cashflow::builder::FixedWindow) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyFixedWindow {
    #[new]
    #[pyo3(text_signature = "(rate, schedule)")]
    /// Create a fixed coupon window.
    ///
    /// Args:
    ///     rate: Fixed coupon rate (annual decimal)
    ///     schedule: Schedule parameters defining frequency and conventions
    ///
    /// Returns:
    ///     FixedWindow: Window specification
    fn ctor(rate: f64, schedule: &PyScheduleParams) -> Self {
        Self::new(finstack_valuations::cashflow::builder::FixedWindow {
            rate: rust_decimal::Decimal::from_f64_retain(rate).unwrap_or_default(),
            schedule: schedule.inner.clone(),
        })
    }

    #[getter]
    fn rate(&self) -> f64 {
        use rust_decimal::prelude::ToPrimitive;
        self.inner.rate.to_f64().unwrap_or(0.0)
    }

    fn __repr__(&self) -> String {
        format!("FixedWindow(rate={:.4})", self.inner.rate)
    }
}

/// Floating coupon window for floating rate periods.
///
/// Defines a period with floating rate parameters and schedule.
///
/// Examples:
///     >>> params = FloatCouponParams.new("USD-SOFR", 50.0)
///     >>> window = FloatWindow(
///     ...     params=params,
///     ...     schedule=ScheduleParams.quarterly_act360()
///     ... )
#[pyclass(
    module = "finstack.valuations.cashflow.builder",
    name = "FloatWindow",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyFloatWindow {
    pub(crate) inner: finstack_valuations::cashflow::builder::FloatWindow,
}

impl PyFloatWindow {
    pub(crate) fn new(inner: finstack_valuations::cashflow::builder::FloatWindow) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyFloatWindow {
    #[new]
    #[pyo3(text_signature = "(params, schedule)")]
    /// Create a floating coupon window.
    ///
    /// Args:
    ///     params: Floating rate parameters (index, margin, gearing)
    ///     schedule: Schedule parameters defining frequency and conventions
    ///
    /// Returns:
    ///     FloatWindow: Window specification
    fn ctor(params: &PyFloatCouponParams, schedule: &PyScheduleParams) -> Self {
        Self::new(finstack_valuations::cashflow::builder::FloatWindow {
            params: params.inner.clone(),
            schedule: schedule.inner.clone(),
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "FloatWindow(index_id={}, margin_bp={:.2})",
            self.inner.params.index_id.as_str(),
            self.inner.params.margin_bp
        )
    }
}

/// Iterator over cashflows in a CashFlowSchedule.
#[pyclass(
    module = "finstack.valuations.cashflow.builder",
    name = "CashFlowScheduleIterator"
)]
pub struct PyCashFlowScheduleIterator {
    flows: Vec<finstack_core::cashflow::CashFlow>,
    index: usize,
}

#[pymethods]
impl PyCashFlowScheduleIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(
        mut slf: PyRefMut<'_, Self>,
    ) -> Option<crate::core::cashflow::primitives::PyCashFlow> {
        if slf.index < slf.flows.len() {
            let flow = slf.flows[slf.index];
            slf.index += 1;
            Some(crate::core::cashflow::primitives::PyCashFlow::new(flow))
        } else {
            None
        }
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
    module.add_class::<PyCashFlowBuilder>()?;
    module.add_class::<PyCashFlowSchedule>()?;
    module.add_class::<PyCashFlowScheduleIterator>()?;
    module.add_class::<PyFeeBase>()?;
    module.add_class::<PyFeeSpec>()?;
    module.add_class::<PyFixedWindow>()?;
    module.add_class::<PyFloatWindow>()?;
    Ok(vec![
        "CouponType",
        "ScheduleParams",
        "FixedCouponSpec",
        "FloatCouponParams",
        "FloatingCouponSpec",
        "CashFlowBuilder",
        "CashFlowSchedule",
        "CashFlowScheduleIterator",
        "FeeBase",
        "FeeSpec",
        "FixedWindow",
        "FloatWindow",
    ])
}
