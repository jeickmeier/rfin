use crate::core::dates::utils::py_to_date;
use crate::core::market_data::term_structures::PyHazardCurve;
use finstack_valuations::instruments::common::models::credit::merton::{
    AssetDynamics as RustAssetDynamics, BarrierType as RustBarrierType,
    MertonModel as RustMertonModel,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyModule, PyType};
use std::sync::Arc;

// ---------------------------------------------------------------------------
// PyAssetDynamics
// ---------------------------------------------------------------------------

/// Asset dynamics specification for the Merton structural credit model.
///
/// Controls the stochastic process assumed for the firm's asset value.
///
/// Examples
/// --------
///     >>> MertonAssetDynamics.GEOMETRIC_BROWNIAN
///     MertonAssetDynamics('GeometricBrownian')
///     >>> MertonAssetDynamics.jump_diffusion(0.5, -0.05, 0.10)
///     MertonAssetDynamics('JumpDiffusion')
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "MertonAssetDynamics",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyAssetDynamics {
    pub(crate) inner: RustAssetDynamics,
}

impl PyAssetDynamics {
    pub(crate) fn new(inner: RustAssetDynamics) -> Self {
        Self { inner }
    }

    fn label(&self) -> &'static str {
        match self.inner {
            RustAssetDynamics::GeometricBrownian => "GeometricBrownian",
            RustAssetDynamics::JumpDiffusion { .. } => "JumpDiffusion",
            RustAssetDynamics::CreditGrades { .. } => "CreditGrades",
        }
    }
}

#[pymethods]
impl PyAssetDynamics {
    /// Standard geometric Brownian motion (lognormal diffusion).
    #[classattr]
    const GEOMETRIC_BROWNIAN: Self = Self {
        inner: RustAssetDynamics::GeometricBrownian,
    };

    /// Jump-diffusion process (Merton 1976) with Poisson jumps.
    ///
    /// Parameters
    /// ----------
    /// jump_intensity : float
    ///     Poisson jump arrival intensity (jumps per year).
    /// jump_mean : float
    ///     Mean log-jump size.
    /// jump_vol : float
    ///     Volatility of log-jump size.
    ///
    /// Returns
    /// -------
    /// MertonAssetDynamics
    #[classmethod]
    #[pyo3(text_signature = "(cls, jump_intensity, jump_mean, jump_vol)")]
    fn jump_diffusion(
        _cls: &Bound<'_, PyType>,
        jump_intensity: f64,
        jump_mean: f64,
        jump_vol: f64,
    ) -> Self {
        Self::new(RustAssetDynamics::JumpDiffusion {
            jump_intensity,
            jump_mean,
            jump_vol,
        })
    }

    /// CreditGrades model extension with uncertain recovery barrier.
    ///
    /// Parameters
    /// ----------
    /// barrier_uncertainty : float
    ///     Uncertainty in the default barrier level.
    /// mean_recovery : float
    ///     Mean recovery rate at default.
    ///
    /// Returns
    /// -------
    /// MertonAssetDynamics
    #[classmethod]
    #[pyo3(text_signature = "(cls, barrier_uncertainty, mean_recovery)")]
    fn credit_grades(
        _cls: &Bound<'_, PyType>,
        barrier_uncertainty: f64,
        mean_recovery: f64,
    ) -> Self {
        Self::new(RustAssetDynamics::CreditGrades {
            barrier_uncertainty,
            mean_recovery,
        })
    }

    /// Canonical name of the dynamics variant.
    #[getter]
    fn name(&self) -> &'static str {
        self.label()
    }

    fn __repr__(&self) -> String {
        format!("MertonAssetDynamics('{}')", self.label())
    }

    fn __str__(&self) -> &'static str {
        self.label()
    }
}

// ---------------------------------------------------------------------------
// PyBarrierType
// ---------------------------------------------------------------------------

/// Barrier monitoring type for default determination.
///
/// Examples
/// --------
///     >>> MertonBarrierType.TERMINAL
///     MertonBarrierType('Terminal')
///     >>> MertonBarrierType.first_passage(0.05)
///     MertonBarrierType('FirstPassage')
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "MertonBarrierType",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyBarrierType {
    pub(crate) inner: RustBarrierType,
}

impl PyBarrierType {
    pub(crate) fn new(inner: RustBarrierType) -> Self {
        Self { inner }
    }

    fn label(&self) -> &'static str {
        match self.inner {
            RustBarrierType::Terminal => "Terminal",
            RustBarrierType::FirstPassage { .. } => "FirstPassage",
        }
    }
}

#[pymethods]
impl PyBarrierType {
    /// Terminal barrier (classic Merton): default only assessed at maturity.
    #[classattr]
    const TERMINAL: Self = Self {
        inner: RustBarrierType::Terminal,
    };

    /// First-passage barrier (Black-Cox extension): continuous monitoring.
    ///
    /// Parameters
    /// ----------
    /// barrier_growth_rate : float
    ///     Growth rate of the default barrier over time.
    ///
    /// Returns
    /// -------
    /// MertonBarrierType
    #[classmethod]
    #[pyo3(text_signature = "(cls, barrier_growth_rate)")]
    fn first_passage(_cls: &Bound<'_, PyType>, barrier_growth_rate: f64) -> Self {
        Self::new(RustBarrierType::FirstPassage {
            barrier_growth_rate,
        })
    }

    /// Canonical name of the barrier type variant.
    #[getter]
    fn name(&self) -> &'static str {
        self.label()
    }

    fn __repr__(&self) -> String {
        format!("MertonBarrierType('{}')", self.label())
    }

    fn __str__(&self) -> &'static str {
        self.label()
    }
}

// ---------------------------------------------------------------------------
// PyMertonModel
// ---------------------------------------------------------------------------

/// Merton structural credit model for estimating firm default probability.
///
/// Models a firm's equity as a call option on its assets, where default
/// occurs when asset value falls below the debt barrier.
///
/// Examples
/// --------
///     >>> m = MertonModel(asset_value=100, asset_vol=0.20, debt_barrier=80, risk_free_rate=0.05)
///     >>> m.distance_to_default(1.0)
///     1.2657...
///     >>> m.default_probability(1.0)
///     0.1028...
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "MertonModel",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyMertonModel {
    pub(crate) inner: RustMertonModel,
}

#[pymethods]
impl PyMertonModel {
    /// Construct a Merton structural credit model.
    ///
    /// Parameters
    /// ----------
    /// asset_value : float
    ///     Current market value of the firm's assets (must be > 0).
    /// asset_vol : float
    ///     Annualized volatility of asset returns (must be >= 0).
    /// debt_barrier : float
    ///     Face value of debt / default point (must be > 0).
    /// risk_free_rate : float
    ///     Continuous risk-free rate.
    /// payout_rate : float, optional
    ///     Continuous dividend / payout yield on assets (default: 0.0).
    /// barrier_type : MertonBarrierType, optional
    ///     Barrier monitoring type (default: Terminal).
    /// dynamics : MertonAssetDynamics, optional
    ///     Asset return dynamics specification (default: GeometricBrownian).
    ///
    /// Returns
    /// -------
    /// MertonModel
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If inputs are invalid.
    #[new]
    #[pyo3(signature = (
        asset_value,
        asset_vol,
        debt_barrier,
        risk_free_rate,
        *,
        payout_rate = 0.0,
        barrier_type = None,
        dynamics = None,
    ))]
    fn new(
        asset_value: f64,
        asset_vol: f64,
        debt_barrier: f64,
        risk_free_rate: f64,
        payout_rate: f64,
        barrier_type: Option<PyBarrierType>,
        dynamics: Option<PyAssetDynamics>,
    ) -> PyResult<Self> {
        let bt = barrier_type
            .map(|b| b.inner)
            .unwrap_or(RustBarrierType::Terminal);
        let dyn_ = dynamics
            .map(|d| d.inner)
            .unwrap_or(RustAssetDynamics::GeometricBrownian);
        let model = RustMertonModel::new_with_dynamics(
            asset_value,
            asset_vol,
            debt_barrier,
            risk_free_rate,
            payout_rate,
            bt,
            dyn_,
        )
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner: model })
    }

    // -----------------------------------------------------------------------
    // Classmethods (calibration)
    // -----------------------------------------------------------------------

    /// KMV calibration from observed equity value and equity volatility.
    ///
    /// Parameters
    /// ----------
    /// equity_value : float
    ///     Observed market equity value.
    /// equity_vol : float
    ///     Observed equity volatility.
    /// total_debt : float
    ///     Face value of debt.
    /// risk_free_rate : float
    ///     Risk-free rate.
    /// payout_rate : float, optional
    ///     Continuous dividend / payout yield (default: 0.0).
    /// maturity : float, optional
    ///     Time to maturity in years (default: 1.0).
    ///
    /// Returns
    /// -------
    /// MertonModel
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If inputs are invalid or calibration fails to converge.
    #[classmethod]
    #[pyo3(signature = (equity_value, equity_vol, total_debt, risk_free_rate, payout_rate=0.0, maturity=1.0))]
    fn from_equity(
        _cls: &Bound<'_, PyType>,
        equity_value: f64,
        equity_vol: f64,
        total_debt: f64,
        risk_free_rate: f64,
        payout_rate: f64,
        maturity: f64,
    ) -> PyResult<Self> {
        let model = RustMertonModel::from_equity(
            equity_value,
            equity_vol,
            total_debt,
            risk_free_rate,
            payout_rate,
            maturity,
        )
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner: model })
    }

    /// Calibrate asset volatility from a target CDS spread.
    ///
    /// Parameters
    /// ----------
    /// cds_spread_bp : float
    ///     Target CDS spread in basis points.
    /// recovery : float
    ///     Recovery rate (fraction).
    /// total_debt : float
    ///     Face value of debt.
    /// risk_free_rate : float
    ///     Risk-free rate.
    /// maturity : float
    ///     Time to maturity in years.
    /// asset_value : float
    ///     Assumed initial asset value.
    ///
    /// Returns
    /// -------
    /// MertonModel
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If inputs are invalid or solver fails.
    #[classmethod]
    #[pyo3(
        text_signature = "(cls, cds_spread_bp, recovery, total_debt, risk_free_rate, maturity, asset_value)"
    )]
    fn from_cds_spread(
        _cls: &Bound<'_, PyType>,
        cds_spread_bp: f64,
        recovery: f64,
        total_debt: f64,
        risk_free_rate: f64,
        maturity: f64,
        asset_value: f64,
    ) -> PyResult<Self> {
        let model = RustMertonModel::from_cds_spread(
            cds_spread_bp,
            recovery,
            total_debt,
            risk_free_rate,
            maturity,
            asset_value,
        )
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner: model })
    }

    /// Calibrate the debt barrier to match a target default probability.
    ///
    /// Parameters
    /// ----------
    /// asset_value : float
    ///     Current asset value V.
    /// asset_vol : float
    ///     Asset volatility sigma_V.
    /// risk_free_rate : float
    ///     Risk-free rate r.
    /// target_pd : float
    ///     Target cumulative default probability (e.g. 0.01 for 1%).
    /// maturity : float, optional
    ///     Time horizon in years (default: 5.0).
    ///
    /// Returns
    /// -------
    /// MertonModel
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If inputs are invalid or calibration fails.
    #[classmethod]
    #[pyo3(signature = (asset_value, asset_vol, risk_free_rate, target_pd, maturity=5.0))]
    fn from_target_pd(
        _cls: &Bound<'_, PyType>,
        asset_value: f64,
        asset_vol: f64,
        risk_free_rate: f64,
        target_pd: f64,
        maturity: f64,
    ) -> PyResult<Self> {
        let model = RustMertonModel::from_target_pd(
            asset_value,
            asset_vol,
            risk_free_rate,
            target_pd,
            maturity,
        )
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner: model })
    }

    /// CreditGrades model construction from equity observables.
    ///
    /// Parameters
    /// ----------
    /// equity_value : float
    ///     Observed market equity value.
    /// equity_vol : float
    ///     Observed equity volatility.
    /// total_debt : float
    ///     Face value of debt.
    /// risk_free_rate : float
    ///     Risk-free rate.
    /// barrier_uncertainty : float
    ///     Uncertainty in the default barrier level.
    /// mean_recovery : float
    ///     Mean recovery rate at default.
    ///
    /// Returns
    /// -------
    /// MertonModel
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If inputs are invalid.
    #[classmethod]
    #[pyo3(
        text_signature = "(cls, equity_value, equity_vol, total_debt, risk_free_rate, barrier_uncertainty, mean_recovery)"
    )]
    fn credit_grades(
        _cls: &Bound<'_, PyType>,
        equity_value: f64,
        equity_vol: f64,
        total_debt: f64,
        risk_free_rate: f64,
        barrier_uncertainty: f64,
        mean_recovery: f64,
    ) -> PyResult<Self> {
        let model = RustMertonModel::credit_grades(
            equity_value,
            equity_vol,
            total_debt,
            risk_free_rate,
            barrier_uncertainty,
            mean_recovery,
        )
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner: model })
    }

    // -----------------------------------------------------------------------
    // Methods
    // -----------------------------------------------------------------------

    /// Distance-to-default over the given horizon.
    ///
    /// Parameters
    /// ----------
    /// horizon : float, optional
    ///     Time horizon in years (default: 1.0).
    ///
    /// Returns
    /// -------
    /// float
    #[pyo3(signature = (horizon=1.0))]
    fn distance_to_default(&self, horizon: f64) -> f64 {
        self.inner.distance_to_default(horizon)
    }

    /// Default probability over the given horizon.
    ///
    /// Parameters
    /// ----------
    /// horizon : float, optional
    ///     Time horizon in years (default: 1.0).
    ///
    /// Returns
    /// -------
    /// float
    #[pyo3(signature = (horizon=1.0))]
    fn default_probability(&self, horizon: f64) -> f64 {
        self.inner.default_probability(horizon)
    }

    /// Implied credit spread from default probability and recovery rate.
    ///
    /// Parameters
    /// ----------
    /// horizon : float
    ///     Time horizon in years.
    /// recovery : float
    ///     Assumed recovery rate (fraction of face value).
    ///
    /// Returns
    /// -------
    /// float
    fn implied_spread(&self, horizon: f64, recovery: f64) -> f64 {
        self.inner.implied_spread(horizon, recovery)
    }

    /// Compute implied equity value and equity volatility.
    ///
    /// Parameters
    /// ----------
    /// horizon : float, optional
    ///     Time horizon in years (default: 1.0).
    ///
    /// Returns
    /// -------
    /// tuple[float, float]
    ///     ``(equity_value, equity_vol)``
    #[pyo3(signature = (horizon=1.0))]
    fn implied_equity(&self, horizon: f64) -> (f64, f64) {
        self.inner.implied_equity(horizon)
    }

    /// Generate a HazardCurve from structural model default probabilities.
    ///
    /// Parameters
    /// ----------
    /// curve_id : str
    ///     Curve identifier.
    /// base_date : datetime.date
    ///     Valuation date for the curve.
    /// tenors : list[float], optional
    ///     Tenor grid in years (default: ``[0.5, 1.0, 2.0, 3.0, 5.0, 7.0, 10.0]``).
    /// recovery : float, optional
    ///     Recovery rate assumption (default: 0.40).
    ///
    /// Returns
    /// -------
    /// HazardCurve
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the curve cannot be built.
    #[pyo3(signature = (curve_id, base_date, tenors=None, recovery=0.40))]
    fn to_hazard_curve(
        &self,
        curve_id: &str,
        base_date: &Bound<'_, pyo3::types::PyAny>,
        tenors: Option<Vec<f64>>,
        recovery: f64,
    ) -> PyResult<PyHazardCurve> {
        let date = py_to_date(base_date)?;
        let default_tenors = vec![0.5, 1.0, 2.0, 3.0, 5.0, 7.0, 10.0];
        let tenor_slice = tenors.as_deref().unwrap_or(&default_tenors);
        let curve = self
            .inner
            .to_hazard_curve(curve_id, date, tenor_slice, recovery)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(PyHazardCurve::new_arc(Arc::new(curve)))
    }

    // -----------------------------------------------------------------------
    // Properties (getters)
    // -----------------------------------------------------------------------

    /// Current market value of the firm's assets.
    #[getter]
    fn asset_value(&self) -> f64 {
        self.inner.asset_value()
    }

    /// Annualized volatility of asset returns.
    #[getter]
    fn asset_vol(&self) -> f64 {
        self.inner.asset_vol()
    }

    /// Face value of debt / default point.
    #[getter]
    fn debt_barrier(&self) -> f64 {
        self.inner.debt_barrier()
    }

    /// Continuous risk-free rate.
    #[getter]
    fn risk_free_rate(&self) -> f64 {
        self.inner.risk_free_rate()
    }

    /// Continuous dividend / payout yield on assets.
    #[getter]
    fn payout_rate(&self) -> f64 {
        self.inner.payout_rate()
    }

    /// Barrier monitoring type.
    #[getter]
    fn barrier_type(&self) -> PyBarrierType {
        PyBarrierType::new(*self.inner.barrier_type())
    }

    /// Asset return dynamics specification.
    #[getter]
    fn dynamics(&self) -> PyAssetDynamics {
        PyAssetDynamics::new(*self.inner.dynamics())
    }

    fn __repr__(&self) -> String {
        format!(
            "MertonModel(asset_value={:.2}, asset_vol={:.4}, debt_barrier={:.2}, risk_free_rate={:.4})",
            self.inner.asset_value(),
            self.inner.asset_vol(),
            self.inner.debt_barrier(),
            self.inner.risk_free_rate(),
        )
    }

    fn __str__(&self) -> String {
        format!(
            "MertonModel(V={:.2}, sigma={:.2}%, B={:.2}, r={:.2}%)",
            self.inner.asset_value(),
            self.inner.asset_vol() * 100.0,
            self.inner.debt_barrier(),
            self.inner.risk_free_rate() * 100.0,
        )
    }
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyAssetDynamics>()?;
    module.add_class::<PyBarrierType>()?;
    module.add_class::<PyMertonModel>()?;
    Ok(vec![
        "MertonAssetDynamics",
        "MertonBarrierType",
        "MertonModel",
    ])
}
