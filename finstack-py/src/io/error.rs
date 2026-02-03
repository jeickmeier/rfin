//! Error mapping for finstack-io Python bindings.

use finstack_io::Error as IoError;
use pyo3::create_exception;
use pyo3::prelude::*;

// =============================================================================
// IO-specific Exception Hierarchy
// =============================================================================

// Import the base FinstackError from the main errors module
use crate::errors::FinstackError;

// IO-specific errors
create_exception!(finstack.io, PyIoError, FinstackError);
create_exception!(finstack.io, NotFoundError, PyIoError);
create_exception!(finstack.io, SchemaVersionError, PyIoError);

/// Register IO exceptions in the module.
pub(crate) fn register_exceptions(py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("IoError", py.get_type::<PyIoError>())?;
    m.add("NotFoundError", py.get_type::<NotFoundError>())?;
    m.add("SchemaVersionError", py.get_type::<SchemaVersionError>())?;
    Ok(())
}

/// Map a finstack-io Error to an appropriate Python exception.
pub(crate) fn map_io_error(err: IoError) -> PyErr {
    match err {
        IoError::NotFound { entity, id } => {
            NotFoundError::new_err(format!("{} not found: {}", entity, id))
        }
        IoError::UnsupportedSchema { found, expected } => SchemaVersionError::new_err(format!(
            "Unsupported schema version: found {}, expected {}",
            found, expected
        )),
        IoError::Sqlite(e) => PyIoError::new_err(format!("SQLite error: {}", e)),
        IoError::SerdeJson(e) => PyIoError::new_err(format!("JSON serialization error: {}", e)),
        IoError::Io(e) => PyIoError::new_err(format!("I/O error: {}", e)),
        IoError::Invariant(msg) => PyIoError::new_err(format!("Internal error: {}", msg)),
        IoError::Core(e) => crate::errors::map_error(e),
        IoError::Portfolio(e) => crate::portfolio::error::portfolio_to_py(e),
        IoError::Statements(e) => crate::statements::error::stmt_to_py(e),
        IoError::Scenarios(e) => crate::scenarios::error::scenario_to_py(e),
        _ => PyIoError::new_err(format!("IO error: {}", err)),
    }
}
