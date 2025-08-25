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
//! from rfin import Currency, Money, Date, DayCount, FixedRateLeg, Frequency
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
//! # Fixed rate leg creation
//! leg = FixedRateLeg(
//!     notional_amount=1000000,
//!     currency=Currency.usd(),
//!     rate=0.05,
//!     start_date=Date(2023, 1, 1),
//!     end_date=Date(2024, 1, 1),
//!     frequency=Frequency.SemiAnnual,
//!     day_count=DayCount.thirty360()
//! )
//! ```

use pyo3::prelude::*;
use pyo3::types::PyModule;

mod cashflow;
/// Python module for primitives functionality  
mod currency;
mod dates;
mod money;
// (compatibility primitives module removed)

// Add market_data module
mod market_data;

/// Import IMM helper functions for registration
use dates::{py_next_cds_date, py_next_imm, py_third_wednesday};

/// Main Python module initialization
#[pymodule]
fn finstack(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Add version information
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;

    // Create currency submodule
    // Submodules should be created with their short names so that they are available as attributes
    // of the parent module (e.g. `rfin.currency`). We still register the fully-qualified name in
    // `sys.modules` for proper import resolution.
    let currency_module = PyModule::new(m.py(), "currency")?;
    currency_module.add_class::<currency::PyCurrency>()?;
    m.add_submodule(&currency_module)?;
    m.py()
        .import("sys")?
        .getattr("modules")?
        .set_item("finstack.currency", &currency_module)?;

    // Create money submodule
    let money_module = PyModule::new(m.py(), "money")?;
    money_module.add_class::<money::PyMoney>()?;
    m.add_submodule(&money_module)?;
    m.py()
        .import("sys")?
        .getattr("modules")?
        .set_item("finstack.money", &money_module)?;

    // ---------------------------
    // Dates submodule
    // ---------------------------

    let dates_module = PyModule::new(m.py(), "dates")?;
    dates_module.add_class::<dates::PyDate>()?;
    dates_module.add_class::<dates::PyDayCount>()?;
    dates_module.add_class::<dates::PyCalendar>()?;
    dates_module.add_class::<dates::PyBusDayConv>()?;
    dates_module.add_class::<dates::PyFrequency>()?;
    dates_module.add_class::<dates::PyStubRule>()?;
    dates_module.add_function(pyo3::wrap_pyfunction!(
        dates::py_generate_schedule,
        &dates_module
    )?)?;
    dates_module.add_function(pyo3::wrap_pyfunction!(
        dates::py_available_calendars,
        &dates_module
    )?)?;
    dates_module.add_function(pyo3::wrap_pyfunction!(py_third_wednesday, &dates_module)?)?;
    dates_module.add_function(pyo3::wrap_pyfunction!(py_next_imm, &dates_module)?)?;
    dates_module.add_function(pyo3::wrap_pyfunction!(py_next_cds_date, &dates_module)?)?;
    m.add_submodule(&dates_module)?;
    m.py()
        .import("sys")?
        .getattr("modules")?
        .set_item("finstack.dates", &dates_module)?;

    // ---------------------------
    // Cashflow submodule
    // ---------------------------

    let cashflow_module = PyModule::new(m.py(), "cashflow")?;
    cashflow_module.add_class::<crate::cashflow::PyFixedRateLeg>()?;
    cashflow_module.add_class::<crate::cashflow::PyCashFlow>()?;
    m.add_submodule(&cashflow_module)?;
    m.py()
        .import("sys")?
        .getattr("modules")?
        .set_item("finstack.cashflow", &cashflow_module)?;

    // ---------------------------
    // Market data submodule
    // ---------------------------

    market_data::register_module(m)?;

    // --------------------------------------------------------------------
    // Top-level re-exports for ergonomic `from rfin import Currency, Money`
    // --------------------------------------------------------------------

    m.add_class::<currency::PyCurrency>()?;
    m.add_class::<money::PyMoney>()?;
    m.add_class::<dates::PyDate>()?;
    m.add_class::<dates::PyDayCount>()?;
    m.add_class::<dates::PyCalendar>()?;
    m.add_class::<dates::PyBusDayConv>()?;
    m.add_class::<dates::PyFrequency>()?;
    m.add_class::<dates::PyStubRule>()?;
    m.add_class::<crate::cashflow::PyFixedRateLeg>()?;
    m.add_class::<crate::cashflow::PyCashFlow>()?;

    use currency::PyCurrency as PC;
    use finstack_core::Currency as CoreCurrency;

    m.add("USD", PC::from_inner(CoreCurrency::USD))?;
    m.add("EUR", PC::from_inner(CoreCurrency::EUR))?;
    m.add("GBP", PC::from_inner(CoreCurrency::GBP))?;
    m.add("JPY", PC::from_inner(CoreCurrency::JPY))?;

    m.add_function(pyo3::wrap_pyfunction!(dates::py_generate_schedule, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(py_third_wednesday, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(py_next_imm, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(py_next_cds_date, m)?)?;

    Ok(())
}
