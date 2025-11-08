use crate::core::dates::periods::{PyPeriod, PyPeriodPlan};
use crate::core::error::core_to_py;
use crate::core::market_data::context::PyMarketContext;
use crate::core::market_data::term_structures::PyDiscountCurve;
use crate::core::money::PyMoney;
use crate::core::utils::py_to_date;
use finstack_core::dates::Period;
use finstack_core::money::Money;
use finstack_core::types::CurveId;
use finstack_valuations::cashflow::builder as val_builder;
use finstack_valuations::cashflow::builder::schedule::PeriodDataFrameOptions;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyList, PyModule, PyType};
use pyo3::Bound;
use std::collections::HashMap;

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
#[derive(Clone, Debug)]
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
        Self {
            inner: val_builder::ScheduleParams {
                freq: freq.inner,
                dc: day_count.inner,
                bdc: bdc.inner,
                calendar_id: calendar_id.map(|s| s.to_string()),
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
    frozen
)]
#[derive(Clone, Debug)]
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
    fn fixed_cf(&mut self, spec: &PyFixedCouponSpec) -> Self {
        self.inner.fixed_cf(spec.inner.clone());
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
        schedule: &PyScheduleParams,
        default_split: PyCouponType,
    ) -> Self {
        let mut rust_steps: Vec<(time::Date, f64)> = Vec::with_capacity(steps.len());
        for (d, r) in steps {
            rust_steps.push((py_to_date(&d).expect("valid date"), r));
        }
        self.inner
            .fixed_stepup(&rust_steps, schedule.inner.clone(), default_split.inner);
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

    #[pyo3(text_signature = "(self, market)")]
    /// Build the cashflow schedule with market curves for floating rate computation.
    ///
    /// When a market context is provided, floating rate coupons include the forward rate
    /// from the curve: `coupon = outstanding * (forward_rate * gearing + margin_bp * 1e-4) * year_fraction`
    ///
    /// Without curves (or using `build()`), only the margin is used:
    /// `coupon = outstanding * (margin_bp * 1e-4 * gearing) * year_fraction`
    fn build_with_curves(
        &self,
        market: crate::core::market_data::PyMarketContext,
    ) -> PyResult<PyCashFlowSchedule> {
        self.inner
            .build_with_curves(Some(&market.inner))
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
    pub(crate) fn new(inner: val_builder::CashFlowSchedule) -> Self {
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

    #[pyo3(text_signature = "(self)")]
    /// Convert the schedule into a dict-of-arrays suitable for constructing a Polars DataFrame.
    ///
    /// Returns a Python dict with keys: "date", "kind", "amount", "accrual_factor",
    /// "reset_date", and "outstanding". Amounts and outstanding are numeric floats.
    fn to_dataframe(&self, py: Python<'_>) -> PyResult<PyObject> {
        let flows = &self.inner.flows;
        let mut dates: Vec<PyObject> = Vec::with_capacity(flows.len());
        let mut kinds: Vec<String> = Vec::with_capacity(flows.len());
        let mut amounts: Vec<f64> = Vec::with_capacity(flows.len());
        let mut accruals: Vec<f64> = Vec::with_capacity(flows.len());
        let mut reset_dates: Vec<Option<PyObject>> = Vec::with_capacity(flows.len());
        let mut outstanding_series: Vec<f64> = Vec::with_capacity(flows.len());

        let mut outstanding = self.inner.notional.initial.amount();

        for cf in flows.iter() {
            dates.push(crate::core::utils::date_to_py(py, cf.date)?);
            let kind_label = crate::core::cashflow::primitives::PyCFKind::new(cf.kind).name();
            kinds.push(kind_label.to_string());
            amounts.push(cf.amount.amount());
            accruals.push(cf.accrual_factor);
            let reset = match cf.reset_date {
                Some(d) => Some(crate::core::utils::date_to_py(py, d)?),
                None => None,
            };
            reset_dates.push(reset);

            // Outstanding convention: amortization reduces; PIK increases; notional exchange does not change
            if kind_label == "amortization" {
                outstanding -= cf.amount.amount();
            } else if kind_label == "pik" {
                outstanding += cf.amount.amount();
            }
            outstanding_series.push(outstanding);
        }

        let out = PyDict::new(py);
        out.set_item("date", PyList::new(py, dates)?)?;
        out.set_item("kind", PyList::new(py, kinds)?)?;
        out.set_item("amount", PyList::new(py, amounts)?)?;
        out.set_item("accrual_factor", PyList::new(py, accruals)?)?;
        out.set_item("reset_date", PyList::new(py, reset_dates)?)?;
        out.set_item("outstanding", PyList::new(py, outstanding_series)?)?;
        Ok(out.into())
    }

    /// Compute pre-period present values aggregated by period.
    ///
    /// Groups cashflows by period and computes the present value of each cashflow
    /// discounted back to the base date. Returns a dictionary mapping period codes
    /// to PV values (for single-currency schedules).
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
    pub(crate) fn per_period_pv(
        &self,
        _py: Python<'_>,
        periods: Bound<'_, PyAny>,
        market_or_curve: Bound<'_, PyAny>,
        discount_curve_id: Option<&str>,
        hazard_curve_id: Option<&str>,
        as_of: Option<Bound<'_, PyAny>>,
        day_count: Option<Bound<'_, PyAny>>,
    ) -> PyResult<HashMap<String, f64>> {
        use crate::core::dates::daycount::PyDayCount;

        // Extract periods
        let periods_vec: Vec<Period> = if let Ok(plan) = periods.extract::<PyRef<PyPeriodPlan>>() {
            plan.periods.clone()
        } else if let Ok(periods_list) = periods.extract::<Vec<PyRef<PyPeriod>>>() {
            periods_list.iter().map(|p| p.inner.clone()).collect()
        } else {
            return Err(pyo3::exceptions::PyTypeError::new_err(
                "periods must be PeriodPlan or list[Period]",
            ));
        };

        // Determine day count
        let dc = if let Some(dc_obj) = day_count {
            if let Ok(py_dc) = dc_obj.extract::<PyRef<PyDayCount>>() {
                py_dc.inner
            } else {
                return Err(pyo3::exceptions::PyTypeError::new_err(
                    "day_count must be DayCount",
                ));
            }
        } else {
            self.inner.day_count
        };

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
                .pre_period_pv_with_market(
                    &periods_vec,
                    &market.inner,
                    &disc_curve_id,
                    hazard_curve_id_opt.as_ref(),
                    base,
                    dc,
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

            let pv_map =
                self.inner
                    .pre_period_pv(&periods_vec, disc_curve.inner.as_ref(), base, dc);
            pv_map
        } else {
            return Err(pyo3::exceptions::PyTypeError::new_err(
                "market_or_curve must be MarketContext or DiscountCurve",
            ));
        };

        // Convert to Python dict: PeriodId.code -> PV (single currency)
        let mut result = HashMap::new();
        for (period_id, currency_map) in pv_map {
            // For single-currency schedules, take the first (and only) currency
            if let Some((_currency, pv_money)) = currency_map.first() {
                result.insert(period_id.to_string(), pv_money.amount());
            }
        }

        Ok(result)
    }

    /// Convert cashflow schedule to a period-aligned DataFrame.
    ///
    /// Returns a dict-of-arrays suitable for pandas/Polars with columns:
    /// Start Date, End Date, PayDate, CFType, Currency, Notional, Rate, YrFraq,
    /// Days, Amount, DiscountFactor, SurvivalProb (optional), PV, Unfunded Amount (optional),
    /// Base Rate (optional), Spread (optional).
    ///
    /// Args:
    ///     periods: List of Period objects or PeriodPlan
    ///     market_or_curve: Either MarketContext (when using curve IDs) or DiscountCurve (backwards compat)
    ///     discount_curve_id: Optional curve ID when using MarketContext
    ///     hazard_curve_id: Optional hazard curve ID for credit adjustment (adds SurvivalProb column)
    ///     forward_curve_id: Optional forward curve ID for floating rate decomposition
    ///     as_of: Optional valuation date (defaults to curve base_date)
    ///     day_count: Optional day count convention (defaults to schedule.day_count)
    ///     facility_limit: Optional facility limit Money for Unfunded Amount column
    ///     include_floating_decomposition: If True, adds Base Rate and Spread columns for floating cashflows
    ///
    /// Returns:
    ///     dict: Dictionary with column names as keys and lists as values
    #[pyo3(
        signature = (periods, market_or_curve, *, discount_curve_id=None, hazard_curve_id=None, forward_curve_id=None, as_of=None, day_count=None, facility_limit=None, include_floating_decomposition=false),
        text_signature = "(periods, market_or_curve, /, *, discount_curve_id=None, hazard_curve_id=None, forward_curve_id=None, as_of=None, day_count=None, facility_limit=None, include_floating_decomposition=False)"
    )]
    pub(crate) fn to_period_dataframe(
        &self,
        py: Python<'_>,
        periods: Bound<'_, PyAny>,
        market_or_curve: Bound<'_, PyAny>,
        discount_curve_id: Option<&str>,
        hazard_curve_id: Option<&str>,
        forward_curve_id: Option<&str>,
        as_of: Option<Bound<'_, PyAny>>,
        day_count: Option<Bound<'_, PyAny>>,
        facility_limit: Option<Bound<'_, PyAny>>,
        include_floating_decomposition: bool,
    ) -> PyResult<PyObject> {
        use crate::core::cashflow::primitives::PyCFKind;
        use crate::core::dates::daycount::PyDayCount;

        // Extract periods
        let periods_vec: Vec<Period> = if let Ok(plan) = periods.extract::<PyRef<PyPeriodPlan>>() {
            plan.periods.clone()
        } else if let Ok(periods_list) = periods.extract::<Vec<PyRef<PyPeriod>>>() {
            periods_list.iter().map(|p| p.inner.clone()).collect()
        } else {
            return Err(pyo3::exceptions::PyTypeError::new_err(
                "periods must be PeriodPlan or list[Period]",
            ));
        };

        // Determine day count
        let dc = if let Some(dc_obj) = day_count {
            if let Ok(py_dc) = dc_obj.extract::<PyRef<PyDayCount>>() {
                py_dc.inner
            } else {
                return Err(pyo3::exceptions::PyTypeError::new_err(
                    "day_count must be DayCount",
                ));
            }
        } else {
            self.inner.day_count
        };

        // Extract facility_limit if provided
        let facility_limit_money = if let Some(limit_obj) = facility_limit {
            Some(crate::core::money::extract_money(&limit_obj)?)
        } else {
            None
        };

        // Build MarketContext and discount curve id
        let (market_ctx, disc_id_owned, as_of_date) =
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
                (Some(market.inner.clone()), Some(disc_id.to_string()), base)
            } else if let Ok(disc_curve) = market_or_curve.extract::<PyRef<PyDiscountCurve>>() {
                // Build a minimal MarketContext with just the discount curve
                use finstack_core::market_data::MarketContext as CoreMarketContext;
                let ctx = CoreMarketContext::new().insert_discount_arc(disc_curve.inner.clone());
                let disc_id = disc_curve.inner.id().to_string();
                let base = if let Some(as_of_obj) = as_of {
                    py_to_date(&as_of_obj)?
                } else {
                    disc_curve.inner.base_date()
                };
                (Some(ctx), Some(disc_id), base)
            } else {
                return Err(pyo3::exceptions::PyTypeError::new_err(
                    "market_or_curve must be MarketContext or DiscountCurve",
                ));
            };
        // Build frame in Rust valuations crate
        let market_ctx = market_ctx.expect("market context always present here");
        let disc_id = disc_id_owned.expect("discount curve id present");
        let frame = self
            .inner
            .to_period_dataframe(
                &periods_vec,
                &market_ctx,
                disc_id.as_str(),
                PeriodDataFrameOptions {
                    hazard_curve_id,
                    forward_curve_id,
                    as_of: Some(as_of_date),
                    day_count: Some(dc),
                    facility_limit: facility_limit_money,
                    include_floating_decomposition,
                },
            )
            .map_err(core_to_py)?;

        // Convert to Python dict
        let out = PyDict::new(py);
        // Dates
        let start_dates: Vec<PyObject> = frame
            .start_dates
            .iter()
            .map(|d| crate::core::utils::date_to_py(py, *d))
            .collect::<PyResult<Vec<_>>>()?;
        let end_dates: Vec<PyObject> = frame
            .end_dates
            .iter()
            .map(|d| crate::core::utils::date_to_py(py, *d))
            .collect::<PyResult<Vec<_>>>()?;
        let pay_dates: Vec<PyObject> = frame
            .pay_dates
            .iter()
            .map(|d| crate::core::utils::date_to_py(py, *d))
            .collect::<PyResult<Vec<_>>>()?;
        out.set_item("Start Date", PyList::new(py, start_dates)?)?;
        out.set_item("End Date", PyList::new(py, end_dates)?)?;
        out.set_item("PayDate", PyList::new(py, pay_dates)?)?;
        // CFType and Currency
        let cf_types: Vec<String> = frame
            .cf_types
            .iter()
            .map(|k| PyCFKind::new(*k).name().to_string())
            .collect();
        let currencies: Vec<String> = frame.currencies.iter().map(|c| c.to_string()).collect();
        out.set_item("CFType", PyList::new(py, cf_types)?)?;
        out.set_item("Currency", PyList::new(py, currencies)?)?;
        // Core numeric columns
        out.set_item("Notional", PyList::new(py, frame.notionals)?)?;
        out.set_item("YrFraq", PyList::new(py, frame.yr_fraqs)?)?;
        out.set_item("Days", PyList::new(py, frame.days)?)?;
        out.set_item("Amount", PyList::new(py, frame.amounts)?)?;
        out.set_item("DiscountFactor", PyList::new(py, frame.discount_factors)?)?;
        if let Some(sp) = frame.survival_probs {
            out.set_item("SurvivalProb", PyList::new(py, sp)?)?;
        }
        out.set_item("PV", PyList::new(py, frame.pvs)?)?;
        if let Some(unf) = frame.unfunded_amounts {
            out.set_item("Unfunded Amount", PyList::new(py, unf)?)?;
        }
        if let Some(comm) = frame.commitment_amounts {
            out.set_item("Commitment Amount", PyList::new(py, comm)?)?;
        }
        if let Some(baser) = frame.base_rates {
            out.set_item("Base Rate", PyList::new(py, baser)?)?;
        }
        if let Some(spreads) = frame.spreads {
            out.set_item("Spread", PyList::new(py, spreads)?)?;
        }
        out.set_item("allin_rate", PyList::new(py, frame.allin_rates)?)?;

        Ok(out.into())
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
    frozen
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
    frozen
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
        use crate::core::money::extract_money;
        use crate::core::utils::py_to_date;

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
    ///     calendar: Optional calendar identifier
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
                bps,
                freq: schedule.inner.freq,
                dc: schedule.inner.dc,
                bdc: schedule.inner.bdc,
                calendar_id: calendar.map(|s| s.to_string()),
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
    frozen
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
            rate,
            schedule: schedule.inner.clone(),
        })
    }

    #[getter]
    fn rate(&self) -> f64 {
        self.inner.rate
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
    frozen
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
        "CashflowBuilder",
        "CashFlowSchedule",
        "FeeBase",
        "FeeSpec",
        "FixedWindow",
        "FloatWindow",
    ])
}
