use crate::core::common::args::{
    BusinessDayConventionArg, CurrencyArg, DayCountArg, StubKindArg, TenorArg,
};
use crate::core::currency::PyCurrency;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::money::{extract_money, PyMoney};
use crate::errors::{core_to_py, PyContext};
use crate::valuations::common::PyInstrumentType;
use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, DayCount, StubKind, Tenor};
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::cashflow::builder::specs::{CouponType, FloatingRateSpec};
use finstack_valuations::instruments::fixed_income::term_loan::spec::MarginStepUp;
use finstack_valuations::instruments::fixed_income::term_loan::{
    AmortizationSpec as TermLoanAmortizationSpec, CashSweepEvent, CommitmentFeeBase,
    CommitmentStepDown, CovenantSpec, DdtlSpec, DrawEvent, LoanCall, LoanCallSchedule,
    LoanCallType, OidEirSpec, OidPolicy, PikToggle, RateSpec, TermLoan,
};
use finstack_valuations::instruments::{Attributes, PricingOverrides};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::{Bound, Py, PyRef, PyRefMut};
use std::fmt;
use std::sync::Arc;

/// Term loan instrument with DDTL (Delayed Draw Term Loan) support.
///
/// A term loan is a debt instrument with a defined maturity, optional amortization,
/// and support for both fixed and floating rates. The DDTL variant allows for
/// delayed draws during an availability period with commitment fees and usage fees.
///
/// Examples:
///     >>> from finstack.valuations.instruments import TermLoan
///     >>> from finstack.core.money import Money
///     >>> from datetime import date
///     >>>
///     >>> # Create a simple fixed-rate term loan
///     >>> loan = TermLoan.from_json('''{
///     ...     "id": "loan_001",
///     ...     "discount_curve_id": "usd_discount",
///     ...     "currency": "USD",
///     ...     "issue": "2024-01-01",
///     ...     "maturity": "2029-01-01",
///     ...     "rate": {"Fixed": {"rate_bp": 500}},
///     ...     "pay_freq": {"months": 3},
///     ...     "day_count": "Act360",
///     ...     "bdc": "Following",
///     ...     "calendar_id": null,
///     ...     "stub": "None",
///     ...     "amortization": "None",
///     ...     "coupon_type": "Cash",
///     ...     "upfront_fee": null,
///     ...     "ddtl": null,
///     ...     "covenants": null,
///     ...     "oid_eir": null,
///     ...     "pricing_overrides": {},
///     ...     "call_schedule": null
///     ... }''')
///     >>> loan.instrument_id
///     'loan_001'
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "TermLoan",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyTermLoan {
    pub(crate) inner: Arc<TermLoan>,
}

impl PyTermLoan {
    pub(crate) fn new(inner: TermLoan) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pymethods]
impl PyTermLoan {
    #[classmethod]
    #[pyo3(text_signature = "(cls, json_str)")]
    /// Create a term loan from a JSON string specification.
    ///
    /// The JSON should match the TermLoan schema from finstack-valuations.
    /// This is the recommended way to create complex term loans with DDTL features,
    /// covenants, and custom amortization schedules.
    ///
    /// Args:
    ///     json_str: JSON string matching the TermLoan schema.
    ///
    /// Returns:
    ///     TermLoan: Configured term loan instrument.
    ///
    /// Raises:
    ///     ValueError: If JSON cannot be parsed or is invalid.
    fn from_json(_cls: &Bound<'_, PyType>, json_str: &str) -> PyResult<Self> {
        serde_json::from_str(json_str)
            .map(Self::new)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    /// Serialize the term loan to a JSON string.
    ///
    /// Returns:
    ///     str: JSON representation of the term loan.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string_pretty(&self.inner)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    /// Instrument identifier.
    ///
    /// Returns:
    ///     str: Unique identifier assigned to the instrument.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Currency for all cashflows.
    ///
    /// Returns:
    ///     Currency: Currency object.
    #[getter]
    fn currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.currency)
    }

    /// Maximum commitment / notional limit.
    ///
    /// Returns:
    ///     Money: Notional limit wrapped as :class:`finstack.core.money.Money`.
    #[getter]
    fn notional_limit(&self) -> PyMoney {
        PyMoney::new(self.inner.notional_limit)
    }

    /// Issue (effective) date.
    ///
    /// Returns:
    ///     datetime.date: Issue date converted to Python.
    #[getter]
    fn issue(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.issue_date)
    }

    /// Maturity date.
    ///
    /// Returns:
    ///     datetime.date: Maturity date converted to Python.
    #[getter]
    fn maturity(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.maturity)
    }

    /// Discount curve identifier.
    ///
    /// Returns:
    ///     str: Identifier for the discount curve.
    #[getter]
    fn discount_curve(&self) -> String {
        self.inner.discount_curve_id.as_str().to_string()
    }

    /// Instrument type enum (``InstrumentType.TERM_LOAN``).
    ///
    /// Returns:
    ///     InstrumentType: Enumeration value identifying the instrument family.
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::TermLoan)
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_id)")]
    fn builder<'py>(
        cls: &Bound<'py, PyType>,
        instrument_id: &str,
    ) -> PyResult<Py<PyTermLoanBuilder>> {
        let py = cls.py();
        let builder = PyTermLoanBuilder::new_with_id(InstrumentId::new(instrument_id));
        Py::new(py, builder)
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "TermLoan(id='{}', issue='{}', maturity='{}')",
            self.inner.id, self.inner.issue_date, self.inner.maturity
        ))
    }
}

impl fmt::Display for PyTermLoan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TermLoan({}, {} -> {})",
            self.inner.id, self.inner.issue_date, self.inner.maturity
        )
    }
}

// ---------------------------------------------------------------------------
// Supporting types
// ---------------------------------------------------------------------------

/// Rate specification for term loans (fixed or floating).
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "RateSpec",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyRateSpec {
    pub(crate) inner: RateSpec,
}

#[pymethods]
impl PyRateSpec {
    #[classmethod]
    #[pyo3(text_signature = "(cls, rate_bp)")]
    fn fixed(_cls: &Bound<'_, PyType>, rate_bp: i32) -> Self {
        Self {
            inner: RateSpec::Fixed { rate_bp },
        }
    }

    #[classmethod]
    #[pyo3(
        text_signature = "(cls, index_id, spread_bp, reset_freq='3M', reset_lag_days=2, floor_bp=None, cap_bp=None)",
        signature = (index_id, spread_bp, reset_freq=TenorArg(Tenor::quarterly()), reset_lag_days=2, floor_bp=None, cap_bp=None)
    )]
    fn floating(
        _cls: &Bound<'_, PyType>,
        index_id: &str,
        spread_bp: f64,
        reset_freq: TenorArg,
        reset_lag_days: i32,
        floor_bp: Option<f64>,
        cap_bp: Option<f64>,
    ) -> Self {
        use rust_decimal::Decimal;
        Self {
            inner: RateSpec::Floating(FloatingRateSpec {
                index_id: CurveId::new(index_id),
                spread_bp: Decimal::from_f64_retain(spread_bp).unwrap_or_default(),
                gearing: Decimal::ONE,
                gearing_includes_spread: true,
                floor_bp: floor_bp.and_then(Decimal::from_f64_retain),
                all_in_floor_bp: None,
                cap_bp: cap_bp.and_then(Decimal::from_f64_retain),
                index_cap_bp: None,
                reset_freq: reset_freq.0,
                reset_lag_days,
                dc: DayCount::Act360,
                bdc: BusinessDayConvention::ModifiedFollowing,
                calendar_id: "weekends_only".to_string(),
                fixing_calendar_id: None,
                end_of_month: false,
                overnight_compounding: None,
                payment_lag_days: 0,
            }),
        }
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            RateSpec::Fixed { rate_bp } => format!("RateSpec.fixed(rate_bp={rate_bp})"),
            RateSpec::Floating(f) => {
                format!(
                    "RateSpec.floating(index='{}', spread_bp={})",
                    f.index_id, f.spread_bp
                )
            }
            _ => "RateSpec(...)".to_string(),
        }
    }
}

/// Amortization specification for term loans.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "TermLoanAmortizationSpec",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyTermLoanAmortizationSpec {
    pub(crate) inner: TermLoanAmortizationSpec,
}

#[pymethods]
impl PyTermLoanAmortizationSpec {
    #[classmethod]
    #[pyo3(text_signature = "(cls,)")]
    fn none(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: TermLoanAmortizationSpec::None,
        }
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, start, end)")]
    fn linear(
        _cls: &Bound<'_, PyType>,
        start: Bound<'_, PyAny>,
        end: Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: TermLoanAmortizationSpec::Linear {
                start: py_to_date(&start).context("start")?,
                end: py_to_date(&end).context("end")?,
            },
        })
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, bp)")]
    fn percent_per_period(_cls: &Bound<'_, PyType>, bp: i32) -> Self {
        Self {
            inner: TermLoanAmortizationSpec::PercentPerPeriod { bp },
        }
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, bp)")]
    fn percent_of_original_notional(_cls: &Bound<'_, PyType>, bp: i32) -> Self {
        Self {
            inner: TermLoanAmortizationSpec::PercentOfOriginalNotional { bp },
        }
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, schedule)")]
    fn custom(
        _cls: &Bound<'_, PyType>,
        schedule: Vec<(Bound<'_, PyAny>, Bound<'_, PyAny>)>,
    ) -> PyResult<Self> {
        let mut items = Vec::with_capacity(schedule.len());
        for (date_obj, money_obj) in schedule {
            let d = py_to_date(&date_obj).context("date")?;
            let m = extract_money(&money_obj).context("amount")?;
            items.push((d, m));
        }
        Ok(Self {
            inner: TermLoanAmortizationSpec::Custom(items),
        })
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            TermLoanAmortizationSpec::None => "TermLoanAmortizationSpec.none()".to_string(),
            TermLoanAmortizationSpec::Linear { start, end } => {
                format!("TermLoanAmortizationSpec.linear({start}, {end})")
            }
            TermLoanAmortizationSpec::PercentPerPeriod { bp } => {
                format!("TermLoanAmortizationSpec.percent_per_period(bp={bp})")
            }
            TermLoanAmortizationSpec::PercentOfOriginalNotional { bp } => {
                format!("TermLoanAmortizationSpec.percent_of_original_notional(bp={bp})")
            }
            TermLoanAmortizationSpec::Custom(items) => {
                format!("TermLoanAmortizationSpec.custom(len={})", items.len())
            }
            _ => "TermLoanAmortizationSpec(...)".to_string(),
        }
    }
}

/// Coupon type for term loans (Cash, PIK, or Split).
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "CouponType",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyCouponType {
    pub(crate) inner: CouponType,
}

#[pymethods]
impl PyCouponType {
    #[classattr]
    const CASH: Self = Self {
        inner: CouponType::Cash,
    };
    #[classattr]
    const PIK: Self = Self {
        inner: CouponType::PIK,
    };

    #[classmethod]
    #[pyo3(text_signature = "(cls, cash_pct, pik_pct)")]
    fn split(_cls: &Bound<'_, PyType>, cash_pct: f64, pik_pct: f64) -> Self {
        use rust_decimal::Decimal;
        Self {
            inner: CouponType::Split {
                cash_pct: Decimal::from_f64_retain(cash_pct).unwrap_or_default(),
                pik_pct: Decimal::from_f64_retain(pik_pct).unwrap_or_default(),
            },
        }
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            CouponType::Cash => "CouponType.CASH".to_string(),
            CouponType::PIK => "CouponType.PIK".to_string(),
            CouponType::Split { cash_pct, pik_pct } => {
                format!("CouponType.split(cash_pct={cash_pct}, pik_pct={pik_pct})")
            }
        }
    }
}

/// Draw event for delayed-draw term loans.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "DrawEvent",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyDrawEvent {
    pub(crate) inner: DrawEvent,
}

#[pymethods]
impl PyDrawEvent {
    #[new]
    #[pyo3(text_signature = "(date, amount)")]
    fn new_py(date: Bound<'_, PyAny>, amount: Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self {
            inner: DrawEvent {
                date: py_to_date(&date).context("date")?,
                amount: extract_money(&amount).context("amount")?,
            },
        })
    }

    #[getter]
    fn date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.date)
    }
    #[getter]
    fn amount(&self) -> PyMoney {
        PyMoney::new(self.inner.amount)
    }

    fn __repr__(&self) -> String {
        format!(
            "DrawEvent(date={}, amount={})",
            self.inner.date, self.inner.amount
        )
    }
}

/// Commitment step-down event for DDTL facilities.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "CommitmentStepDown",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyCommitmentStepDown {
    pub(crate) inner: CommitmentStepDown,
}

#[pymethods]
impl PyCommitmentStepDown {
    #[new]
    #[pyo3(text_signature = "(date, new_limit)")]
    fn new_py(date: Bound<'_, PyAny>, new_limit: Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self {
            inner: CommitmentStepDown {
                date: py_to_date(&date).context("date")?,
                new_limit: extract_money(&new_limit).context("new_limit")?,
            },
        })
    }

    #[getter]
    fn date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.date)
    }
    #[getter]
    fn new_limit(&self) -> PyMoney {
        PyMoney::new(self.inner.new_limit)
    }

    fn __repr__(&self) -> String {
        format!(
            "CommitmentStepDown(date={}, new_limit={})",
            self.inner.date, self.inner.new_limit
        )
    }
}

/// Basis for calculating commitment fees on undrawn portions.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "CommitmentFeeBase",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyCommitmentFeeBase {
    pub(crate) inner: CommitmentFeeBase,
}

#[pymethods]
impl PyCommitmentFeeBase {
    #[classattr]
    const UNDRAWN: Self = Self {
        inner: CommitmentFeeBase::Undrawn,
    };
    #[classattr]
    const COMMITMENT_MINUS_OUTSTANDING: Self = Self {
        inner: CommitmentFeeBase::CommitmentMinusOutstanding,
    };

    fn __repr__(&self) -> String {
        match &self.inner {
            CommitmentFeeBase::Undrawn => "CommitmentFeeBase.UNDRAWN".to_string(),
            CommitmentFeeBase::CommitmentMinusOutstanding => {
                "CommitmentFeeBase.COMMITMENT_MINUS_OUTSTANDING".to_string()
            }
            _ => "CommitmentFeeBase(...)".to_string(),
        }
    }
}

/// Margin step-up event (covenant penalty or scheduled increase).
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "MarginStepUp",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyMarginStepUp {
    pub(crate) inner: MarginStepUp,
}

#[pymethods]
impl PyMarginStepUp {
    #[new]
    #[pyo3(text_signature = "(date, delta_bp)")]
    fn new_py(date: Bound<'_, PyAny>, delta_bp: i32) -> PyResult<Self> {
        Ok(Self {
            inner: MarginStepUp {
                date: py_to_date(&date).context("date")?,
                delta_bp,
            },
        })
    }

    #[getter]
    fn date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.date)
    }
    #[getter]
    fn delta_bp(&self) -> i32 {
        self.inner.delta_bp
    }

    fn __repr__(&self) -> String {
        format!(
            "MarginStepUp(date={}, delta_bp={})",
            self.inner.date, self.inner.delta_bp
        )
    }
}

/// Payment-in-kind (PIK) toggle event.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "PikToggle",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyPikToggle {
    pub(crate) inner: PikToggle,
}

#[pymethods]
impl PyPikToggle {
    #[new]
    #[pyo3(text_signature = "(date, enable_pik)")]
    fn new_py(date: Bound<'_, PyAny>, enable_pik: bool) -> PyResult<Self> {
        Ok(Self {
            inner: PikToggle {
                date: py_to_date(&date).context("date")?,
                enable_pik,
            },
        })
    }

    #[getter]
    fn date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.date)
    }
    #[getter]
    fn enable_pik(&self) -> bool {
        self.inner.enable_pik
    }

    fn __repr__(&self) -> String {
        format!(
            "PikToggle(date={}, enable_pik={})",
            self.inner.date, self.inner.enable_pik
        )
    }
}

/// Cash sweep event (mandatory prepayment).
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "CashSweepEvent",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyCashSweepEvent {
    pub(crate) inner: CashSweepEvent,
}

#[pymethods]
impl PyCashSweepEvent {
    #[new]
    #[pyo3(text_signature = "(date, amount)")]
    fn new_py(date: Bound<'_, PyAny>, amount: Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self {
            inner: CashSweepEvent {
                date: py_to_date(&date).context("date")?,
                amount: extract_money(&amount).context("amount")?,
            },
        })
    }

    #[getter]
    fn date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.date)
    }
    #[getter]
    fn amount(&self) -> PyMoney {
        PyMoney::new(self.inner.amount)
    }

    fn __repr__(&self) -> String {
        format!(
            "CashSweepEvent(date={}, amount={})",
            self.inner.date, self.inner.amount
        )
    }
}

/// Covenant-driven events for term loans.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "CovenantSpec",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyCovenantSpec {
    pub(crate) inner: CovenantSpec,
}

#[pymethods]
impl PyCovenantSpec {
    #[new]
    #[pyo3(
        text_signature = "(margin_stepups=None, pik_toggles=None, cash_sweeps=None, draw_stop_dates=None)",
        signature = (margin_stepups=None, pik_toggles=None, cash_sweeps=None, draw_stop_dates=None)
    )]
    fn new_py(
        margin_stepups: Option<Vec<PyRef<'_, PyMarginStepUp>>>,
        pik_toggles: Option<Vec<PyRef<'_, PyPikToggle>>>,
        cash_sweeps: Option<Vec<PyRef<'_, PyCashSweepEvent>>>,
        draw_stop_dates: Option<Vec<Bound<'_, PyAny>>>,
    ) -> PyResult<Self> {
        let dates = match draw_stop_dates {
            Some(ds) => ds
                .iter()
                .map(|d| py_to_date(d).context("draw_stop_date"))
                .collect::<PyResult<Vec<_>>>()?,
            None => Vec::new(),
        };
        Ok(Self {
            inner: CovenantSpec {
                margin_stepups: margin_stepups
                    .unwrap_or_default()
                    .iter()
                    .map(|s| s.inner.clone())
                    .collect(),
                pik_toggles: pik_toggles
                    .unwrap_or_default()
                    .iter()
                    .map(|t| t.inner.clone())
                    .collect(),
                cash_sweeps: cash_sweeps
                    .unwrap_or_default()
                    .iter()
                    .map(|c| c.inner.clone())
                    .collect(),
                draw_stop_dates: dates,
            },
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "CovenantSpec(stepups={}, pik_toggles={}, sweeps={}, draw_stops={})",
            self.inner.margin_stepups.len(),
            self.inner.pik_toggles.len(),
            self.inner.cash_sweeps.len(),
            self.inner.draw_stop_dates.len(),
        )
    }
}

/// Delayed-draw term loan (DDTL) specification.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "DdtlSpec",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyDdtlSpec {
    pub(crate) inner: DdtlSpec,
}

#[pymethods]
impl PyDdtlSpec {
    #[new]
    #[pyo3(
        text_signature = "(commitment_limit, availability_start, availability_end, draws=None, commitment_step_downs=None, usage_fee_bp=0, commitment_fee_bp=0, fee_base=None, oid_policy=None)",
        signature = (commitment_limit, availability_start, availability_end, draws=None, commitment_step_downs=None, usage_fee_bp=0, commitment_fee_bp=0, fee_base=None, oid_policy=None)
    )]
    #[allow(clippy::too_many_arguments)]
    fn new_py(
        commitment_limit: Bound<'_, PyAny>,
        availability_start: Bound<'_, PyAny>,
        availability_end: Bound<'_, PyAny>,
        draws: Option<Vec<PyRef<'_, PyDrawEvent>>>,
        commitment_step_downs: Option<Vec<PyRef<'_, PyCommitmentStepDown>>>,
        usage_fee_bp: i32,
        commitment_fee_bp: i32,
        fee_base: Option<&PyCommitmentFeeBase>,
        oid_policy: Option<&PyOidPolicy>,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: DdtlSpec {
                commitment_limit: extract_money(&commitment_limit).context("commitment_limit")?,
                availability_start: py_to_date(&availability_start)
                    .context("availability_start")?,
                availability_end: py_to_date(&availability_end).context("availability_end")?,
                draws: draws
                    .unwrap_or_default()
                    .iter()
                    .map(|d| d.inner.clone())
                    .collect(),
                commitment_step_downs: commitment_step_downs
                    .unwrap_or_default()
                    .iter()
                    .map(|s| s.inner.clone())
                    .collect(),
                usage_fee_bp,
                commitment_fee_bp,
                fee_base: fee_base
                    .map(|f| f.inner.clone())
                    .unwrap_or(CommitmentFeeBase::Undrawn),
                oid_policy: oid_policy.map(|p| p.inner.clone()),
            },
        })
    }

    #[getter]
    fn commitment_limit(&self) -> PyMoney {
        PyMoney::new(self.inner.commitment_limit)
    }
    #[getter]
    fn usage_fee_bp(&self) -> i32 {
        self.inner.usage_fee_bp
    }
    #[getter]
    fn commitment_fee_bp(&self) -> i32 {
        self.inner.commitment_fee_bp
    }

    fn __repr__(&self) -> String {
        format!(
            "DdtlSpec(commitment={}, draws={}, usage_fee_bp={}, commitment_fee_bp={})",
            self.inner.commitment_limit,
            self.inner.draws.len(),
            self.inner.usage_fee_bp,
            self.inner.commitment_fee_bp,
        )
    }
}

/// OID effective interest rate amortization settings.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "OidEirSpec",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyOidEirSpec {
    pub(crate) inner: OidEirSpec,
}

#[pymethods]
impl PyOidEirSpec {
    #[new]
    #[pyo3(
        text_signature = "(include_fees=True)",
        signature = (include_fees=true)
    )]
    fn new_py(include_fees: bool) -> Self {
        Self {
            inner: OidEirSpec { include_fees },
        }
    }

    #[getter]
    fn include_fees(&self) -> bool {
        self.inner.include_fees
    }

    fn __repr__(&self) -> String {
        format!("OidEirSpec(include_fees={})", self.inner.include_fees)
    }
}

/// Original Issue Discount (OID) policy.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "OidPolicy",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyOidPolicy {
    pub(crate) inner: OidPolicy,
}

#[pymethods]
impl PyOidPolicy {
    #[classmethod]
    #[pyo3(text_signature = "(cls, pct_bp)")]
    fn withheld_pct(_cls: &Bound<'_, PyType>, pct_bp: i32) -> Self {
        Self {
            inner: OidPolicy::WithheldPct(pct_bp),
        }
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, amount)")]
    fn withheld_amount(_cls: &Bound<'_, PyType>, amount: Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self {
            inner: OidPolicy::WithheldAmount(extract_money(&amount).context("amount")?),
        })
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, pct_bp)")]
    fn separate_pct(_cls: &Bound<'_, PyType>, pct_bp: i32) -> Self {
        Self {
            inner: OidPolicy::SeparatePct(pct_bp),
        }
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, amount)")]
    fn separate_amount(_cls: &Bound<'_, PyType>, amount: Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self {
            inner: OidPolicy::SeparateAmount(extract_money(&amount).context("amount")?),
        })
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            OidPolicy::WithheldPct(bp) => format!("OidPolicy.withheld_pct({bp})"),
            OidPolicy::WithheldAmount(m) => format!("OidPolicy.withheld_amount({m})"),
            OidPolicy::SeparatePct(bp) => format!("OidPolicy.separate_pct({bp})"),
            OidPolicy::SeparateAmount(m) => format!("OidPolicy.separate_amount({m})"),
            _ => "OidPolicy(...)".to_string(),
        }
    }
}

/// Type of borrower call provision.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "LoanCallType",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyLoanCallType {
    pub(crate) inner: LoanCallType,
}

#[pymethods]
impl PyLoanCallType {
    #[classattr]
    const HARD: Self = Self {
        inner: LoanCallType::Hard,
    };
    #[classattr]
    const SOFT: Self = Self {
        inner: LoanCallType::Soft,
    };

    #[classmethod]
    #[pyo3(text_signature = "(cls, treasury_spread_bp)")]
    fn make_whole(_cls: &Bound<'_, PyType>, treasury_spread_bp: i32) -> Self {
        Self {
            inner: LoanCallType::MakeWhole { treasury_spread_bp },
        }
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            LoanCallType::Hard => "LoanCallType.HARD".to_string(),
            LoanCallType::Soft => "LoanCallType.SOFT".to_string(),
            LoanCallType::MakeWhole { treasury_spread_bp } => {
                format!("LoanCallType.make_whole({treasury_spread_bp})")
            }
            _ => "LoanCallType(...)".to_string(),
        }
    }
}

/// Individual borrower call option on a term loan.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "LoanCall",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyLoanCall {
    pub(crate) inner: LoanCall,
}

#[pymethods]
impl PyLoanCall {
    #[new]
    #[pyo3(
        text_signature = "(date, price_pct_of_par, call_type=None)",
        signature = (date, price_pct_of_par, call_type=None)
    )]
    fn new_py(
        date: Bound<'_, PyAny>,
        price_pct_of_par: f64,
        call_type: Option<&PyLoanCallType>,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: LoanCall {
                date: py_to_date(&date).context("date")?,
                price_pct_of_par,
                call_type: call_type.map(|ct| ct.inner.clone()).unwrap_or_default(),
            },
        })
    }

    #[getter]
    fn date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.date)
    }
    #[getter]
    fn price_pct_of_par(&self) -> f64 {
        self.inner.price_pct_of_par
    }

    fn __repr__(&self) -> String {
        format!(
            "LoanCall(date={}, price_pct={})",
            self.inner.date, self.inner.price_pct_of_par
        )
    }
}

/// Complete call schedule for callable term loans.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "LoanCallSchedule",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyLoanCallSchedule {
    pub(crate) inner: LoanCallSchedule,
}

#[pymethods]
impl PyLoanCallSchedule {
    #[new]
    #[pyo3(text_signature = "(calls)")]
    fn new_py(calls: Vec<PyRef<'_, PyLoanCall>>) -> Self {
        Self {
            inner: LoanCallSchedule {
                calls: calls.iter().map(|c| c.inner.clone()).collect(),
            },
        }
    }

    #[getter]
    fn call_count(&self) -> usize {
        self.inner.calls.len()
    }

    fn __repr__(&self) -> String {
        format!("LoanCallSchedule(calls={})", self.inner.calls.len())
    }
}

// ---------------------------------------------------------------------------
// Builder
// ---------------------------------------------------------------------------

/// Fluent builder for :class:`TermLoan` instruments.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "TermLoanBuilder",
    unsendable,
    skip_from_py_object
)]
pub struct PyTermLoanBuilder {
    instrument_id: InstrumentId,
    pending_currency: Option<Currency>,
    pending_notional: Option<Money>,
    issue: Option<time::Date>,
    maturity: Option<time::Date>,
    rate: Option<RateSpec>,
    frequency: Tenor,
    day_count: DayCount,
    bdc: BusinessDayConvention,
    calendar_id: Option<String>,
    stub: StubKind,
    discount_curve: Option<CurveId>,
    credit_curve: Option<CurveId>,
    amortization: TermLoanAmortizationSpec,
    coupon_type: CouponType,
    upfront_fee: Option<Money>,
    ddtl: Option<DdtlSpec>,
    covenants: Option<CovenantSpec>,
    oid_eir: Option<OidEirSpec>,
    call_schedule: Option<LoanCallSchedule>,
    settlement_days: u32,
}

impl PyTermLoanBuilder {
    fn new_with_id(id: InstrumentId) -> Self {
        Self {
            instrument_id: id,
            pending_currency: None,
            pending_notional: None,
            issue: None,
            maturity: None,
            rate: None,
            frequency: Tenor::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: None,
            stub: StubKind::ShortFront,
            discount_curve: None,
            credit_curve: None,
            amortization: TermLoanAmortizationSpec::None,
            coupon_type: CouponType::Cash,
            upfront_fee: None,
            ddtl: None,
            covenants: None,
            oid_eir: None,
            call_schedule: None,
            settlement_days: 2,
        }
    }
}

#[pymethods]
impl PyTermLoanBuilder {
    #[new]
    #[pyo3(text_signature = "(instrument_id)")]
    fn new_py(instrument_id: &str) -> Self {
        Self::new_with_id(InstrumentId::new(instrument_id))
    }

    fn currency(mut slf: PyRefMut<'_, Self>, currency: CurrencyArg) -> PyRefMut<'_, Self> {
        slf.pending_currency = Some(currency.0);
        slf
    }

    fn notional<'py>(
        mut slf: PyRefMut<'py, Self>,
        amount: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let money = extract_money(&amount).context("notional")?;
        slf.pending_notional = Some(money);
        slf.pending_currency = Some(money.currency());
        Ok(slf)
    }

    fn issue<'py>(
        mut slf: PyRefMut<'py, Self>,
        date: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.issue = Some(py_to_date(&date).context("issue")?);
        Ok(slf)
    }

    fn maturity<'py>(
        mut slf: PyRefMut<'py, Self>,
        date: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.maturity = Some(py_to_date(&date).context("maturity")?);
        Ok(slf)
    }

    fn rate<'py>(mut slf: PyRefMut<'py, Self>, spec: &'py PyRateSpec) -> PyRefMut<'py, Self> {
        slf.rate = Some(spec.inner.clone());
        slf
    }

    fn frequency(mut slf: PyRefMut<'_, Self>, freq: TenorArg) -> PyRefMut<'_, Self> {
        slf.frequency = freq.0;
        slf
    }

    fn day_count(mut slf: PyRefMut<'_, Self>, dc: DayCountArg) -> PyRefMut<'_, Self> {
        slf.day_count = dc.0;
        slf
    }

    fn bdc(mut slf: PyRefMut<'_, Self>, bdc: BusinessDayConventionArg) -> PyRefMut<'_, Self> {
        slf.bdc = bdc.0;
        slf
    }

    #[pyo3(signature = (calendar_id=None))]
    fn calendar(mut slf: PyRefMut<'_, Self>, calendar_id: Option<String>) -> PyRefMut<'_, Self> {
        slf.calendar_id = calendar_id;
        slf
    }

    fn stub(mut slf: PyRefMut<'_, Self>, stub: StubKindArg) -> PyRefMut<'_, Self> {
        slf.stub = stub.0;
        slf
    }

    fn disc_id(mut slf: PyRefMut<'_, Self>, curve_id: String) -> PyRefMut<'_, Self> {
        slf.discount_curve = Some(CurveId::new(&curve_id));
        slf
    }

    #[pyo3(signature = (curve_id=None))]
    fn credit_curve(mut slf: PyRefMut<'_, Self>, curve_id: Option<String>) -> PyRefMut<'_, Self> {
        slf.credit_curve = curve_id.map(|id| CurveId::new(&id));
        slf
    }

    fn amortization<'py>(
        mut slf: PyRefMut<'py, Self>,
        spec: &'py PyTermLoanAmortizationSpec,
    ) -> PyRefMut<'py, Self> {
        slf.amortization = spec.inner.clone();
        slf
    }

    fn coupon_type<'py>(
        mut slf: PyRefMut<'py, Self>,
        ct: &'py PyCouponType,
    ) -> PyRefMut<'py, Self> {
        slf.coupon_type = ct.inner.clone();
        slf
    }

    #[pyo3(signature = (fee=None))]
    fn upfront_fee<'py>(
        mut slf: PyRefMut<'py, Self>,
        fee: Option<Bound<'py, PyAny>>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.upfront_fee = fee
            .map(|f| extract_money(&f).context("upfront_fee"))
            .transpose()?;
        Ok(slf)
    }

    #[pyo3(signature = (spec=None))]
    fn ddtl<'py>(
        mut slf: PyRefMut<'py, Self>,
        spec: Option<&'py PyDdtlSpec>,
    ) -> PyRefMut<'py, Self> {
        slf.ddtl = spec.map(|s| s.inner.clone());
        slf
    }

    #[pyo3(signature = (spec=None))]
    fn covenants<'py>(
        mut slf: PyRefMut<'py, Self>,
        spec: Option<&'py PyCovenantSpec>,
    ) -> PyRefMut<'py, Self> {
        slf.covenants = spec.map(|s| s.inner.clone());
        slf
    }

    #[pyo3(signature = (spec=None))]
    fn oid_eir<'py>(
        mut slf: PyRefMut<'py, Self>,
        spec: Option<&'py PyOidEirSpec>,
    ) -> PyRefMut<'py, Self> {
        slf.oid_eir = spec.map(|s| s.inner.clone());
        slf
    }

    #[pyo3(signature = (schedule=None))]
    fn call_schedule<'py>(
        mut slf: PyRefMut<'py, Self>,
        schedule: Option<&'py PyLoanCallSchedule>,
    ) -> PyRefMut<'py, Self> {
        slf.call_schedule = schedule.map(|s| s.inner.clone());
        slf
    }

    fn settlement_days(mut slf: PyRefMut<'_, Self>, days: u32) -> PyRefMut<'_, Self> {
        slf.settlement_days = days;
        slf
    }

    fn build(slf: PyRefMut<'_, Self>) -> PyResult<PyTermLoan> {
        let notional = slf.pending_notional.ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err("notional() must be called before build()")
        })?;
        let currency = slf.pending_currency.ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err("currency() must be called before build()")
        })?;
        let issue = slf.issue.ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err("issue() must be called before build()")
        })?;
        let maturity = slf.maturity.ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err("maturity() must be called before build()")
        })?;
        let rate = slf.rate.clone().ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err("rate() must be called before build()")
        })?;
        let discount = slf.discount_curve.clone().ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err("disc_id() must be called before build()")
        })?;

        TermLoan::builder()
            .id(slf.instrument_id.clone())
            .currency(currency)
            .notional_limit(notional)
            .issue_date(issue)
            .maturity(maturity)
            .rate(rate)
            .frequency(slf.frequency)
            .day_count(slf.day_count)
            .bdc(slf.bdc)
            .calendar_id_opt(slf.calendar_id.clone())
            .stub(slf.stub)
            .discount_curve_id(discount)
            .credit_curve_id_opt(slf.credit_curve.clone())
            .amortization(slf.amortization.clone())
            .coupon_type(slf.coupon_type.clone())
            .upfront_fee_opt(slf.upfront_fee)
            .ddtl_opt(slf.ddtl.clone())
            .covenants_opt(slf.covenants.clone())
            .pricing_overrides(PricingOverrides::default())
            .oid_eir_opt(slf.oid_eir.clone())
            .call_schedule_opt(slf.call_schedule.clone())
            .settlement_days(slf.settlement_days)
            .attributes(Attributes::new())
            .build()
            .map(PyTermLoan::new)
            .map_err(core_to_py)
    }

    fn __repr__(&self) -> String {
        "TermLoanBuilder(...)".to_string()
    }
}

// ---------------------------------------------------------------------------
// Module registration
// ---------------------------------------------------------------------------

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyTermLoan>()?;
    module.add_class::<PyTermLoanBuilder>()?;
    module.add_class::<PyRateSpec>()?;
    module.add_class::<PyTermLoanAmortizationSpec>()?;
    module.add_class::<PyCouponType>()?;
    module.add_class::<PyDrawEvent>()?;
    module.add_class::<PyCommitmentStepDown>()?;
    module.add_class::<PyCommitmentFeeBase>()?;
    module.add_class::<PyMarginStepUp>()?;
    module.add_class::<PyPikToggle>()?;
    module.add_class::<PyCashSweepEvent>()?;
    module.add_class::<PyCovenantSpec>()?;
    module.add_class::<PyDdtlSpec>()?;
    module.add_class::<PyOidEirSpec>()?;
    module.add_class::<PyOidPolicy>()?;
    module.add_class::<PyLoanCallType>()?;
    module.add_class::<PyLoanCall>()?;
    module.add_class::<PyLoanCallSchedule>()?;
    Ok(vec![
        "TermLoan",
        "TermLoanBuilder",
        "RateSpec",
        "TermLoanAmortizationSpec",
        "CouponType",
        "DrawEvent",
        "CommitmentStepDown",
        "CommitmentFeeBase",
        "MarginStepUp",
        "PikToggle",
        "CashSweepEvent",
        "CovenantSpec",
        "DdtlSpec",
        "OidEirSpec",
        "OidPolicy",
        "LoanCallType",
        "LoanCall",
        "LoanCallSchedule",
    ])
}
