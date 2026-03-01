//! Python bindings for stochastic pricing types.
//!
//! Provides PyO3 wrappers for:
//! - PricingMode, CorrelationStructure
//! - StochasticDefaultSpec, StochasticPrepaySpec
//! - StochasticPricingResult, TranchePricingResult

use crate::core::money::PyMoney;
use finstack_valuations::instruments::fixed_income::structured_credit::{
    CorrelationStructure as RustCorrelationStructure, PricingMode as RustPricingMode,
    StochasticDefaultSpec as RustStochasticDefaultSpec,
    StochasticPrepaySpec as RustStochasticPrepaySpec,
    StochasticPricingResult as RustStochasticPricingResult,
    TranchePricingResult as RustTranchePricingResult,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyModule, PyType};

// ============================================================================
// HELPERS
// ============================================================================

fn to_py_dict<T: serde::Serialize>(py: Python<'_>, value: &T) -> PyResult<Py<PyAny>> {
    let json_str = serde_json::to_string(value)
        .map_err(|e| PyValueError::new_err(format!("Serialization failed: {e}")))?;
    let json_mod = PyModule::import(py, "json")?;
    let obj = json_mod.call_method1("loads", (json_str,))?;
    Ok(obj.into())
}

fn from_py_dict<T: serde::de::DeserializeOwned>(data: &Bound<'_, PyAny>) -> PyResult<T> {
    let json_str = if let Ok(s) = data.extract::<String>() {
        s
    } else if let Ok(dict) = data.cast::<pyo3::types::PyDict>() {
        let py = dict.py();
        PyModule::import(py, "json")?
            .call_method1("dumps", (dict,))?
            .extract::<String>()
            .map_err(|e| PyValueError::new_err(format!("Failed to serialize dict: {e}")))?
    } else {
        return Err(PyValueError::new_err("Expected JSON string or dict"));
    };
    serde_json::from_str(&json_str)
        .map_err(|e| PyValueError::new_err(format!("Deserialization failed: {e}")))
}

// ============================================================================
// PRICING MODE
// ============================================================================

/// Stochastic pricing mode selection (tree, Monte Carlo, or hybrid).
///
/// Examples:
///     >>> mode = PricingMode.tree()
///     >>> mode = PricingMode.monte_carlo(num_paths=10000)
///     >>> mode = PricingMode.hybrid(tree_periods=12, mc_paths=5000)
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "PricingMode",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyPricingMode {
    pub(crate) inner: RustPricingMode,
}

#[pymethods]
impl PyPricingMode {
    #[classmethod]
    fn tree(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: RustPricingMode::Tree,
        }
    }

    #[classmethod]
    #[pyo3(signature = (num_paths=10000, antithetic=true))]
    fn monte_carlo(_cls: &Bound<'_, PyType>, num_paths: usize, antithetic: bool) -> Self {
        Self {
            inner: RustPricingMode::MonteCarlo {
                num_paths: num_paths.max(100),
                antithetic,
            },
        }
    }

    #[classmethod]
    #[pyo3(signature = (tree_periods=12, mc_paths=5000))]
    fn hybrid(_cls: &Bound<'_, PyType>, tree_periods: usize, mc_paths: usize) -> Self {
        Self {
            inner: RustPricingMode::Hybrid {
                tree_periods: tree_periods.max(6),
                mc_paths: mc_paths.max(100),
            },
        }
    }

    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        to_py_dict(py, &self.inner)
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, data)")]
    fn from_dict(_cls: &Bound<'_, PyType>, data: Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self {
            inner: from_py_dict(&data)?,
        })
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            RustPricingMode::Tree => "PricingMode.Tree".to_string(),
            RustPricingMode::MonteCarlo {
                num_paths,
                antithetic,
            } => format!("PricingMode.MonteCarlo(num_paths={num_paths}, antithetic={antithetic})"),
            RustPricingMode::Hybrid {
                tree_periods,
                mc_paths,
            } => format!("PricingMode.Hybrid(tree_periods={tree_periods}, mc_paths={mc_paths})"),
            _ => format!("PricingMode({:?})", self.inner),
        }
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

// ============================================================================
// CORRELATION STRUCTURE
// ============================================================================

/// Correlation structure for stochastic structured credit models.
///
/// Supports flat, sectored, and full-matrix specifications.
///
/// Examples:
///     >>> corr = CorrelationStructure.flat(0.10, prepay_default_corr=-0.30)
///     >>> corr = CorrelationStructure.clo_standard()
///     >>> corr.asset_correlation()
///     0.2
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "CorrelationStructure",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyCorrelationStructure {
    pub(crate) inner: RustCorrelationStructure,
}

#[pymethods]
impl PyCorrelationStructure {
    #[classmethod]
    #[pyo3(signature = (asset_corr, prepay_default_corr=0.3))]
    fn flat(_cls: &Bound<'_, PyType>, asset_corr: f64, prepay_default_corr: f64) -> Self {
        Self {
            inner: RustCorrelationStructure::flat(asset_corr, prepay_default_corr),
        }
    }

    #[classmethod]
    #[pyo3(signature = (intra, inter, prepay_default=0.3))]
    fn sectored(_cls: &Bound<'_, PyType>, intra: f64, inter: f64, prepay_default: f64) -> Self {
        Self {
            inner: RustCorrelationStructure::sectored(intra, inter, prepay_default),
        }
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, correlations, labels)")]
    fn matrix(_cls: &Bound<'_, PyType>, correlations: Vec<f64>, labels: Vec<String>) -> Self {
        Self {
            inner: RustCorrelationStructure::matrix(correlations, labels),
        }
    }

    #[classmethod]
    fn rmbs_standard(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: RustCorrelationStructure::rmbs_standard(),
        }
    }

    #[classmethod]
    fn clo_standard(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: RustCorrelationStructure::clo_standard(),
        }
    }

    #[classmethod]
    fn cmbs_standard(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: RustCorrelationStructure::cmbs_standard(),
        }
    }

    #[classmethod]
    fn abs_auto_standard(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: RustCorrelationStructure::abs_auto_standard(),
        }
    }

    /// Average asset (default) correlation.
    fn asset_correlation(&self) -> f64 {
        self.inner.asset_correlation()
    }

    /// Prepay-default correlation.
    fn prepay_default_correlation(&self) -> f64 {
        self.inner.prepay_default_correlation()
    }

    /// Intra-sector correlation (for sectored structures).
    fn intra_sector_correlation(&self) -> Option<f64> {
        self.inner.intra_sector_correlation()
    }

    /// Inter-sector correlation (for sectored structures).
    fn inter_sector_correlation(&self) -> Option<f64> {
        self.inner.inter_sector_correlation()
    }

    /// Whether this is a flat correlation structure.
    fn is_flat(&self) -> bool {
        self.inner.is_flat()
    }

    /// Whether this is a sectored correlation structure.
    fn is_sectored(&self) -> bool {
        self.inner.is_sectored()
    }

    /// Return a new structure with bumped prepay-default correlation.
    fn bump_prepay_default(&self, delta: f64) -> Self {
        Self {
            inner: self.inner.bump_prepay_default(delta),
        }
    }

    /// Validate the correlation structure; raises ``ValueError`` on failure.
    fn validate(&self) -> PyResult<()> {
        self.inner.validate().map_err(|e| PyValueError::new_err(e))
    }

    /// Return a new structure with bumped asset correlation.
    fn bump_asset(&self, delta: f64) -> Self {
        Self {
            inner: self.inner.bump_asset(delta),
        }
    }

    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        to_py_dict(py, &self.inner)
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, data)")]
    fn from_dict(_cls: &Bound<'_, PyType>, data: Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self {
            inner: from_py_dict(&data)?,
        })
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            RustCorrelationStructure::Flat {
                asset_correlation,
                prepay_default_correlation,
            } => format!(
                "CorrelationStructure.Flat(asset={asset_correlation:.4}, prepay_default={prepay_default_correlation:.4})"
            ),
            RustCorrelationStructure::Sectored {
                intra_sector,
                inter_sector,
                prepay_default,
            } => format!(
                "CorrelationStructure.Sectored(intra={intra_sector:.4}, inter={inter_sector:.4}, prepay_default={prepay_default:.4})"
            ),
            RustCorrelationStructure::Matrix { labels, .. } => {
                format!("CorrelationStructure.Matrix(labels={labels:?})")
            }
            _ => format!("CorrelationStructure({:?})", self.inner),
        }
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

// ============================================================================
// STOCHASTIC DEFAULT SPEC
// ============================================================================

/// Stochastic default model specification.
///
/// Selects and configures a default model: deterministic, Gaussian copula,
/// factor-correlated, or intensity process.
///
/// Examples:
///     >>> spec = StochasticDefaultSpec.gaussian_copula(0.02, correlation=0.20)
///     >>> spec.is_stochastic()
///     True
///     >>> spec.base_rate()
///     0.02
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "StochasticDefaultSpec",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyStochasticDefaultSpec {
    pub(crate) inner: RustStochasticDefaultSpec,
}

#[pymethods]
impl PyStochasticDefaultSpec {
    /// Wrap a deterministic default spec from a dict/JSON.
    #[classmethod]
    #[pyo3(text_signature = "(cls, spec_dict)")]
    fn deterministic(_cls: &Bound<'_, PyType>, spec_dict: Bound<'_, PyAny>) -> PyResult<Self> {
        let spec = from_py_dict(&spec_dict)?;
        Ok(Self {
            inner: RustStochasticDefaultSpec::deterministic(spec),
        })
    }

    /// Gaussian copula default model.
    #[classmethod]
    #[pyo3(signature = (base_cdr, correlation=0.3))]
    fn gaussian_copula(_cls: &Bound<'_, PyType>, base_cdr: f64, correlation: f64) -> Self {
        Self {
            inner: RustStochasticDefaultSpec::gaussian_copula(base_cdr, correlation),
        }
    }

    /// Factor-correlated default model.
    #[classmethod]
    #[pyo3(signature = (base_spec_dict, factor_loading=0.3, cdr_volatility=0.5))]
    fn factor_correlated(
        _cls: &Bound<'_, PyType>,
        base_spec_dict: Bound<'_, PyAny>,
        factor_loading: f64,
        cdr_volatility: f64,
    ) -> PyResult<Self> {
        let base_spec = from_py_dict(&base_spec_dict)?;
        Ok(Self {
            inner: RustStochasticDefaultSpec::factor_correlated(
                base_spec,
                factor_loading,
                cdr_volatility,
            ),
        })
    }

    #[classmethod]
    fn rmbs_standard(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: RustStochasticDefaultSpec::rmbs_standard(),
        }
    }

    #[classmethod]
    fn clo_standard(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: RustStochasticDefaultSpec::clo_standard(),
        }
    }

    /// Whether this specification includes a stochastic component.
    fn is_stochastic(&self) -> bool {
        self.inner.is_stochastic()
    }

    /// Base annual CDR / hazard rate.
    fn base_rate(&self) -> f64 {
        self.inner.base_rate()
    }

    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        to_py_dict(py, &self.inner)
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, data)")]
    fn from_dict(_cls: &Bound<'_, PyType>, data: Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self {
            inner: from_py_dict(&data)?,
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "StochasticDefaultSpec(stochastic={}, base_rate={:.4})",
            self.inner.is_stochastic(),
            self.inner.base_rate()
        )
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

// ============================================================================
// STOCHASTIC PREPAY SPEC
// ============================================================================

/// Stochastic prepayment model specification.
///
/// Selects and configures a prepayment model: deterministic, factor-correlated,
/// Richard-Roll, or regime-switching.
///
/// Examples:
///     >>> spec = StochasticPrepaySpec.richard_roll(0.06, refi_sensitivity=0.5)
///     >>> spec.is_stochastic()
///     True
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "StochasticPrepaySpec",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyStochasticPrepaySpec {
    pub(crate) inner: RustStochasticPrepaySpec,
}

#[pymethods]
impl PyStochasticPrepaySpec {
    /// Wrap a deterministic prepayment spec from a dict/JSON.
    #[classmethod]
    #[pyo3(text_signature = "(cls, spec_dict)")]
    fn deterministic(_cls: &Bound<'_, PyType>, spec_dict: Bound<'_, PyAny>) -> PyResult<Self> {
        let spec = from_py_dict(&spec_dict)?;
        Ok(Self {
            inner: RustStochasticPrepaySpec::deterministic(spec),
        })
    }

    /// Factor-correlated prepayment model.
    #[classmethod]
    #[pyo3(signature = (base_spec_dict, factor_loading=0.3, cpr_volatility=0.3))]
    fn factor_correlated(
        _cls: &Bound<'_, PyType>,
        base_spec_dict: Bound<'_, PyAny>,
        factor_loading: f64,
        cpr_volatility: f64,
    ) -> PyResult<Self> {
        let base_spec = from_py_dict(&base_spec_dict)?;
        Ok(Self {
            inner: RustStochasticPrepaySpec::factor_correlated(
                base_spec,
                factor_loading,
                cpr_volatility,
            ),
        })
    }

    /// Richard-Roll prepayment model (RMBS).
    #[classmethod]
    #[pyo3(signature = (base_cpr, refi_sensitivity=0.5, pool_coupon=0.06, burnout_rate=0.10))]
    fn richard_roll(
        _cls: &Bound<'_, PyType>,
        base_cpr: f64,
        refi_sensitivity: f64,
        pool_coupon: f64,
        burnout_rate: f64,
    ) -> Self {
        Self {
            inner: RustStochasticPrepaySpec::richard_roll(
                base_cpr,
                refi_sensitivity,
                pool_coupon,
                burnout_rate,
            ),
        }
    }

    /// Regime-switching two-state Markov prepayment model.
    #[classmethod]
    #[pyo3(signature = (low_cpr, high_cpr, transition_up=0.1, transition_down=0.2))]
    fn regime_switching(
        _cls: &Bound<'_, PyType>,
        low_cpr: f64,
        high_cpr: f64,
        transition_up: f64,
        transition_down: f64,
    ) -> Self {
        Self {
            inner: RustStochasticPrepaySpec::regime_switching(
                low_cpr,
                high_cpr,
                transition_up,
                transition_down,
            ),
        }
    }

    /// RMBS agency standard calibration.
    #[classmethod]
    #[pyo3(signature = (pool_coupon=0.045))]
    fn rmbs_agency(_cls: &Bound<'_, PyType>, pool_coupon: f64) -> Self {
        Self {
            inner: RustStochasticPrepaySpec::rmbs_agency(pool_coupon),
        }
    }

    /// CLO standard calibration.
    #[classmethod]
    fn clo_standard(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: RustStochasticPrepaySpec::clo_standard(),
        }
    }

    /// Whether this specification includes a stochastic component.
    fn is_stochastic(&self) -> bool {
        self.inner.is_stochastic()
    }

    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        to_py_dict(py, &self.inner)
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, data)")]
    fn from_dict(_cls: &Bound<'_, PyType>, data: Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self {
            inner: from_py_dict(&data)?,
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "StochasticPrepaySpec(stochastic={})",
            self.inner.is_stochastic()
        )
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

// ============================================================================
// STOCHASTIC PRICING RESULT
// ============================================================================

/// Result of stochastic pricing for a structured credit deal.
///
/// Contains NPV, risk metrics (EL, UL, ES), and tranche-level results.
///
/// Examples:
///     >>> result.npv
///     Money(1000000.00, 'USD')
///     >>> result.loss_ratio()
///     0.05
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "StochasticPricingResult",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyStochasticPricingResult {
    pub(crate) inner: RustStochasticPricingResult,
}

impl PyStochasticPricingResult {
    #[allow(dead_code)]
    pub(crate) fn new(inner: RustStochasticPricingResult) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyStochasticPricingResult {
    #[getter]
    fn npv(&self) -> PyMoney {
        PyMoney::new(self.inner.npv)
    }

    #[getter]
    fn clean_price(&self) -> f64 {
        self.inner.clean_price
    }

    #[getter]
    fn dirty_price(&self) -> f64 {
        self.inner.dirty_price
    }

    #[getter]
    fn expected_loss(&self) -> PyMoney {
        PyMoney::new(self.inner.expected_loss)
    }

    #[getter]
    fn unexpected_loss(&self) -> PyMoney {
        PyMoney::new(self.inner.unexpected_loss)
    }

    #[getter]
    fn expected_shortfall(&self) -> PyMoney {
        PyMoney::new(self.inner.expected_shortfall)
    }

    #[getter]
    fn es_confidence(&self) -> f64 {
        self.inner.es_confidence
    }

    #[getter]
    fn num_paths(&self) -> usize {
        self.inner.num_paths
    }

    #[getter]
    fn pricing_mode(&self) -> &str {
        &self.inner.pricing_mode
    }

    #[getter]
    fn tranche_results(&self) -> Vec<PyTranchePricingResult> {
        self.inner
            .tranche_results
            .iter()
            .map(|r| PyTranchePricingResult { inner: r.clone() })
            .collect()
    }

    /// EL / NPV ratio.
    fn loss_ratio(&self) -> f64 {
        self.inner.loss_ratio()
    }

    fn __repr__(&self) -> String {
        format!(
            "StochasticPricingResult(npv={}, el={}, paths={}, mode='{}')",
            self.inner.npv, self.inner.expected_loss, self.inner.num_paths, self.inner.pricing_mode,
        )
    }
}

// ============================================================================
// TRANCHE PRICING RESULT
// ============================================================================

/// Tranche-level result from stochastic pricing.
///
/// Contains NPV, risk metrics, attachment/detachment, average life, spread,
/// and credit duration for a single tranche.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "TranchePricingResult",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyTranchePricingResult {
    pub(crate) inner: RustTranchePricingResult,
}

#[pymethods]
impl PyTranchePricingResult {
    #[getter]
    fn tranche_id(&self) -> &str {
        &self.inner.tranche_id
    }

    #[getter]
    fn seniority(&self) -> &str {
        &self.inner.seniority
    }

    #[getter]
    fn npv(&self) -> PyMoney {
        PyMoney::new(self.inner.npv)
    }

    #[getter]
    fn expected_loss(&self) -> PyMoney {
        PyMoney::new(self.inner.expected_loss)
    }

    #[getter]
    fn unexpected_loss(&self) -> PyMoney {
        PyMoney::new(self.inner.unexpected_loss)
    }

    #[getter]
    fn expected_shortfall(&self) -> PyMoney {
        PyMoney::new(self.inner.expected_shortfall)
    }

    #[getter]
    fn attachment(&self) -> f64 {
        self.inner.attachment
    }

    #[getter]
    fn detachment(&self) -> f64 {
        self.inner.detachment
    }

    #[getter]
    fn average_life(&self) -> f64 {
        self.inner.average_life
    }

    #[getter]
    fn spread(&self) -> f64 {
        self.inner.spread
    }

    #[getter]
    fn credit_duration(&self) -> f64 {
        self.inner.credit_duration
    }

    /// Tranche thickness (detachment - attachment).
    fn thickness(&self) -> f64 {
        self.inner.thickness()
    }

    /// Loss multiple (EL / thickness).
    fn loss_multiple(&self) -> f64 {
        self.inner.loss_multiple()
    }

    fn __repr__(&self) -> String {
        format!(
            "TranchePricingResult(id='{}', seniority='{}', npv={}, {:.1}/{:.1})",
            self.inner.tranche_id,
            self.inner.seniority,
            self.inner.npv,
            self.inner.attachment,
            self.inner.detachment,
        )
    }
}

// ============================================================================
// REGISTRATION
// ============================================================================

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyPricingMode>()?;
    module.add_class::<PyCorrelationStructure>()?;
    module.add_class::<PyStochasticDefaultSpec>()?;
    module.add_class::<PyStochasticPrepaySpec>()?;
    module.add_class::<PyStochasticPricingResult>()?;
    module.add_class::<PyTranchePricingResult>()?;

    Ok(vec![
        "PricingMode",
        "CorrelationStructure",
        "StochasticDefaultSpec",
        "StochasticPrepaySpec",
        "StochasticPricingResult",
        "TranchePricingResult",
    ])
}
