//! Python bindings for CurveId.

use pyo3::prelude::*;
use finstack_core::market_data::id::CurveId;

/// Identifier for a market data curve or surface.
///
/// CurveId is a lightweight, immutable identifier used to uniquely identify
/// curves and surfaces in the market data system. It wraps a string identifier
/// and provides efficient comparison and hashing.
///
/// Examples:
///     >>> from rfin.market_data import CurveId
///     
///     # Create a curve ID
///     >>> usd_ois = CurveId("USD-OIS")
///     >>> usd_ois.value
///     'USD-OIS'
///     
///     # Use in comparisons
///     >>> usd_ois == CurveId("USD-OIS")
///     True
///     >>> usd_ois == CurveId("EUR-OIS")
///     False
///     
///     # String representation
///     >>> str(usd_ois)
///     'USD-OIS'
#[pyclass(name = "CurveId", module = "finstack.market_data")]
#[derive(Clone)]
pub struct PyCurveId {
    pub(crate) inner: CurveId,
}

#[pymethods]
impl PyCurveId {
    /// Create a new CurveId.
    ///
    /// Args:
    ///     id (str): The curve identifier string
    ///
    /// Returns:
    ///     CurveId: A new curve identifier
    ///
    /// Raises:
    ///     ValueError: If the id is empty
    #[new]
    fn new(id: String) -> PyResult<Self> {
        if id.is_empty() {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "CurveId cannot be empty",
            ));
        }
        // Leak the string to get 'static lifetime
        let id_static: &'static str = Box::leak(id.into_boxed_str());
        Ok(PyCurveId {
            inner: CurveId::new(id_static),
        })
    }

    /// The string value of the curve ID.
    #[getter]
    fn value(&self) -> &'static str {
        self.inner.as_str()
    }

    fn __str__(&self) -> &'static str {
        self.inner.as_str()
    }

    fn __repr__(&self) -> String {
        format!("CurveId('{}')", self.inner.as_str())
    }

    fn __eq__(&self, other: &PyCurveId) -> bool {
        self.inner == other.inner
    }

    fn __hash__(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.inner.hash(&mut hasher);
        hasher.finish()
    }
}

impl PyCurveId {
    /// Create from core CurveId
    pub fn from_core(inner: CurveId) -> Self {
        Self { inner }
    }

    /// Get the inner CurveId
    pub fn to_core(&self) -> CurveId {
        self.inner
    }
}
