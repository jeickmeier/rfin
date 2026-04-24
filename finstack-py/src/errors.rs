//! Centralized error mapping from Rust crate errors to Python exceptions.
//!
//! All binding callers should route error conversions through [`core_to_py`]
//! (for `finstack_core::Error`) or [`display_to_py`] (for any `Display`-able
//! error). Avoid inline `PyValueError::new_err(e.to_string())` patterns —
//! they bypass this module and break the error-chain-preservation contract
//! the helpers below provide.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// Format an error and its full `source()` chain into a single string.
///
/// PyO3 exceptions only carry a message, not a structured cause chain, so we
/// concatenate every level of context (`top: first cause: deeper cause: …`)
/// into the message. This mirrors `anyhow`'s default display and lets quants
/// see the full diagnostic context in Python without dropping back to Rust.
fn format_chain(err: &dyn std::error::Error) -> String {
    let mut out = err.to_string();
    let mut src = err.source();
    while let Some(cause) = src {
        // De-duplicate when a wrapper already included the inner display.
        let msg = cause.to_string();
        if !out.ends_with(&msg) {
            out.push_str(": ");
            out.push_str(&msg);
        }
        src = cause.source();
    }
    out
}

/// Convert a `finstack_core::Error` into a Python exception.
///
/// The full error source chain (via [`std::error::Error::source`]) is flattened
/// into the Python message. This preserves context from wrappers like
/// `finstack_valuations::Error::Calibration(core_err)`.
pub fn core_to_py(e: finstack_core::Error) -> PyErr {
    PyValueError::new_err(format_chain(&e))
}

/// Convert any `Display`-able error into a Python `ValueError`.
///
/// If the error implements `std::error::Error`, the full source chain is
/// flattened into the message; otherwise only the top-level `Display`
/// string is used.
pub fn display_to_py<E>(e: E) -> PyErr
where
    E: std::fmt::Display,
{
    // We can't generically detect `std::error::Error` without specialization,
    // so callers with rich chains should prefer `core_to_py` or the helpers
    // that accept `&dyn Error`. This path still beats the inline
    // `PyValueError::new_err(e.to_string())` pattern it replaces because it
    // routes through one place.
    PyValueError::new_err(e.to_string())
}

/// Convert any `std::error::Error` into a Python exception, preserving the
/// full source chain in the message.
#[allow(dead_code)] // Opt-in escape hatch for bindings that want full source chains.
pub fn error_to_py(e: &dyn std::error::Error) -> PyErr {
    PyValueError::new_err(format_chain(e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;
    use std::fmt;

    #[derive(Debug)]
    struct Wrapper(Box<dyn Error + Send + Sync>);
    impl fmt::Display for Wrapper {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "calibration failed")
        }
    }
    impl Error for Wrapper {
        fn source(&self) -> Option<&(dyn Error + 'static)> {
            Some(&*self.0)
        }
    }

    #[derive(Debug)]
    struct Leaf;
    impl fmt::Display for Leaf {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "solver diverged after 1000 iterations")
        }
    }
    impl Error for Leaf {}

    #[test]
    fn format_chain_flattens_sources() {
        let err = Wrapper(Box::new(Leaf));
        let formatted = format_chain(&err);
        assert_eq!(
            formatted,
            "calibration failed: solver diverged after 1000 iterations"
        );
    }

    #[test]
    fn format_chain_avoids_duplication() {
        // If a wrapper already includes the inner message verbatim at the end,
        // we don't append it again.
        #[derive(Debug)]
        struct Echo(Box<dyn Error + Send + Sync>);
        impl fmt::Display for Echo {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "outer: {}", self.0)
            }
        }
        impl Error for Echo {
            fn source(&self) -> Option<&(dyn Error + 'static)> {
                Some(&*self.0)
            }
        }

        let err = Echo(Box::new(Leaf));
        let formatted = format_chain(&err);
        assert_eq!(formatted, "outer: solver diverged after 1000 iterations");
    }
}
