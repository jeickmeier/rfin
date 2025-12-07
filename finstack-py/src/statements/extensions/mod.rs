//! Extension system for statements crate.

mod builtins;

use crate::statements::error::stmt_to_py;
use crate::statements::evaluator::PyResults;
use crate::statements::types::model::PyFinancialModelSpec;
use finstack_statements::extensions::{
    ExtensionContext, ExtensionMetadata, ExtensionRegistry, ExtensionResult, ExtensionStatus,
};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule, PyType};
use pyo3::Bound;

pub use builtins::{PyCorkscrewExtension, PyCreditScorecardExtension};

/// Extension metadata.
#[pyclass(
    module = "finstack.statements.extensions",
    name = "ExtensionMetadata",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyExtensionMetadata {
    pub(crate) inner: ExtensionMetadata,
}

impl PyExtensionMetadata {
    pub(crate) fn new(inner: ExtensionMetadata) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyExtensionMetadata {
    #[new]
    #[pyo3(text_signature = "(name, version, description=None, author=None)")]
    /// Create extension metadata.
    ///
    /// Parameters
    /// ----------
    /// name : str
    ///     Unique extension name
    /// version : str
    ///     Semantic version
    /// description : str, optional
    ///     Human-readable description
    /// author : str, optional
    ///     Extension author
    ///
    /// Returns
    /// -------
    /// ExtensionMetadata
    ///     Metadata instance
    fn new_py(
        name: String,
        version: String,
        description: Option<String>,
        author: Option<String>,
    ) -> Self {
        Self::new(ExtensionMetadata {
            name,
            version,
            description,
            author,
        })
    }

    #[getter]
    fn name(&self) -> String {
        self.inner.name.clone()
    }

    #[getter]
    fn version(&self) -> String {
        self.inner.version.clone()
    }

    #[getter]
    fn description(&self) -> Option<String> {
        self.inner.description.clone()
    }

    #[getter]
    fn author(&self) -> Option<String> {
        self.inner.author.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "ExtensionMetadata(name='{}', version='{}')",
            self.inner.name, self.inner.version
        )
    }
}

/// Extension execution status.
#[pyclass(
    module = "finstack.statements.extensions",
    name = "ExtensionStatus",
    frozen
)]
#[derive(Clone, Copy, Debug)]
pub struct PyExtensionStatus {
    pub(crate) inner: ExtensionStatus,
}

impl PyExtensionStatus {
    pub(crate) fn new(inner: ExtensionStatus) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyExtensionStatus {
    #[classattr]
    const SUCCESS: Self = Self {
        inner: ExtensionStatus::Success,
    };
    #[classattr]
    const FAILED: Self = Self {
        inner: ExtensionStatus::Failed,
    };
    #[classattr]
    const NOT_IMPLEMENTED: Self = Self {
        inner: ExtensionStatus::NotImplemented,
    };
    #[classattr]
    const SKIPPED: Self = Self {
        inner: ExtensionStatus::Skipped,
    };

    fn __repr__(&self) -> String {
        format!("ExtensionStatus.{:?}", self.inner)
    }
}

/// Extension execution result.
#[pyclass(module = "finstack.statements.extensions", name = "ExtensionResult")]
#[derive(Clone, Debug)]
pub struct PyExtensionResult {
    pub(crate) inner: ExtensionResult,
}

impl PyExtensionResult {
    pub(crate) fn new(inner: ExtensionResult) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyExtensionResult {
    #[staticmethod]
    #[pyo3(text_signature = "(message)")]
    /// Create a success result.
    ///
    /// Parameters
    /// ----------
    /// message : str
    ///     Success message
    ///
    /// Returns
    /// -------
    /// ExtensionResult
    ///     Success result
    fn success(message: String) -> Self {
        Self::new(ExtensionResult::success(message))
    }

    #[staticmethod]
    #[pyo3(text_signature = "(message)")]
    /// Create a failure result.
    ///
    /// Parameters
    /// ----------
    /// message : str
    ///     Failure message
    ///
    /// Returns
    /// -------
    /// ExtensionResult
    ///     Failure result
    fn failure(message: String) -> Self {
        Self::new(ExtensionResult::failure(message))
    }

    #[staticmethod]
    #[pyo3(text_signature = "(message)")]
    /// Create a skipped result.
    ///
    /// Parameters
    /// ----------
    /// message : str
    ///     Skip reason
    ///
    /// Returns
    /// -------
    /// ExtensionResult
    ///     Skipped result
    fn skipped(message: String) -> Self {
        Self::new(ExtensionResult::skipped(message))
    }

    #[getter]
    fn status(&self) -> PyExtensionStatus {
        PyExtensionStatus::new(self.inner.status)
    }

    #[getter]
    fn message(&self) -> String {
        self.inner.message.clone()
    }

    #[getter]
    fn data(&self, py: Python<'_>) -> Py<PyAny> {
        let dict = PyDict::new(py);
        for (key, value) in &self.inner.data {
            dict.set_item(key, crate::statements::utils::json_to_py(py, value))
                .ok();
        }
        dict.into()
    }

    fn __repr__(&self) -> String {
        format!(
            "ExtensionResult(status={:?}, message='{}')",
            self.inner.status, self.inner.message
        )
    }
}

/// Extension context.
///
/// Context passed to extensions during execution.
#[pyclass(
    module = "finstack.statements.extensions",
    name = "ExtensionContext",
    frozen
)]
pub struct PyExtensionContext {
    model: PyFinancialModelSpec,
    results: PyResults,
    config: Py<PyAny>,
}

#[pymethods]
impl PyExtensionContext {
    #[getter]
    fn model(&self) -> PyFinancialModelSpec {
        self.model.clone()
    }

    #[getter]
    fn results(&self) -> PyResults {
        self.results.clone()
    }

    #[getter]
    fn config(&self, py: Python<'_>) -> Py<PyAny> {
        self.config.clone_ref(py)
    }
}

/// Extension registry.
///
/// Manages and executes extensions for financial models.
#[pyclass(
    module = "finstack.statements.extensions",
    name = "ExtensionRegistry",
    unsendable
)]
pub struct PyExtensionRegistry {
    inner: ExtensionRegistry,
}

#[pymethods]
impl PyExtensionRegistry {
    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    /// Create a new extension registry.
    ///
    /// Returns
    /// -------
    /// ExtensionRegistry
    ///     Registry instance
    fn new(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: ExtensionRegistry::new(),
        }
    }

    // Note: register() is not exposed for now as it requires cloning extensions
    // which don't implement Clone. This will be added when the API is updated.

    #[pyo3(text_signature = "(self, model, results)")]
    /// Execute all registered extensions.
    ///
    /// Parameters
    /// ----------
    /// model : FinancialModelSpec
    ///     Financial model
    /// results : Results
    ///     Evaluation results
    ///
    /// Returns
    /// -------
    /// dict[str, ExtensionResult]
    ///     Map of extension name to result
    fn execute_all(
        &mut self,
        model: &PyFinancialModelSpec,
        results: &PyResults,
        py: Python<'_>,
    ) -> PyResult<Py<PyAny>> {
        let context = ExtensionContext::new(&model.inner, &results.inner);
        let extension_results = self.inner.execute_all(&context).map_err(stmt_to_py)?;

        let dict = PyDict::new(py);
        for (name, result) in extension_results {
            dict.set_item(name, PyExtensionResult::new(result))?;
        }
        Ok(dict.into())
    }

    // Note: list_extensions() not available in current Rust API
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(_py, "extensions")?;
    module.setattr("__doc__", "Extension system for statement models.")?;

    module.add_class::<PyExtensionMetadata>()?;
    module.add_class::<PyExtensionStatus>()?;
    module.add_class::<PyExtensionResult>()?;
    module.add_class::<PyExtensionContext>()?;
    module.add_class::<PyExtensionRegistry>()?;
    module.add_class::<PyCorkscrewExtension>()?;
    module.add_class::<PyCreditScorecardExtension>()?;

    parent.add_submodule(&module)?;
    parent.setattr("extensions", &module)?;

    Ok(vec![
        "ExtensionMetadata",
        "ExtensionStatus",
        "ExtensionResult",
        "ExtensionContext",
        "ExtensionRegistry",
        "CorkscrewExtension",
        "CreditScorecardExtension",
    ])
}
