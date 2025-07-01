//! Python bindings for the RustFin library.

use pyo3::prelude::*;
use pyo3::types::PyModule;

/// Python module for dates functionality
mod dates;
/// Python module for primitives functionality  
mod primitives;

/// Dates submodule
#[pymodule]
fn dates_submodule(m: &Bound<'_, PyModule>) -> PyResult<()> {
    dates::register_module(m)
}

/// Primitives submodule
#[pymodule]
fn primitives_submodule(m: &Bound<'_, PyModule>) -> PyResult<()> {
    primitives::register_module(m)
}

/// Main Python module initialization
#[pymodule]
fn rfin(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Add version information
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;

    // Create and add submodules
    let dates_module = PyModule::new_bound(m.py(), "rfin.dates")?;
    dates::register_module(&dates_module)?;
    m.add_submodule(&dates_module)?;
    let sys = m.py().import_bound("sys")?;
    sys.getattr("modules")?
        .set_item("rfin.dates", &dates_module)?;

    let primitives_module = PyModule::new_bound(m.py(), "rfin.primitives")?;
    primitives::register_module(&primitives_module)?;
    m.add_submodule(&primitives_module)?;
    let sys = m.py().import_bound("sys")?;
    sys.getattr("modules")?
        .set_item("rfin.primitives", &primitives_module)?;

    Ok(())
}
