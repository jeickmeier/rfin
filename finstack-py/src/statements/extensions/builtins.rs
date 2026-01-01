//! Built-in extension implementations.

use finstack_statements::extensions::{
    AccountType, CorkscrewAccount, CorkscrewConfig, CorkscrewExtension, CreditScorecardExtension,
    ScorecardConfig, ScorecardMetric,
};
use pyo3::basic::CompareOp;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyType};
use pyo3::{Bound, IntoPyObjectExt};

/// Balance sheet account type for corkscrew analysis.
#[pyclass(
    module = "finstack.statements.extensions",
    name = "AccountType",
    frozen
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyAccountType {
    pub(crate) inner: AccountType,
}

impl PyAccountType {
    pub(crate) fn new(inner: AccountType) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyAccountType {
    #[classattr]
    const ASSET: Self = Self {
        inner: AccountType::Asset,
    };
    #[classattr]
    const LIABILITY: Self = Self {
        inner: AccountType::Liability,
    };
    #[classattr]
    const EQUITY: Self = Self {
        inner: AccountType::Equity,
    };

    fn __repr__(&self) -> String {
        format!("AccountType.{:?}", self.inner)
    }

    fn __richcmp__(
        &self,
        other: PyRef<'_, Self>,
        op: CompareOp,
        py: Python<'_>,
    ) -> PyResult<Py<PyAny>> {
        let result = match op {
            CompareOp::Eq => self.inner == other.inner,
            CompareOp::Ne => self.inner != other.inner,
            _ => return Err(PyValueError::new_err("Unsupported comparison")),
        };
        let py_bool = result.into_bound_py_any(py)?;
        Ok(py_bool.into())
    }
}

/// Configuration for a single corkscrew account.
///
/// Defines balance sheet account to validate roll-forward.
#[pyclass(module = "finstack.statements.extensions", name = "CorkscrewAccount")]
#[derive(Clone, Debug)]
pub struct PyCorkscrewAccount {
    pub(crate) inner: CorkscrewAccount,
}

impl PyCorkscrewAccount {
    pub(crate) fn new(inner: CorkscrewAccount) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCorkscrewAccount {
    #[new]
    #[pyo3(signature = (node_id, account_type, changes=None, beginning_balance_node=None))]
    #[pyo3(text_signature = "(node_id, account_type, changes=None, beginning_balance_node=None)")]
    /// Create corkscrew account configuration.
    ///
    /// Parameters
    /// ----------
    /// node_id : str
    ///     Node ID for the balance account
    /// account_type : AccountType
    ///     Account type (Asset, Liability, or Equity)
    /// changes : list[str], optional
    ///     Node IDs representing changes to the balance
    /// beginning_balance_node : str, optional
    ///     Node ID for beginning balance override
    ///
    /// Returns
    /// -------
    /// CorkscrewAccount
    ///     Account configuration
    ///
    /// Examples
    /// --------
    /// >>> from finstack.statements.extensions import CorkscrewAccount, AccountType
    /// >>> account = CorkscrewAccount(
    /// ...     "cash",
    /// ...     AccountType.ASSET,
    /// ...     changes=["cash_inflows", "cash_outflows"]
    /// ... )
    fn new_py(
        node_id: String,
        account_type: PyAccountType,
        changes: Option<Vec<String>>,
        beginning_balance_node: Option<String>,
    ) -> Self {
        Self::new(CorkscrewAccount {
            node_id,
            account_type: account_type.inner,
            changes: changes.unwrap_or_default(),
            beginning_balance_node,
        })
    }

    #[getter]
    fn node_id(&self) -> String {
        self.inner.node_id.clone()
    }

    #[getter]
    fn account_type(&self) -> PyAccountType {
        PyAccountType::new(self.inner.account_type)
    }

    #[getter]
    fn changes(&self) -> Vec<String> {
        self.inner.changes.clone()
    }

    #[getter]
    fn beginning_balance_node(&self) -> Option<String> {
        self.inner.beginning_balance_node.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "CorkscrewAccount(node_id='{}', account_type={:?})",
            self.inner.node_id, self.inner.account_type
        )
    }
}

/// Configuration for corkscrew analysis.
///
/// Defines accounts and validation parameters for balance sheet roll-forward.
#[pyclass(module = "finstack.statements.extensions", name = "CorkscrewConfig")]
#[derive(Clone, Debug)]
pub struct PyCorkscrewConfig {
    pub(crate) inner: CorkscrewConfig,
}

impl PyCorkscrewConfig {
    pub(crate) fn new(inner: CorkscrewConfig) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCorkscrewConfig {
    #[new]
    #[pyo3(signature = (accounts=None, tolerance=None, fail_on_error=None))]
    #[pyo3(text_signature = "(accounts=None, tolerance=None, fail_on_error=None)")]
    /// Create corkscrew configuration.
    ///
    /// Parameters
    /// ----------
    /// accounts : list[CorkscrewAccount], optional
    ///     List of balance sheet accounts to validate
    /// tolerance : float, optional
    ///     Tolerance for rounding differences (default: 0.01)
    /// fail_on_error : bool, optional
    ///     Whether to fail on inconsistencies (default: False)
    ///
    /// Returns
    /// -------
    /// CorkscrewConfig
    ///     Configuration instance
    ///
    /// Examples
    /// --------
    /// >>> from finstack.statements.extensions import CorkscrewConfig, CorkscrewAccount, AccountType
    /// >>> config = CorkscrewConfig(
    /// ...     accounts=[
    /// ...         CorkscrewAccount("cash", AccountType.ASSET, ["cash_inflows", "cash_outflows"]),
    /// ...         CorkscrewAccount("debt", AccountType.LIABILITY, ["debt_issuance", "debt_repayment"])
    /// ...     ],
    /// ...     tolerance=0.01,
    /// ...     fail_on_error=False
    /// ... )
    fn new_py(
        accounts: Option<Vec<PyCorkscrewAccount>>,
        tolerance: Option<f64>,
        fail_on_error: Option<bool>,
    ) -> Self {
        let accounts = accounts
            .map(|accts| accts.into_iter().map(|a| a.inner).collect())
            .unwrap_or_default();
        let tolerance = tolerance.unwrap_or(0.01);
        let fail_on_error = fail_on_error.unwrap_or(false);

        Self::new(CorkscrewConfig {
            accounts,
            tolerance,
            fail_on_error,
        })
    }

    #[getter]
    fn accounts(&self) -> Vec<PyCorkscrewAccount> {
        self.inner
            .accounts
            .iter()
            .map(|a| PyCorkscrewAccount::new(a.clone()))
            .collect()
    }

    #[getter]
    fn tolerance(&self) -> f64 {
        self.inner.tolerance
    }

    #[getter]
    fn fail_on_error(&self) -> bool {
        self.inner.fail_on_error
    }

    fn __repr__(&self) -> String {
        format!(
            "CorkscrewConfig(accounts={}, tolerance={}, fail_on_error={})",
            self.inner.accounts.len(),
            self.inner.tolerance,
            self.inner.fail_on_error
        )
    }

    /// Convert to JSON string.
    ///
    /// Returns
    /// -------
    /// str
    ///     JSON representation
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner)
            .map_err(|e| PyValueError::new_err(format!("Serialization error: {}", e)))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, json_str)")]
    /// Create from JSON string.
    ///
    /// Parameters
    /// ----------
    /// json_str : str
    ///     JSON string
    ///
    /// Returns
    /// -------
    /// CorkscrewConfig
    ///     Deserialized configuration
    fn from_json(_cls: &Bound<'_, PyType>, json_str: &str) -> PyResult<Self> {
        serde_json::from_str(json_str)
            .map(Self::new)
            .map_err(|e| PyValueError::new_err(format!("Deserialization error: {}", e)))
    }
}

/// Definition of a scorecard metric.
///
/// Defines metric calculation formula, weight, and rating thresholds.
#[pyclass(module = "finstack.statements.extensions", name = "ScorecardMetric")]
#[derive(Clone, Debug)]
pub struct PyScorecardMetric {
    pub(crate) inner: ScorecardMetric,
}

impl PyScorecardMetric {
    pub(crate) fn new(inner: ScorecardMetric) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyScorecardMetric {
    #[new]
    #[pyo3(signature = (name, formula, weight=None, thresholds=None, description=None))]
    #[pyo3(text_signature = "(name, formula, weight=None, thresholds=None, description=None)")]
    /// Create scorecard metric configuration.
    ///
    /// Parameters
    /// ----------
    /// name : str
    ///     Metric name
    /// formula : str
    ///     Formula to calculate the metric (DSL syntax)
    /// weight : float, optional
    ///     Weight in overall score (0.0 to 1.0, default: 1.0)
    /// thresholds : dict[str, tuple[float, float]], optional
    ///     Rating thresholds: rating → (min, max)
    /// description : str, optional
    ///     Metric description
    ///
    /// Returns
    /// -------
    /// ScorecardMetric
    ///     Metric configuration
    ///
    /// Examples
    /// --------
    /// >>> from finstack.statements.extensions import ScorecardMetric
    /// >>> metric = ScorecardMetric(
    /// ...     "debt_to_ebitda",
    /// ...     "total_debt / ttm(ebitda)",
    /// ...     weight=0.3,
    /// ...     thresholds={
    /// ...         "AAA": (0.0, 1.0),
    /// ...         "AA": (1.0, 2.0),
    /// ...         "A": (2.0, 3.0)
    /// ...     }
    /// ... )
    fn new_py(
        name: String,
        formula: String,
        weight: Option<f64>,
        thresholds: Option<Bound<'_, PyDict>>,
        description: Option<String>,
    ) -> PyResult<Self> {
        let weight = weight.unwrap_or(1.0);
        let mut threshold_map = indexmap::IndexMap::new();

        if let Some(thresholds) = thresholds {
            for (key, value) in thresholds.iter() {
                let rating: String = key.extract()?;
                let (min, max): (f64, f64) = value.extract()?;
                threshold_map.insert(rating, (min, max));
            }
        }

        Ok(Self::new(ScorecardMetric {
            name,
            formula,
            weight,
            thresholds: threshold_map,
            description,
        }))
    }

    #[getter]
    fn name(&self) -> String {
        self.inner.name.clone()
    }

    #[getter]
    fn formula(&self) -> String {
        self.inner.formula.clone()
    }

    #[getter]
    fn weight(&self) -> f64 {
        self.inner.weight
    }

    #[getter]
    fn thresholds(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new(py);
        for (rating, (min, max)) in &self.inner.thresholds {
            dict.set_item(rating, (min, max))?;
        }
        Ok(dict.into())
    }

    #[getter]
    fn description(&self) -> Option<String> {
        self.inner.description.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "ScorecardMetric(name='{}', weight={})",
            self.inner.name, self.inner.weight
        )
    }
}

/// Configuration for credit scorecard analysis.
///
/// Defines rating scale, metrics, and thresholds for credit rating assignment.
#[pyclass(module = "finstack.statements.extensions", name = "ScorecardConfig")]
#[derive(Clone, Debug)]
pub struct PyScorecardConfig {
    pub(crate) inner: ScorecardConfig,
}

impl PyScorecardConfig {
    pub(crate) fn new(inner: ScorecardConfig) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyScorecardConfig {
    #[new]
    #[pyo3(signature = (rating_scale=None, metrics=None, min_rating=None))]
    #[pyo3(text_signature = "(rating_scale=None, metrics=None, min_rating=None)")]
    /// Create scorecard configuration.
    ///
    /// Parameters
    /// ----------
    /// rating_scale : str, optional
    ///     Rating scale to use ("S&P", "Moody's", or "Fitch", default: "S&P")
    /// metrics : list[ScorecardMetric], optional
    ///     List of metrics to evaluate
    /// min_rating : str, optional
    ///     Minimum acceptable rating
    ///
    /// Returns
    /// -------
    /// ScorecardConfig
    ///     Configuration instance
    ///
    /// Examples
    /// --------
    /// >>> from finstack.statements.extensions import ScorecardConfig, ScorecardMetric
    /// >>> config = ScorecardConfig(
    /// ...     rating_scale="S&P",
    /// ...     metrics=[
    /// ...         ScorecardMetric("debt_to_ebitda", "total_debt / ttm(ebitda)", weight=0.3),
    /// ...         ScorecardMetric("interest_coverage", "ebitda / interest_expense", weight=0.25)
    /// ...     ],
    /// ...     min_rating="BB"
    /// ... )
    fn new_py(
        rating_scale: Option<String>,
        metrics: Option<Vec<PyScorecardMetric>>,
        min_rating: Option<String>,
    ) -> Self {
        let rating_scale = rating_scale.unwrap_or_else(|| "S&P".to_string());
        let metrics = metrics
            .map(|metrics| metrics.into_iter().map(|m| m.inner).collect())
            .unwrap_or_default();

        Self::new(ScorecardConfig {
            rating_scale,
            metrics,
            min_rating,
        })
    }

    #[getter]
    fn rating_scale(&self) -> String {
        self.inner.rating_scale.clone()
    }

    #[getter]
    fn metrics(&self) -> Vec<PyScorecardMetric> {
        self.inner
            .metrics
            .iter()
            .map(|m| PyScorecardMetric::new(m.clone()))
            .collect()
    }

    #[getter]
    fn min_rating(&self) -> Option<String> {
        self.inner.min_rating.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "ScorecardConfig(rating_scale='{}', metrics={})",
            self.inner.rating_scale,
            self.inner.metrics.len()
        )
    }

    /// Convert to JSON string.
    ///
    /// Returns
    /// -------
    /// str
    ///     JSON representation
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner)
            .map_err(|e| PyValueError::new_err(format!("Serialization error: {}", e)))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, json_str)")]
    /// Create from JSON string.
    ///
    /// Parameters
    /// ----------
    /// json_str : str
    ///     JSON string
    ///
    /// Returns
    /// -------
    /// ScorecardConfig
    ///     Deserialized configuration
    fn from_json(_cls: &Bound<'_, PyType>, json_str: &str) -> PyResult<Self> {
        serde_json::from_str(json_str)
            .map(Self::new)
            .map_err(|e| PyValueError::new_err(format!("Deserialization error: {}", e)))
    }
}

/// Corkscrew extension for balance sheet roll-forward validation.
///
/// Validates that balance sheet accounts properly roll forward:
/// Ending Balance = Beginning Balance + Additions - Reductions
///
/// Examples
/// --------
/// >>> from finstack.statements.extensions import (
/// ...     CorkscrewExtension, CorkscrewConfig, CorkscrewAccount, AccountType
/// ... )
/// >>> config = CorkscrewConfig(
/// ...     accounts=[
/// ...         CorkscrewAccount("cash", AccountType.ASSET, ["cash_inflows", "cash_outflows"]),
/// ...         CorkscrewAccount("debt", AccountType.LIABILITY, ["debt_issuance", "debt_repayment"])
/// ...     ],
/// ...     tolerance=0.01
/// ... )
/// >>> extension = CorkscrewExtension.with_config(config)
#[pyclass(
    module = "finstack.statements.extensions",
    name = "CorkscrewExtension",
    unsendable
)]
pub struct PyCorkscrewExtension {
    pub(crate) inner: CorkscrewExtension,
}

#[pymethods]
impl PyCorkscrewExtension {
    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    /// Create a corkscrew extension with default configuration.
    ///
    /// Returns
    /// -------
    /// CorkscrewExtension
    ///     Extension instance
    fn new(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: CorkscrewExtension::new(),
        }
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, config)")]
    /// Create a corkscrew extension with the given configuration.
    ///
    /// Parameters
    /// ----------
    /// config : CorkscrewConfig
    ///     Extension configuration
    ///
    /// Returns
    /// -------
    /// CorkscrewExtension
    ///     Configured extension instance
    fn with_config(_cls: &Bound<'_, PyType>, config: &PyCorkscrewConfig) -> Self {
        Self {
            inner: CorkscrewExtension::with_config(config.inner.clone()),
        }
    }

    #[pyo3(text_signature = "(self, config)")]
    /// Set the extension configuration.
    ///
    /// Parameters
    /// ----------
    /// config : CorkscrewConfig
    ///     New configuration to assign
    fn set_config(&mut self, config: &PyCorkscrewConfig) {
        self.inner.set_config(config.inner.clone());
    }

    #[pyo3(text_signature = "(self)")]
    /// Get the current configuration.
    ///
    /// Returns
    /// -------
    /// CorkscrewConfig | None
    ///     Current configuration if set, None otherwise
    fn config(&self) -> Option<PyCorkscrewConfig> {
        self.inner
            .config()
            .map(|c| PyCorkscrewConfig::new(c.clone()))
    }

    fn __repr__(&self) -> String {
        if let Some(config) = self.inner.config() {
            format!(
                "CorkscrewExtension(accounts={}, tolerance={})",
                config.accounts.len(),
                config.tolerance
            )
        } else {
            "CorkscrewExtension()".to_string()
        }
    }
}

/// Credit scorecard extension for rating assignment.
///
/// Assigns credit ratings based on financial ratios and thresholds.
///
/// Examples
/// --------
/// >>> from finstack.statements.extensions import (
/// ...     CreditScorecardExtension, ScorecardConfig, ScorecardMetric
/// ... )
/// >>> config = ScorecardConfig(
/// ...     rating_scale="S&P",
/// ...     metrics=[
/// ...         ScorecardMetric("debt_to_ebitda", "total_debt / ttm(ebitda)", weight=0.3),
/// ...         ScorecardMetric("interest_coverage", "ebitda / interest_expense", weight=0.25)
/// ...     ]
/// ... )
/// >>> extension = CreditScorecardExtension.with_config(config)
#[pyclass(
    module = "finstack.statements.extensions",
    name = "CreditScorecardExtension",
    unsendable
)]
pub struct PyCreditScorecardExtension {
    pub(crate) inner: CreditScorecardExtension,
}

#[pymethods]
impl PyCreditScorecardExtension {
    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    /// Create a credit scorecard extension with default configuration.
    ///
    /// Returns
    /// -------
    /// CreditScorecardExtension
    ///     Extension instance
    fn new(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: CreditScorecardExtension::new(),
        }
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, config)")]
    /// Create a credit scorecard extension with the given configuration.
    ///
    /// Parameters
    /// ----------
    /// config : ScorecardConfig
    ///     Extension configuration
    ///
    /// Returns
    /// -------
    /// CreditScorecardExtension
    ///     Configured extension instance
    fn with_config(_cls: &Bound<'_, PyType>, config: &PyScorecardConfig) -> Self {
        Self {
            inner: CreditScorecardExtension::with_config(config.inner.clone()),
        }
    }

    #[pyo3(text_signature = "(self, config)")]
    /// Set the extension configuration.
    ///
    /// Parameters
    /// ----------
    /// config : ScorecardConfig
    ///     New configuration to assign
    fn set_config(&mut self, config: &PyScorecardConfig) {
        self.inner.set_config(config.inner.clone());
    }

    #[pyo3(text_signature = "(self)")]
    /// Get the current configuration.
    ///
    /// Returns
    /// -------
    /// ScorecardConfig | None
    ///     Current configuration if set, None otherwise
    fn config(&self) -> Option<PyScorecardConfig> {
        self.inner
            .config()
            .map(|c| PyScorecardConfig::new(c.clone()))
    }

    fn __repr__(&self) -> String {
        if let Some(config) = self.inner.config() {
            format!(
                "CreditScorecardExtension(rating_scale='{}', metrics={})",
                config.rating_scale,
                config.metrics.len()
            )
        } else {
            "CreditScorecardExtension()".to_string()
        }
    }
}
