//! Python bindings for structured credit pool types.
//!
//! Exposes `PoolAsset`, `RepLine`, `Pool`, `PoolStats`, `ReinvestmentPeriod`,
//! `ReinvestmentCriteria`, `ConcentrationCheckResult`, `ConcentrationViolation`,
//! and the `calculate_pool_stats` standalone function.

use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::money::PyMoney;
use crate::errors::core_to_py;
use finstack_core::dates::DayCount;
use finstack_core::money::Money;
use finstack_core::types::CreditRating;
use finstack_valuations::instruments::fixed_income::structured_credit::{
    calculate_pool_stats as rust_calculate_pool_stats,
    ConcentrationCheckResult as RustConcentrationCheckResult,
    ConcentrationViolation as RustConcentrationViolation, DealType, Pool as RustPool,
    PoolAsset as RustPoolAsset, PoolStats as RustPoolStats,
    ReinvestmentCriteria as RustReinvestmentCriteria, ReinvestmentPeriod as RustReinvestmentPeriod,
    RepLine as RustRepLine,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyModule, PyType};
use std::str::FromStr;

// ============================================================================
// HELPERS
// ============================================================================

fn parse_deal_type(s: &str) -> PyResult<DealType> {
    match s.to_uppercase().as_str() {
        "CLO" => Ok(DealType::CLO),
        "CBO" => Ok(DealType::CBO),
        "ABS" => Ok(DealType::ABS),
        "RMBS" => Ok(DealType::RMBS),
        "CMBS" => Ok(DealType::CMBS),
        "AUTO" => Ok(DealType::Auto),
        "CARD" => Ok(DealType::Card),
        other => Err(PyValueError::new_err(format!("Unknown deal type: {other}"))),
    }
}

fn parse_credit_rating(s: &str) -> PyResult<CreditRating> {
    CreditRating::from_str(s)
        .map_err(|e| PyValueError::new_err(format!("Invalid credit rating '{s}': {e}")))
}

fn format_day_count(dc: DayCount) -> &'static str {
    match dc {
        DayCount::Act360 => "ACT_360",
        DayCount::Act365F => "ACT_365F",
        DayCount::Act365L => "ACT_365L",
        DayCount::Thirty360 => "THIRTY_360",
        DayCount::ThirtyE360 => "THIRTY_E360",
        DayCount::ActAct => "ACT_ACT",
        DayCount::ActActIsma => "ACT_ACT_ISMA",
        DayCount::Bus252 => "BUS_252",
        _ => "UNKNOWN",
    }
}

fn parse_day_count_str(s: &str) -> PyResult<DayCount> {
    let n = s.to_ascii_lowercase().replace([' ', '-'], "_");
    match n.as_str() {
        "act360" | "act/360" | "act_360" | "actual/360" => Ok(DayCount::Act360),
        "act365f" | "act/365f" | "act_365f" | "actual/365f" => Ok(DayCount::Act365F),
        "act365l" | "act/365l" | "act_365l" | "actual/365l" => Ok(DayCount::Act365L),
        "30/360" | "30_360" | "thirty/360" | "30u/360" | "thirty_360" => Ok(DayCount::Thirty360),
        "30e/360" | "30e_360" | "30/360e" | "thirty_e360" => Ok(DayCount::ThirtyE360),
        "actact" | "act/act" | "act_act" | "actual/actual" => Ok(DayCount::ActAct),
        "actact_isma" | "act/act_isma" | "act_act_isma" => Ok(DayCount::ActActIsma),
        "bus/252" | "bus_252" | "business/252" => Ok(DayCount::Bus252),
        other => Err(PyValueError::new_err(format!("Unknown day-count: {other}"))),
    }
}

fn parse_currency_str(s: &str) -> PyResult<finstack_core::currency::Currency> {
    s.parse()
        .map_err(|_| PyValueError::new_err(format!("Unknown currency: {s}")))
}

// ============================================================================
// POOL ASSET
// ============================================================================

/// Individual asset in a structured credit collateral pool.
///
/// Create via factory classmethods ``floating_rate_loan`` or ``fixed_rate_bond``,
/// then chain builder methods like ``with_rating``, ``with_industry``, ``with_obligor``.
///
/// Examples:
///     >>> import datetime
///     >>> asset = PoolAsset.floating_rate_loan(
///     ...     "LOAN001", 10_000_000.0, "USD", 450.0, "SOFR-3M",
///     ...     datetime.date(2030, 1, 15))
///     >>> asset = asset.with_rating("BB").with_industry("Technology")
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "PoolAsset",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyPoolAsset {
    pub(crate) inner: RustPoolAsset,
}

#[pymethods]
impl PyPoolAsset {
    #[classmethod]
    #[pyo3(signature = (id, balance_amount, currency, spread_bps, index_id, maturity, day_count = "ACT_360"))]
    #[pyo3(
        text_signature = "(cls, id, balance_amount, currency, spread_bps, index_id, maturity, day_count='ACT_360')"
    )]
    #[allow(clippy::too_many_arguments)]
    fn floating_rate_loan(
        _cls: &Bound<'_, PyType>,
        id: &str,
        balance_amount: f64,
        currency: &str,
        spread_bps: f64,
        index_id: &str,
        maturity: Bound<'_, PyAny>,
        day_count: &str,
    ) -> PyResult<Self> {
        let ccy = parse_currency_str(currency)?;
        let dc = parse_day_count_str(day_count)?;
        let mat = py_to_date(&maturity)?;
        let balance = Money::new(balance_amount, ccy);
        let inner = RustPoolAsset::floating_rate_loan(id, balance, index_id, spread_bps, mat, dc);
        Ok(Self { inner })
    }

    #[classmethod]
    #[pyo3(signature = (id, balance_amount, currency, rate, maturity, day_count = "THIRTY_360"))]
    #[pyo3(
        text_signature = "(cls, id, balance_amount, currency, rate, maturity, day_count='THIRTY_360')"
    )]
    fn fixed_rate_bond(
        _cls: &Bound<'_, PyType>,
        id: &str,
        balance_amount: f64,
        currency: &str,
        rate: f64,
        maturity: Bound<'_, PyAny>,
        day_count: &str,
    ) -> PyResult<Self> {
        let ccy = parse_currency_str(currency)?;
        let dc = parse_day_count_str(day_count)?;
        let mat = py_to_date(&maturity)?;
        let balance = Money::new(balance_amount, ccy);
        let inner = RustPoolAsset::fixed_rate_bond(id, balance, rate, mat, dc);
        Ok(Self { inner })
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, data)")]
    fn from_dict(_cls: &Bound<'_, PyType>, data: &Bound<'_, PyAny>) -> PyResult<Self> {
        let json_value: serde_json::Value = pythonize::depythonize(data)
            .map_err(|e| PyValueError::new_err(format!("Failed to convert from Python: {e}")))?;
        let inner: RustPoolAsset = serde_json::from_value(json_value)
            .map_err(|e| PyValueError::new_err(format!("Failed to deserialize PoolAsset: {e}")))?;
        Ok(Self { inner })
    }

    #[pyo3(text_signature = "(self)")]
    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let json_str = serde_json::to_string(&self.inner)
            .map_err(|e| PyValueError::new_err(format!("Failed to serialize: {e}")))?;
        let json_value: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| PyValueError::new_err(format!("Failed to parse JSON: {e}")))?;
        pythonize::pythonize(py, &json_value)
            .map(|bound| bound.into())
            .map_err(|e| PyValueError::new_err(format!("Failed to convert to Python: {e}")))
    }

    // -- builder methods (clone + modify + return new) --

    #[pyo3(text_signature = "(self, rating)")]
    fn with_rating(&self, rating: &str) -> PyResult<Self> {
        let cr = parse_credit_rating(rating)?;
        Ok(Self {
            inner: self.inner.clone().with_rating(cr),
        })
    }

    #[pyo3(text_signature = "(self, industry)")]
    fn with_industry(&self, industry: &str) -> Self {
        Self {
            inner: self.inner.clone().with_industry(industry),
        }
    }

    #[pyo3(text_signature = "(self, obligor_id)")]
    fn with_obligor(&self, obligor_id: &str) -> Self {
        Self {
            inner: self.inner.clone().with_obligor(obligor_id),
        }
    }

    #[pyo3(text_signature = "(self, day_count)")]
    fn with_day_count(&self, dc_str: &str) -> PyResult<Self> {
        let dc = parse_day_count_str(dc_str)?;
        let mut inner = self.inner.clone();
        inner.day_count = dc;
        Ok(Self { inner })
    }

    // -- computed methods --

    #[pyo3(text_signature = "(self)")]
    fn current_yield(&self) -> f64 {
        self.inner.current_yield()
    }

    #[pyo3(text_signature = "(self)")]
    fn spread_bps_value(&self) -> f64 {
        self.inner.spread_bps()
    }

    #[pyo3(text_signature = "(self, as_of)")]
    fn remaining_term(&self, as_of: Bound<'_, PyAny>) -> PyResult<f64> {
        let d = py_to_date(&as_of)?;
        self.inner
            .remaining_term(d, self.inner.day_count)
            .map_err(core_to_py)
    }

    #[pyo3(text_signature = "(self)")]
    fn is_amortizing(&self) -> bool {
        self.inner.asset_type.is_amortizing()
    }

    // -- getters --

    #[getter]
    fn id(&self) -> &str {
        self.inner.id.as_str()
    }

    #[getter]
    fn asset_type(&self) -> String {
        format!("{:?}", self.inner.asset_type)
    }

    #[getter]
    fn balance(&self) -> PyMoney {
        PyMoney::new(self.inner.balance)
    }

    #[getter]
    fn rate(&self) -> f64 {
        self.inner.rate
    }

    #[getter]
    fn spread_bps(&self) -> Option<f64> {
        self.inner.spread_bps
    }

    #[getter]
    fn index_id(&self) -> Option<&str> {
        self.inner.index_id.as_deref()
    }

    #[getter]
    fn maturity(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.maturity)
    }

    #[getter]
    fn credit_quality(&self) -> Option<String> {
        self.inner.credit_quality.map(|r| format!("{:?}", r))
    }

    #[getter]
    fn industry(&self) -> Option<&str> {
        self.inner.industry.as_deref()
    }

    #[getter]
    fn obligor_id(&self) -> Option<&str> {
        self.inner.obligor_id.as_deref()
    }

    #[getter]
    fn is_defaulted(&self) -> bool {
        self.inner.is_defaulted
    }

    #[getter]
    fn recovery_amount(&self) -> Option<PyMoney> {
        self.inner.recovery_amount.map(PyMoney::new)
    }

    #[getter]
    fn purchase_price(&self) -> Option<PyMoney> {
        self.inner.purchase_price.map(PyMoney::new)
    }

    #[getter]
    fn acquisition_date(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        self.inner
            .acquisition_date
            .map(|d| date_to_py(py, d))
            .transpose()
    }

    #[getter]
    fn day_count(&self) -> &'static str {
        format_day_count(self.inner.day_count)
    }

    #[getter]
    fn smm_override(&self) -> Option<f64> {
        self.inner.smm_override
    }

    #[getter]
    fn mdr_override(&self) -> Option<f64> {
        self.inner.mdr_override
    }

    fn __repr__(&self) -> String {
        format!(
            "PoolAsset(id='{}', type={:?}, balance={}, rate={:.4})",
            self.inner.id, self.inner.asset_type, self.inner.balance, self.inner.rate,
        )
    }
}

// ============================================================================
// REP LINE
// ============================================================================

/// Representative line for aggregated pool modeling.
///
/// Groups similar assets into a single weighted-average line for efficient
/// cashflow projection.
///
/// Examples:
///     >>> import datetime
///     >>> rep = RepLine("REP_0", 50_000_000.0, "USD", 0.055,
///     ...              datetime.date(2030, 6, 15), seasoning_months=24)
///     >>> rep = rep.with_cpr(0.10).with_cdr(0.02)
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "RepLine",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyRepLine {
    pub(crate) inner: RustRepLine,
}

#[pymethods]
impl PyRepLine {
    #[new]
    #[pyo3(signature = (id, balance_amount, currency, rate, maturity, seasoning_months = 0, spread_bps = None, index_id = None))]
    #[pyo3(
        text_signature = "(id, balance_amount, currency, rate, maturity, seasoning_months=0, spread_bps=None, index_id=None)"
    )]
    #[allow(clippy::too_many_arguments)]
    fn new(
        id: &str,
        balance_amount: f64,
        currency: &str,
        rate: f64,
        maturity: Bound<'_, PyAny>,
        seasoning_months: u32,
        spread_bps: Option<f64>,
        index_id: Option<String>,
    ) -> PyResult<Self> {
        let ccy = parse_currency_str(currency)?;
        let mat = py_to_date(&maturity)?;
        let balance = Money::new(balance_amount, ccy);
        let inner = RustRepLine::new(
            id,
            balance,
            rate,
            spread_bps,
            index_id,
            mat,
            seasoning_months,
            DayCount::Act360,
        );
        Ok(Self { inner })
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, data)")]
    fn from_dict(_cls: &Bound<'_, PyType>, data: &Bound<'_, PyAny>) -> PyResult<Self> {
        let json_value: serde_json::Value = pythonize::depythonize(data)
            .map_err(|e| PyValueError::new_err(format!("Failed to convert from Python: {e}")))?;
        let inner: RustRepLine = serde_json::from_value(json_value)
            .map_err(|e| PyValueError::new_err(format!("Failed to deserialize RepLine: {e}")))?;
        Ok(Self { inner })
    }

    #[pyo3(text_signature = "(self)")]
    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let json_str = serde_json::to_string(&self.inner)
            .map_err(|e| PyValueError::new_err(format!("Failed to serialize: {e}")))?;
        let json_value: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| PyValueError::new_err(format!("Failed to parse JSON: {e}")))?;
        pythonize::pythonize(py, &json_value)
            .map(|bound| bound.into())
            .map_err(|e| PyValueError::new_err(format!("Failed to convert to Python: {e}")))
    }

    // -- builder methods --

    #[pyo3(text_signature = "(self, cpr)")]
    fn with_cpr(&self, cpr: f64) -> Self {
        Self {
            inner: self.inner.clone().with_cpr(cpr),
        }
    }

    #[pyo3(text_signature = "(self, cdr)")]
    fn with_cdr(&self, cdr: f64) -> Self {
        Self {
            inner: self.inner.clone().with_cdr(cdr),
        }
    }

    #[pyo3(text_signature = "(self, rate)")]
    fn with_recovery_rate(&self, rate: f64) -> Self {
        Self {
            inner: self.inner.clone().with_recovery_rate(rate),
        }
    }

    // -- computed --

    #[pyo3(text_signature = "(self)")]
    fn spread_bps_value(&self) -> f64 {
        self.inner.spread_bps()
    }

    // -- getters --

    #[getter]
    fn id(&self) -> &str {
        &self.inner.id
    }

    #[getter]
    fn balance(&self) -> PyMoney {
        PyMoney::new(self.inner.balance)
    }

    #[getter]
    fn rate(&self) -> f64 {
        self.inner.rate
    }

    #[getter]
    fn spread_bps(&self) -> Option<f64> {
        self.inner.spread_bps
    }

    #[getter]
    fn index_id(&self) -> Option<&str> {
        self.inner.index_id.as_deref()
    }

    #[getter]
    fn maturity(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.maturity)
    }

    #[getter]
    fn seasoning_months(&self) -> u32 {
        self.inner.seasoning_months
    }

    #[getter]
    fn day_count(&self) -> &'static str {
        format_day_count(self.inner.day_count)
    }

    #[getter]
    fn cpr(&self) -> Option<f64> {
        self.inner.cpr
    }

    #[getter]
    fn cdr(&self) -> Option<f64> {
        self.inner.cdr
    }

    #[getter]
    fn recovery_rate(&self) -> Option<f64> {
        self.inner.recovery_rate
    }

    fn __repr__(&self) -> String {
        format!(
            "RepLine(id='{}', balance={}, rate={:.4})",
            self.inner.id, self.inner.balance, self.inner.rate,
        )
    }
}

// ============================================================================
// POOL
// ============================================================================

/// Structured credit collateral pool containing assets and performance tracking.
///
/// Construct with an id and deal type, then add assets via ``add_asset``.
///
/// Examples:
///     >>> pool = Pool("CLO_2024_1_POOL", "CLO")
///     >>> pool = pool.add_asset(asset)
///     >>> pool.total_balance()
///     Money(amount=10000000, currency='USD')
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "Pool",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyPool {
    pub(crate) inner: RustPool,
}

#[pymethods]
impl PyPool {
    #[new]
    #[pyo3(signature = (id, deal_type, currency = "USD"))]
    #[pyo3(text_signature = "(id, deal_type, currency='USD')")]
    fn new(id: &str, deal_type: &str, currency: &str) -> PyResult<Self> {
        let dt = parse_deal_type(deal_type)?;
        let ccy = parse_currency_str(currency)?;
        let inner = RustPool::new(id, dt, ccy);
        Ok(Self { inner })
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, data)")]
    fn from_dict(_cls: &Bound<'_, PyType>, data: &Bound<'_, PyAny>) -> PyResult<Self> {
        let json_value: serde_json::Value = pythonize::depythonize(data)
            .map_err(|e| PyValueError::new_err(format!("Failed to convert from Python: {e}")))?;
        let inner: RustPool = serde_json::from_value(json_value)
            .map_err(|e| PyValueError::new_err(format!("Failed to deserialize Pool: {e}")))?;
        Ok(Self { inner })
    }

    #[pyo3(text_signature = "(self)")]
    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let json_str = serde_json::to_string(&self.inner)
            .map_err(|e| PyValueError::new_err(format!("Failed to serialize: {e}")))?;
        let json_value: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| PyValueError::new_err(format!("Failed to parse JSON: {e}")))?;
        pythonize::pythonize(py, &json_value)
            .map(|bound| bound.into())
            .map_err(|e| PyValueError::new_err(format!("Failed to convert to Python: {e}")))
    }

    // -- builder methods --

    #[pyo3(text_signature = "(self, asset)")]
    fn add_asset(&self, asset: &PyPoolAsset) -> Self {
        let mut inner = self.inner.clone();
        inner.assets.push(asset.inner.clone());
        Self { inner }
    }

    #[pyo3(text_signature = "(self, as_of)")]
    fn aggregate_to_rep_lines(&self, as_of: Bound<'_, PyAny>) -> PyResult<Self> {
        let d = py_to_date(&as_of)?;
        let mut inner = self.inner.clone();
        inner.aggregate_to_rep_lines(d);
        Ok(Self { inner })
    }

    // -- computed methods --

    #[pyo3(text_signature = "(self)")]
    fn total_balance(&self) -> PyResult<PyMoney> {
        self.inner
            .total_balance()
            .map(PyMoney::new)
            .map_err(core_to_py)
    }

    #[pyo3(text_signature = "(self)")]
    fn performing_balance(&self) -> PyResult<PyMoney> {
        self.inner
            .performing_balance()
            .map(PyMoney::new)
            .map_err(core_to_py)
    }

    #[pyo3(text_signature = "(self)")]
    fn weighted_avg_coupon(&self) -> f64 {
        self.inner.weighted_avg_coupon()
    }

    #[pyo3(text_signature = "(self, as_of)")]
    fn weighted_avg_maturity(&self, as_of: Bound<'_, PyAny>) -> PyResult<f64> {
        let d = py_to_date(&as_of)?;
        Ok(self.inner.weighted_avg_maturity(d))
    }

    #[pyo3(text_signature = "(self)")]
    fn diversity_score(&self) -> f64 {
        self.inner.diversity_score()
    }

    #[pyo3(text_signature = "(self)")]
    fn weighted_avg_spread(&self) -> f64 {
        self.inner.weighted_avg_spread()
    }

    // -- getters --

    #[getter]
    fn pool_id(&self) -> &str {
        self.inner.id.as_str()
    }

    #[getter]
    fn base_currency(&self) -> String {
        format!("{}", self.inner.base_currency())
    }

    #[getter]
    fn deal_type(&self) -> String {
        format!("{:?}", self.inner.deal_type)
    }

    #[getter]
    fn asset_count(&self) -> usize {
        self.inner.assets.len()
    }

    #[getter]
    fn rep_line_count(&self) -> usize {
        self.inner.rep_lines.as_ref().map_or(0, |v| v.len())
    }

    fn __repr__(&self) -> String {
        format!(
            "Pool(id='{}', deal_type={:?}, assets={}, rep_lines={})",
            self.inner.id,
            self.inner.deal_type,
            self.inner.assets.len(),
            self.inner.rep_lines.as_ref().map_or(0, |v| v.len()),
        )
    }
}

// ============================================================================
// POOL STATS
// ============================================================================

/// Read-only pool-level performance statistics.
///
/// Returned by ``calculate_pool_stats``. All fields are floating-point.
///
/// Examples:
///     >>> stats = calculate_pool_stats(pool, datetime.date.today())
///     >>> stats.weighted_avg_coupon
///     0.055
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "PoolStats",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyPoolStats {
    pub(crate) inner: RustPoolStats,
}

#[pymethods]
impl PyPoolStats {
    #[classmethod]
    #[pyo3(text_signature = "(cls, data)")]
    fn from_dict(_cls: &Bound<'_, PyType>, data: &Bound<'_, PyAny>) -> PyResult<Self> {
        let json_value: serde_json::Value = pythonize::depythonize(data)
            .map_err(|e| PyValueError::new_err(format!("Failed to convert from Python: {e}")))?;
        let inner: RustPoolStats = serde_json::from_value(json_value)
            .map_err(|e| PyValueError::new_err(format!("Failed to deserialize PoolStats: {e}")))?;
        Ok(Self { inner })
    }

    #[pyo3(text_signature = "(self)")]
    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let json_str = serde_json::to_string(&self.inner)
            .map_err(|e| PyValueError::new_err(format!("Failed to serialize: {e}")))?;
        let json_value: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| PyValueError::new_err(format!("Failed to parse JSON: {e}")))?;
        pythonize::pythonize(py, &json_value)
            .map(|bound| bound.into())
            .map_err(|e| PyValueError::new_err(format!("Failed to convert to Python: {e}")))
    }

    #[getter]
    fn weighted_avg_coupon(&self) -> f64 {
        self.inner.weighted_avg_coupon
    }

    #[getter]
    fn weighted_avg_spread(&self) -> f64 {
        self.inner.weighted_avg_spread
    }

    #[getter]
    fn weighted_avg_life(&self) -> f64 {
        self.inner.weighted_avg_life
    }

    #[getter]
    fn weighted_avg_maturity(&self) -> f64 {
        self.inner.weighted_avg_maturity
    }

    #[getter]
    fn weighted_avg_rating_factor(&self) -> f64 {
        self.inner.weighted_avg_rating_factor
    }

    #[getter]
    fn diversity_score(&self) -> f64 {
        self.inner.diversity_score
    }

    #[getter]
    fn num_obligors(&self) -> usize {
        self.inner.num_obligors
    }

    #[getter]
    fn num_industries(&self) -> usize {
        self.inner.num_industries
    }

    #[getter]
    fn cumulative_default_rate(&self) -> f64 {
        self.inner.cumulative_default_rate
    }

    #[getter]
    fn recovery_rate(&self) -> f64 {
        self.inner.recovery_rate
    }

    #[getter]
    fn prepayment_rate(&self) -> f64 {
        self.inner.prepayment_rate
    }

    fn __repr__(&self) -> String {
        format!(
            "PoolStats(wac={:.4}, was={:.1}bps, div={:.1}, obligors={}, industries={})",
            self.inner.weighted_avg_coupon,
            self.inner.weighted_avg_spread,
            self.inner.diversity_score,
            self.inner.num_obligors,
            self.inner.num_industries,
        )
    }
}

// ============================================================================
// REINVESTMENT PERIOD
// ============================================================================

/// Reinvestment period configuration.
///
/// Describes the reinvestment window and associated criteria for a CLO deal.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "ReinvestmentPeriod",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyReinvestmentPeriod {
    pub(crate) inner: RustReinvestmentPeriod,
}

#[pymethods]
impl PyReinvestmentPeriod {
    #[classmethod]
    #[pyo3(text_signature = "(cls, data)")]
    fn from_dict(_cls: &Bound<'_, PyType>, data: &Bound<'_, PyAny>) -> PyResult<Self> {
        let json_value: serde_json::Value = pythonize::depythonize(data)
            .map_err(|e| PyValueError::new_err(format!("Failed to convert from Python: {e}")))?;
        let inner: RustReinvestmentPeriod = serde_json::from_value(json_value).map_err(|e| {
            PyValueError::new_err(format!("Failed to deserialize ReinvestmentPeriod: {e}"))
        })?;
        Ok(Self { inner })
    }

    #[pyo3(text_signature = "(self)")]
    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let json_str = serde_json::to_string(&self.inner)
            .map_err(|e| PyValueError::new_err(format!("Failed to serialize: {e}")))?;
        let json_value: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| PyValueError::new_err(format!("Failed to parse JSON: {e}")))?;
        pythonize::pythonize(py, &json_value)
            .map(|bound| bound.into())
            .map_err(|e| PyValueError::new_err(format!("Failed to convert to Python: {e}")))
    }

    #[getter]
    fn end_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.end_date)
    }

    #[getter]
    fn is_active(&self) -> bool {
        self.inner.is_active
    }

    #[getter]
    fn criteria(&self) -> PyReinvestmentCriteria {
        PyReinvestmentCriteria {
            inner: self.inner.criteria.clone(),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "ReinvestmentPeriod(end_date={}, active={})",
            self.inner.end_date, self.inner.is_active,
        )
    }
}

// ============================================================================
// REINVESTMENT CRITERIA
// ============================================================================

/// Criteria governing reinvestment during the revolving period.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "ReinvestmentCriteria",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyReinvestmentCriteria {
    pub(crate) inner: RustReinvestmentCriteria,
}

#[pymethods]
impl PyReinvestmentCriteria {
    #[classmethod]
    #[pyo3(text_signature = "(cls, data)")]
    fn from_dict(_cls: &Bound<'_, PyType>, data: &Bound<'_, PyAny>) -> PyResult<Self> {
        let json_value: serde_json::Value = pythonize::depythonize(data)
            .map_err(|e| PyValueError::new_err(format!("Failed to convert from Python: {e}")))?;
        let inner: RustReinvestmentCriteria = serde_json::from_value(json_value).map_err(|e| {
            PyValueError::new_err(format!("Failed to deserialize ReinvestmentCriteria: {e}"))
        })?;
        Ok(Self { inner })
    }

    #[pyo3(text_signature = "(self)")]
    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let json_str = serde_json::to_string(&self.inner)
            .map_err(|e| PyValueError::new_err(format!("Failed to serialize: {e}")))?;
        let json_value: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| PyValueError::new_err(format!("Failed to parse JSON: {e}")))?;
        pythonize::pythonize(py, &json_value)
            .map(|bound| bound.into())
            .map_err(|e| PyValueError::new_err(format!("Failed to convert to Python: {e}")))
    }

    #[getter]
    fn max_price(&self) -> f64 {
        self.inner.max_price
    }

    #[getter]
    fn min_yield(&self) -> f64 {
        self.inner.min_yield
    }

    #[getter]
    fn maintain_credit_quality(&self) -> bool {
        self.inner.maintain_credit_quality
    }

    #[getter]
    fn maintain_wal(&self) -> bool {
        self.inner.maintain_wal
    }

    #[getter]
    fn apply_eligibility_criteria(&self) -> bool {
        self.inner.apply_eligibility_criteria
    }

    fn __repr__(&self) -> String {
        format!(
            "ReinvestmentCriteria(max_price={:.1}, min_yield={:.4})",
            self.inner.max_price, self.inner.min_yield,
        )
    }
}

// ============================================================================
// CONCENTRATION CHECK RESULT
// ============================================================================

/// Result of concentration limit checking across the pool.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "ConcentrationCheckResult",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyConcentrationCheckResult {
    pub(crate) inner: RustConcentrationCheckResult,
}

#[pymethods]
impl PyConcentrationCheckResult {
    #[getter]
    fn has_violations(&self) -> bool {
        self.inner.has_violations()
    }

    #[getter]
    fn violations(&self) -> Vec<PyConcentrationViolation> {
        self.inner
            .violations
            .iter()
            .map(|v| PyConcentrationViolation { inner: v.clone() })
            .collect()
    }

    #[getter]
    fn violation_count(&self) -> usize {
        self.inner.violations.len()
    }

    fn __repr__(&self) -> String {
        format!(
            "ConcentrationCheckResult(violations={})",
            self.inner.violations.len(),
        )
    }
}

// ============================================================================
// CONCENTRATION VIOLATION
// ============================================================================

/// Individual concentration limit violation.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "ConcentrationViolation",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyConcentrationViolation {
    pub(crate) inner: RustConcentrationViolation,
}

#[pymethods]
impl PyConcentrationViolation {
    #[getter]
    fn violation_type(&self) -> &str {
        &self.inner.violation_type
    }

    #[getter]
    fn identifier(&self) -> &str {
        &self.inner.identifier
    }

    #[getter]
    fn current_level(&self) -> f64 {
        self.inner.current_level
    }

    #[getter]
    fn limit(&self) -> f64 {
        self.inner.limit
    }

    fn __repr__(&self) -> String {
        format!(
            "ConcentrationViolation(type='{}', id='{}', current={:.2}%, limit={:.2}%)",
            self.inner.violation_type,
            self.inner.identifier,
            self.inner.current_level,
            self.inner.limit,
        )
    }
}

// ============================================================================
// STANDALONE FUNCTION
// ============================================================================

/// Calculate current pool statistics for a given as-of date.
///
/// Args:
///     pool: The asset pool to analyze.
///     as_of: Valuation date for maturity calculations.
///
/// Returns:
///     PoolStats: Computed pool-level statistics.
#[pyfunction(name = "calculate_pool_stats", text_signature = "(pool, as_of)")]
fn py_calculate_pool_stats(pool: &PyPool, as_of: Bound<'_, PyAny>) -> PyResult<PyPoolStats> {
    let d = py_to_date(&as_of)?;
    let stats = rust_calculate_pool_stats(&pool.inner, d);
    Ok(PyPoolStats { inner: stats })
}

// ============================================================================
// REGISTRATION
// ============================================================================

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyPoolAsset>()?;
    module.add_class::<PyRepLine>()?;
    module.add_class::<PyPool>()?;
    module.add_class::<PyPoolStats>()?;
    module.add_class::<PyReinvestmentPeriod>()?;
    module.add_class::<PyReinvestmentCriteria>()?;
    module.add_class::<PyConcentrationCheckResult>()?;
    module.add_class::<PyConcentrationViolation>()?;
    module.add_function(wrap_pyfunction!(py_calculate_pool_stats, module)?)?;

    Ok(vec![
        "PoolAsset",
        "RepLine",
        "Pool",
        "PoolStats",
        "ReinvestmentPeriod",
        "ReinvestmentCriteria",
        "ConcentrationCheckResult",
        "ConcentrationViolation",
        "calculate_pool_stats",
    ])
}
