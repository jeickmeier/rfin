//! Python bindings for CurveSet container.

use pyo3::prelude::*;
use pyo3::types::PyList;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use super::curves::{PyDiscountCurve, PyForwardCurve, PyHazardCurve, PyInflationCurve};
use super::surfaces::PyVolSurface;

/// Type-safe curve storage
#[derive(Clone)]
enum CurveType {
    Discount(PyDiscountCurve),
    Forward(PyForwardCurve),
    Hazard(PyHazardCurve),
    Inflation(PyInflationCurve),
    VolSurface(PyVolSurface),
}

/// Container for multiple market data curves and surfaces.
///
/// CurveSet provides a dictionary-like interface for managing multiple curves
/// of different types. It supports type-safe access and collateral mapping.
///
/// The container acts like a Python dictionary with additional type checking
/// and convenience methods for accessing specific curve types.
///
/// Examples:
///     >>> from rfin.market_data import CurveSet, DiscountCurve, ForwardCurve
///     >>> from rfin import Date
///     
///     # Create a curve set
///     >>> curves = CurveSet()
///     
///     # Add curves using dictionary syntax
///     >>> usd_ois = DiscountCurve(
///     ...     id="USD-OIS",
///     ...     base_date=Date(2025, 1, 1),
///     ...     times=[0.0, 1.0, 5.0],
///     ...     discount_factors=[1.0, 0.98, 0.88]
///     ... )
///     >>> curves["USD-OIS"] = usd_ois
///     
///     # Check if curve exists
///     >>> "USD-OIS" in curves
///     True
///     
///     # Access curves
///     >>> curve = curves["USD-OIS"]
///     >>> curve.df(2.0)
///     0.95
///     
///     # Type-safe access
///     >>> discount = curves.discount_curve("USD-OIS")
///     >>> forward = curves.forward_curve("USD-SOFR3M")  # Raises TypeError if wrong type
///     
///     # Iterate over curves
///     >>> for id in curves.keys():
///     ...     print(id)
///     USD-OIS
///     
///     # Collateral mapping
///     >>> curves.map_collateral("CSA-USD", "USD-OIS")
///     >>> collateral_curve = curves.collateral_curve("CSA-USD")
#[pyclass(name = "CurveSet", module = "finstack.market_data")]
#[derive(Clone)]
pub struct PyCurveSet {
    curves: Arc<RwLock<HashMap<String, CurveType>>>,
    collateral: Arc<RwLock<HashMap<String, String>>>,
}

#[pymethods]
impl PyCurveSet {
    /// Create an empty CurveSet.
    #[new]
    fn new() -> Self {
        PyCurveSet {
            curves: Arc::new(RwLock::new(HashMap::new())),
            collateral: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a curve with automatic type detection.
    ///
    /// Args:
    ///     key (str): The curve identifier
    ///     curve: A curve object (DiscountCurve, ForwardCurve, etc.)
    ///
    /// Raises:
    ///     TypeError: If the curve type is not recognized
    fn __setitem__(&self, key: String, curve: &Bound<'_, PyAny>) -> PyResult<()> {
        let curve_type = if let Ok(dc) = curve.extract::<PyDiscountCurve>() {
            CurveType::Discount(dc)
        } else if let Ok(fc) = curve.extract::<PyForwardCurve>() {
            CurveType::Forward(fc)
        } else if let Ok(hc) = curve.extract::<PyHazardCurve>() {
            CurveType::Hazard(hc)
        } else if let Ok(ic) = curve.extract::<PyInflationCurve>() {
            CurveType::Inflation(ic)
        } else if let Ok(vs) = curve.extract::<PyVolSurface>() {
            CurveType::VolSurface(vs)
        } else {
            return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Expected a curve type (DiscountCurve, ForwardCurve, HazardCurve, InflationCurve, or VolSurface)"
            ));
        };

        let mut curves = self.curves.write().unwrap();
        curves.insert(key, curve_type);
        Ok(())
    }

    /// Retrieve a curve by ID.
    ///
    /// Args:
    ///     key (str): The curve identifier
    ///
    /// Returns:
    ///     The curve object
    ///
    /// Raises:
    ///     KeyError: If the curve is not found
    fn __getitem__(&self, key: &str) -> PyResult<PyObject> {
        Python::with_gil(|py| {
            let curves = self.curves.read().unwrap();
            match curves.get(key) {
                Some(curve) => match curve {
                    CurveType::Discount(c) => Ok(Py::new(py, c.clone())?.into_any()),
                    CurveType::Forward(c) => Ok(Py::new(py, c.clone())?.into_any()),
                    CurveType::Hazard(c) => Ok(Py::new(py, c.clone())?.into_any()),
                    CurveType::Inflation(c) => Ok(Py::new(py, c.clone())?.into_any()),
                    CurveType::VolSurface(c) => Ok(Py::new(py, c.clone())?.into_any()),
                },
                None => Err(PyErr::new::<pyo3::exceptions::PyKeyError, _>(format!(
                    "Curve '{}' not found",
                    key
                ))),
            }
        })
    }

    /// Check if a curve exists.
    ///
    /// Args:
    ///     key (str): The curve identifier
    ///
    /// Returns:
    ///     bool: True if the curve exists
    fn __contains__(&self, key: &str) -> bool {
        let curves = self.curves.read().unwrap();
        curves.contains_key(key)
    }

    /// Delete a curve.
    ///
    /// Args:
    ///     key (str): The curve identifier
    ///
    /// Raises:
    ///     KeyError: If the curve is not found
    fn __delitem__(&self, key: &str) -> PyResult<()> {
        let mut curves = self.curves.write().unwrap();
        if curves.remove(key).is_none() {
            return Err(PyErr::new::<pyo3::exceptions::PyKeyError, _>(format!(
                "Curve '{}' not found",
                key
            )));
        }
        Ok(())
    }

    /// Number of curves in the set.
    fn __len__(&self) -> usize {
        let curves = self.curves.read().unwrap();
        curves.len()
    }

    /// Safe retrieval with default.
    ///
    /// Args:
    ///     key (str): The curve identifier
    ///     default: Value to return if curve not found (default: None)
    ///
    /// Returns:
    ///     The curve object or the default value
    fn get(&self, py: Python<'_>, key: &str, default: Option<PyObject>) -> PyObject {
        match self.__getitem__(key) {
            Ok(curve) => curve,
            Err(_) => default.unwrap_or_else(|| py.None()),
        }
    }

    /// Get a discount curve, with type checking.
    ///
    /// Args:
    ///     id (str): The curve identifier
    ///
    /// Returns:
    ///     DiscountCurve: The discount curve
    ///
    /// Raises:
    ///     KeyError: If the curve is not found
    ///     TypeError: If the curve is not a DiscountCurve
    fn discount_curve(&self, id: &str) -> PyResult<PyDiscountCurve> {
        let curves = self.curves.read().unwrap();
        match curves.get(id) {
            Some(CurveType::Discount(c)) => Ok(c.clone()),
            Some(_) => Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
                "Curve '{}' is not a DiscountCurve",
                id
            ))),
            None => Err(PyErr::new::<pyo3::exceptions::PyKeyError, _>(format!(
                "Curve '{}' not found",
                id
            ))),
        }
    }

    /// Get a forward curve, with type checking.
    fn forward_curve(&self, id: &str) -> PyResult<PyForwardCurve> {
        let curves = self.curves.read().unwrap();
        match curves.get(id) {
            Some(CurveType::Forward(c)) => Ok(c.clone()),
            Some(_) => Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
                "Curve '{}' is not a ForwardCurve",
                id
            ))),
            None => Err(PyErr::new::<pyo3::exceptions::PyKeyError, _>(format!(
                "Curve '{}' not found",
                id
            ))),
        }
    }

    /// Get a hazard curve, with type checking.
    fn hazard_curve(&self, id: &str) -> PyResult<PyHazardCurve> {
        let curves = self.curves.read().unwrap();
        match curves.get(id) {
            Some(CurveType::Hazard(c)) => Ok(c.clone()),
            Some(_) => Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
                "Curve '{}' is not a HazardCurve",
                id
            ))),
            None => Err(PyErr::new::<pyo3::exceptions::PyKeyError, _>(format!(
                "Curve '{}' not found",
                id
            ))),
        }
    }

    /// Get an inflation curve, with type checking.
    fn inflation_curve(&self, id: &str) -> PyResult<PyInflationCurve> {
        let curves = self.curves.read().unwrap();
        match curves.get(id) {
            Some(CurveType::Inflation(c)) => Ok(c.clone()),
            Some(_) => Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
                "Curve '{}' is not an InflationCurve",
                id
            ))),
            None => Err(PyErr::new::<pyo3::exceptions::PyKeyError, _>(format!(
                "Curve '{}' not found",
                id
            ))),
        }
    }

    /// Get a volatility surface, with type checking.
    fn vol_surface(&self, id: &str) -> PyResult<PyVolSurface> {
        let curves = self.curves.read().unwrap();
        match curves.get(id) {
            Some(CurveType::VolSurface(c)) => Ok(c.clone()),
            Some(_) => Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
                "Curve '{}' is not a VolSurface",
                id
            ))),
            None => Err(PyErr::new::<pyo3::exceptions::PyKeyError, _>(format!(
                "Curve '{}' not found",
                id
            ))),
        }
    }

    /// Get all curve IDs.
    ///
    /// Returns:
    ///     List[str]: List of all curve identifiers
    fn keys(&self) -> Vec<String> {
        let curves = self.curves.read().unwrap();
        curves.keys().cloned().collect()
    }

    /// Get all curves as a list
    pub fn values(&self, py: Python) -> PyResult<PyObject> {
        let curves = self.curves.read().unwrap();
        let values: Vec<PyObject> = curves
            .values()
            .map(|curve| match curve {
                CurveType::Discount(c) => Py::new(py, c.clone()).unwrap().into_any(),
                CurveType::Forward(c) => Py::new(py, c.clone()).unwrap().into_any(),
                CurveType::Hazard(c) => Py::new(py, c.clone()).unwrap().into_any(),
                CurveType::Inflation(c) => Py::new(py, c.clone()).unwrap().into_any(),
                CurveType::VolSurface(c) => Py::new(py, c.clone()).unwrap().into_any(),
            })
            .collect();
        Ok(PyList::new(py, values)?.into_any().unbind())
    }

    /// Get all (key, curve) pairs as a list of tuples
    pub fn items(&self, py: Python) -> PyResult<PyObject> {
        let curves = self.curves.read().unwrap();
        let items: Vec<(String, PyObject)> = curves
            .iter()
            .map(|(key, curve)| {
                let obj = match curve {
                    CurveType::Discount(c) => Py::new(py, c.clone()).unwrap().into_any(),
                    CurveType::Forward(c) => Py::new(py, c.clone()).unwrap().into_any(),
                    CurveType::Hazard(c) => Py::new(py, c.clone()).unwrap().into_any(),
                    CurveType::Inflation(c) => Py::new(py, c.clone()).unwrap().into_any(),
                    CurveType::VolSurface(c) => Py::new(py, c.clone()).unwrap().into_any(),
                };
                (key.clone(), obj)
            })
            .collect();
        Ok(PyList::new(py, items)?.into_any().unbind())
    }

    /// Map CSA code to discount curve.
    ///
    /// Args:
    ///     csa_code (str): The collateral agreement code
    ///     discount_id (str): The discount curve identifier
    fn map_collateral(&self, csa_code: String, discount_id: String) {
        let mut collateral = self.collateral.write().unwrap();
        collateral.insert(csa_code, discount_id);
    }

    /// Get discount curve for collateral.
    ///
    /// Args:
    ///     csa_code (str): The collateral agreement code
    ///
    /// Returns:
    ///     DiscountCurve: The associated discount curve
    ///
    /// Raises:
    ///     KeyError: If the CSA code is not mapped or curve not found
    ///     TypeError: If the mapped curve is not a DiscountCurve
    fn collateral_curve(&self, csa_code: &str) -> PyResult<PyDiscountCurve> {
        let collateral = self.collateral.read().unwrap();
        match collateral.get(csa_code) {
            Some(discount_id) => self.discount_curve(discount_id),
            None => Err(PyErr::new::<pyo3::exceptions::PyKeyError, _>(format!(
                "CSA code '{}' not mapped",
                csa_code
            ))),
        }
    }

    /// Clear all curves and mappings.
    fn clear(&self) {
        let mut curves = self.curves.write().unwrap();
        let mut collateral = self.collateral.write().unwrap();
        curves.clear();
        collateral.clear();
    }

    fn __repr__(&self) -> String {
        let curves = self.curves.read().unwrap();
        format!("CurveSet({} curves)", curves.len())
    }
}
