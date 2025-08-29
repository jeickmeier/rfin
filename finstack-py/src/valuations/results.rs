//! Valuation results and metrics.

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use finstack_valuations::pricing::result::ValuationResult;

use crate::core::{
    money::PyMoney,
    dates::PyDate,
};

/// Result of a valuation calculation.
///
/// Contains the present value of the instrument along with computed risk
/// metrics such as duration, convexity, DV01, etc.
///
/// Examples:
///     >>> result = bond.price(market_context, Date(2024, 1, 1))
///     >>> print(f"PV: ${result.value.amount:,.2f}")
///     >>> print(f"YTM: {result.metrics.get('Ytm', 0):.2%}")
///     >>> print(f"Duration: {result.metrics.get('DurationMod', 0):.2f}")
#[pyclass(name = "ValuationResult", module = "finstack.valuations")]
#[derive(Clone)]
pub struct PyValuationResult {
    inner: ValuationResult,
}

#[pymethods]
impl PyValuationResult {
    /// The unique identifier of the instrument.
    #[getter]
    fn instrument_id(&self) -> String {
        self.inner.instrument_id.clone()
    }
    
    /// The valuation date.
    #[getter]
    fn as_of(&self) -> PyDate {
        PyDate::from_core(self.inner.as_of)
    }
    
    /// The present value of the instrument.
    #[getter]
    fn value(&self) -> PyMoney {
        PyMoney::from_inner(self.inner.value)
    }
    
    /// Get all computed metrics as a dictionary.
    ///
    /// Returns:
    ///     dict: Dictionary mapping metric names to values
    ///
    /// Examples:
    ///     >>> metrics = result.metrics
    ///     >>> ytm = metrics.get('Ytm', 0)
    ///     >>> duration = metrics.get('DurationMod', 0)
    #[getter]
    fn metrics(&self, py: Python) -> PyResult<Py<PyDict>> {
        let dict = PyDict::new(py);
        for (key, value) in &self.inner.measures {
            dict.set_item(key, value)?;
        }
        Ok(dict.into())
    }
    
    /// Get covenant check results if available.
    ///
    /// Returns:
    ///     dict or None: Dictionary mapping covenant names to their check results
    ///
    /// Each covenant result contains:
    /// - 'covenant_type': Type of covenant
    /// - 'passed': Whether the covenant passed
    /// - 'actual_value': Actual value if applicable
    /// - 'threshold': Required threshold if applicable
    /// - 'details': Additional details if available
    ///
    /// Examples:
    ///     >>> covenants = result.covenants
    ///     >>> if covenants:
    ///     ...     for name, report in covenants.items():
    ///     ...         if not report['passed']:
    ///     ...             print(f"Failed: {name}")
    #[getter]
    fn covenants(&self, py: Python) -> PyResult<Option<Py<PyDict>>> {
        match &self.inner.covenants {
            Some(covenants) => {
                let dict = PyDict::new(py);
                for (key, report) in covenants {
                    let report_dict = PyDict::new(py);
                    report_dict.set_item("covenant_type", &report.covenant_type)?;
                    report_dict.set_item("passed", report.passed)?;
                    if let Some(actual) = report.actual_value {
                        report_dict.set_item("actual_value", actual)?;
                    }
                    if let Some(threshold) = report.threshold {
                        report_dict.set_item("threshold", threshold)?;
                    }
                    if let Some(ref details) = report.details {
                        report_dict.set_item("details", details)?;
                    }
                    dict.set_item(key, report_dict)?;
                }
                Ok(Some(dict.into()))
            }
            None => Ok(None)
        }
    }
    
    /// Get FX policy metadata if available.
    ///
    /// Returns:
    ///     dict or None: Dictionary mapping policy names to their metadata
    ///
    /// Each FX policy contains:
    /// - 'policy_name': Name of the policy
    /// - 'source': Source of FX rates (e.g., "market", "fixed")
    /// - 'effective_date': Date the rates are effective
    /// - 'conversions': Dictionary of (from, to) currency pairs to rates
    ///
    /// Examples:
    ///     >>> fx_policies = result.fx_policies
    ///     >>> if fx_policies:
    ///     ...     for name, policy in fx_policies.items():
    ///     ...         print(f"Policy: {name}, Source: {policy['source']}")
    #[getter]
    fn fx_policies(&self, py: Python) -> PyResult<Option<Py<PyDict>>> {
        if self.inner.meta.fx_policies.is_empty() {
            return Ok(None);
        }
        
        let dict = PyDict::new(py);
        for (key, policy) in &self.inner.meta.fx_policies {
            let policy_dict = PyDict::new(py);
            policy_dict.set_item("policy_name", &policy.policy_name)?;
            policy_dict.set_item("source", &policy.source)?;
            policy_dict.set_item("effective_date", format!("{}", policy.effective_date))?;
            
            // Convert currency pair conversions
            let conversions_dict = PyDict::new(py);
            for ((from, to), rate) in &policy.conversions {
                let pair_key = format!("{}-{}", from, to);
                conversions_dict.set_item(pair_key, rate)?;
            }
            policy_dict.set_item("conversions", conversions_dict)?;
            
            dict.set_item(key, policy_dict)?;
        }
        Ok(Some(dict.into()))
    }
    
    /// Check if all covenants passed.
    ///
    /// Returns:
    ///     bool: True if all covenants passed or no covenants exist
    ///
    /// Examples:
    ///     >>> if not result.all_covenants_passed():
    ///     ...     print("Some covenants failed")
    fn all_covenants_passed(&self) -> bool {
        self.inner.all_covenants_passed()
    }
    
    /// Get list of failed covenant names.
    ///
    /// Returns:
    ///     list: List of covenant names that failed
    ///
    /// Examples:
    ///     >>> failed = result.failed_covenants()
    ///     >>> for name in failed:
    ///     ...     print(f"Failed covenant: {name}")
    fn failed_covenants(&self) -> Vec<String> {
        self.inner.failed_covenants()
            .into_iter()
            .map(|s| s.to_string())
            .collect()
    }
    
    /// Get a specific metric value.
    ///
    /// Args:
    ///     metric_name: Name of the metric (e.g., 'Ytm', 'DurationMod', 'Convexity')
    ///     default: Default value if metric not found
    ///
    /// Returns:
    ///     The metric value or default
    ///
    /// Examples:
    ///     >>> ytm = result.get_metric('Ytm', 0.0)
    ///     >>> duration = result.get_metric('DurationMod', 0.0)
    pub fn get_metric(&self, metric_name: &str, default: Option<f64>) -> f64 {
        self.inner.measures
            .get(metric_name)
            .copied()
            .unwrap_or_else(|| default.unwrap_or(0.0))
    }
    
    /// Check if a metric was computed.
    ///
    /// Args:
    ///     metric_name: Name of the metric
    ///
    /// Returns:
    ///     True if the metric exists in the results
    fn has_metric(&self, metric_name: &str) -> bool {
        self.inner.measures.contains_key(metric_name)
    }
    
    /// Get list of all computed metric names.
    ///
    /// Returns:
    ///     List of metric names
    fn metric_names(&self, py: Python) -> PyResult<Py<PyList>> {
        let list = PyList::new(py, self.inner.measures.keys())?;
        Ok(list.into())
    }
    
    /// Convert results to a dictionary.
    ///
    /// Returns:
    ///     dict: Dictionary with all result fields
    ///
    /// Examples:
    ///     >>> data = result.to_dict()
    ///     >>> df = pd.DataFrame([data])  # Create DataFrame for analysis
    fn to_dict(&self, py: Python) -> PyResult<Py<PyDict>> {
        let dict = PyDict::new(py);
        dict.set_item("instrument_id", &self.inner.instrument_id)?;
        dict.set_item("as_of", format!("{}", self.inner.as_of))?;
        dict.set_item("value", self.inner.value.amount())?;
        dict.set_item("currency", format!("{}", self.inner.value.currency()))?;
        
        // Add all metrics
        let metrics_dict = PyDict::new(py);
        for (key, value) in &self.inner.measures {
            metrics_dict.set_item(key, value)?;
        }
        dict.set_item("metrics", metrics_dict)?;
        
        Ok(dict.into())
    }
    
    fn __repr__(&self) -> String {
        format!(
            "ValuationResult(instrument='{}', value={:.2} {}, {} metrics)",
            self.inner.instrument_id,
            self.inner.value.amount(),
            self.inner.value.currency(),
            self.inner.measures.len()
        )
    }
    
    /// Get computed metrics as a dictionary (convenience method).
    ///
    /// Returns:
    ///     dict: Dictionary mapping metric names to values
    ///
    /// Examples:
    ///     >>> measures = result.measures_dict()
    ///     >>> ytm = measures.get('ytm', 0.0)
    pub fn measures_dict(&self, py: Python) -> PyResult<Py<PyDict>> {
        self.metrics(py)
    }
}

impl PyValuationResult {
    /// Create from a Rust ValuationResult
    pub fn from_inner(result: ValuationResult) -> Self {
        Self { inner: result }
    }
}
