//! Python bindings for portfolio books.
//!
//! Books provide an optional hierarchical organization structure for portfolios.

use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyList, PyModule};
use pyo3::Bound;
use pythonize::pythonize;

use finstack_portfolio::{Book, BookId};

/// Book identifier.
#[pyclass(module = "finstack.portfolio", name = "BookId", from_py_object)]
#[derive(Clone)]
pub struct PyBookId {
    pub(crate) inner: BookId,
}

impl PyBookId {
    pub(crate) fn new(inner: BookId) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyBookId {
    #[new]
    #[pyo3(text_signature = "(id)")]
    /// Create a new book identifier.
    fn new_py(id: String) -> Self {
        Self::new(BookId::new(id))
    }

    #[getter]
    /// Get the identifier as a string.
    fn id(&self) -> String {
        self.inner.to_string()
    }

    fn __repr__(&self) -> String {
        format!("BookId('{}')", self.inner)
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }
}

/// A book in the portfolio hierarchy.
#[pyclass(module = "finstack.portfolio", name = "Book", from_py_object)]
#[derive(Clone)]
pub struct PyBook {
    pub(crate) inner: Book,
}

impl PyBook {
    pub(crate) fn new(inner: Book) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyBook {
    #[new]
    #[pyo3(signature = (id, name=None, parent_id=None))]
    #[pyo3(text_signature = "(id, name=None, parent_id=None)")]
    /// Create a new book.
    ///
    /// Args:
    ///     id: Unique book identifier (string or BookId).
    ///     name: Optional human-readable name.
    ///     parent_id: Optional parent book identifier (string or BookId).
    fn new_py(
        id: &Bound<'_, PyAny>,
        name: Option<String>,
        parent_id: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        let book_id = extract_book_id(id)?;
        let book = match parent_id {
            Some(parent) => Book::with_parent(book_id, name, extract_book_id(parent)?),
            None => Book::new(book_id, name),
        };
        Ok(Self::new(book))
    }

    #[getter]
    /// Get the book ID.
    fn id(&self) -> String {
        self.inner.id.to_string()
    }

    #[getter]
    /// Get the book name.
    fn name(&self) -> Option<String> {
        self.inner.name.clone()
    }

    #[getter]
    /// Get the parent book ID (None for root books).
    fn parent_id(&self) -> Option<String> {
        self.inner.parent_id.as_ref().map(|id| id.to_string())
    }

    #[getter]
    /// Get position IDs directly assigned to this book (non-recursive).
    fn position_ids(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let ids: Vec<String> = self
            .inner
            .position_ids
            .iter()
            .map(|id| id.to_string())
            .collect();
        Ok(PyList::new(py, ids)?.into())
    }

    #[getter]
    /// Get child book IDs (non-recursive).
    fn child_book_ids(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let ids: Vec<String> = self
            .inner
            .child_book_ids
            .iter()
            .map(|id| id.to_string())
            .collect();
        Ok(PyList::new(py, ids)?.into())
    }

    #[getter]
    /// Get book tags.
    fn tags(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new(py);
        for (k, v) in &self.inner.tags {
            dict.set_item(k, v)?;
        }
        Ok(dict.into())
    }

    #[getter]
    /// Get book metadata.
    fn meta(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let bound = pythonize(py, &self.inner.meta)
            .map_err(|e| PyValueError::new_err(format!("Failed to convert meta: {}", e)))?;
        Ok(bound.unbind())
    }

    fn __repr__(&self) -> String {
        if let Some(name) = &self.inner.name {
            format!("Book(id='{}', name='{}')", self.inner.id, name)
        } else {
            format!("Book(id='{}')", self.inner.id)
        }
    }

    fn __str__(&self) -> String {
        self.inner
            .name
            .clone()
            .unwrap_or_else(|| self.inner.id.to_string())
    }
}

pub(crate) fn extract_book_id(value: &Bound<'_, PyAny>) -> PyResult<BookId> {
    if let Ok(py_id) = value.extract::<PyRef<PyBookId>>() {
        Ok(py_id.inner.clone())
    } else if let Ok(s) = value.extract::<String>() {
        Ok(BookId::new(s))
    } else {
        Err(PyTypeError::new_err("Expected BookId or string"))
    }
}

pub(crate) fn extract_book(value: &Bound<'_, PyAny>) -> PyResult<Book> {
    if let Ok(py_book) = value.extract::<PyRef<PyBook>>() {
        Ok(py_book.inner.clone())
    } else {
        Err(PyTypeError::new_err("Expected Book"))
    }
}

/// Register book module exports.
pub(crate) fn register<'py>(
    _py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<String>> {
    parent.add_class::<PyBookId>()?;
    parent.add_class::<PyBook>()?;
    Ok(vec!["BookId".to_string(), "Book".to_string()])
}
