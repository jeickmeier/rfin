//! Python bindings for structured credit pricing functions and types.
//!
//! Exposes `generate_cashflows`, `generate_tranche_cashflows`, `run_simulation`,
//! `execute_waterfall`, plus `CoverageTest`, `DiversionRule`, and `DiversionEngine`.

use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::market_data::PyMarketContext;
use crate::core::money::PyMoney;
use crate::errors::core_to_py;
use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_valuations::instruments::fixed_income::structured_credit::{
    execute_waterfall as rust_execute_waterfall, generate_cashflows as rust_generate_cashflows,
    generate_tranche_cashflows as rust_generate_tranche_cashflows,
    run_simulation as rust_run_simulation, CoverageTest as RustCoverageTest, DiversionCondition,
    DiversionEngine as RustDiversionEngine, DiversionRule as RustDiversionRule, TrancheCashflows,
    WaterfallContext,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule, PyType};
use std::collections::HashMap;

use super::pool::PyPool;
use super::tranches::PyTrancheStructure;
use super::waterfall::PyWaterfall;
use super::PyStructuredCredit;

// ============================================================================
// STANDALONE PRICING FUNCTIONS
// ============================================================================

/// Generate aggregated cashflows for all tranches of a structured credit instrument.
///
/// Args:
///     instrument: The structured credit instrument.
///     market: Market data context for rate lookups.
///     as_of: Valuation date.
///
/// Returns:
///     list[tuple[datetime.date, Money]]: Dated cashflow schedule.
///
/// Raises:
///     ValueError: If the simulation fails.
#[pyfunction(
    name = "generate_cashflows",
    text_signature = "(instrument, market, as_of)"
)]
fn py_generate_cashflows(
    py: Python<'_>,
    instrument: &PyStructuredCredit,
    market: &PyMarketContext,
    as_of: Bound<'_, PyAny>,
) -> PyResult<Vec<(Py<PyAny>, PyMoney)>> {
    let as_of_date = py_to_date(&as_of)?;
    let flows = rust_generate_cashflows(&instrument.inner, &market.inner, as_of_date)
        .map_err(core_to_py)?;
    flows
        .into_iter()
        .map(|(date, money)| {
            let py_date = date_to_py(py, date)?;
            Ok((py_date, PyMoney::new(money)))
        })
        .collect()
}

/// Generate cashflows for a specific tranche.
///
/// Args:
///     instrument: The structured credit instrument.
///     tranche_id: Identifier of the target tranche.
///     market: Market data context.
///     as_of: Valuation date.
///
/// Returns:
///     dict: Tranche cashflow details including interest, principal, PIK,
///         and writedown flows.
///
/// Raises:
///     ValueError: If the tranche is not found or simulation fails.
#[pyfunction(
    name = "generate_tranche_cashflows",
    text_signature = "(instrument, tranche_id, market, as_of)"
)]
fn py_generate_tranche_cashflows(
    py: Python<'_>,
    instrument: &PyStructuredCredit,
    tranche_id: &str,
    market: &PyMarketContext,
    as_of: Bound<'_, PyAny>,
) -> PyResult<Py<PyAny>> {
    let as_of_date = py_to_date(&as_of)?;
    let result =
        rust_generate_tranche_cashflows(&instrument.inner, tranche_id, &market.inner, as_of_date)
            .map_err(core_to_py)?;
    tranche_cashflows_to_py(py, &result)
}

/// Run full cashflow simulation for a structured credit instrument.
///
/// Args:
///     instrument: The structured credit instrument.
///     market: Market data context.
///     as_of: Valuation date.
///
/// Returns:
///     dict[str, dict]: Mapping of tranche ID to tranche cashflow details.
///
/// Raises:
///     ValueError: If the simulation fails.
#[pyfunction(
    name = "run_simulation",
    text_signature = "(instrument, market, as_of)"
)]
fn py_run_simulation(
    py: Python<'_>,
    instrument: &PyStructuredCredit,
    market: &PyMarketContext,
    as_of: Bound<'_, PyAny>,
) -> PyResult<Py<PyAny>> {
    let as_of_date = py_to_date(&as_of)?;
    let results =
        rust_run_simulation(&instrument.inner, &market.inner, as_of_date).map_err(core_to_py)?;

    let dict = pyo3::types::PyDict::new(py);
    for (tranche_id, tcf) in &results {
        let py_tcf = tranche_cashflows_to_py(py, tcf)?;
        dict.set_item(tranche_id, py_tcf)?;
    }
    Ok(dict.into())
}

/// Execute the waterfall to distribute available cash across tiers.
///
/// Args:
///     waterfall: Full waterfall configuration (tiers, coverage triggers, coverage rules).
///     tranches: Capital structure (tranche collection).
///     pool: Collateral pool.
///     available_cash_amount: Total cash available for distribution.
///     interest_collections_amount: Interest collected from the pool.
///     currency: Currency code (e.g. "USD").
///     payment_date: Payment date for this period.
///     period_start: Start of the accrual period.
///     pool_balance_amount: Current pool balance.
///     market: Market data context.
///
/// Returns:
///     dict: Waterfall distribution results with tier allocations,
///         recipient distributions, coverage test results, and payment records.
///
/// Raises:
///     ValueError: If currency is invalid or waterfall execution fails.
#[pyfunction(name = "execute_waterfall")]
#[pyo3(signature = (waterfall, tranches, pool, available_cash_amount, interest_collections_amount, currency, payment_date, period_start, pool_balance_amount, market))]
#[allow(clippy::too_many_arguments)]
fn py_execute_waterfall(
    py: Python<'_>,
    waterfall: &PyWaterfall,
    tranches: &PyTrancheStructure,
    pool: &PyPool,
    available_cash_amount: f64,
    interest_collections_amount: f64,
    currency: &str,
    payment_date: Bound<'_, PyAny>,
    period_start: Bound<'_, PyAny>,
    pool_balance_amount: f64,
    market: &PyMarketContext,
) -> PyResult<Py<PyAny>> {
    let ccy: Currency = currency
        .parse()
        .map_err(|e| PyValueError::new_err(format!("Invalid currency '{currency}': {e:?}")))?;
    let pay_date = py_to_date(&payment_date)?;
    let period_start_date = py_to_date(&period_start)?;

    let waterfall_inner = waterfall.inner.clone();

    let context = WaterfallContext {
        available_cash: Money::new(available_cash_amount, ccy),
        interest_collections: Money::new(interest_collections_amount, ccy),
        payment_date: pay_date,
        period_start: period_start_date,
        pool_balance: Money::new(pool_balance_amount, ccy),
        market: &market.inner,
        tranche_balances: None,
        reserve_balance: Money::new(0.0, ccy),
        recovery_proceeds: Money::new(0.0, ccy),
    };

    let result = rust_execute_waterfall(&waterfall_inner, &tranches.inner, &pool.inner, context)
        .map_err(core_to_py)?;

    let dict = pyo3::types::PyDict::new(py);
    dict.set_item("payment_date", date_to_py(py, result.payment_date)?)?;
    dict.set_item(
        "total_available",
        PyMoney::new(result.total_available).into_pyobject(py)?,
    )?;

    let tier_allocs = pyo3::types::PyList::empty(py);
    for (tier_id, amount) in &result.tier_allocations {
        let pair = (tier_id, PyMoney::new(*amount));
        tier_allocs.append(pair)?;
    }
    dict.set_item("tier_allocations", tier_allocs)?;

    let dist_dict = pyo3::types::PyDict::new(py);
    for (recipient, amount) in &result.distributions {
        dist_dict.set_item(format!("{recipient:?}"), PyMoney::new(*amount))?;
    }
    dict.set_item("distributions", dist_dict)?;

    let tests = pyo3::types::PyList::empty(py);
    for (name, value, passed) in &result.coverage_tests {
        let entry = (name, *value, *passed);
        tests.append(entry)?;
    }
    dict.set_item("coverage_tests", tests)?;

    Ok(dict.into())
}

// ============================================================================
// HELPERS
// ============================================================================

fn tranche_cashflows_to_py(py: Python<'_>, tcf: &TrancheCashflows) -> PyResult<Py<PyAny>> {
    let dict = pyo3::types::PyDict::new(py);
    dict.set_item("tranche_id", &tcf.tranche_id)?;

    let cfs = dated_flows_to_py(py, &tcf.cashflows)?;
    dict.set_item("cashflows", cfs)?;

    let interest = dated_flows_to_py(py, &tcf.interest_flows)?;
    dict.set_item("interest_flows", interest)?;

    let principal = dated_flows_to_py(py, &tcf.principal_flows)?;
    dict.set_item("principal_flows", principal)?;

    let pik = dated_flows_to_py(py, &tcf.pik_flows)?;
    dict.set_item("pik_flows", pik)?;

    let writedown = dated_flows_to_py(py, &tcf.writedown_flows)?;
    dict.set_item("writedown_flows", writedown)?;

    dict.set_item(
        "final_balance",
        PyMoney::new(tcf.final_balance).into_pyobject(py)?,
    )?;
    dict.set_item(
        "total_interest",
        PyMoney::new(tcf.total_interest).into_pyobject(py)?,
    )?;
    dict.set_item(
        "total_principal",
        PyMoney::new(tcf.total_principal).into_pyobject(py)?,
    )?;
    dict.set_item("total_pik", PyMoney::new(tcf.total_pik).into_pyobject(py)?)?;
    dict.set_item(
        "total_writedown",
        PyMoney::new(tcf.total_writedown).into_pyobject(py)?,
    )?;

    Ok(dict.into())
}

fn dated_flows_to_py(
    py: Python<'_>,
    flows: &[(finstack_core::dates::Date, Money)],
) -> PyResult<Py<PyAny>> {
    let list = pyo3::types::PyList::empty(py);
    for (date, money) in flows {
        let py_date = date_to_py(py, *date)?;
        let py_money = PyMoney::new(*money);
        let tuple = (py_date, py_money);
        list.append(tuple)?;
    }
    Ok(list.into())
}

// ============================================================================
// PyCoverageTest
// ============================================================================

/// Coverage test specification (OC or IC).
///
/// OC (overcollateralization) and IC (interest coverage) tests are used
/// to determine whether waterfall diversions should be triggered.
///
/// Examples:
///     >>> oc = CoverageTest.new_oc(1.25)
///     >>> oc.required_level()
///     1.25
///     >>> ic = CoverageTest.new_ic(1.20)
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "CoverageTest",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyCoverageTest {
    pub(crate) inner: RustCoverageTest,
}

#[pymethods]
impl PyCoverageTest {
    /// Create a new OC test with auto-generated ID.
    ///
    /// Args:
    ///     required_ratio: Required OC ratio (e.g. 1.25 for 125%).
    ///     include_cash: Include cash in numerator (default True).
    ///     performing_only: Include only performing assets (default True).
    ///
    /// Returns:
    ///     CoverageTest: New OC coverage test instance.
    #[classmethod]
    #[pyo3(signature = (required_ratio, include_cash=true, performing_only=true))]
    #[pyo3(text_signature = "(cls, required_ratio, include_cash=True, performing_only=True)")]
    fn new_oc(
        _cls: &Bound<'_, PyType>,
        required_ratio: f64,
        include_cash: bool,
        performing_only: bool,
    ) -> Self {
        let mut inner = RustCoverageTest::new_oc(required_ratio);
        if let RustCoverageTest::OC {
            include_cash: ref mut ic,
            performing_only: ref mut po,
            ..
        } = inner
        {
            *ic = include_cash;
            *po = performing_only;
        }
        Self { inner }
    }

    /// Create a new OC test with explicit ID.
    ///
    /// Args:
    ///     id: Unique test identifier.
    ///     required_ratio: Required OC ratio.
    ///     include_cash: Include cash in numerator (default True).
    ///     performing_only: Include only performing assets (default True).
    ///
    /// Returns:
    ///     CoverageTest: New OC coverage test instance.
    #[classmethod]
    #[pyo3(signature = (id, required_ratio, include_cash=true, performing_only=true))]
    #[pyo3(text_signature = "(cls, id, required_ratio, include_cash=True, performing_only=True)")]
    fn new_oc_with_id(
        _cls: &Bound<'_, PyType>,
        id: &str,
        required_ratio: f64,
        include_cash: bool,
        performing_only: bool,
    ) -> Self {
        let mut inner = RustCoverageTest::new_oc_with_id(id, required_ratio);
        if let RustCoverageTest::OC {
            include_cash: ref mut ic,
            performing_only: ref mut po,
            ..
        } = inner
        {
            *ic = include_cash;
            *po = performing_only;
        }
        Self { inner }
    }

    /// Create a new IC test with auto-generated ID.
    ///
    /// Args:
    ///     required_ratio: Required IC ratio (e.g. 1.20 for 120%).
    ///
    /// Returns:
    ///     CoverageTest: New IC coverage test instance.
    #[classmethod]
    #[pyo3(text_signature = "(cls, required_ratio)")]
    fn new_ic(_cls: &Bound<'_, PyType>, required_ratio: f64) -> Self {
        Self {
            inner: RustCoverageTest::new_ic(required_ratio),
        }
    }

    /// Create a new IC test with explicit ID.
    ///
    /// Args:
    ///     id: Unique test identifier.
    ///     required_ratio: Required IC ratio.
    ///
    /// Returns:
    ///     CoverageTest: New IC coverage test instance.
    #[classmethod]
    #[pyo3(text_signature = "(cls, id, required_ratio)")]
    fn new_ic_with_id(_cls: &Bound<'_, PyType>, id: &str, required_ratio: f64) -> Self {
        Self {
            inner: RustCoverageTest::new_ic_with_id(id, required_ratio),
        }
    }

    /// Get the test identifier.
    #[pyo3(text_signature = "($self)")]
    fn id(&self) -> &str {
        self.inner.id()
    }

    /// Get the required coverage level (ratio).
    #[pyo3(text_signature = "($self)")]
    fn required_level(&self) -> f64 {
        self.inner.required_level()
    }

    /// Deserialize from a Python dict.
    #[classmethod]
    #[pyo3(text_signature = "(cls, data)")]
    fn from_dict(_cls: &Bound<'_, PyType>, data: &Bound<'_, PyAny>) -> PyResult<Self> {
        let json_value: serde_json::Value = pythonize::depythonize(data)
            .map_err(|e| PyValueError::new_err(format!("Failed to convert from Python: {e}")))?;
        let inner: RustCoverageTest = serde_json::from_value(json_value).map_err(|e| {
            PyValueError::new_err(format!("Failed to deserialize CoverageTest: {e}"))
        })?;
        Ok(Self { inner })
    }

    /// Serialize to a Python dict.
    #[pyo3(text_signature = "($self)")]
    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let json_str = serde_json::to_string(&self.inner)
            .map_err(|e| PyValueError::new_err(format!("Failed to serialize: {e}")))?;
        let json_value: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| PyValueError::new_err(format!("Failed to parse JSON: {e}")))?;
        pythonize::pythonize(py, &json_value)
            .map(|bound| bound.into())
            .map_err(|e| PyValueError::new_err(format!("Failed to convert to Python: {e}")))
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            RustCoverageTest::OC {
                id,
                required_ratio,
                include_cash,
                performing_only,
            } => format!(
                "CoverageTest.OC(id='{id}', ratio={required_ratio:.4}, \
                 cash={include_cash}, performing={performing_only})"
            ),
            RustCoverageTest::IC { id, required_ratio } => {
                format!("CoverageTest.IC(id='{id}', ratio={required_ratio:.4})")
            }
            _ => format!("CoverageTest({:?})", self.inner),
        }
    }
}

// ============================================================================
// PyDiversionRule
// ============================================================================

/// Diversion rule that redirects cash from one waterfall tier to another.
///
/// Args:
///     id: Unique rule identifier.
///     source_tier_id: Tier from which cash is diverted.
///     target_tier_id: Tier to which cash is redirected.
///     condition_json: JSON string describing the diversion condition.
///     priority: Evaluation priority (lower = higher priority).
///
/// Examples:
///     >>> rule = DiversionRule.on_test_failure(
///     ...     "oc_divert", "mezz_interest", "senior_principal", "oc_test_125", 1)
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "DiversionRule",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyDiversionRule {
    pub(crate) inner: RustDiversionRule,
}

#[pymethods]
impl PyDiversionRule {
    #[new]
    #[pyo3(text_signature = "(id, source_tier_id, target_tier_id, condition_json, priority)")]
    fn new(
        id: &str,
        source_tier_id: &str,
        target_tier_id: &str,
        condition_json: &str,
        priority: usize,
    ) -> PyResult<Self> {
        let condition: DiversionCondition = serde_json::from_str(condition_json)
            .map_err(|e| PyValueError::new_err(format!("Invalid condition JSON: {e}")))?;
        Ok(Self {
            inner: RustDiversionRule::new(id, source_tier_id, target_tier_id, condition, priority),
        })
    }

    /// Create a rule triggered by coverage test failure.
    ///
    /// Args:
    ///     id: Unique rule identifier.
    ///     source_tier_id: Source tier for diversion.
    ///     target_tier_id: Target tier for redirected cash.
    ///     test_id: Coverage test ID that triggers the diversion.
    ///     priority: Evaluation priority (lower = higher).
    ///
    /// Returns:
    ///     DiversionRule: New rule instance.
    #[classmethod]
    #[pyo3(text_signature = "(cls, id, source_tier_id, target_tier_id, test_id, priority)")]
    fn on_test_failure(
        _cls: &Bound<'_, PyType>,
        id: &str,
        source_tier_id: &str,
        target_tier_id: &str,
        test_id: &str,
        priority: usize,
    ) -> Self {
        Self {
            inner: RustDiversionRule::on_test_failure(
                id,
                source_tier_id,
                target_tier_id,
                test_id,
                priority,
            ),
        }
    }

    /// Rule identifier.
    #[getter]
    fn rule_id(&self) -> &str {
        &self.inner.id
    }

    /// Source tier identifier.
    #[getter]
    fn source_tier_id(&self) -> &str {
        &self.inner.source_tier_id
    }

    /// Target tier identifier.
    #[getter]
    fn target_tier_id(&self) -> &str {
        &self.inner.target_tier_id
    }

    /// Evaluation priority.
    #[getter]
    fn priority(&self) -> usize {
        self.inner.priority
    }

    /// Condition that triggers this diversion (string representation).
    #[getter]
    fn condition(&self) -> String {
        match &self.inner.condition {
            DiversionCondition::CoverageTestFailed { test_id } => {
                format!("CoverageTestFailed(test_id='{test_id}')")
            }
            DiversionCondition::CustomExpression { expr } => {
                format!("CustomExpression(expr='{expr}')")
            }
            DiversionCondition::Always => "Always".to_string(),
            _ => format!("{:?}", self.inner.condition),
        }
    }

    /// Check if this rule's condition is met given coverage test results.
    ///
    /// Args:
    ///     test_results: Mapping of test ID to pass/fail (True = passed, False = failed).
    ///
    /// Returns:
    ///     True if the rule is active (e.g. coverage test failed for CoverageTestFailed condition).
    #[pyo3(text_signature = "($self, test_results)")]
    fn is_active(&self, test_results: &Bound<'_, PyDict>) -> PyResult<bool> {
        let map: HashMap<String, bool> = test_results.extract()?;
        Ok(self.inner.is_active(&map))
    }

    /// Deserialize from a Python dict.
    #[classmethod]
    #[pyo3(text_signature = "(cls, data)")]
    fn from_dict(_cls: &Bound<'_, PyType>, data: &Bound<'_, PyAny>) -> PyResult<Self> {
        let json_value: serde_json::Value = pythonize::depythonize(data)
            .map_err(|e| PyValueError::new_err(format!("Failed to convert from Python: {e}")))?;
        let inner: RustDiversionRule = serde_json::from_value(json_value).map_err(|e| {
            PyValueError::new_err(format!("Failed to deserialize DiversionRule: {e}"))
        })?;
        Ok(Self { inner })
    }

    /// Serialize to a Python dict.
    #[pyo3(text_signature = "($self)")]
    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let json_str = serde_json::to_string(&self.inner)
            .map_err(|e| PyValueError::new_err(format!("Failed to serialize: {e}")))?;
        let json_value: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| PyValueError::new_err(format!("Failed to parse JSON: {e}")))?;
        pythonize::pythonize(py, &json_value)
            .map(|bound| bound.into())
            .map_err(|e| PyValueError::new_err(format!("Failed to convert to Python: {e}")))
    }

    fn __repr__(&self) -> String {
        format!(
            "DiversionRule(id='{}', {} -> {}, priority={})",
            self.inner.id,
            self.inner.source_tier_id,
            self.inner.target_tier_id,
            self.inner.priority
        )
    }
}

// ============================================================================
// PyDiversionEngine
// ============================================================================

/// Engine for managing and validating diversion rules.
///
/// Collects rules, validates for cycles/duplicates, and determines active
/// diversions based on coverage test results.
///
/// Examples:
///     >>> engine = DiversionEngine()
///     >>> engine = engine.add_rule(rule)
///     >>> engine.validate()  # raises on error
///     >>> engine.rule_count
///     1
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "DiversionEngine",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyDiversionEngine {
    pub(crate) inner: RustDiversionEngine,
}

#[pymethods]
impl PyDiversionEngine {
    #[new]
    fn new() -> Self {
        Self {
            inner: RustDiversionEngine::new(),
        }
    }

    /// Add a diversion rule to the engine.
    ///
    /// Returns a new engine instance with the rule added (immutable pattern).
    ///
    /// Args:
    ///     rule: Diversion rule to add.
    ///
    /// Returns:
    ///     DiversionEngine: New engine with the rule included.
    #[pyo3(text_signature = "($self, rule)")]
    fn add_rule(&self, rule: &PyDiversionRule) -> Self {
        Self {
            inner: self.inner.clone().add_rule(rule.inner.clone()),
        }
    }

    /// Validate all rules for cycles, self-references, and duplicate IDs.
    ///
    /// Raises:
    ///     ValueError: If validation fails.
    #[pyo3(text_signature = "($self)")]
    fn validate(&self) -> PyResult<()> {
        self.inner.validate().map_err(core_to_py)
    }

    /// Number of rules in the engine.
    #[getter]
    fn rule_count(&self) -> usize {
        self.inner.rules().len()
    }

    /// Get active diversions based on coverage test results.
    ///
    /// Args:
    ///     test_results: Mapping of test ID to pass/fail (True = passed, False = failed).
    ///
    /// Returns:
    ///     Mapping of source tier ID to target tier ID for each active diversion.
    #[pyo3(text_signature = "($self, test_results)")]
    fn get_active_diversions(
        &self,
        py: Python<'_>,
        test_results: &Bound<'_, PyDict>,
    ) -> PyResult<Py<PyAny>> {
        let map: HashMap<String, bool> = test_results.extract()?;
        let active = self.inner.get_active_diversions(&map);
        let out = PyDict::new(py);
        for (k, v) in active {
            out.set_item(k, v)?;
        }
        Ok(out.into())
    }

    /// Deserialize from a Python dict.
    #[classmethod]
    #[pyo3(text_signature = "(cls, data)")]
    fn from_dict(_cls: &Bound<'_, PyType>, data: &Bound<'_, PyAny>) -> PyResult<Self> {
        let json_value: serde_json::Value = pythonize::depythonize(data)
            .map_err(|e| PyValueError::new_err(format!("Failed to convert from Python: {e}")))?;
        let inner: RustDiversionEngine = serde_json::from_value(json_value).map_err(|e| {
            PyValueError::new_err(format!("Failed to deserialize DiversionEngine: {e}"))
        })?;
        Ok(Self { inner })
    }

    /// Serialize to a Python dict.
    #[pyo3(text_signature = "($self)")]
    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let json_str = serde_json::to_string(&self.inner)
            .map_err(|e| PyValueError::new_err(format!("Failed to serialize: {e}")))?;
        let json_value: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| PyValueError::new_err(format!("Failed to parse JSON: {e}")))?;
        pythonize::pythonize(py, &json_value)
            .map(|bound| bound.into())
            .map_err(|e| PyValueError::new_err(format!("Failed to convert to Python: {e}")))
    }

    fn __repr__(&self) -> String {
        format!("DiversionEngine(rules={})", self.inner.rules().len())
    }
}

// ============================================================================
// REGISTRATION
// ============================================================================

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyCoverageTest>()?;
    module.add_class::<PyDiversionRule>()?;
    module.add_class::<PyDiversionEngine>()?;
    module.add_function(wrap_pyfunction!(py_generate_cashflows, module)?)?;
    module.add_function(wrap_pyfunction!(py_generate_tranche_cashflows, module)?)?;
    module.add_function(wrap_pyfunction!(py_run_simulation, module)?)?;
    module.add_function(wrap_pyfunction!(py_execute_waterfall, module)?)?;

    Ok(vec![
        "CoverageTest",
        "DiversionRule",
        "DiversionEngine",
        "generate_cashflows",
        "generate_tranche_cashflows",
        "run_simulation",
        "execute_waterfall",
    ])
}
