//! Python bindings for the RustFin library.

use pyo3::prelude::*;
use pyo3::types::PyModule;

mod calendar;
mod cashflow;
/// Python module for primitives functionality  
mod currency;
mod dates;
mod daycount;
mod money;
mod schedule;
// (compatibility primitives module removed)

/// Import IMM helper functions for registration
use dates::{py_next_cds_date, py_next_imm, py_third_wednesday};

/// Main Python module initialization
#[pymodule]
fn rfin(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Add version information
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;

    // Create currency submodule
    // Submodules should be created with their short names so that they are available as attributes
    // of the parent module (e.g. `rfin.currency`). We still register the fully-qualified name in
    // `sys.modules` for proper import resolution.
    let currency_module = PyModule::new_bound(m.py(), "currency")?;
    currency_module.add_class::<currency::PyCurrency>()?;
    m.add_submodule(&currency_module)?;
    m.py()
        .import_bound("sys")?
        .getattr("modules")?
        .set_item("rfin.currency", &currency_module)?;

    // Create money submodule
    let money_module = PyModule::new_bound(m.py(), "money")?;
    money_module.add_class::<money::PyMoney>()?;
    m.add_submodule(&money_module)?;
    m.py()
        .import_bound("sys")?
        .getattr("modules")?
        .set_item("rfin.money", &money_module)?;

    // ---------------------------
    // Dates submodule
    // ---------------------------

    let dates_module = PyModule::new_bound(m.py(), "dates")?;
    dates_module.add_class::<dates::PyDate>()?;
    dates_module.add_class::<daycount::PyDayCount>()?;
    dates_module.add_class::<calendar::PyCalendar>()?;
    dates_module.add_class::<calendar::PyBusDayConv>()?;
    dates_module.add_class::<schedule::PyFrequency>()?;
    dates_module.add_class::<schedule::PyStubRule>()?;
    dates_module.add_function(pyo3::wrap_pyfunction_bound!(
        schedule::py_generate_schedule,
        m.py()
    )?)?;
    dates_module.add_function(pyo3::wrap_pyfunction_bound!(
        calendar::py_available_calendars,
        m.py()
    )?)?;
    dates_module.add_function(pyo3::wrap_pyfunction_bound!(py_third_wednesday, m.py())?)?;
    dates_module.add_function(pyo3::wrap_pyfunction_bound!(py_next_imm, m.py())?)?;
    dates_module.add_function(pyo3::wrap_pyfunction_bound!(py_next_cds_date, m.py())?)?;
    m.add_submodule(&dates_module)?;
    m.py()
        .import_bound("sys")?
        .getattr("modules")?
        .set_item("rfin.dates", &dates_module)?;

    // ---------------------------
    // Cashflow submodule
    // ---------------------------

    let cashflow_module = PyModule::new_bound(m.py(), "cashflow")?;
    cashflow_module.add_class::<crate::cashflow::PyFixedRateLeg>()?;
    cashflow_module.add_class::<crate::cashflow::PyCashFlow>()?;
    m.add_submodule(&cashflow_module)?;
    m.py()
        .import_bound("sys")?
        .getattr("modules")?
        .set_item("rfin.cashflow", &cashflow_module)?;

    // --------------------------------------------------------------------
    // Top-level re-exports for ergonomic `from rfin import Currency, Money`
    // --------------------------------------------------------------------

    m.add_class::<currency::PyCurrency>()?;
    m.add_class::<money::PyMoney>()?;
    m.add_class::<dates::PyDate>()?;
    m.add_class::<daycount::PyDayCount>()?;
    m.add_class::<calendar::PyCalendar>()?;
    m.add_class::<calendar::PyBusDayConv>()?;
    m.add_class::<schedule::PyFrequency>()?;
    m.add_class::<schedule::PyStubRule>()?;
    m.add_class::<crate::cashflow::PyFixedRateLeg>()?;
    m.add_class::<crate::cashflow::PyCashFlow>()?;

    use currency::PyCurrency as PC;
    use rfin_core::Currency as CoreCurrency;

    m.add("USD", PC::from_inner(CoreCurrency::USD))?;
    m.add("EUR", PC::from_inner(CoreCurrency::EUR))?;
    m.add("GBP", PC::from_inner(CoreCurrency::GBP))?;
    m.add("JPY", PC::from_inner(CoreCurrency::JPY))?;

    m.add_function(pyo3::wrap_pyfunction_bound!(
        schedule::py_generate_schedule,
        m.py()
    )?)?;
    m.add_function(pyo3::wrap_pyfunction_bound!(py_third_wednesday, m.py())?)?;
    m.add_function(pyo3::wrap_pyfunction_bound!(py_next_imm, m.py())?)?;
    m.add_function(pyo3::wrap_pyfunction_bound!(py_next_cds_date, m.py())?)?;

    Ok(())
}
