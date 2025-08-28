//! Python bindings for loan instruments.

use pyo3::prelude::*;

/// Loan instrument for private credit valuation.
///
/// A Loan represents a term loan or credit facility with support for
/// various amortization schedules, PIK/cash toggle features, and fees.
///
/// This is a placeholder implementation - full functionality to be added
/// as part of the Python bindings expansion plan.
#[pyclass(name = "Loan", module = "finstack.instruments")]
#[derive(Clone)]
pub struct PyLoan {
    id: String,
}

#[pymethods]
impl PyLoan {
    #[new]
    fn new(id: String) -> Self {
        Self { id }
    }
    
    #[getter]
    fn id(&self) -> String {
        self.id.clone()
    }
    
    fn __repr__(&self) -> String {
        format!("Loan('{}')", self.id)
    }
}
