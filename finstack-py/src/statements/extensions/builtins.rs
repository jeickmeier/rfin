//! Built-in extension implementations.

use finstack_statements::extensions::{CorkscrewExtension, CreditScorecardExtension};
use pyo3::prelude::*;
use pyo3::types::PyType;
use pyo3::Bound;

/// Corkscrew extension for balance sheet roll-forward validation.
///
/// Validates that balance sheet accounts properly roll forward:
/// Ending Balance = Beginning Balance + Additions - Reductions
#[pyclass(
    module = "finstack.statements.extensions",
    name = "CorkscrewExtension",
    unsendable
)]
pub struct PyCorkscrewExtension {
    #[allow(dead_code)]
    pub(crate) inner: CorkscrewExtension,
}

#[pymethods]
impl PyCorkscrewExtension {
    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    /// Create a corkscrew extension with default configuration.
    ///
    /// Returns
    /// -------
    /// CorkscrewExtension
    ///     Extension instance
    fn new(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: CorkscrewExtension::new(),
        }
    }

    fn __repr__(&self) -> String {
        "CorkscrewExtension()".to_string()
    }
}

/// Credit scorecard extension for rating assignment.
///
/// Assigns credit ratings based on financial ratios and thresholds.
#[pyclass(
    module = "finstack.statements.extensions",
    name = "CreditScorecardExtension",
    unsendable
)]
pub struct PyCreditScorecardExtension {
    #[allow(dead_code)]
    pub(crate) inner: CreditScorecardExtension,
}

#[pymethods]
impl PyCreditScorecardExtension {
    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    /// Create a credit scorecard extension with default configuration.
    ///
    /// Returns
    /// -------
    /// CreditScorecardExtension
    ///     Extension instance
    fn new(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: CreditScorecardExtension::new(),
        }
    }

    fn __repr__(&self) -> String {
        "CreditScorecardExtension()".to_string()
    }
}
