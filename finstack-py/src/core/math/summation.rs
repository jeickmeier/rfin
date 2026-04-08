use finstack_core::math::summation::{
    kahan_sum as core_kahan_sum, neumaier_sum as core_neumaier_sum,
};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;

#[pyfunction(name = "kahan_sum")]
#[pyo3(text_signature = "(values)")]
/// Kahan compensated summation.
///
/// Best for sequences where all values have the same sign.
/// For mixed-sign values, prefer `neumaier_sum`.
///
/// Parameters
/// ----------
/// values : list[float]
///     Sequence of values to sum.
///
/// Returns
/// -------
/// float
///     Compensated sum with reduced floating-point error.
pub fn kahan_sum_py(values: Vec<f64>) -> PyResult<f64> {
    Ok(core_kahan_sum(values.iter().copied()))
}

#[pyfunction(name = "neumaier_sum")]
#[pyo3(text_signature = "(values)")]
/// Neumaier compensated summation - handles mixed-sign values better than Kahan.
///
/// This is the recommended summation algorithm for most use cases,
/// especially for financial calculations with mixed-sign cashflows.
///
/// Parameters
/// ----------
/// values : list[float]
///     Sequence of values to sum.
///
/// Returns
/// -------
/// float
///     Numerically stable sum.
///
/// Examples
/// --------
/// >>> from finstack.core.math.summation import neumaier_sum
/// >>> neumaier_sum([1e16, 1.0, -1e16])
/// 1.0
pub fn neumaier_sum_py(values: Vec<f64>) -> PyResult<f64> {
    Ok(core_neumaier_sum(values.iter().copied()))
}

#[pyclass(name = "NeumaierAccumulator", module = "finstack.core.math.summation")]
/// Incremental Neumaier compensated summation accumulator.
///
/// Useful when you want stable accumulation without collecting all values
/// into a list first. Handles both same-sign and mixed-sign values correctly.
///
/// Examples
/// --------
/// >>> from finstack.core.math.summation import NeumaierAccumulator
/// >>> acc = NeumaierAccumulator()
/// >>> for v in [1e16, 1.0, -1e16]:
/// ...     acc.add(v)
/// >>> acc.total()
/// 1.0
pub struct PyNeumaierAccumulator {
    inner: finstack_core::math::summation::NeumaierAccumulator,
}

#[pymethods]
impl PyNeumaierAccumulator {
    #[new]
    /// Create a new accumulator with zero state.
    fn new() -> Self {
        Self {
            inner: finstack_core::math::summation::NeumaierAccumulator::new(),
        }
    }

    /// Add a value to the running total.
    ///
    /// Args:
    ///     value (float): Value to accumulate (must be finite).
    fn add(&mut self, value: f64) {
        self.inner.add(value);
    }

    /// Return the compensated total (consumes internal state snapshot).
    ///
    /// Returns:
    ///     float: Compensated sum of all added values.
    fn total(&self) -> f64 {
        self.inner.total()
    }

    /// Return the current compensated total without consuming the accumulator.
    ///
    /// Returns:
    ///     float: Current compensated sum.
    fn current(&self) -> f64 {
        self.inner.current()
    }

    fn __repr__(&self) -> String {
        format!("NeumaierAccumulator(total={})", self.inner.current())
    }
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "summation")?;
    module.setattr(
        "__doc__",
        "Numerically stable summation algorithms.\n\n\
         - `kahan_sum`: Compensated summation (best for same-sign values)\n\
         - `neumaier_sum`: Improved compensated summation (recommended for mixed-sign values)",
    )?;
    module.add_function(wrap_pyfunction!(kahan_sum_py, &module)?)?;
    module.add_function(wrap_pyfunction!(neumaier_sum_py, &module)?)?;
    module.add_class::<PyNeumaierAccumulator>()?;

    let exports = ["kahan_sum", "neumaier_sum", "NeumaierAccumulator"];
    module.setattr("__all__", PyList::new(py, exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}
