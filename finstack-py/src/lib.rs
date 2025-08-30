//! Python bindings for the RustFin library.
//!
//! This module provides Python bindings for the RustFin quantitative finance library,
//! offering a comprehensive set of tools for financial calculations, date handling,
//! and instrument valuation.
//!
//! The library is organized into several submodules:
//! - `currency`: Currency handling and representation
//! - `money`: Monetary amounts with currency-safe arithmetic
//! - `dates`: Date handling, calendars, and business day calculations
//! - `cashflow`: Cash flow generation and valuation
//!
//! # Quick Start
//!
//! ```python
//! from finstack import Currency, Money, Date, DayCount
//! from finstack.cashflow import CashflowBuilder, Amortization
//! from finstack.dates import Frequency
//!
//! # Create monetary amounts
//! usd_100 = Money(100.0, Currency.usd())
//! eur_75 = Money(75.0, Currency.eur())
//!
//! # Date handling
//! date = Date(2023, 12, 25)
//! print(date.is_weekend())  # Check if it's a weekend
//!
//! # Day count calculations
//! dc = DayCount.act360()
//! yf = dc.year_fraction(Date(2023, 1, 1), Date(2023, 7, 1))
//!
//! # Build cashflow schedule
//! builder = CashflowBuilder()
//! schedule = (builder
//!     .principal(Money(1000000, Currency.usd()),
//!                Date(2023, 1, 1), Date(2024, 1, 1))
//!     .fixed_coupon(rate=0.05, frequency=Frequency.SemiAnnual,
//!                   day_count=DayCount.thirty360())
//!     .build())
//! ```

use pyo3::prelude::*;
use pyo3::types::PyModule;

// Core module contains primitives and basic types
mod core;
// Valuations module contains cashflow, instruments, and pricing
mod valuations;

/// Import IMM helper functions for registration
use core::dates::{py_next_cds_date, py_next_imm, py_third_wednesday};

/// Main Python module initialization
#[pymodule]
fn finstack(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Add version information
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;

    // =============================
    // Core Submodules
    // =============================

    // Create currency submodule
    let currency_module = PyModule::new(m.py(), "currency")?;
    currency_module.add_class::<core::currency::PyCurrency>()?;
    m.add_submodule(&currency_module)?;
    m.py()
        .import("sys")?
        .getattr("modules")?
        .set_item("finstack.currency", &currency_module)?;

    // Create money submodule
    let money_module = PyModule::new(m.py(), "money")?;
    money_module.add_class::<core::money::PyMoney>()?;
    m.add_submodule(&money_module)?;
    m.py()
        .import("sys")?
        .getattr("modules")?
        .set_item("finstack.money", &money_module)?;

    // Create dates submodule
    let dates_module = PyModule::new(m.py(), "dates")?;
    dates_module.add_class::<core::dates::PyDate>()?;
    dates_module.add_class::<core::dates::PyDayCount>()?;
    dates_module.add_class::<core::dates::PyCalendar>()?;
    dates_module.add_class::<core::dates::PyBusDayConv>()?;
    dates_module.add_class::<core::dates::PyFrequency>()?;
    dates_module.add_class::<core::dates::PyStubRule>()?;
    dates_module.add_class::<core::dates::PyPeriodId>()?;
    dates_module.add_class::<core::dates::PyPeriod>()?;
    dates_module.add_class::<core::dates::PyFiscalConfig>()?;
    dates_module.add_function(pyo3::wrap_pyfunction!(
        core::dates::py_generate_schedule,
        &dates_module
    )?)?;
    dates_module.add_function(pyo3::wrap_pyfunction!(
        core::dates::py_available_calendars,
        &dates_module
    )?)?;
    dates_module.add_function(pyo3::wrap_pyfunction!(py_third_wednesday, &dates_module)?)?;
    dates_module.add_function(pyo3::wrap_pyfunction!(py_next_imm, &dates_module)?)?;
    dates_module.add_function(pyo3::wrap_pyfunction!(py_next_cds_date, &dates_module)?)?;
    dates_module.add_function(pyo3::wrap_pyfunction!(
        core::dates::py_build_periods,
        &dates_module
    )?)?;
    dates_module.add_function(pyo3::wrap_pyfunction!(
        core::dates::py_build_fiscal_periods,
        &dates_module
    )?)?;
    m.add_submodule(&dates_module)?;
    m.py()
        .import("sys")?
        .getattr("modules")?
        .set_item("finstack.dates", &dates_module)?;

    // Register market data submodule
    core::market_data::register_module(m)?;

    // =============================
    // Valuations Submodules
    // =============================

    // Create cashflow submodule
    let cashflow_module = PyModule::new(m.py(), "cashflow")?;
    cashflow_module.add_class::<valuations::cashflow::PyCashFlow>()?;
    cashflow_module.add_class::<valuations::cashflow::PyCouponType>()?;
    cashflow_module.add_class::<valuations::cashflow::PyAmortization>()?;
    cashflow_module.add_class::<valuations::cashflow::PyCashFlowSchedule>()?;
    cashflow_module.add_class::<valuations::cashflow::PyCashflowBuilder>()?;
    valuations::cashflow::register_functions(&cashflow_module)?;
    m.add_submodule(&cashflow_module)?;
    m.py()
        .import("sys")?
        .getattr("modules")?
        .set_item("finstack.cashflow", &cashflow_module)?;

    // Register instruments submodule
    valuations::instruments::register_module(m)?;

    // Register risk metrics submodule
    valuations::risk::register_module(m)?;

    // Register covenants submodule
    valuations::covenants::register_module(m)?;

    // Register valuation results and attributes
    m.add_class::<valuations::results::PyValuationResult>()?;
    m.add_class::<valuations::attributes::PyAttributes>()?;

    // =============================
    // Top-level Re-exports
    // =============================
    // For ergonomic `from finstack import Currency, Money, Date` etc.

    // Core types
    m.add_class::<core::currency::PyCurrency>()?;
    m.add_class::<core::money::PyMoney>()?;
    m.add_class::<core::dates::PyDate>()?;
    m.add_class::<core::dates::PyDayCount>()?;
    m.add_class::<core::dates::PyCalendar>()?;
    m.add_class::<core::dates::PyBusDayConv>()?;
    m.add_class::<core::dates::PyFrequency>()?;
    m.add_class::<core::dates::PyStubRule>()?;
    m.add_class::<core::dates::PyPeriodId>()?;
    m.add_class::<core::dates::PyPeriod>()?;
    m.add_class::<core::dates::PyFiscalConfig>()?;

    // Valuation types
    m.add_class::<valuations::cashflow::PyCashFlow>()?;
    m.add_class::<valuations::cashflow::PyCouponType>()?;
    m.add_class::<valuations::cashflow::PyAmortization>()?;
    m.add_class::<valuations::cashflow::PyCashFlowSchedule>()?;
    m.add_class::<valuations::cashflow::PyCashflowBuilder>()?;

    // Covenant types
    m.add_class::<valuations::covenants::PyCovenantType>()?;
    m.add_class::<valuations::covenants::PyCovenantConsequence>()?;
    m.add_class::<valuations::covenants::PyCovenant>()?;
    m.add_class::<valuations::covenants::PyCovenantReport>()?;
    m.add_class::<valuations::covenants::PyCovenantBreach>()?;
    m.add_class::<valuations::covenants::PyCovenantEngine>()?;

    // Workout and Policy bindings removed from Python surface

    // Currency constants
    use core::currency::PyCurrency as PC;
    use finstack_core::Currency as CoreCurrency;

    m.add("USD", PC::from_inner(CoreCurrency::USD))?;
    m.add("EUR", PC::from_inner(CoreCurrency::EUR))?;
    m.add("GBP", PC::from_inner(CoreCurrency::GBP))?;
    m.add("JPY", PC::from_inner(CoreCurrency::JPY))?;

    // Top-level functions
    m.add_function(pyo3::wrap_pyfunction!(
        core::dates::py_generate_schedule,
        m
    )?)?;
    m.add_function(pyo3::wrap_pyfunction!(py_third_wednesday, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(py_next_imm, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(py_next_cds_date, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(core::dates::py_build_periods, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(
        core::dates::py_build_fiscal_periods,
        m
    )?)?;
    m.add_function(pyo3::wrap_pyfunction!(
        valuations::cashflow::py_cashflows_to_dataframe,
        m
    )?)?;

    Ok(())
}
