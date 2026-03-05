use finstack_valuations::instruments::common::models::credit::dynamic_recovery::{
    DynamicRecoverySpec as RustDynamicRecoverySpec, RecoveryModel,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyModule, PyType};

// ---------------------------------------------------------------------------
// PyDynamicRecoverySpec
// ---------------------------------------------------------------------------

/// Dynamic (notional-dependent) recovery rate specification.
///
/// Recovery rates decline as PIK accrual increases the notional relative to
/// the asset base, capturing the intuition that higher leverage dilutes
/// recovery in default.
///
/// Examples
/// --------
///     >>> spec = DynamicRecoverySpec.constant(0.40)
///     >>> spec.recovery_at_notional(150.0)
///     0.4
///     >>> spec = DynamicRecoverySpec.inverse_linear(0.40, 100.0)
///     >>> spec.recovery_at_notional(200.0)
///     0.2
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "DynamicRecoverySpec",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyDynamicRecoverySpec {
    pub(crate) inner: RustDynamicRecoverySpec,
}

impl PyDynamicRecoverySpec {
    fn label(&self) -> &'static str {
        match self.inner.model() {
            RecoveryModel::Constant => "Constant",
            RecoveryModel::InverseLinear => "InverseLinear",
            RecoveryModel::InversePower { .. } => "InversePower",
            RecoveryModel::FlooredInverse { .. } => "FlooredInverse",
            RecoveryModel::LinearDecline { .. } => "LinearDecline",
        }
    }
}

#[pymethods]
impl PyDynamicRecoverySpec {
    /// Create a constant recovery spec (ignores notional changes).
    ///
    /// Parameters
    /// ----------
    /// recovery : float
    ///     Fixed recovery rate.
    ///
    /// Returns
    /// -------
    /// DynamicRecoverySpec
    #[classmethod]
    #[pyo3(text_signature = "(cls, recovery)")]
    fn constant(_cls: &Bound<'_, PyType>, recovery: f64) -> PyResult<Self> {
        let inner = RustDynamicRecoverySpec::constant(recovery)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Create an inverse-linear recovery spec.
    ///
    /// ``R(N) = R_0 * (N_0 / N)``, clamped to ``[0, R_0]``.
    ///
    /// Parameters
    /// ----------
    /// base_recovery : float
    ///     Base recovery rate at the base notional.
    /// base_notional : float
    ///     Reference notional.
    ///
    /// Returns
    /// -------
    /// DynamicRecoverySpec
    #[classmethod]
    #[pyo3(text_signature = "(cls, base_recovery, base_notional)")]
    fn inverse_linear(
        _cls: &Bound<'_, PyType>,
        base_recovery: f64,
        base_notional: f64,
    ) -> PyResult<Self> {
        let inner = RustDynamicRecoverySpec::inverse_linear(base_recovery, base_notional)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Create an inverse-power recovery spec.
    ///
    /// ``R(N) = R_0 * (N_0 / N)^exponent``, clamped to ``[0, R_0]``.
    ///
    /// Parameters
    /// ----------
    /// base_recovery : float
    ///     Base recovery rate at the base notional.
    /// base_notional : float
    ///     Reference notional.
    /// exponent : float
    ///     Power exponent controlling the rate of decline.
    ///
    /// Returns
    /// -------
    /// DynamicRecoverySpec
    #[classmethod]
    #[pyo3(text_signature = "(cls, base_recovery, base_notional, exponent)")]
    fn inverse_power(
        _cls: &Bound<'_, PyType>,
        base_recovery: f64,
        base_notional: f64,
        exponent: f64,
    ) -> PyResult<Self> {
        let inner = RustDynamicRecoverySpec::inverse_power(base_recovery, base_notional, exponent)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Create a floored inverse recovery spec.
    ///
    /// ``R(N) = max(floor, R_0 * (N_0 / N))``, clamped to ``[0, R_0]``.
    ///
    /// Parameters
    /// ----------
    /// base_recovery : float
    ///     Base recovery rate at the base notional.
    /// base_notional : float
    ///     Reference notional.
    /// floor : float
    ///     Minimum recovery rate floor.
    ///
    /// Returns
    /// -------
    /// DynamicRecoverySpec
    #[classmethod]
    #[pyo3(text_signature = "(cls, base_recovery, base_notional, floor)")]
    fn floored_inverse(
        _cls: &Bound<'_, PyType>,
        base_recovery: f64,
        base_notional: f64,
        floor: f64,
    ) -> PyResult<Self> {
        let inner = RustDynamicRecoverySpec::floored_inverse(base_recovery, base_notional, floor)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Create a linear-decline recovery spec.
    ///
    /// ``R(N) = clamp(R_0 * (1 - sensitivity * (N/N_0 - 1)), floor, R_0)``
    ///
    /// Parameters
    /// ----------
    /// base_recovery : float
    ///     Base recovery rate at the base notional.
    /// base_notional : float
    ///     Reference notional.
    /// sensitivity : float
    ///     Sensitivity of recovery to notional increase.
    /// floor : float
    ///     Minimum recovery rate floor.
    ///
    /// Returns
    /// -------
    /// DynamicRecoverySpec
    #[classmethod]
    #[pyo3(text_signature = "(cls, base_recovery, base_notional, sensitivity, floor)")]
    fn linear_decline(
        _cls: &Bound<'_, PyType>,
        base_recovery: f64,
        base_notional: f64,
        sensitivity: f64,
        floor: f64,
    ) -> PyResult<Self> {
        let inner = RustDynamicRecoverySpec::linear_decline(
            base_recovery,
            base_notional,
            sensitivity,
            floor,
        )
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Compute recovery rate given current accreted notional.
    ///
    /// Parameters
    /// ----------
    /// current_notional : float
    ///     Current (PIK-augmented) notional outstanding.
    ///
    /// Returns
    /// -------
    /// float
    ///     Recovery rate clamped to ``[0, base_recovery]``.
    fn recovery_at_notional(&self, current_notional: f64) -> f64 {
        self.inner.recovery_at_notional(current_notional)
    }

    /// Base (reference) recovery rate.
    #[getter]
    fn base_recovery(&self) -> f64 {
        self.inner.base_recovery()
    }

    /// Base (reference) notional.
    #[getter]
    fn base_notional(&self) -> f64 {
        self.inner.base_notional()
    }

    /// Canonical name of the recovery model variant.
    #[getter]
    fn name(&self) -> &'static str {
        self.label()
    }

    fn __repr__(&self) -> String {
        format!("DynamicRecoverySpec('{}')", self.label())
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
    module.add_class::<PyDynamicRecoverySpec>()?;
    Ok(vec!["DynamicRecoverySpec"])
}
