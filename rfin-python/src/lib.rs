//! Python bindings for the RustFin library.

use pyo3::prelude::*;
use pyo3::types::PyModule;

/// Python module for primitives functionality  
mod currency;
mod money;
mod dates;
mod daycount;
// (compatibility primitives module removed)

/// Main Python module initialization
#[pymodule]
fn rfin(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Add version information
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;

    // Create currency submodule
    let currency_module = PyModule::new_bound(m.py(), "rfin.currency")?;
    currency_module.add_class::<currency::PyCurrency>()?;
    m.add_submodule(&currency_module)?;
    m.py().import_bound("sys")?.getattr("modules")?
        .set_item("rfin.currency", &currency_module)?;

    // Create money submodule
    let money_module = PyModule::new_bound(m.py(), "rfin.money")?;
    money_module.add_class::<money::PyMoney>()?;
    m.add_submodule(&money_module)?;
    m.py().import_bound("sys")?.getattr("modules")?
        .set_item("rfin.money", &money_module)?;

    // ---------------------------
    // Dates submodule
    // ---------------------------

    let dates_module = PyModule::new_bound(m.py(), "rfin.dates")?;
    dates_module.add_class::<dates::PyDate>()?;
    dates_module.add_class::<daycount::PyDayCount>()?;
    m.add_submodule(&dates_module)?;
    m.py().import_bound("sys")?.getattr("modules")?
        .set_item("rfin.dates", &dates_module)?;

    // --------------------------------------------------------------------
    // Top-level re-exports for ergonomic `from rfin import Currency, Money`
    // --------------------------------------------------------------------

    m.add_class::<currency::PyCurrency>()?;
    m.add_class::<money::PyMoney>()?;
    m.add_class::<dates::PyDate>()?;
    m.add_class::<daycount::PyDayCount>()?;

    use rfin_core::Currency as CoreCurrency;
    use currency::PyCurrency as PC;

    m.add("USD", PC::from_inner(CoreCurrency::USD))?;
    m.add("EUR", PC::from_inner(CoreCurrency::EUR))?;
    m.add("GBP", PC::from_inner(CoreCurrency::GBP))?;
    m.add("JPY", PC::from_inner(CoreCurrency::JPY))?;

    Ok(())
}
