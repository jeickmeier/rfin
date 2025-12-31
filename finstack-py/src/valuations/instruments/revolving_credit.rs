use crate::core::currency::PyCurrency;
use crate::core::dates::utils::date_to_py;
use crate::core::market_data::PyMarketContext;
use crate::core::money::PyMoney;
use crate::valuations::cashflow::builder::PyCashFlowSchedule;
use crate::valuations::common::PyInstrumentType;
use crate::valuations::results::PyValuationResult;
use finstack_valuations::instruments::fixed_income::revolving_credit::RevolvingCredit;
use pyo3::prelude::*;
use pyo3::types::{PyModule, PyType};
use pyo3::Bound;
use std::fmt;

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
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyRevolvingCredit {
    pub(crate) inner: RevolvingCredit,
}

impl PyRevolvingCredit {
    pub(crate) fn new(inner: RevolvingCredit) -> Self {
        Self { inner }
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
        date_to_py(py, self.inner.maturity_date)
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
            .hazard_curve_id
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
    fn value(&self, market: &PyMarketContext, as_of: Bound<'_, pyo3::PyAny>) -> PyResult<PyMoney> {
        use crate::core::dates::utils::py_to_date;
        use finstack_valuations::instruments::Instrument;

        let as_of_date = py_to_date(&as_of)?;
        self.inner
            .value(&market.inner, as_of_date)
            .map(PyMoney::new)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
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
    ///     >>> result = rc.price_with_metrics(market, date.today(), ["DV01", "CS01"])
    ///     >>> print(f"PV: {result.value}")
    ///     >>> print(f"DV01: {result.metrics['DV01']}")
    fn price_with_metrics(
        &self,
        market: &PyMarketContext,
        as_of: Bound<'_, pyo3::PyAny>,
        metrics: Vec<String>,
    ) -> PyResult<PyValuationResult> {
        use crate::core::dates::utils::py_to_date;
        use finstack_valuations::instruments::Instrument;
        use finstack_valuations::metrics::MetricId;

        let as_of_date = py_to_date(&as_of)?;
        let metric_ids: Vec<MetricId> = metrics
            .iter()
            .map(|s| s.parse().unwrap_or_else(|_| MetricId::custom(s)))
            .collect();

        self.inner
            .price_with_metrics(&market.inner, as_of_date, &metric_ids)
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
    ///     >>> schedule = rc.build_dated_flows(market, date.today())
    ///     >>> for flow in schedule.flows:
    ///     ...     print(f"{flow.date}: {flow.amount} - {flow.description}")
    fn build_dated_flows(
        &self,
        market: &PyMarketContext,
        as_of: Bound<'_, pyo3::PyAny>,
    ) -> PyResult<PyCashFlowSchedule> {
        use crate::core::dates::utils::py_to_date;
        use finstack_valuations::cashflow::CashflowProvider;

        let as_of_date = py_to_date(&as_of)?;
        self.inner
            .build_full_schedule(&market.inner, as_of_date)
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
        use finstack_valuations::instruments::Instrument;

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

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "RevolvingCredit(id='{}', commitment={}, drawn={}, commitment_date='{}', maturity='{}')",
            self.inner.id,
            self.inner.commitment_amount,
            self.inner.drawn_amount,
            self.inner.commitment_date,
            self.inner.maturity_date
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
            self.inner.maturity_date,
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
#[pyclass(module = "finstack.valuations.instruments", name = "PathResult")]
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
    name = "ThreeFactorPathData"
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

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyRevolvingCredit>()?;
    module.add_class::<PyEnhancedMonteCarloResult>()?;
    module.add_class::<PyPathResult>()?;
    module.add_class::<PyThreeFactorPathData>()?;
    Ok(vec![
        "RevolvingCredit",
        "EnhancedMonteCarloResult",
        "PathResult",
        "ThreeFactorPathData",
    ])
}
