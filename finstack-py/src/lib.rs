//! Python bindings for the finstack library.
//!
//! ## WASM Parity Note
//!
//! All business logic must stay in Rust core crates to ensure WASM bindings can share
//! the same functionality. These Python bindings only handle:
//! - Type conversion (Python → Rust, Rust → Python)
//! - Wrapper construction and accessor methods
//! - Error mapping to Python exceptions
//! - Ergonomic helpers (operator overloading, flexible argument parsing)
//!
//! No financial calculations, validation logic, or algorithms should be implemented here.
//! Centralized argument extraction types are in `crate::core::common::args`.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(clippy::new_without_default)]
#![warn(clippy::float_cmp)]
#![cfg_attr(test, allow(clippy::float_cmp))]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule, PyModuleMethods};
use pyo3::Bound;

mod analytics;
mod core;
mod errors;
mod portfolio;
mod scenarios;
mod statements;
mod valuations;

use core::currency::PyCurrency;
use core::market_data::PyDiscountCurve;
use core::money::PyMoney;

/// Python bindings for the `finstack-core` crate.
#[pymodule]
fn finstack(py: Python<'_>, m: Bound<'_, PyModule>) -> PyResult<()> {
    m.setattr("__package__", "finstack")?;
    m.setattr(
        "__doc__",
        concat!(
            "High-level financial primitives from the Rust finstack core crate.\n\n",
            "These bindings surface currencies, configuration, money arithmetic, business-day ",
            "calendars, and market data (curves, FX, surfaces, scalars) with Python-friendly docs",
            " and type hints. Downstream code should import from `finstack` directly.",
        ),
    )?;

    // Core module tree mirroring finstack-core layout
    let core_mod = PyModule::new(py, "core")?;
    core_mod.setattr(
        "__doc__",
        "Bindings for the finstack-core crate organised by domain modules.",
    )?;

    core::cashflow::register(py, &core_mod)?;
    core::currency::register(py, &core_mod)?;
    core::config::register(py, &core_mod)?;
    core::money::register(py, &core_mod)?;
    core::dates::register(py, &core_mod)?;
    core::explain::register(py, &core_mod)?;
    core::market_data::register(py, &core_mod)?;
    core::math::register(py, &core_mod)?;
    core::volatility::register(py, &core_mod)?;
    core::types::register(py, &core_mod)?;
    core::expr::register(py, &core_mod)?;

    let core_exports = PyList::new(
        py,
        [
            "cashflow",
            "currency",
            "config",
            "money",
            "dates",
            "explain",
            "market_data",
            "math",
            "volatility",
            "types",
            "expr",
        ],
    )?;
    core_mod.setattr("__all__", core_exports)?;
    m.add_submodule(&core_mod)?;
    m.setattr("core", &core_mod)?;

    // Expose core submodules at package root for convenience and stub generation
    for name in [
        "cashflow",
        "currency",
        "config",
        "money",
        "dates",
        "explain",
        "market_data",
        "math",
        "volatility",
        "types",
        "expr",
    ] {
        if let Ok(sub) = core_mod.getattr(name) {
            m.setattr(name, &sub)?;
        }
    }

    // Register custom exception hierarchy
    errors::register_exceptions(py, &m)?;

    // Root-level placeholders for backwards compatibility (minimal set)
    m.add_class::<PyCurrency>()?;
    m.add_class::<PyMoney>()?;
    m.add_class::<PyDiscountCurve>()?;

    // Valuations bindings (module registers itself under `valuations`)
    valuations::register(py, &m)?;

    // Statements bindings (module registers itself under `statements`)
    statements::register(py, &m)?;

    // Scenarios bindings (module registers itself under `scenarios`)
    scenarios::register(py, &m)?;

    // Portfolio bindings (module registers itself under `portfolio`)
    portfolio::register(py, &m)?;

    // Analytics bindings (module registers itself under `analytics`)
    analytics::register(py, &m)?;

    // Re-export selected helpers at package root for convenience
    let dates_binding = core_mod.getattr("dates")?;
    let dates_mod = dates_binding.cast::<PyModule>()?;
    let adjust = dates_mod.getattr("adjust")?;
    core_mod.setattr("adjust", &adjust)?;
    m.setattr("adjust", adjust)?;
    for attr in [
        "available_calendars",
        "available_calendar_codes",
        "get_calendar",
        "next_imm",
        "next_cds_date",
        "next_imm_option_expiry",
        "imm_option_expiry",
        "next_equity_option_expiry",
        "third_friday",
        "third_wednesday",
        "build_periods",
        "build_fiscal_periods",
        "add_months",
        "last_day_of_month",
        "days_in_month",
        "is_leap_year",
        "date_to_days_since_epoch",
        "days_since_epoch_to_date",
    ] {
        if let Ok(value) = dates_mod.getattr(attr) {
            core_mod.setattr(attr, &value)?;
            m.setattr(attr, value)?;
        }
    }

    // Convenience re-exports for covenant forecasting at package root
    if let Ok(valuations_mod) = m.getattr("valuations") {
        if let Ok(val_mod) = valuations_mod.cast::<PyModule>() {
            if let Ok(cov_mod) = val_mod.getattr("covenants") {
                let cov_mod = cov_mod.cast::<PyModule>()?;
                for attr in [
                    "CovenantType",
                    "Covenant",
                    "CovenantSpec",
                    "CovenantScope",
                    "SpringingCondition",
                    "CovenantForecastConfig",
                    "CovenantForecast",
                    "forecast_covenant",
                    "forecast_breaches",
                ] {
                    if let Ok(value) = cov_mod.getattr(attr) {
                        m.setattr(attr, &value)?;
                    }
                }
            }
        }
    }

    let all = PyList::new(
        py,
        [
            "core",
            "cashflow",
            "currency",
            "config",
            "money",
            "dates",
            "explain",
            "market_data",
            "math",
            "types",
            "volatility",
            "expr",
            "valuations",
            "statements",
            "scenarios",
            "portfolio",
            "analytics",
            "Currency",
            "Money",
            "DiscountCurve",
            // Common convenience functions
            "build_periods",
            "build_fiscal_periods",
            "CovenantType",
            "Covenant",
            "CovenantSpec",
            "CovenantScope",
            "SpringingCondition",
            "CovenantForecastConfig",
            "CovenantForecast",
            "forecast_covenant",
            "forecast_breaches",
            "FinstackError",
            "ConfigurationError",
            "MissingCurveError",
            "MissingFxRateError",
            "InvalidConfigError",
            "ComputationError",
            "ConvergenceError",
            "CalibrationError",
            "PricingError",
            "ValidationError",
            "CurrencyMismatchError",
            "DateError",
            "ParameterError",
            "InternalError",
        ],
    )?;
    m.setattr("__all__", all)?;

    Ok(())
}
