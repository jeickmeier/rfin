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
                rate: rust_decimal::Decimal::from_f64_retain(rate).unwrap_or_default(),
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
                    fixing_calendar_id: calendar_id,
                },
                coupon_type: coupon_type
                    .map(|c| c.inner)
                    .unwrap_or(val_builder::CouponType::Cash),
                freq: schedule.inner.freq,
                stub: schedule.inner.stub,
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
    inner: val_builder::CashFlowBuilder,
}

#[pymethods]
impl PyCashflowBuilder {
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

    #[pyo3(
        signature = (*, market=None, discount_curve_id=None, as_of=None),
        text_signature = "(*, market=None, discount_curve_id=None, as_of=None)"
    )]
    /// Convert the cashflow schedule to a Polars DataFrame.
    ///
    /// Returns a Polars DataFrame with columns: "start_date", "end_date", "kind", "amount",
    /// "accrual_factor", "reset_date", "outstanding", "rate", and optionally
    /// "outstanding_undrawn" (if facility limit exists), "discount_factor", "pv" (if market provided).
    ///
    /// All cashflow amounts are computed in Rust for deterministic, fast processing.
    /// Outstanding balances track drawn amounts using standard conventions:
    /// - Amortization reduces outstanding
    /// - PIK increases outstanding
    /// - Notional flows: draws (negative) increase outstanding, repays (positive) decrease
    /// - The outstanding balance shown is AFTER applying the cashflow and will be used for
    ///   interest/fee calculations in the NEXT period (step function approach)
    ///
    /// For revolving credit facilities with a facility limit, an additional "outstanding_undrawn"
    /// column shows the unused commitment (facility_limit - outstanding).
    ///
    /// Args:
    ///     market: Optional MarketContext for discount factor and PV calculations
    ///     discount_curve_id: Curve ID to use (required if market provided)
    ///     as_of: Valuation date (defaults to curve base_date if not provided)
    ///
    /// Returns:
    ///     PyDataFrame: Polars DataFrame with cashflow data
    ///
    /// Examples:
    ///     >>> # Basic usage (no pricing)
    ///     >>> df = schedule.to_dataframe()
    ///     >>>
    ///     >>> # With pricing
    ///     >>> df = schedule.to_dataframe(market=market, discount_curve_id="USD-OIS")
    fn to_dataframe(
        &self,
        market: Option<PyRef<PyMarketContext>>,
        discount_curve_id: Option<&str>,
        as_of: Option<Bound<'_, PyAny>>,
    ) -> PyResult<pyo3_polars::PyDataFrame> {
        use polars::prelude::*;

        // Market context is required for DataFrame export
        let mkt = market.ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err("market context required for DataFrame export")
        })?;

        let curve_id = discount_curve_id.ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err(
                "discount_curve_id required when market is provided",
            )
        })?;

        // Parse as_of if provided
        let as_of_date = as_of.map(|d| py_to_date(&d)).transpose()?;

        let options = val_builder::PeriodDataFrameOptions {
            hazard_curve_id: None,
            forward_curve_id: None,
            as_of: as_of_date,
            day_count: None,
            discount_day_count: None,
            facility_limit: None,
            include_floating_decomposition: false,
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
            .to_period_dataframe(&periods, &mkt.inner, curve_id, options)
            .map_err(core_to_py)?;

        // Convert dates to strings for Polars
        let start_dates: Vec<String> = frame.start_dates.iter().map(|d| d.to_string()).collect();
        let end_dates: Vec<String> = frame.end_dates.iter().map(|d| d.to_string()).collect();
        let pay_dates: Vec<String> = frame.pay_dates.iter().map(|d| d.to_string()).collect();

        // Convert kinds to string labels
        let kinds: Vec<String> = frame
            .cf_types
            .iter()
            .map(|k| {
                crate::core::cashflow::primitives::PyCFKind::new(*k)
                    .name()
                    .to_string()
            })
            .collect();

        // Convert reset dates to Option<String>
        let reset_dates: Vec<Option<String>> = frame
            .reset_dates
            .iter()
            .map(|opt_d| opt_d.map(|d| d.to_string()))
            .collect();

        // Convert notionals from Option<f64> to f64 (None -> NaN)
        let notionals_f64: Vec<f64> = frame
            .notionals
            .iter()
            .map(|opt| opt.unwrap_or(f64::NAN))
            .collect();

        // Build base DataFrame with core columns
        let mut df = df![
            "start_date" => start_dates,
            "end_date" => end_dates,
            "pay_date" => pay_dates,
            "reset_date" => reset_dates,
            "kind" => kinds,
            "amount" => frame.amounts,
            "accrual_factor" => frame.accrual_factors,
            "rate" => frame.rates,
            "notional" => notionals_f64,
            "yr_fraq" => frame.yr_fraqs,
            "discount_factor" => frame.discount_factors,
            "pv" => frame.pvs,
        ]
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        // Add optional columns
        if let Some(undrawn) = frame.undrawn_notionals {
            let undrawn_f64: Vec<f64> = undrawn.iter().map(|opt| opt.unwrap_or(f64::NAN)).collect();
            let col = Series::new("undrawn_notional".into(), undrawn_f64);
            df.with_column(col)
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        }

        Ok(pyo3_polars::PyDataFrame(df))
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
                bps: rust_decimal::Decimal::from_f64_retain(bps).unwrap_or_default(),
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
