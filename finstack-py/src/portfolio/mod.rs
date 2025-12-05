//! Python bindings for the finstack-portfolio crate.
//!
//! This module provides Python bindings for portfolio management, aggregation,
//! valuation, and metrics calculation.

pub(crate) mod attribution;
pub(crate) mod builder;
pub(crate) mod cashflows;
pub(crate) mod dataframe;
pub(crate) mod error;
pub(crate) mod grouping;
pub(crate) mod margin;
pub(crate) mod metrics;
pub(crate) mod optimization;
pub(crate) mod portfolio;
pub(crate) mod results;
pub(crate) mod types;
pub(crate) mod valuation;

#[cfg(feature = "scenarios")]
pub(crate) mod scenarios;

use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;

/// Register the portfolio module and all its submodules.
pub(crate) fn register<'py>(py: Python<'py>, parent: &Bound<'py, PyModule>) -> PyResult<()> {
    let module = PyModule::new(py, "portfolio")?;
    module.setattr(
        "__doc__",
        concat!(
            "Portfolio management and aggregation for finstack.\n\n",
            "This module provides portfolio-level operations including entity and position ",
            "management, valuation aggregation, metrics calculation, attribute-based grouping, ",
            "and DataFrame exports for analysis."
        ),
    )?;

    // Register types
    let type_exports = types::register(py, &module)?;

    // Register portfolio and builder
    let portfolio_exports = portfolio::register(py, &module)?;
    let builder_exports = builder::register(py, &module)?;

    // Register valuation and metrics
    let valuation_exports = valuation::register(py, &module)?;
    let metrics_exports = metrics::register(py, &module)?;

    // Register optimization helpers
    let optimization_exports = optimization::register(py, &module)?;

    // Register results
    let results_exports = results::register(py, &module)?;

    // Register grouping functions
    let grouping_exports = grouping::register(py, &module)?;

    // Register attribution and cashflows
    let attribution_exports = attribution::register(py, &module)?;
    let cashflow_exports = cashflows::register(py, &module)?;

    // Register dataframe exports
    let dataframe_exports = dataframe::register(py, &module)?;

    // Register margin utilities
    let margin_exports = margin::register(py, &module)?;

    // Register scenarios integration if feature enabled
    #[cfg(feature = "scenarios")]
    let scenarios_exports = scenarios::register(py, &module)?;

    // Collect all exports
    let mut all_exports = Vec::new();
    all_exports.extend(type_exports);
    all_exports.extend(portfolio_exports);
    all_exports.extend(builder_exports);
    all_exports.extend(valuation_exports);
    all_exports.extend(metrics_exports);
    all_exports.extend(optimization_exports);
    all_exports.extend(results_exports);
    all_exports.extend(grouping_exports);
    all_exports.extend(attribution_exports);
    all_exports.extend(cashflow_exports);
    all_exports.extend(dataframe_exports);
    all_exports.extend(margin_exports);

    #[cfg(feature = "scenarios")]
    all_exports.extend(scenarios_exports);

    // Set __all__ for the module
    let all_list = PyList::new(py, &all_exports)?;
    module.setattr("__all__", all_list)?;

    parent.add_submodule(&module)?;
    parent.setattr("portfolio", &module)?;

    Ok(())
}
