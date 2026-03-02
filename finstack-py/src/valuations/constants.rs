//! Python bindings for shared numerical constants used across pricing and risk.
//!
//! Exposes top-level rate-conversion constants plus nested submodules for
//! numerical tolerances, ISDA conventions, credit-derivatives parameters,
//! and business-day-count conventions.

use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;

use finstack_valuations::constants;

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "constants")?;
    module.setattr(
        "__doc__",
        "Shared numerical constants for pricing and risk calculations.\n\n\
         Submodules: ``numerical``, ``isda``, ``credit``, ``time``.",
    )?;

    // ── top-level rate-conversion constants ──────────────────────────────
    module.add("ONE_BASIS_POINT", constants::ONE_BASIS_POINT)?;
    module.add("BASIS_POINTS_PER_UNIT", constants::BASIS_POINTS_PER_UNIT)?;
    module.add("PERCENT_TO_DECIMAL", constants::PERCENT_TO_DECIMAL)?;
    module.add("DECIMAL_TO_PERCENT", constants::DECIMAL_TO_PERCENT)?;

    // ── numerical submodule ─────────────────────────────────────────────
    let numerical = PyModule::new(py, "numerical")?;
    numerical.setattr(
        "__doc__",
        "Numerical tolerances used by solvers and comparisons.",
    )?;
    numerical.add("ZERO_TOLERANCE", constants::numerical::ZERO_TOLERANCE)?;
    numerical.add(
        "INTEGRATION_STEP_FACTOR",
        constants::numerical::INTEGRATION_STEP_FACTOR,
    )?;
    numerical.add("SOLVER_TOLERANCE", constants::numerical::SOLVER_TOLERANCE)?;
    numerical.add(
        "RATE_COMPARISON_TOLERANCE",
        constants::numerical::RATE_COMPARISON_TOLERANCE,
    )?;
    numerical.add("DIVISION_EPSILON", constants::numerical::DIVISION_EPSILON)?;
    numerical.add(
        "RELATIVE_TOLERANCE",
        constants::numerical::RELATIVE_TOLERANCE,
    )?;
    numerical.add("DF_EPSILON", constants::numerical::DF_EPSILON)?;

    let numerical_exports: Vec<&str> = vec![
        "ZERO_TOLERANCE",
        "INTEGRATION_STEP_FACTOR",
        "SOLVER_TOLERANCE",
        "RATE_COMPARISON_TOLERANCE",
        "DIVISION_EPSILON",
        "RELATIVE_TOLERANCE",
        "DF_EPSILON",
    ];
    numerical.setattr("__all__", PyList::new(py, &numerical_exports)?)?;
    module.add_submodule(&numerical)?;

    // ── isda submodule ──────────────────────────────────────────────────
    let isda = PyModule::new(py, "isda")?;
    isda.setattr(
        "__doc__",
        "ISDA standard conventions for credit derivatives.",
    )?;
    isda.add(
        "STANDARD_RECOVERY_SENIOR",
        constants::isda::STANDARD_RECOVERY_SENIOR,
    )?;
    isda.add(
        "STANDARD_RECOVERY_SUB",
        constants::isda::STANDARD_RECOVERY_SUB,
    )?;
    isda.add(
        "STANDARD_INTEGRATION_POINTS",
        constants::isda::STANDARD_INTEGRATION_POINTS,
    )?;
    isda.add("STANDARD_COUPON_DAY", constants::isda::STANDARD_COUPON_DAY)?;

    let isda_exports: Vec<&str> = vec![
        "STANDARD_RECOVERY_SENIOR",
        "STANDARD_RECOVERY_SUB",
        "STANDARD_INTEGRATION_POINTS",
        "STANDARD_COUPON_DAY",
    ];
    isda.setattr("__all__", PyList::new(py, &isda_exports)?)?;
    module.add_submodule(&isda)?;

    // ── credit submodule ────────────────────────────────────────────────
    let credit = PyModule::new(py, "credit")?;
    credit.setattr(
        "__doc__",
        "Credit-derivatives specific constants (survival floors, Greeks guards, bootstrapping bounds).",
    )?;
    credit.add(
        "SURVIVAL_PROBABILITY_FLOOR",
        constants::credit::SURVIVAL_PROBABILITY_FLOOR,
    )?;
    credit.add(
        "MIN_TIME_TO_EXPIRY_GREEKS",
        constants::credit::MIN_TIME_TO_EXPIRY_GREEKS,
    )?;
    credit.add(
        "MIN_VOLATILITY_GREEKS",
        constants::credit::MIN_VOLATILITY_GREEKS,
    )?;
    credit.add("MIN_FORWARD_SPREAD", constants::credit::MIN_FORWARD_SPREAD)?;
    credit.add("MIN_HAZARD_RATE", constants::credit::MIN_HAZARD_RATE)?;
    credit.add(
        "DEFAULT_MAX_HAZARD_RATE",
        constants::credit::DEFAULT_MAX_HAZARD_RATE,
    )?;
    credit.add(
        "HAZARD_RATE_BRACKET_MULTIPLIER",
        constants::credit::HAZARD_RATE_BRACKET_MULTIPLIER,
    )?;
    credit.add(
        "PAR_SPREAD_DENOM_TOLERANCE",
        constants::credit::PAR_SPREAD_DENOM_TOLERANCE,
    )?;
    credit.add(
        "SMALL_POOL_THRESHOLD",
        constants::credit::SMALL_POOL_THRESHOLD,
    )?;
    credit.add(
        "CALENDAR_DAYS_PER_YEAR",
        constants::credit::CALENDAR_DAYS_PER_YEAR,
    )?;

    let credit_exports: Vec<&str> = vec![
        "SURVIVAL_PROBABILITY_FLOOR",
        "MIN_TIME_TO_EXPIRY_GREEKS",
        "MIN_VOLATILITY_GREEKS",
        "MIN_FORWARD_SPREAD",
        "MIN_HAZARD_RATE",
        "DEFAULT_MAX_HAZARD_RATE",
        "HAZARD_RATE_BRACKET_MULTIPLIER",
        "PAR_SPREAD_DENOM_TOLERANCE",
        "SMALL_POOL_THRESHOLD",
        "CALENDAR_DAYS_PER_YEAR",
    ];
    credit.setattr("__all__", PyList::new(py, &credit_exports)?)?;
    module.add_submodule(&credit)?;

    // ── time submodule ──────────────────────────────────────────────────
    let time = PyModule::new(py, "time")?;
    time.setattr("__doc__", "Business day counts per year by market region.")?;
    time.add(
        "BUSINESS_DAYS_PER_YEAR_US",
        constants::time::BUSINESS_DAYS_PER_YEAR_US,
    )?;
    time.add(
        "BUSINESS_DAYS_PER_YEAR_UK",
        constants::time::BUSINESS_DAYS_PER_YEAR_UK,
    )?;
    time.add(
        "BUSINESS_DAYS_PER_YEAR_JP",
        constants::time::BUSINESS_DAYS_PER_YEAR_JP,
    )?;

    let time_exports: Vec<&str> = vec![
        "BUSINESS_DAYS_PER_YEAR_US",
        "BUSINESS_DAYS_PER_YEAR_UK",
        "BUSINESS_DAYS_PER_YEAR_JP",
    ];
    time.setattr("__all__", PyList::new(py, &time_exports)?)?;
    module.add_submodule(&time)?;

    // ── module-level __all__ ────────────────────────────────────────────
    let exports: Vec<&'static str> = vec![
        "ONE_BASIS_POINT",
        "BASIS_POINTS_PER_UNIT",
        "PERCENT_TO_DECIMAL",
        "DECIMAL_TO_PERCENT",
        "numerical",
        "isda",
        "credit",
        "time",
    ];
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;

    // We do not promote constants to the parent `valuations` namespace;
    // users access them via `finstack.valuations.constants.*`.
    Ok(vec![])
}
