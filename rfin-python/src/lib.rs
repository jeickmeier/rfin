//! Python bindings for the RustFin library.

use pyo3::prelude::*;
use pyo3::wrap_pymodule;

/// Python module for dates functionality
mod dates;
/// Python module for primitives functionality  
mod primitives;

/// Dates submodule
#[pymodule]
fn dates_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    dates::register_module(m)
}

/// Primitives submodule
#[pymodule]
fn primitives_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    primitives::register_module(m)
}

/// Main Python module initialization
#[pymodule]
fn rfin(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Add version information
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;

    // Add submodules
    m.add_wrapped(wrap_pymodule!(dates_module))?;
    m.add_wrapped(wrap_pymodule!(primitives_module))?;

    Ok(())
}
