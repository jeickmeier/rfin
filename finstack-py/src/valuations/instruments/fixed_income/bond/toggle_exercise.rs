use finstack_valuations::instruments::common::models::credit::toggle_exercise::{
    CreditStateVariable, OptimalToggle, StochasticToggle, ThresholdDirection, ThresholdToggle,
    ToggleExerciseModel as RustToggleExerciseModel,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyModule, PyType};

// ---------------------------------------------------------------------------
// Helpers: parse string parameters into Rust enums
// ---------------------------------------------------------------------------

fn parse_state_variable(s: &str) -> PyResult<CreditStateVariable> {
    match s.to_ascii_lowercase().replace('-', "_").as_str() {
        "hazard_rate" | "hazard" => Ok(CreditStateVariable::HazardRate),
        "distance_to_default" | "dd" => Ok(CreditStateVariable::DistanceToDefault),
        "leverage" => Ok(CreditStateVariable::Leverage),
        other => Err(PyValueError::new_err(format!(
            "Unknown state variable: '{}'. Expected 'hazard_rate', 'distance_to_default', or 'leverage'",
            other
        ))),
    }
}

fn parse_direction(s: &str) -> PyResult<ThresholdDirection> {
    match s.to_ascii_lowercase().as_str() {
        "above" => Ok(ThresholdDirection::Above),
        "below" => Ok(ThresholdDirection::Below),
        other => Err(PyValueError::new_err(format!(
            "Unknown direction: '{}'. Expected 'above' or 'below'",
            other
        ))),
    }
}

// ---------------------------------------------------------------------------
// PyToggleExerciseModel
// ---------------------------------------------------------------------------

/// Toggle exercise model for PIK/cash coupon decisions.
///
/// Models the borrower's decision to pay-in-kind (PIK) or pay cash at each
/// coupon date. The toggle decision depends on observable credit state.
///
/// Examples
/// --------
///     >>> model = ToggleExerciseModel.threshold("hazard_rate", 0.15)
///     >>> model.name
///     'Threshold'
///     >>> model = ToggleExerciseModel.stochastic("hazard_rate", -3.0, 20.0)
///     >>> model.name
///     'Stochastic'
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "ToggleExerciseModel",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyToggleExerciseModel {
    pub(crate) inner: RustToggleExerciseModel,
}

impl PyToggleExerciseModel {
    fn label(&self) -> &'static str {
        match &self.inner {
            RustToggleExerciseModel::Threshold(_) => "Threshold",
            RustToggleExerciseModel::Stochastic(_) => "Stochastic",
            RustToggleExerciseModel::OptimalExercise(_) => "OptimalExercise",
        }
    }
}

#[pymethods]
impl PyToggleExerciseModel {
    /// Create a threshold toggle model.
    ///
    /// PIK is elected when the credit metric crosses the boundary in the
    /// specified direction.
    ///
    /// Parameters
    /// ----------
    /// variable : str
    ///     Credit state variable: ``"hazard_rate"``, ``"distance_to_default"``,
    ///     or ``"leverage"``.
    /// threshold : float
    ///     Threshold value for the comparison.
    /// direction : str, optional
    ///     Direction for comparison: ``"above"`` (default) or ``"below"``.
    ///
    /// Returns
    /// -------
    /// ToggleExerciseModel
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If ``variable`` or ``direction`` is not recognised.
    #[classmethod]
    #[pyo3(signature = (variable, threshold, direction = "above"))]
    fn threshold(
        _cls: &Bound<'_, PyType>,
        variable: &str,
        threshold: f64,
        direction: &str,
    ) -> PyResult<Self> {
        let state_variable = parse_state_variable(variable)?;
        let dir = parse_direction(direction)?;
        Ok(Self {
            inner: RustToggleExerciseModel::Threshold(ThresholdToggle {
                state_variable,
                threshold,
                direction: dir,
            }),
        })
    }

    /// Create a stochastic (sigmoid) toggle model.
    ///
    /// PIK probability follows a logistic function:
    /// ``P(PIK) = 1 / (1 + exp(-(intercept + sensitivity * state)))``
    ///
    /// Parameters
    /// ----------
    /// variable : str
    ///     Credit state variable: ``"hazard_rate"``, ``"distance_to_default"``,
    ///     or ``"leverage"``.
    /// intercept : float
    ///     Intercept of the logistic function.
    /// sensitivity : float
    ///     Sensitivity (slope) of the logistic function.
    ///
    /// Returns
    /// -------
    /// ToggleExerciseModel
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If ``variable`` is not recognised.
    #[classmethod]
    #[pyo3(text_signature = "(cls, variable, intercept, sensitivity)")]
    fn stochastic(
        _cls: &Bound<'_, PyType>,
        variable: &str,
        intercept: f64,
        sensitivity: f64,
    ) -> PyResult<Self> {
        let state_variable = parse_state_variable(variable)?;
        Ok(Self {
            inner: RustToggleExerciseModel::Stochastic(StochasticToggle {
                state_variable,
                intercept,
                sensitivity,
            }),
        })
    }

    /// Create an optimal exercise toggle model using nested Monte Carlo.
    ///
    /// At each coupon date, a small nested MC simulation estimates equity
    /// value under cash vs PIK scenarios to make the optimal toggle
    /// decision.
    ///
    /// Parameters
    /// ----------
    /// nested_paths : int, optional
    ///     Number of nested Monte Carlo paths (default: 200).
    /// equity_discount_rate : float, optional
    ///     Equity holder discount rate for NPV (default: 0.10).
    /// asset_vol : float, optional
    ///     Annualised asset volatility for the nested GBM simulation
    ///     (default: 0.30).
    /// risk_free_rate : float, optional
    ///     Risk-free rate (continuous) used as drift in the nested
    ///     simulation (default: 0.03).
    /// horizon : float, optional
    ///     Forward-looking horizon in years (default: 1.0).
    ///
    /// Returns
    /// -------
    /// ToggleExerciseModel
    #[classmethod]
    #[pyo3(signature = (nested_paths = 200, equity_discount_rate = 0.10, asset_vol = 0.30, risk_free_rate = 0.03, horizon = 1.0))]
    fn optimal_exercise(
        _cls: &Bound<'_, PyType>,
        nested_paths: usize,
        equity_discount_rate: f64,
        asset_vol: f64,
        risk_free_rate: f64,
        horizon: f64,
    ) -> Self {
        Self {
            inner: RustToggleExerciseModel::OptimalExercise(OptimalToggle {
                nested_paths,
                equity_discount_rate,
                asset_vol,
                risk_free_rate,
                horizon,
            }),
        }
    }

    /// Canonical name of the toggle model variant.
    #[getter]
    fn name(&self) -> &'static str {
        self.label()
    }

    fn __repr__(&self) -> String {
        format!("ToggleExerciseModel('{}')", self.label())
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
    module.add_class::<PyToggleExerciseModel>()?;
    Ok(vec!["ToggleExerciseModel"])
}
