//! Python bindings for the finstack library.
//!
//! All business logic stays in Rust core crates. These bindings handle only:
//! - Type conversion (Python <-> Rust)
//! - Wrapper construction and accessor methods
//! - Error mapping to Python exceptions

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use pyo3::prelude::*;

mod bindings;
mod errors;

/// Python bindings for the finstack library.
#[pymodule]
fn finstack(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    bindings::register_root(py, m)
}
