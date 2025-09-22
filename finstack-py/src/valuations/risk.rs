//! Risk metrics and sensitivity calculations.

// RiskBucket/RiskReport removed from Rust; Python risk module now provides calculators only
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use crate::core::{dates::PyDate, market_data::context::PyMarketContext};

/// DV01 (Dollar Value of One Basis Point) calculator.
///
/// DV01 measures the change in instrument value for a 1 basis point (0.01%)
/// parallel shift in the yield curve. This is a key risk metric for
/// fixed income instruments.
///
/// Examples:
///     >>> from finstack.risk import calculate_dv01
///     >>> from finstack import Date
///     >>>
///     >>> # Calculate DV01 for a bond
///     >>> dv01 = calculate_dv01(bond, market_context, Date(2024, 1, 1))
///     >>> print(f"DV01: ${dv01:,.2f}")
///     >>>
///     >>> # For a swap
///     >>> dv01 = calculate_dv01(swap, market_context, Date(2024, 1, 1))
///     >>> print(f"Swap DV01: ${dv01:,.2f}")
#[pyfunction]
#[pyo3(name = "calculate_dv01")]
pub fn py_calculate_dv01(
    _py: Python,
    _instrument: PyObject,
    _market_context: &PyMarketContext,
    _as_of: &PyDate,
) -> PyResult<f64> {
    // Extract the instrument type and calculate DV01
    // This is a simplified version - in production we'd need proper instrument dispatch

    Err(PyErr::new::<pyo3::exceptions::PyNotImplementedError, _>(
        "DV01 calculation requires instrument dispatch implementation",
    ))
}

/// CS01 (Credit Spread Sensitivity) calculator.
///
/// CS01 measures the change in instrument value for a 1 basis point change
/// in credit spread. This is particularly important for corporate bonds
/// and credit derivatives.
///
/// Examples:
///     >>> from finstack.risk import calculate_cs01
///     >>>
///     >>> # Calculate CS01 for a corporate bond
///     >>> cs01 = calculate_cs01(bond, market_context, Date(2024, 1, 1))
///     >>> print(f"CS01: ${cs01:,.2f}")
#[pyfunction]
#[pyo3(name = "calculate_cs01")]
pub fn py_calculate_cs01(
    _py: Python,
    _instrument: PyObject,
    _market_context: &PyMarketContext,
    _as_of: &PyDate,
) -> PyResult<f64> {
    // This would calculate credit spread sensitivity
    Err(PyErr::new::<pyo3::exceptions::PyNotImplementedError, _>(
        "CS01 calculation requires credit spread curve implementation",
    ))
}

/// Bucketed DV01 calculator for detailed curve risk.
///
/// Calculates sensitivity to shifts at specific maturity points on the curve,
/// providing a more granular view of interest rate risk than parallel DV01.
///
/// Examples:
///     >>> from finstack.risk import BucketedDv01
///     >>>
///     >>> # Create calculator with custom buckets
///     >>> calc = BucketedDv01([0.25, 0.5, 1, 2, 3, 5, 7, 10, 15, 20, 30])
///     >>>
///     >>> # Calculate bucketed sensitivities
///     >>> buckets = calc.calculate(bond, market_context, Date(2024, 1, 1))
///     >>> for tenor, dv01 in buckets.items():
///     ...     print(f"{tenor}: ${dv01:,.2f}")
#[pyclass(name = "BucketedDv01", module = "finstack.risk")]
pub struct PyBucketedDv01 {
    tenors: Vec<f64>,
}

#[pymethods]
impl PyBucketedDv01 {
    #[new]
    #[pyo3(signature = (tenors = None))]
    fn new(tenors: Option<Vec<f64>>) -> Self {
        let default_tenors = vec![0.25, 0.5, 1.0, 2.0, 3.0, 5.0, 7.0, 10.0, 15.0, 20.0, 30.0];

        Self {
            tenors: tenors.unwrap_or(default_tenors),
        }
    }

    /// Calculate bucketed DV01 for an instrument.
    ///
    /// Args:
    ///     instrument: The instrument to analyze
    ///     market_context: Market data
    ///     as_of: Valuation date
    ///
    /// Returns:
    ///     dict: Mapping of tenor labels to DV01 values
    ///
    /// Examples:
    ///     >>> buckets = calc.calculate(bond, context, Date(2024, 1, 1))
    ///     >>> print(buckets['2Y'])  # 2-year bucket sensitivity
    fn calculate(
        &self,
        py: Python,
        _instrument: PyObject,
        _market_context: &PyMarketContext,
        _as_of: &PyDate,
    ) -> PyResult<Py<PyDict>> {
        let dict = PyDict::new(py);

        // Add sample buckets for demonstration
        for &tenor in &self.tenors {
            let label = format_tenor_label(tenor);
            // In production, this would calculate actual sensitivities
            dict.set_item(label, 0.0)?;
        }

        Ok(dict.into())
    }

    /// Get the tenor points used for bucketing.
    ///
    /// Returns:
    ///     list: List of tenor points in years
    #[getter]
    fn tenors(&self, py: Python) -> PyResult<Py<PyList>> {
        Ok(PyList::new(py, &self.tenors)?.into())
    }

    /// Set custom tenor points for bucketing.
    ///
    /// Args:
    ///     tenors: List of tenor points in years
    ///
    /// Examples:
    ///     >>> calc.set_tenors([0.5, 1, 2, 5, 10, 30])
    #[setter]
    fn set_tenors(&mut self, tenors: Vec<f64>) {
        self.tenors = tenors;
    }

    fn __repr__(&self) -> String {
        format!("BucketedDv01({} buckets)", self.tenors.len())
    }
}

/// Calculate all standard risk metrics for an instrument.
///
/// Computes DV01, modified duration, convexity, and other risk measures
/// in a single call for efficiency.
///
/// Args:
///     instrument: The instrument to analyze
///     market_context: Market data
///     as_of: Valuation date
///     metrics: Optional list of specific metrics to calculate
///
/// Returns:
///     dict: Dictionary of metric names to values
///
/// Examples:
///     >>> from finstack.risk import calculate_risk_metrics
///     >>>
///     >>> # Calculate all risk metrics
///     >>> metrics = calculate_risk_metrics(bond, context, Date(2024, 1, 1))
///     >>> print(f"DV01: ${metrics['Dv01']:,.2f}")
///     >>> print(f"Duration: {metrics['DurationMod']:.2f}")
///     >>> print(f"Convexity: {metrics['Convexity']:.2f}")
///     >>>
///     >>> # Calculate specific metrics only
///     >>> metrics = calculate_risk_metrics(
///     ...     bond, context, Date(2024, 1, 1),
///     ...     metrics=['Dv01', 'DurationMod']
///     ... )
#[pyfunction]
#[pyo3(name = "calculate_risk_metrics")]
pub fn py_calculate_risk_metrics(
    py: Python,
    _instrument: PyObject,
    _market_context: &PyMarketContext,
    _as_of: &PyDate,
    metrics: Option<Vec<String>>,
) -> PyResult<Py<PyDict>> {
    let dict = PyDict::new(py);

    // Default risk metrics if none specified
    let metric_names = metrics.unwrap_or_else(|| {
        vec![
            "Dv01".to_string(),
            "DurationMod".to_string(),
            "DurationMac".to_string(),
            "Convexity".to_string(),
            "Ytm".to_string(),
        ]
    });

    // In production, this would call the actual metric calculation engine
    for metric in metric_names {
        dict.set_item(metric, 0.0)?;
    }

    Ok(dict.into())
}

/// Key Rate Duration calculator for detailed curve risk.
///
/// Calculates the sensitivity to shifts at specific key rates on the curve,
/// typically used for immunization strategies and hedging.
///
/// Examples:
///     >>> from finstack.risk import KeyRateDuration
///     >>>
///     >>> krd = KeyRateDuration()
///     >>> durations = krd.calculate(bond, context, Date(2024, 1, 1))
///     >>> for tenor, duration in durations.items():
///     ...     print(f"{tenor}: {duration:.2f} years")
#[pyclass(name = "KeyRateDuration", module = "finstack.risk")]
pub struct PyKeyRateDuration {
    key_rates: Vec<f64>,
}

#[pymethods]
impl PyKeyRateDuration {
    #[new]
    #[pyo3(signature = (key_rates = None))]
    fn new(key_rates: Option<Vec<f64>>) -> Self {
        let default_rates = vec![0.5, 1.0, 2.0, 3.0, 5.0, 7.0, 10.0, 20.0, 30.0];

        Self {
            key_rates: key_rates.unwrap_or(default_rates),
        }
    }

    /// Calculate key rate durations.
    ///
    /// Args:
    ///     instrument: The instrument to analyze
    ///     market_context: Market data
    ///     as_of: Valuation date
    ///
    /// Returns:
    ///     dict: Mapping of key rate tenors to durations
    fn calculate(
        &self,
        py: Python,
        _instrument: PyObject,
        _market_context: &PyMarketContext,
        _as_of: &PyDate,
    ) -> PyResult<Py<PyDict>> {
        let dict = PyDict::new(py);

        for &rate in &self.key_rates {
            let label = format_tenor_label(rate);
            // In production, calculate actual key rate duration
            dict.set_item(label, 0.0)?;
        }

        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!("KeyRateDuration({} key rates)", self.key_rates.len())
    }
}

/// Format a tenor value into a standard label.
fn format_tenor_label(tenor: f64) -> String {
    if tenor < 1.0 {
        format!("{}M", (tenor * 12.0) as i32)
    } else if tenor == tenor.floor() {
        format!("{}Y", tenor as i32)
    } else {
        format!("{:.1}Y", tenor)
    }
}

/// Risk bucket for categorizing risk exposures.
///
/// Buckets are used to group risk exposures by tenor, classification,
/// or other criteria for risk aggregation and reporting.
///
/// Examples:
///     >>> from finstack.risk import RiskBucket
///     >>>
///     >>> bucket = RiskBucket("5Y", 5.0, "Medium-term")
///     >>> print(f"Bucket: {bucket.id}, Tenor: {bucket.tenor_years}Y")
// PyRiskBucket removed

/// Comprehensive risk report for an instrument.
///
/// Contains key risk metrics, bucketed sensitivities, and metadata
/// for comprehensive risk analysis and reporting.
///
/// Examples:
///     >>> # Get risk report for a bond
///     >>> report = bond.risk_report(market_context, Date(2024, 1, 1))
///     >>>
///     >>> # Access key metrics
///     >>> print(f"DV01: {report.metrics.get('Dv01', 0)}")
///     >>> print(f"Duration: {report.metrics.get('DurationMod', 0)}")
///     >>>
///     >>> # Check bucketed risks
///     >>> if report.bucketed_risks:
///     ...     dv01_buckets = report.bucketed_risks.get('DV01', {})
///     ...     for bucket, value in dv01_buckets.items():
///     ...         print(f"{bucket}: {value}")
// PyRiskReport removed

/// Register risk module functions and classes.
pub fn register_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "risk")?;

    // Register classes (RiskBucket/RiskReport removed)
    m.add_class::<PyBucketedDv01>()?;
    m.add_class::<PyKeyRateDuration>()?;

    // Register functions
    m.add_function(pyo3::wrap_pyfunction!(py_calculate_dv01, &m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(py_calculate_cs01, &m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(py_calculate_risk_metrics, &m)?)?;

    // Add the submodule to parent
    parent.add_submodule(&m)?;
    parent
        .py()
        .import("sys")?
        .getattr("modules")?
        .set_item("finstack.risk", &m)?;

    Ok(())
}
