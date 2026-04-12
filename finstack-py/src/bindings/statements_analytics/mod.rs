//! Python bindings for the `finstack-statements-analytics` crate.
//!
//! Exposes financial statement analysis: sensitivity, variance, scenario sets,
//! backtesting, goal seek, introspection, DCF valuation, corporate analysis
//! pipeline, Monte Carlo, and reports.

mod analysis;

use pyo3::prelude::*;
use pyo3::types::PyList;

/// Register the `statements_analytics` submodule on the parent module.
pub fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "statements_analytics")?;
    m.setattr(
        "__doc__",
        "Statement analysis: sensitivity, variance, scenarios, backtesting, goal seek, DCF, corporate, Monte Carlo, reports, introspection.",
    )?;

    analysis::register(py, &m)?;

    let all = PyList::new(
        py,
        [
            "run_sensitivity",
            "generate_tornado_entries",
            "run_variance",
            "evaluate_scenario_set",
            "run_monte_carlo",
            "backtest_forecast",
            "goal_seek",
            "evaluate_dcf",
            "run_corporate_analysis",
            "pl_summary_report",
            "credit_assessment_report",
            "DependencyTracer",
            "trace_dependencies",
            "trace_dependencies_detailed",
            "direct_dependencies",
            "all_dependencies",
            "dependents",
            "explain_formula",
            "explain_formula_text",
            "run_checks",
            "run_three_statement_checks",
            "run_credit_underwriting_checks",
            "render_check_report_text",
            "render_check_report_html",
        ],
    )?;
    m.setattr("__all__", all)?;
    parent.add_submodule(&m)?;

    let parent_name: String = match parent.getattr("__name__") {
        Ok(attr) => match attr.extract::<String>() {
            Ok(s) => s,
            Err(_) => "finstack.finstack".to_string(),
        },
        Err(_) => "finstack.finstack".to_string(),
    };
    let qual = format!("{parent_name}.statements_analytics");
    m.setattr("__package__", &qual)?;
    let sys = PyModule::import(py, "sys")?;
    let modules = sys.getattr("modules")?;
    modules.set_item(&qual, &m)?;

    Ok(())
}
