use finstack_valuations::instruments::common::models::credit::endogenous_hazard::{
    EndogenousHazardSpec as RustEndogenousHazardSpec, LeverageHazardMap,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyModule, PyType};

// ---------------------------------------------------------------------------
// PyEndogenousHazardSpec
// ---------------------------------------------------------------------------

/// Endogenous (leverage-dependent) hazard rate specification.
///
/// Provides a feedback loop where PIK accrual increases leverage, which in turn
/// increases the hazard rate and expected loss.
///
/// Examples
/// --------
///     >>> spec = EndogenousHazardSpec.power_law(0.10, 1.5, 2.5)
///     >>> spec.hazard_at_leverage(1.5)
///     0.1
///     >>> spec.hazard_at_leverage(2.0)  # higher leverage -> higher hazard
///     0.246...
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "EndogenousHazardSpec",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyEndogenousHazardSpec {
    pub(crate) inner: RustEndogenousHazardSpec,
}

impl PyEndogenousHazardSpec {
    fn label(&self) -> &'static str {
        match self.inner.leverage_hazard_map() {
            LeverageHazardMap::PowerLaw { .. } => "PowerLaw",
            LeverageHazardMap::Exponential { .. } => "Exponential",
            LeverageHazardMap::Tabular { .. } => "Tabular",
        }
    }
}

#[pymethods]
impl PyEndogenousHazardSpec {
    /// Create a power-law endogenous hazard spec.
    ///
    /// ``lambda(L) = base_hazard * (L / base_leverage)^exponent``
    ///
    /// Parameters
    /// ----------
    /// base_hazard : float
    ///     Base (reference) hazard rate at the base leverage.
    /// base_leverage : float
    ///     Base (reference) leverage level.
    /// exponent : float
    ///     Power-law exponent controlling sensitivity to leverage changes.
    ///
    /// Returns
    /// -------
    /// EndogenousHazardSpec
    #[classmethod]
    #[pyo3(text_signature = "(cls, base_hazard, base_leverage, exponent)")]
    fn power_law(
        _cls: &Bound<'_, PyType>,
        base_hazard: f64,
        base_leverage: f64,
        exponent: f64,
    ) -> Self {
        Self {
            inner: RustEndogenousHazardSpec::power_law(base_hazard, base_leverage, exponent),
        }
    }

    /// Create an exponential endogenous hazard spec.
    ///
    /// ``lambda(L) = base_hazard * exp(sensitivity * (L - base_leverage))``
    ///
    /// Parameters
    /// ----------
    /// base_hazard : float
    ///     Base (reference) hazard rate at the base leverage.
    /// base_leverage : float
    ///     Base (reference) leverage level.
    /// sensitivity : float
    ///     Exponential sensitivity to leverage changes.
    ///
    /// Returns
    /// -------
    /// EndogenousHazardSpec
    #[classmethod]
    #[pyo3(text_signature = "(cls, base_hazard, base_leverage, sensitivity)")]
    fn exponential(
        _cls: &Bound<'_, PyType>,
        base_hazard: f64,
        base_leverage: f64,
        sensitivity: f64,
    ) -> Self {
        Self {
            inner: RustEndogenousHazardSpec::exponential(base_hazard, base_leverage, sensitivity),
        }
    }

    /// Create a tabular endogenous hazard spec from empirical calibration.
    ///
    /// Uses linear interpolation between the given points and flat
    /// extrapolation beyond the edges.
    ///
    /// Parameters
    /// ----------
    /// leverage_points : list[float]
    ///     Leverage breakpoints (must be sorted ascending).
    /// hazard_points : list[float]
    ///     Corresponding hazard rates at each breakpoint.
    ///
    /// Returns
    /// -------
    /// EndogenousHazardSpec
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If ``leverage_points`` and ``hazard_points`` have different lengths
    ///     or are empty.
    #[classmethod]
    #[pyo3(text_signature = "(cls, leverage_points, hazard_points)")]
    fn tabular(
        _cls: &Bound<'_, PyType>,
        leverage_points: Vec<f64>,
        hazard_points: Vec<f64>,
    ) -> PyResult<Self> {
        if leverage_points.is_empty() || hazard_points.is_empty() {
            return Err(PyValueError::new_err(
                "leverage_points and hazard_points must be non-empty",
            ));
        }
        if leverage_points.len() != hazard_points.len() {
            return Err(PyValueError::new_err(format!(
                "leverage_points (len={}) and hazard_points (len={}) must have the same length",
                leverage_points.len(),
                hazard_points.len()
            )));
        }
        Ok(Self {
            inner: RustEndogenousHazardSpec::tabular(leverage_points, hazard_points),
        })
    }

    /// Compute the hazard rate at a given leverage level.
    ///
    /// Parameters
    /// ----------
    /// leverage : float
    ///     Current leverage ratio.
    ///
    /// Returns
    /// -------
    /// float
    ///     Hazard rate (floored at 0.0).
    fn hazard_at_leverage(&self, leverage: f64) -> f64 {
        self.inner.hazard_at_leverage(leverage)
    }

    /// Compute the hazard rate after PIK accrual changes the notional.
    ///
    /// Leverage is computed as ``accreted_notional / asset_value``.
    ///
    /// Parameters
    /// ----------
    /// original_notional : float
    ///     Original face notional.
    /// accreted_notional : float
    ///     Current (PIK-augmented) notional.
    /// asset_value : float
    ///     Current asset value.
    ///
    /// Returns
    /// -------
    /// float
    ///     Hazard rate at the implied leverage.
    fn hazard_after_pik_accrual(
        &self,
        original_notional: f64,
        accreted_notional: f64,
        asset_value: f64,
    ) -> f64 {
        self.inner
            .hazard_after_pik_accrual(original_notional, accreted_notional, asset_value)
    }

    /// Base (reference) hazard rate.
    #[getter]
    fn base_hazard_rate(&self) -> f64 {
        self.inner.base_hazard_rate()
    }

    /// Base (reference) leverage level.
    #[getter]
    fn base_leverage(&self) -> f64 {
        self.inner.base_leverage()
    }

    /// Canonical name of the hazard map variant.
    #[getter]
    fn name(&self) -> &'static str {
        self.label()
    }

    fn __repr__(&self) -> String {
        format!("EndogenousHazardSpec('{}')", self.label())
    }

    fn __str__(&self) -> &'static str {
        self.label()
    }
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyEndogenousHazardSpec>()?;
    Ok(vec!["EndogenousHazardSpec"])
}
