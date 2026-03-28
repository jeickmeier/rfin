use crate::core::common::args::{CurrencyArg, DayCountArg, StubKindArg, TenorArg};
use crate::core::currency::PyCurrency;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::market_data::PyMarketContext;
use crate::core::money::PyMoney;
use crate::errors::{core_to_py, PyContext};
use crate::valuations::cashflow::builder::PyCashFlowSchedule;
use crate::valuations::common::PyInstrumentType;
use crate::valuations::results::PyValuationResult;
use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, DayCount, StubKind, Tenor};
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::cashflow::builder::specs::FloatingRateSpec;
use finstack_valuations::cashflow::builder::FeeTier;
use finstack_valuations::instruments::fixed_income::revolving_credit::{
    BaseRateSpec, DrawRepayEvent, DrawRepaySpec, RevolvingCredit, RevolvingCreditFees,
    StochasticUtilizationSpec, UtilizationProcess,
};
use finstack_valuations::instruments::{Attributes, PricingOverrides};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::{Bound, Py, PyRefMut};
use rust_decimal::Decimal;
use std::fmt;
use std::sync::Arc;

fn parse_day_count_str(s: &str) -> PyResult<DayCount> {
    let n = s.to_ascii_lowercase().replace([' ', '-'], "_");
    match n.as_str() {
        "act360" | "act/360" | "act_360" | "actual/360" => Ok(DayCount::Act360),
        "act365f" | "act/365f" | "act_365f" | "actual/365f" => Ok(DayCount::Act365F),
        "act365l" | "act/365l" | "act_365l" | "actual/365l" => Ok(DayCount::Act365L),
        "30/360" | "30_360" | "thirty/360" | "30u/360" => Ok(DayCount::Thirty360),
        "30e/360" | "30e_360" | "30/360e" => Ok(DayCount::ThirtyE360),
        "actact" | "act/act" | "act_act" | "actual/actual" => Ok(DayCount::ActAct),
        other => Err(PyValueError::new_err(format!("Unknown day-count: {other}"))),
    }
}

/// Revolving credit facility instrument with deterministic and stochastic pricing.
///
/// Models a credit facility with draws/repayments, interest payments on drawn
/// amounts, and fees (commitment, usage, facility, upfront). Supports both
/// deterministic schedules and stochastic utilization via Monte Carlo.
///
/// Examples:
///     >>> from finstack.valuations.instruments import RevolvingCredit
///     >>> import json
///     ///
///     >>> # Create a simple fixed-rate revolver
///     >>> facility_spec = {
///     ...     "id": "RC001",
///     ...     "commitment_amount": {"amount": 100_000_000, "currency": "USD"},
///     ...     "drawn_amount": {"amount": 50_000_000, "currency": "USD"},
///     ...     "commitment_date": "2025-01-01",
///     ...     "maturity_date": "2030-01-01",
///     ...     "base_rate_spec": {"Fixed": {"rate": 0.055}},
///     ...     "day_count": "Act360",
///     ...     "payment_frequency": {"months": 3},
///     ...     "fees": {
///     ...         "upfront_fee": {"amount": 500_000, "currency": "USD"},
///     ...         "commitment_fee_tiers": [{"threshold": 0.0, "bps": 35}],
///     ...         "usage_fee_tiers": [],
///     ...         "facility_fee_bp": 10
///     ...     },
///     ...     "draw_repay_spec": {"Deterministic": []},
///     ...     "discount_curve_id": "USD-OIS",
///     ...     "attributes": {}
///     ... }
///     >>> rc = RevolvingCredit.from_json(json.dumps(facility_spec))
///     >>> rc.instrument_id
///     'RC001'
///     >>> rc.utilization_rate()
///     0.5
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "RevolvingCredit",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyRevolvingCredit {
    pub(crate) inner: Arc<RevolvingCredit>,
}

impl PyRevolvingCredit {
    pub(crate) fn new(inner: RevolvingCredit) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pymethods]
impl PyRevolvingCredit {
    #[classmethod]
    #[pyo3(text_signature = "(cls, json_str)")]
    /// Create a revolving credit facility from a JSON string specification.
    ///
    /// The JSON should match the RevolvingCredit schema from finstack-valuations.
    /// This is the recommended way to create facilities with complex features like
    /// stochastic utilization, tiered fees, and multi-factor Monte Carlo.
    ///
    /// Args:
    ///     json_str: JSON string matching the RevolvingCredit schema.
    ///
    /// Returns:
    ///     RevolvingCredit: Configured revolving credit facility.
    ///
    /// Raises:
    ///     ValueError: If JSON cannot be parsed or is invalid.
    ///
    /// Examples:
    ///     >>> import json
    ///     >>> spec = {
    ///     ...     "id": "RC001",
    ///     ...     "commitment_amount": {"amount": 100_000_000, "currency": "USD"},
    ///     ...     "drawn_amount": {"amount": 0, "currency": "USD"},
    ///     ...     "commitment_date": "2025-01-01",
    ///     ...     "maturity_date": "2027-01-01",
    ///     ...     "base_rate_spec": {"Fixed": {"rate": 0.05}},
    ///     ...     "day_count": "Act360",
    ///     ...     "payment_frequency": {"months": 3},
    ///     ...     "fees": {"facility_fee_bp": 25},
    ///     ...     "draw_repay_spec": {"Deterministic": []},
    ///     ...     "discount_curve_id": "USD-OIS",
    ///     ...     "attributes": {}
    ///     ... }
    ///     >>> rc = RevolvingCredit.from_json(json.dumps(spec))
    fn from_json(_cls: &Bound<'_, PyType>, json_str: &str) -> PyResult<Self> {
        serde_json::from_str(json_str)
            .map(Self::new)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    /// Serialize the revolving credit facility to a JSON string.
    ///
    /// Returns:
    ///     str: JSON representation of the facility.
    ///
    /// Examples:
    ///     >>> json_str = rc.to_json()
    ///     >>> # Can be saved to file or transmitted
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string_pretty(&self.inner)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    /// Instrument identifier.
    ///
    /// Returns:
    ///     str: Unique identifier assigned to the facility.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Total commitment amount (maximum drawable).
    ///
    /// Returns:
    ///     Money: Total commitment as :class:`finstack.core.money.Money`.
    #[getter]
    fn commitment_amount(&self) -> PyMoney {
        PyMoney::new(self.inner.commitment_amount)
    }

    /// Current drawn amount (initial utilization).
    ///
    /// Returns:
    ///     Money: Currently drawn amount as :class:`finstack.core.money.Money`.
    #[getter]
    fn drawn_amount(&self) -> PyMoney {
        PyMoney::new(self.inner.drawn_amount)
    }

    /// Commitment date (facility start date).
    ///
    /// Returns:
    ///     datetime.date: Commitment date converted to Python.
    #[getter]
    fn commitment_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.commitment_date)
    }

    /// Maturity date (facility expiration).
    ///
    /// Returns:
    ///     datetime.date: Maturity date converted to Python.
    #[getter]
    fn maturity_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.maturity)
    }

    /// Currency for all cashflows.
    ///
    /// Returns:
    ///     Currency: Currency object.
    #[getter]
    fn currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.commitment_amount.currency())
    }

    /// Discount curve identifier.
    ///
    /// Returns:
    ///     str: Identifier for the discount curve.
    #[getter]
    fn discount_curve(&self) -> String {
        self.inner.discount_curve_id.as_str().to_string()
    }

    /// Optional hazard curve identifier for credit risk modeling.
    ///
    /// Returns:
    ///     Optional[str]: Hazard curve ID if present, None otherwise.
    #[getter]
    fn hazard_curve(&self) -> Option<String> {
        self.inner
            .credit_curve_id
            .as_ref()
            .map(|id| id.as_str().to_string())
    }

    /// Recovery rate on default.
    ///
    /// Returns:
    ///     float: Recovery rate (0.0 to 1.0).
    #[getter]
    fn recovery_rate(&self) -> f64 {
        self.inner.recovery_rate
    }

    /// Instrument type enum (``InstrumentType.REVOLVING_CREDIT``).
    ///
    /// Returns:
    ///     InstrumentType: Enumeration value identifying the instrument family.
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::RevolvingCredit)
    }

    /// Calculate current utilization rate (drawn / commitment).
    ///
    /// Returns:
    ///     float: Utilization rate between 0.0 and 1.0.
    ///
    /// Examples:
    ///     >>> rc.utilization_rate()
    ///     0.5  # 50% utilized
    fn utilization_rate(&self) -> f64 {
        self.inner.utilization_rate()
    }

    /// Calculate current undrawn amount (available capacity).
    ///
    /// Returns:
    ///     Money: Undrawn amount as :class:`finstack.core.money.Money`.
    ///
    /// Raises:
    ///     ValueError: If drawn amount exceeds commitment.
    ///
    /// Examples:
    ///     >>> undrawn = rc.undrawn_amount()
    ///     >>> print(f"Available: {undrawn}")
    fn undrawn_amount(&self) -> PyResult<PyMoney> {
        self.inner
            .undrawn_amount()
            .map(PyMoney::new)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    /// Check if the facility uses deterministic cashflows.
    ///
    /// Returns:
    ///     bool: True if using deterministic draw/repay schedule.
    fn is_deterministic(&self) -> bool {
        self.inner.is_deterministic()
    }

    /// Check if the facility uses stochastic utilization.
    ///
    /// Returns:
    ///     bool: True if using Monte Carlo simulation.
    fn is_stochastic(&self) -> bool {
        self.inner.is_stochastic()
    }

    /// Price the facility using the standard value() method.
    ///
    /// For deterministic facilities, prices directly. For stochastic facilities,
    /// falls back to deterministic pricing with empty draw schedule (for fast path).
    /// Use price_with_paths() for full Monte Carlo with path capture.
    ///
    /// Args:
    ///     market: Market context with required curves.
    ///     as_of: Valuation date.
    ///
    /// Returns:
    ///     Money: Present value as :class:`finstack.core.money.Money`.
    ///
    /// Raises:
    ///     ValueError: If required curves are missing or valuation fails.
    ///
    /// Examples:
    ///     >>> from datetime import date
    ///     >>> pv = rc.value(market, date.today())
    ///     >>> print(f"PV: {pv}")
    fn value(
        &self,
        py: Python<'_>,
        market: &PyMarketContext,
        as_of: Bound<'_, pyo3::PyAny>,
    ) -> PyResult<PyMoney> {
        use crate::core::dates::utils::py_to_date;
        use finstack_valuations::instruments::internal::InstrumentExt as Instrument;

        let as_of_date = py_to_date(&as_of)?;
        let value = py
            .detach(|| self.inner.value(&market.inner, as_of_date))
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        Ok(PyMoney::new(value))
    }

    /// Price with requested risk metrics.
    ///
    /// Calculates present value along with requested metrics like DV01, CS01, etc.
    ///
    /// Args:
    ///     market: Market context with required curves.
    ///     as_of: Valuation date.
    ///     metrics: List of metric identifiers (e.g., ["DV01", "CS01"]).
    ///
    /// Returns:
    ///     ValuationResult: Result with value and computed metrics.
    ///
    /// Raises:
    ///     ValueError: If required curves are missing or valuation fails.
    ///
    /// Examples:
    ///     >>> result = rc.price_with_metrics(market, date.today(), ["DV01", "CS01"], finstack_valuations::instruments::PricingOptions::default())
    ///     >>> print(f"PV: {result.value}")
    ///     >>> print(f"DV01: {result.metrics['DV01']}")
    fn price_with_metrics(
        &self,
        market: &PyMarketContext,
        as_of: Bound<'_, pyo3::PyAny>,
        metrics: Vec<String>,
    ) -> PyResult<PyValuationResult> {
        use crate::core::dates::utils::py_to_date;
        use finstack_valuations::instruments::internal::InstrumentExt as Instrument;
        use finstack_valuations::metrics::MetricId;

        let as_of_date = py_to_date(&as_of)?;
        let metric_ids: Vec<MetricId> = metrics
            .iter()
            .map(|s| s.parse().unwrap_or_else(|_| MetricId::custom(s)))
            .collect();

        self.inner
            .price_with_metrics(
                &market.inner,
                as_of_date,
                &metric_ids,
                finstack_valuations::instruments::PricingOptions::default(),
            )
            .map(PyValuationResult::new)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    /// Build cashflow schedule for deterministic facilities.
    ///
    /// Generates the complete cashflow schedule including interest payments,
    /// fees, draws, and repayments. Only works for deterministic specifications.
    ///
    /// Args:
    ///     market: Market context with required curves.
    ///     as_of: Valuation date.
    ///
    /// Returns:
    ///     CashFlowSchedule: Detailed cashflow schedule.
    ///
    /// Raises:
    ///     ValueError: If facility is stochastic or valuation fails.
    ///
    /// Examples:
    ///     >>> schedule = rc.cashflow_schedule(market, date.today())
    ///     >>> for flow in schedule.flows:
    ///     ...     print(f"{flow.date}: {flow.amount} - {flow.description}")
    fn cashflow_schedule(
        &self,
        market: &PyMarketContext,
        as_of: Bound<'_, pyo3::PyAny>,
    ) -> PyResult<PyCashFlowSchedule> {
        use crate::core::dates::utils::py_to_date;
        use finstack_valuations::cashflow::CashflowProvider;

        let as_of_date = py_to_date(&as_of)?;
        self.inner
            .cashflow_schedule(&market.inner, as_of_date)
            .map(PyCashFlowSchedule::new)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    /// Price deterministically (explicit method for API clarity).
    ///
    /// Forces deterministic pricing even if the facility has a stochastic spec
    /// (treats as empty draw schedule). For true Monte Carlo, use price_with_paths().
    ///
    /// Args:
    ///     market: Market context with required curves.
    ///     as_of: Valuation date.
    ///
    /// Returns:
    ///     Money: Present value as :class:`finstack.core.money.Money`.
    ///
    /// Raises:
    ///     ValueError: If required curves are missing or valuation fails.
    fn price_deterministic(
        &self,
        market: &PyMarketContext,
        as_of: Bound<'_, pyo3::PyAny>,
    ) -> PyResult<PyMoney> {
        use crate::core::dates::utils::py_to_date;
        use finstack_valuations::instruments::internal::InstrumentExt as Instrument;

        let as_of_date = py_to_date(&as_of)?;
        self.inner
            .value(&market.inner, as_of_date)
            .map(PyMoney::new)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    /// Price with full Monte Carlo path capture for distribution analysis.
    ///
    /// Runs Monte Carlo simulation and returns detailed results including
    /// individual path PVs, cashflows, and 3-factor path data for analysis.
    ///
    /// Only available when the facility uses a Stochastic draw/repay specification.
    ///
    /// Args:
    ///     market: Market context with required curves.
    ///     as_of: Valuation date.
    ///
    /// Returns:
    ///     EnhancedMonteCarloResult: Full MC results with path details.
    ///
    /// Raises:
    ///     ValueError: If facility is not stochastic or MC fails.
    ///
    /// Examples:
    ///     >>> result = rc.price_with_paths(market, date.today())
    ///     >>> print(f"Mean PV: {result.mean}")
    ///     >>> print(f"Std Error: {result.std_error}")
    ///     >>> # Analyze individual paths
    ///     >>> for path in result.path_results[:10]:
    ///     ...     print(f"Path PV: {path.pv}")
    fn price_with_paths(
        &self,
        market: &PyMarketContext,
        as_of: Bound<'_, pyo3::PyAny>,
    ) -> PyResult<PyEnhancedMonteCarloResult> {
        use crate::core::dates::utils::py_to_date;
        use finstack_valuations::instruments::fixed_income::revolving_credit::RevolvingCreditPricer;

        let as_of_date = py_to_date(&as_of)?;
        RevolvingCreditPricer::price_with_paths(&self.inner, &market.inner, as_of_date)
            .map(PyEnhancedMonteCarloResult::new)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    /// Start a fluent builder (``RevolvingCredit.builder("ID")``).
    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_id)")]
    fn builder<'py>(
        cls: &Bound<'py, PyType>,
        instrument_id: &str,
    ) -> PyResult<Py<PyRevolvingCreditBuilder>> {
        let py = cls.py();
        Py::new(
            py,
            PyRevolvingCreditBuilder::new_with_id(InstrumentId::new(instrument_id)),
        )
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "RevolvingCredit(id='{}', commitment={}, drawn={}, commitment_date='{}', maturity='{}')",
            self.inner.id,
            self.inner.commitment_amount,
            self.inner.drawn_amount,
            self.inner.commitment_date,
            self.inner.maturity
        ))
    }
}

impl fmt::Display for PyRevolvingCredit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "RevolvingCredit({}, {} -> {}, util={:.1}%)",
            self.inner.id,
            self.inner.commitment_date,
            self.inner.maturity,
            self.inner.utilization_rate() * 100.0
        )
    }
}

/// Enhanced Monte Carlo result with full path details.
///
/// Contains Monte Carlo statistics (mean, std error, confidence interval)
/// along with individual path results for distribution analysis and visualization.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "EnhancedMonteCarloResult"
)]
pub struct PyEnhancedMonteCarloResult {
    inner:
        finstack_valuations::instruments::fixed_income::revolving_credit::EnhancedMonteCarloResult,
}

impl PyEnhancedMonteCarloResult {
    pub(crate) fn new(
        inner: finstack_valuations::instruments::fixed_income::revolving_credit::EnhancedMonteCarloResult,
    ) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyEnhancedMonteCarloResult {
    /// Mean present value across all paths.
    ///
    /// Returns:
    ///     Money: Mean PV estimate.
    #[getter]
    fn mean(&self) -> PyMoney {
        PyMoney::new(self.inner.mc_result.estimate.mean)
    }

    /// Standard error of the mean.
    ///
    /// Returns:
    ///     float: Standard error in currency units.
    #[getter]
    fn std_error(&self) -> f64 {
        self.inner.mc_result.estimate.stderr
    }

    /// Lower bound of 95% confidence interval.
    ///
    /// Returns:
    ///     Money: Lower confidence bound.
    #[getter]
    fn ci_lower(&self) -> PyMoney {
        PyMoney::new(self.inner.mc_result.estimate.ci_95.0)
    }

    /// Upper bound of 95% confidence interval.
    ///
    /// Returns:
    ///     Money: Upper confidence bound.
    #[getter]
    fn ci_upper(&self) -> PyMoney {
        PyMoney::new(self.inner.mc_result.estimate.ci_95.1)
    }

    /// Number of simulated paths.
    ///
    /// Returns:
    ///     int: Path count.
    #[getter]
    fn num_paths(&self) -> usize {
        self.inner.path_results.len()
    }

    /// Individual path results for distribution analysis.
    ///
    /// Returns:
    ///     List[PathResult]: List of path results with PV, cashflows, and factor data.
    #[getter]
    fn path_results(&self) -> Vec<PyPathResult> {
        self.inner
            .path_results
            .iter()
            .map(|pr| PyPathResult::new(pr.clone()))
            .collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "EnhancedMonteCarloResult(mean={}, stderr={}, paths={})",
            self.inner.mc_result.estimate.mean,
            self.inner.mc_result.estimate.stderr,
            self.inner.path_results.len()
        )
    }
}

/// Individual path result from Monte Carlo simulation.
///
/// Contains the present value, optional 3-factor path data, and cashflow schedule
/// for a single simulated path.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "PathResult",
    from_py_object
)]
#[derive(Clone)]
pub struct PyPathResult {
    inner: finstack_valuations::instruments::fixed_income::revolving_credit::PathResult,
}

impl PyPathResult {
    pub(crate) fn new(
        inner: finstack_valuations::instruments::fixed_income::revolving_credit::PathResult,
    ) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyPathResult {
    /// Present value for this path.
    ///
    /// Returns:
    ///     Money: Path PV.
    #[getter]
    fn pv(&self) -> PyMoney {
        PyMoney::new(self.inner.pv)
    }

    /// Cashflow schedule for this path.
    ///
    /// Returns:
    ///     CashFlowSchedule: Detailed cashflows.
    #[getter]
    fn cashflows(&self) -> PyCashFlowSchedule {
        PyCashFlowSchedule::new(self.inner.cashflows.clone())
    }

    /// Optional 3-factor path data (utilization, credit spread, short rate).
    ///
    /// Returns:
    ///     Optional[ThreeFactorPathData]: Path data if available.
    #[getter]
    fn path_data(&self) -> Option<PyThreeFactorPathData> {
        self.inner
            .path_data
            .as_ref()
            .map(|pd| PyThreeFactorPathData::new(pd.clone()))
    }

    fn __repr__(&self) -> String {
        format!(
            "PathResult(pv={}, num_flows={})",
            self.inner.pv,
            self.inner.cashflows.flows.len()
        )
    }
}

/// Three-factor path data from Monte Carlo simulation.
///
/// Contains the simulated time series for utilization rate, credit spread,
/// and short rate factors, along with time points and payment dates.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "ThreeFactorPathData",
    from_py_object
)]
#[derive(Clone)]
pub struct PyThreeFactorPathData {
    inner: finstack_valuations::instruments::fixed_income::revolving_credit::ThreeFactorPathData,
}

impl PyThreeFactorPathData {
    pub(crate) fn new(
        inner: finstack_valuations::instruments::fixed_income::revolving_credit::ThreeFactorPathData,
    ) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyThreeFactorPathData {
    /// Utilization rate path (0.0 to 1.0).
    ///
    /// Returns:
    ///     List[float]: Utilization rates at each time point.
    #[getter]
    fn utilization_path(&self) -> Vec<f64> {
        self.inner.utilization_path.clone()
    }

    /// Credit spread path (annualized).
    ///
    /// Returns:
    ///     List[float]: Credit spreads at each time point.
    #[getter]
    fn credit_spread_path(&self) -> Vec<f64> {
        self.inner.credit_spread_path.clone()
    }

    /// Short rate path (annualized).
    ///
    /// Returns:
    ///     List[float]: Short rates at each time point.
    #[getter]
    fn short_rate_path(&self) -> Vec<f64> {
        self.inner.short_rate_path.clone()
    }

    /// Time points (in years from as_of date).
    ///
    /// Returns:
    ///     List[float]: Time points for factor values.
    #[getter]
    fn time_points(&self) -> Vec<f64> {
        self.inner.time_points.clone()
    }

    /// Payment dates corresponding to time points.
    ///
    /// Returns:
    ///     List[date]: Payment dates.
    #[getter]
    fn payment_dates(&self, py: Python<'_>) -> PyResult<Vec<Py<PyAny>>> {
        self.inner
            .payment_dates
            .iter()
            .map(|d| date_to_py(py, *d))
            .collect()
    }

    fn __repr__(&self) -> String {
        let avg_util = self.inner.utilization_path.iter().sum::<f64>()
            / self.inner.utilization_path.len() as f64;
        let avg_spread = self.inner.credit_spread_path.iter().sum::<f64>()
            / self.inner.credit_spread_path.len() as f64;
        format!(
            "ThreeFactorPathData(time_points={}, avg_util={:.1}%, avg_spread={:.2}%)",
            self.inner.time_points.len(),
            avg_util * 100.0,
            avg_spread * 100.0
        )
    }
}

// ============================================================================
// Builder-supporting types
// ============================================================================

/// A single fee tier (utilization threshold -> basis points).
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "FeeTier",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyFeeTier {
    pub(crate) inner: FeeTier,
}

#[pymethods]
impl PyFeeTier {
    #[new]
    #[pyo3(text_signature = "(threshold, bps)")]
    fn new_py(threshold: f64, bps: f64) -> PyResult<Self> {
        use crate::valuations::common::f64_to_decimal;
        Ok(Self {
            inner: FeeTier {
                threshold: f64_to_decimal(threshold, "threshold")?,
                bps: f64_to_decimal(bps, "bps")?,
            },
        })
    }

    #[getter]
    fn threshold(&self) -> f64 {
        self.inner.threshold.to_string().parse().unwrap_or(0.0)
    }

    #[getter]
    fn bps(&self) -> f64 {
        self.inner.bps.to_string().parse().unwrap_or(0.0)
    }

    fn __repr__(&self) -> String {
        format!(
            "FeeTier(threshold={}, bps={})",
            self.inner.threshold, self.inner.bps
        )
    }
}

/// Base rate specification (fixed or floating).
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "BaseRateSpec",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyBaseRateSpec {
    pub(crate) inner: BaseRateSpec,
}

#[pymethods]
impl PyBaseRateSpec {
    /// Create a fixed-rate spec.
    #[classmethod]
    #[pyo3(text_signature = "(cls, rate)")]
    fn fixed(_cls: &Bound<'_, PyType>, rate: f64) -> Self {
        Self {
            inner: BaseRateSpec::Fixed { rate },
        }
    }

    /// Create a floating-rate spec with simplified parameters.
    #[classmethod]
    #[pyo3(
        text_signature = "(cls, index_id, spread_bp, reset_freq='3M', day_count='ACT360', calendar_id='weekends_only')",
        signature = (index_id, spread_bp, reset_freq="3M", day_count="ACT360", calendar_id="weekends_only")
    )]
    fn floating(
        _cls: &Bound<'_, PyType>,
        index_id: &str,
        spread_bp: f64,
        reset_freq: &str,
        day_count: &str,
        calendar_id: &str,
    ) -> PyResult<Self> {
        let freq: Tenor = reset_freq
            .parse()
            .map_err(|e| PyValueError::new_err(format!("Invalid reset_freq: {e}")))?;
        let dc = parse_day_count_str(day_count)?;
        let spec = FloatingRateSpec {
            index_id: CurveId::new(index_id),
            spread_bp: crate::valuations::common::f64_to_decimal(spread_bp, "spread_bp")?,
            gearing: Decimal::ONE,
            gearing_includes_spread: true,
            floor_bp: None,
            all_in_floor_bp: None,
            cap_bp: None,
            index_cap_bp: None,
            reset_freq: freq,
            reset_lag_days: 2,
            dc,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: calendar_id.to_string(),
            fixing_calendar_id: None,
            end_of_month: false,
            overnight_compounding: None,
            fallback: Default::default(),
            payment_lag_days: 0,
        };
        Ok(Self {
            inner: BaseRateSpec::Floating(spec),
        })
    }

    #[getter]
    fn spec_type(&self) -> &'static str {
        match &self.inner {
            BaseRateSpec::Fixed { .. } => "fixed",
            BaseRateSpec::Floating(_) => "floating",
        }
    }

    #[getter]
    fn rate(&self) -> Option<f64> {
        match &self.inner {
            BaseRateSpec::Fixed { rate } => Some(*rate),
            BaseRateSpec::Floating(_) => None,
        }
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            BaseRateSpec::Fixed { rate } => format!("BaseRateSpec.fixed({rate})"),
            BaseRateSpec::Floating(s) => {
                format!(
                    "BaseRateSpec.floating('{}', {})",
                    s.index_id.as_str(),
                    s.spread_bp
                )
            }
        }
    }
}

/// Fee structure for a revolving credit facility.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "RevolvingCreditFees",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyRevolvingCreditFees {
    pub(crate) inner: RevolvingCreditFees,
}

#[pymethods]
impl PyRevolvingCreditFees {
    #[new]
    #[pyo3(
        text_signature = "(facility_fee_bp, commitment_fee_tiers=None, usage_fee_tiers=None, upfront_fee=None)",
        signature = (facility_fee_bp, commitment_fee_tiers=None, usage_fee_tiers=None, upfront_fee=None)
    )]
    fn new_py(
        facility_fee_bp: f64,
        commitment_fee_tiers: Option<Vec<PyRef<'_, PyFeeTier>>>,
        usage_fee_tiers: Option<Vec<PyRef<'_, PyFeeTier>>>,
        upfront_fee: Option<PyRef<'_, PyMoney>>,
    ) -> Self {
        Self {
            inner: RevolvingCreditFees {
                upfront_fee: upfront_fee.map(|m| m.inner),
                commitment_fee_tiers: commitment_fee_tiers
                    .map(|v| v.iter().map(|t| t.inner.clone()).collect())
                    .unwrap_or_default(),
                usage_fee_tiers: usage_fee_tiers
                    .map(|v| v.iter().map(|t| t.inner.clone()).collect())
                    .unwrap_or_default(),
                facility_fee_bp,
            },
        }
    }

    /// Convenience: create flat (non-tiered) fees.
    #[classmethod]
    #[pyo3(text_signature = "(cls, commitment_fee_bp, usage_fee_bp, facility_fee_bp)")]
    fn flat(
        _cls: &Bound<'_, PyType>,
        commitment_fee_bp: f64,
        usage_fee_bp: f64,
        facility_fee_bp: f64,
    ) -> Self {
        Self {
            inner: RevolvingCreditFees::flat(commitment_fee_bp, usage_fee_bp, facility_fee_bp),
        }
    }

    #[getter]
    fn facility_fee_bp(&self) -> f64 {
        self.inner.facility_fee_bp
    }

    #[getter]
    fn commitment_fee_tiers(&self) -> Vec<PyFeeTier> {
        self.inner
            .commitment_fee_tiers
            .iter()
            .map(|t| PyFeeTier { inner: t.clone() })
            .collect()
    }

    #[getter]
    fn usage_fee_tiers(&self) -> Vec<PyFeeTier> {
        self.inner
            .usage_fee_tiers
            .iter()
            .map(|t| PyFeeTier { inner: t.clone() })
            .collect()
    }

    #[getter]
    fn upfront_fee(&self) -> Option<PyMoney> {
        self.inner.upfront_fee.map(PyMoney::new)
    }

    fn __repr__(&self) -> String {
        format!(
            "RevolvingCreditFees(facility_fee_bp={}, commitment_tiers={}, usage_tiers={})",
            self.inner.facility_fee_bp,
            self.inner.commitment_fee_tiers.len(),
            self.inner.usage_fee_tiers.len()
        )
    }
}

/// A single draw or repayment event.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "DrawRepayEvent",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyDrawRepayEvent {
    pub(crate) inner: DrawRepayEvent,
}

#[pymethods]
impl PyDrawRepayEvent {
    #[new]
    #[pyo3(text_signature = "(date, amount, currency, is_draw)")]
    fn new_py(
        date: Bound<'_, PyAny>,
        amount: f64,
        currency: CurrencyArg,
        is_draw: bool,
    ) -> PyResult<Self> {
        let d = py_to_date(&date).context("DrawRepayEvent date")?;
        Ok(Self {
            inner: DrawRepayEvent {
                date: d,
                amount: Money::new(amount, currency.0),
                is_draw,
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

    #[getter]
    fn is_draw(&self) -> bool {
        self.inner.is_draw
    }

    fn __repr__(&self) -> String {
        let kind = if self.inner.is_draw { "Draw" } else { "Repay" };
        format!(
            "DrawRepayEvent({kind}, {}, {})",
            self.inner.date, self.inner.amount
        )
    }
}

/// Utilization process for stochastic simulation.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "UtilizationProcess",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyUtilizationProcess {
    pub(crate) inner: UtilizationProcess,
}

#[pymethods]
impl PyUtilizationProcess {
    /// Create a mean-reverting Ornstein-Uhlenbeck utilization process.
    #[classmethod]
    #[pyo3(text_signature = "(cls, target_rate, speed, volatility)")]
    fn mean_reverting(
        _cls: &Bound<'_, PyType>,
        target_rate: f64,
        speed: f64,
        volatility: f64,
    ) -> Self {
        Self {
            inner: UtilizationProcess::MeanReverting {
                target_rate,
                speed,
                volatility,
            },
        }
    }

    #[getter]
    fn process_type(&self) -> &'static str {
        match &self.inner {
            UtilizationProcess::MeanReverting { .. } => "mean_reverting",
        }
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            UtilizationProcess::MeanReverting {
                target_rate,
                speed,
                volatility,
            } => format!(
                "UtilizationProcess.mean_reverting(target={target_rate}, speed={speed}, vol={volatility})"
            ),
        }
    }
}

/// Stochastic utilization specification for Monte Carlo.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "StochasticUtilizationSpec",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyStochasticUtilizationSpec {
    pub(crate) inner: StochasticUtilizationSpec,
}

#[pymethods]
impl PyStochasticUtilizationSpec {
    #[new]
    #[pyo3(
        text_signature = "(process, num_paths, seed=None, antithetic=False, use_sobol_qmc=False)",
        signature = (process, num_paths, seed=None, antithetic=false, use_sobol_qmc=false)
    )]
    fn new_py(
        process: &PyUtilizationProcess,
        num_paths: usize,
        seed: Option<u64>,
        antithetic: bool,
        use_sobol_qmc: bool,
    ) -> Self {
        Self {
            inner: StochasticUtilizationSpec {
                utilization_process: process.inner.clone(),
                num_paths,
                seed,
                antithetic,
                use_sobol_qmc,
                mc_config: None,
            },
        }
    }

    #[getter]
    fn num_paths(&self) -> usize {
        self.inner.num_paths
    }

    #[getter]
    fn seed(&self) -> Option<u64> {
        self.inner.seed
    }

    #[getter]
    fn antithetic(&self) -> bool {
        self.inner.antithetic
    }

    #[getter]
    fn use_sobol_qmc(&self) -> bool {
        self.inner.use_sobol_qmc
    }

    fn __repr__(&self) -> String {
        format!(
            "StochasticUtilizationSpec(paths={}, seed={:?})",
            self.inner.num_paths, self.inner.seed
        )
    }
}

/// Draw/repay specification (deterministic schedule or stochastic).
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "DrawRepaySpec",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyDrawRepaySpec {
    pub(crate) inner: DrawRepaySpec,
}

#[pymethods]
impl PyDrawRepaySpec {
    /// Create a deterministic draw/repay specification from a list of events.
    #[classmethod]
    #[pyo3(text_signature = "(cls, events)")]
    fn deterministic(_cls: &Bound<'_, PyType>, events: Vec<PyRef<'_, PyDrawRepayEvent>>) -> Self {
        Self {
            inner: DrawRepaySpec::Deterministic(events.iter().map(|e| e.inner.clone()).collect()),
        }
    }

    /// Create a deterministic spec with an empty schedule (no draws/repays).
    #[classmethod]
    fn empty(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: DrawRepaySpec::Deterministic(Vec::new()),
        }
    }

    /// Create a stochastic draw/repay specification for Monte Carlo.
    #[classmethod]
    #[pyo3(text_signature = "(cls, spec)")]
    fn stochastic(_cls: &Bound<'_, PyType>, spec: &PyStochasticUtilizationSpec) -> Self {
        Self {
            inner: DrawRepaySpec::Stochastic(Box::new(spec.inner.clone())),
        }
    }

    #[getter]
    fn spec_type(&self) -> &'static str {
        match &self.inner {
            DrawRepaySpec::Deterministic(_) => "deterministic",
            DrawRepaySpec::Stochastic(_) => "stochastic",
        }
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            DrawRepaySpec::Deterministic(events) => {
                format!("DrawRepaySpec.deterministic({} events)", events.len())
            }
            DrawRepaySpec::Stochastic(spec) => {
                format!("DrawRepaySpec.stochastic({} paths)", spec.num_paths)
            }
        }
    }
}

// ============================================================================
// RevolvingCreditBuilder
// ============================================================================

/// Fluent builder for constructing a RevolvingCredit instrument.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "RevolvingCreditBuilder",
    unsendable,
    skip_from_py_object
)]
pub struct PyRevolvingCreditBuilder {
    instrument_id: InstrumentId,
    pending_commitment_amount: Option<f64>,
    pending_drawn_amount: Option<f64>,
    pending_currency: Option<Currency>,
    commitment_date: Option<time::Date>,
    maturity: Option<time::Date>,
    base_rate_spec: Option<BaseRateSpec>,
    day_count: DayCount,
    frequency: Tenor,
    fees: Option<RevolvingCreditFees>,
    draw_repay_spec: Option<DrawRepaySpec>,
    discount_curve_id: Option<CurveId>,
    credit_curve_id: Option<CurveId>,
    recovery_rate: f64,
    stub: StubKind,
}

impl PyRevolvingCreditBuilder {
    fn new_with_id(id: InstrumentId) -> Self {
        Self {
            instrument_id: id,
            pending_commitment_amount: None,
            pending_drawn_amount: None,
            pending_currency: None,
            commitment_date: None,
            maturity: None,
            base_rate_spec: None,
            day_count: DayCount::Act360,
            frequency: Tenor::quarterly(),
            fees: None,
            draw_repay_spec: None,
            discount_curve_id: None,
            credit_curve_id: None,
            recovery_rate: 0.0,
            stub: StubKind::ShortFront,
        }
    }

    fn ensure_ready(&self) -> PyResult<()> {
        if self.pending_commitment_amount.is_none() || self.pending_currency.is_none() {
            return Err(PyValueError::new_err(
                "commitment_amount() and currency() must be set before build().",
            ));
        }
        if self.commitment_date.is_none() {
            return Err(PyValueError::new_err(
                "commitment_date() must be set before build().",
            ));
        }
        if self.maturity.is_none() {
            return Err(PyValueError::new_err(
                "maturity() must be set before build().",
            ));
        }
        if self.discount_curve_id.is_none() {
            return Err(PyValueError::new_err(
                "disc_id() must be set before build().",
            ));
        }
        Ok(())
    }
}

#[pymethods]
impl PyRevolvingCreditBuilder {
    #[new]
    #[pyo3(text_signature = "(instrument_id)")]
    fn new_py(instrument_id: &str) -> Self {
        Self::new_with_id(InstrumentId::new(instrument_id))
    }

    #[pyo3(text_signature = "($self, amount)")]
    fn commitment_amount(mut slf: PyRefMut<'_, Self>, amount: f64) -> PyRefMut<'_, Self> {
        slf.pending_commitment_amount = Some(amount);
        slf
    }

    #[pyo3(text_signature = "($self, amount)")]
    fn drawn_amount(mut slf: PyRefMut<'_, Self>, amount: f64) -> PyRefMut<'_, Self> {
        slf.pending_drawn_amount = Some(amount);
        slf
    }

    #[pyo3(text_signature = "($self, currency)")]
    fn currency(mut slf: PyRefMut<'_, Self>, currency: CurrencyArg) -> PyRefMut<'_, Self> {
        slf.pending_currency = Some(currency.0);
        slf
    }

    #[pyo3(text_signature = "($self, date)")]
    fn commitment_date<'py>(
        mut slf: PyRefMut<'py, Self>,
        date: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.commitment_date = Some(py_to_date(&date).context("commitment_date")?);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, date)")]
    fn maturity<'py>(
        mut slf: PyRefMut<'py, Self>,
        date: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.maturity = Some(py_to_date(&date).context("maturity")?);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, spec)")]
    fn base_rate<'py>(mut slf: PyRefMut<'py, Self>, spec: &PyBaseRateSpec) -> PyRefMut<'py, Self> {
        slf.base_rate_spec = Some(spec.inner.clone());
        slf
    }

    #[pyo3(text_signature = "($self, day_count)")]
    fn day_count(mut slf: PyRefMut<'_, Self>, day_count: DayCountArg) -> PyRefMut<'_, Self> {
        slf.day_count = day_count.0;
        slf
    }

    #[pyo3(text_signature = "($self, frequency)")]
    fn frequency(mut slf: PyRefMut<'_, Self>, frequency: TenorArg) -> PyRefMut<'_, Self> {
        slf.frequency = frequency.0;
        slf
    }

    #[pyo3(text_signature = "($self, fees)")]
    fn fees<'py>(
        mut slf: PyRefMut<'py, Self>,
        fees: &PyRevolvingCreditFees,
    ) -> PyRefMut<'py, Self> {
        slf.fees = Some(fees.inner.clone());
        slf
    }

    #[pyo3(text_signature = "($self, spec)")]
    fn draw_repay<'py>(
        mut slf: PyRefMut<'py, Self>,
        spec: &PyDrawRepaySpec,
    ) -> PyRefMut<'py, Self> {
        slf.draw_repay_spec = Some(spec.inner.clone());
        slf
    }

    #[pyo3(text_signature = "($self, curve_id)")]
    fn disc_id(mut slf: PyRefMut<'_, Self>, curve_id: String) -> PyRefMut<'_, Self> {
        slf.discount_curve_id = Some(CurveId::new(&curve_id));
        slf
    }

    #[pyo3(
        text_signature = "($self, curve_id=None)",
        signature = (curve_id=None)
    )]
    fn credit_curve(mut slf: PyRefMut<'_, Self>, curve_id: Option<String>) -> PyRefMut<'_, Self> {
        slf.credit_curve_id = curve_id.map(|id| CurveId::new(&id));
        slf
    }

    #[pyo3(text_signature = "($self, rate)")]
    fn recovery_rate(mut slf: PyRefMut<'_, Self>, rate: f64) -> PyRefMut<'_, Self> {
        slf.recovery_rate = rate;
        slf
    }

    #[pyo3(text_signature = "($self, stub)")]
    fn stub(mut slf: PyRefMut<'_, Self>, stub: StubKindArg) -> PyRefMut<'_, Self> {
        slf.stub = stub.0;
        slf
    }

    #[pyo3(text_signature = "($self)")]
    fn build(&self) -> PyResult<PyRevolvingCredit> {
        self.ensure_ready()?;

        let ccy = self.pending_currency.ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "RevolvingCreditBuilder internal error: missing currency after validation",
            )
        })?;
        let commitment = Money::new(
            self.pending_commitment_amount.ok_or_else(|| {
                pyo3::exceptions::PyRuntimeError::new_err(
                    "RevolvingCreditBuilder internal error: missing commitment amount after validation",
                )
            })?,
            ccy,
        );
        let drawn = Money::new(self.pending_drawn_amount.unwrap_or(0.0), ccy);

        let base_rate = self
            .base_rate_spec
            .clone()
            .unwrap_or(BaseRateSpec::Fixed { rate: 0.0 });

        let fees = self.fees.clone().unwrap_or_default();

        let draw_repay = self
            .draw_repay_spec
            .clone()
            .unwrap_or_else(|| DrawRepaySpec::Deterministic(Vec::new()));

        RevolvingCredit::builder()
            .id(self.instrument_id.clone())
            .commitment_amount(commitment)
            .drawn_amount(drawn)
            .commitment_date(self.commitment_date.ok_or_else(|| {
                pyo3::exceptions::PyRuntimeError::new_err(
                    "RevolvingCreditBuilder internal error: missing commitment date after validation",
                )
            })?)
            .maturity(self.maturity.ok_or_else(|| {
                pyo3::exceptions::PyRuntimeError::new_err(
                    "RevolvingCreditBuilder internal error: missing maturity after validation",
                )
            })?)
            .base_rate_spec(base_rate)
            .day_count(self.day_count)
            .frequency(self.frequency)
            .fees(fees)
            .draw_repay_spec(draw_repay)
            .discount_curve_id(self.discount_curve_id.clone().ok_or_else(|| {
                pyo3::exceptions::PyRuntimeError::new_err(
                    "RevolvingCreditBuilder internal error: missing discount curve after validation",
                )
            })?)
            .credit_curve_id_opt(self.credit_curve_id.clone())
            .recovery_rate(self.recovery_rate)
            .stub(self.stub)
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .map(PyRevolvingCredit::new)
            .map_err(core_to_py)
    }

    fn __repr__(&self) -> String {
        format!("RevolvingCreditBuilder('{}')", self.instrument_id)
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyRevolvingCredit>()?;
    module.add_class::<PyEnhancedMonteCarloResult>()?;
    module.add_class::<PyPathResult>()?;
    module.add_class::<PyThreeFactorPathData>()?;
    module.add_class::<PyFeeTier>()?;
    module.add_class::<PyBaseRateSpec>()?;
    module.add_class::<PyRevolvingCreditFees>()?;
    module.add_class::<PyDrawRepayEvent>()?;
    module.add_class::<PyUtilizationProcess>()?;
    module.add_class::<PyStochasticUtilizationSpec>()?;
    module.add_class::<PyDrawRepaySpec>()?;
    module.add_class::<PyRevolvingCreditBuilder>()?;
    Ok(vec![
        "RevolvingCredit",
        "RevolvingCreditBuilder",
        "EnhancedMonteCarloResult",
        "PathResult",
        "ThreeFactorPathData",
        "FeeTier",
        "BaseRateSpec",
        "RevolvingCreditFees",
        "DrawRepayEvent",
        "UtilizationProcess",
        "StochasticUtilizationSpec",
        "DrawRepaySpec",
    ])
}
