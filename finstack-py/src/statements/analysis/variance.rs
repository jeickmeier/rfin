use crate::statements::error::stmt_to_py;
use crate::statements::evaluator::PyStatementResult;
use finstack_core::dates::PeriodId;
use finstack_statements::analysis::{
    BridgeChart, BridgeStep, VarianceAnalyzer, VarianceConfig, VarianceReport,
};
use pyo3::prelude::*;
use pyo3::types::PyModule;
use pyo3::{Bound, PyResult};

/// Configuration for variance analysis (Python wrapper).
#[pyclass(
    module = "finstack.statements.analysis",
    name = "VarianceConfig",
    from_py_object
)]
#[derive(Clone)]
pub struct PyVarianceConfig {
    pub(crate) inner: VarianceConfig,
}

#[pymethods]
impl PyVarianceConfig {
    #[new]
    #[pyo3(signature = (baseline_label, comparison_label, periods, metrics))]
    /// Create a new variance configuration.
    ///
    /// Parameters
    /// ----------
    /// baseline_label : str
    ///     Human-readable name for the baseline scenario (e.g. "management_case").
    /// comparison_label : str
    ///     Human-readable name for the comparison scenario (e.g. "bank_case").
    /// periods : list[PeriodId]
    ///     Periods to include in the variance report.
    /// metrics : list[str]
    ///     Node identifiers to compare (e.g. ["revenue", "ebitda"]).
    fn new(
        baseline_label: String,
        comparison_label: String,
        periods: Vec<crate::core::dates::periods::PyPeriodId>,
        metrics: Vec<String>,
    ) -> Self {
        let rust_periods: Vec<PeriodId> = periods.into_iter().map(|p| p.inner).collect();

        Self {
            inner: VarianceConfig::new(baseline_label, comparison_label, metrics, rust_periods),
        }
    }

    fn baseline_label(&self) -> String {
        self.inner.baseline_label.clone()
    }

    fn comparison_label(&self) -> String {
        self.inner.comparison_label.clone()
    }

    fn metrics(&self) -> Vec<String> {
        self.inner.metrics.clone()
    }

    fn periods(&self) -> Vec<crate::core::dates::periods::PyPeriodId> {
        self.inner
            .periods
            .iter()
            .copied()
            .map(crate::core::dates::periods::PyPeriodId::new)
            .collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "VarianceConfig(baseline_label='{}', comparison_label='{}', metrics={}, periods={})",
            self.inner.baseline_label,
            self.inner.comparison_label,
            self.inner.metrics.len(),
            self.inner.periods.len()
        )
    }
}

/// One row of variance output.
#[pyclass(
    module = "finstack.statements.analysis",
    name = "VarianceRow",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub struct PyVarianceRow {
    pub(crate) inner: finstack_statements::analysis::VarianceRow,
}

#[pymethods]
impl PyVarianceRow {
    fn period(&self) -> crate::core::dates::periods::PyPeriodId {
        crate::core::dates::periods::PyPeriodId::new(self.inner.period)
    }

    fn metric(&self) -> &str {
        &self.inner.metric
    }

    fn baseline(&self) -> f64 {
        self.inner.baseline
    }

    fn comparison(&self) -> f64 {
        self.inner.comparison
    }

    fn abs_var(&self) -> f64 {
        self.inner.abs_var
    }

    fn pct_var(&self) -> f64 {
        self.inner.pct_var
    }

    fn driver_contribution(&self) -> Vec<(String, f64)> {
        self.inner
            .driver_contribution
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "VarianceRow(metric='{}', period='{}', abs_var={}, pct_var={})",
            self.inner.metric, self.inner.period, self.inner.abs_var, self.inner.pct_var
        )
    }
}

/// Variance report between two scenarios.
#[pyclass(
    module = "finstack.statements.analysis",
    name = "VarianceReport",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub struct PyVarianceReport {
    pub(crate) inner: VarianceReport,
}

#[pymethods]
impl PyVarianceReport {
    fn baseline_label(&self) -> String {
        self.inner.baseline_label.clone()
    }

    fn comparison_label(&self) -> String {
        self.inner.comparison_label.clone()
    }

    fn rows(&self) -> Vec<PyVarianceRow> {
        self.inner
            .rows
            .iter()
            .cloned()
            .map(|inner| PyVarianceRow { inner })
            .collect()
    }

    #[pyo3(text_signature = "(self)")]
    /// Export variance rows to a Polars DataFrame.
    ///
    /// Returns
    /// -------
    /// polars.DataFrame
    ///     DataFrame with columns: period, metric, baseline, comparison, abs_var, pct_var, driver_contribution
    fn to_polars(&self) -> PyResult<pyo3_polars::PyDataFrame> {
        let df = self.inner.to_polars().map_err(stmt_to_py)?;
        Ok(pyo3_polars::PyDataFrame(df))
    }

    fn __repr__(&self) -> String {
        format!(
            "VarianceReport(baseline='{}', comparison='{}', rows={})",
            self.inner.baseline_label,
            self.inner.comparison_label,
            self.inner.rows.len()
        )
    }
}

/// Bridge step entry for variance decomposition.
#[pyclass(
    module = "finstack.statements.analysis",
    name = "BridgeStep",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub struct PyBridgeStep {
    inner: BridgeStep,
}

#[pymethods]
impl PyBridgeStep {
    fn driver(&self) -> &str {
        &self.inner.driver
    }

    fn contribution(&self) -> f64 {
        self.inner.contribution
    }

    fn __repr__(&self) -> String {
        format!(
            "BridgeStep(driver='{}', contribution={})",
            self.inner.driver, self.inner.contribution
        )
    }
}

/// Bridge chart for a single metric and period.
#[pyclass(
    module = "finstack.statements.analysis",
    name = "BridgeChart",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub struct PyBridgeChart {
    inner: BridgeChart,
}

#[pymethods]
impl PyBridgeChart {
    fn target_metric(&self) -> &str {
        &self.inner.target_metric
    }

    fn period(&self) -> crate::core::dates::periods::PyPeriodId {
        crate::core::dates::periods::PyPeriodId::new(self.inner.period)
    }

    fn baseline_label(&self) -> &str {
        &self.inner.baseline_label
    }

    fn comparison_label(&self) -> &str {
        &self.inner.comparison_label
    }

    fn baseline_value(&self) -> f64 {
        self.inner.baseline_value
    }

    fn comparison_value(&self) -> f64 {
        self.inner.comparison_value
    }

    fn steps(&self) -> Vec<PyBridgeStep> {
        self.inner
            .steps
            .iter()
            .cloned()
            .map(|inner| PyBridgeStep { inner })
            .collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "BridgeChart(target='{}', period='{}', steps={})",
            self.inner.target_metric,
            self.inner.period,
            self.inner.steps.len()
        )
    }
}

/// Variance analyzer between two evaluated StatementResult objects.
#[pyclass(
    module = "finstack.statements.analysis",
    name = "VarianceAnalyzer",
    unsendable
)]
pub struct PyVarianceAnalyzer {
    baseline: PyStatementResult,
    comparison: PyStatementResult,
    baseline_label: String,
    comparison_label: String,
}

#[pymethods]
impl PyVarianceAnalyzer {
    #[new]
    #[pyo3(signature = (baseline, comparison, baseline_label=None, comparison_label=None))]
    /// Create a new variance analyzer.
    ///
    /// Parameters
    /// ----------
    /// baseline : StatementResult
    ///     Baseline evaluation results (e.g. management case).
    /// comparison : StatementResult
    ///     Comparison evaluation results (e.g. bank base case or actuals).
    /// baseline_label : str, default \"baseline\"
    ///     Human-readable label for the baseline scenario.
    /// comparison_label : str, default \"comparison\"
    ///     Human-readable label for the comparison scenario.
    fn new(
        baseline: &PyStatementResult,
        comparison: &PyStatementResult,
        baseline_label: Option<String>,
        comparison_label: Option<String>,
    ) -> Self {
        Self {
            baseline: baseline.clone(),
            comparison: comparison.clone(),
            baseline_label: baseline_label.unwrap_or_else(|| "baseline".to_string()),
            comparison_label: comparison_label.unwrap_or_else(|| "comparison".to_string()),
        }
    }

    #[pyo3(signature = (metrics, periods=None))]
    /// Compute variance between baseline and comparison.
    ///
    /// Parameters
    /// ----------
    /// metrics : list[str]
    ///     Node identifiers to compare.
    /// periods : list[PeriodId] | None, default None
    ///     Periods to include. If None, uses all periods where the first metric is present.
    ///
    /// Returns
    /// -------
    /// VarianceReport
    ///     Structured variance report with per-period rows.
    fn compute(
        &self,
        py: Python<'_>,
        metrics: Vec<String>,
        periods: Option<Vec<crate::core::dates::periods::PyPeriodId>>,
    ) -> PyResult<PyVarianceReport> {
        let periods_inner: Vec<PeriodId> = if let Some(p) = periods {
            p.into_iter().map(|pid| pid.inner).collect()
        } else {
            infer_periods_from_results(&self.baseline.inner, &metrics)?
        };

        let config = VarianceConfig::new(
            self.baseline_label.clone(),
            self.comparison_label.clone(),
            metrics,
            periods_inner,
        );

        let analyzer = VarianceAnalyzer::new(&self.baseline.inner, &self.comparison.inner);
        let report = py.detach(|| analyzer.compute(&config).map_err(stmt_to_py))?;

        Ok(PyVarianceReport { inner: report })
    }

    #[pyo3(signature = (target_metric, drivers, period=None))]
    /// Compute a simple bridge decomposition for a target metric.
    ///
    /// Parameters
    /// ----------
    /// target_metric : str
    ///     Target metric identifier (e.g. "ebitda").
    /// drivers : list[str]
    ///     Driver node identifiers to attribute variance to.
    /// period : PeriodId | None, default None
    ///     Period to analyze. If None, uses the latest period where the target metric exists.
    ///
    /// Returns
    /// -------
    /// BridgeChart
    ///     Bridge chart with driver contributions.
    fn bridge(
        &self,
        py: Python<'_>,
        target_metric: &str,
        drivers: Vec<String>,
        period: Option<crate::core::dates::periods::PyPeriodId>,
    ) -> PyResult<PyBridgeChart> {
        let period_id = if let Some(p) = period {
            p.inner
        } else {
            infer_latest_period_for_metric(&self.baseline.inner, target_metric)?
        };

        let driver_refs: Vec<&str> = drivers.iter().map(|s| s.as_str()).collect();

        let analyzer = VarianceAnalyzer::new(&self.baseline.inner, &self.comparison.inner);
        let chart = py.detach(|| {
            analyzer
                .bridge_decomposition(
                    target_metric,
                    period_id,
                    &driver_refs,
                    &self.baseline_label,
                    &self.comparison_label,
                )
                .map_err(stmt_to_py)
        })?;

        Ok(PyBridgeChart { inner: chart })
    }

    fn __repr__(&self) -> String {
        format!(
            "VarianceAnalyzer(baseline_label='{}', comparison_label='{}')",
            self.baseline_label, self.comparison_label
        )
    }
}

fn infer_periods_from_results(
    results: &finstack_statements::evaluator::StatementResult,
    metrics: &[String],
) -> PyResult<Vec<PeriodId>> {
    let first_metric = metrics
        .first()
        .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("metrics cannot be empty"))?;

    let periods = results
        .nodes
        .get(first_metric)
        .ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err(format!(
                "Metric '{}' not found in baseline results",
                first_metric
            ))
        })?
        .keys()
        .copied()
        .collect::<Vec<_>>();

    Ok(periods)
}

fn infer_latest_period_for_metric(
    results: &finstack_statements::evaluator::StatementResult,
    metric: &str,
) -> PyResult<PeriodId> {
    let periods = results
        .nodes
        .get(metric)
        .ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err(format!(
                "Metric '{}' not found in baseline results",
                metric
            ))
        })?
        .keys()
        .copied()
        .collect::<Vec<_>>();

    periods.into_iter().max().ok_or_else(|| {
        pyo3::exceptions::PyValueError::new_err(format!("No periods found for metric '{}'", metric))
    })
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let _ = py;

    parent.add_class::<PyVarianceConfig>()?;
    parent.add_class::<PyVarianceRow>()?;
    parent.add_class::<PyVarianceReport>()?;
    parent.add_class::<PyBridgeStep>()?;
    parent.add_class::<PyBridgeChart>()?;
    parent.add_class::<PyVarianceAnalyzer>()?;

    Ok(vec![
        "VarianceConfig",
        "VarianceRow",
        "VarianceReport",
        "BridgeStep",
        "BridgeChart",
        "VarianceAnalyzer",
    ])
}
