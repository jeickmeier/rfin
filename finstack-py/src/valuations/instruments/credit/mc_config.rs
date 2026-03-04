use crate::valuations::instruments::credit::dynamic_recovery::PyDynamicRecoverySpec;
use crate::valuations::instruments::credit::endogenous_hazard::PyEndogenousHazardSpec;
use crate::valuations::instruments::credit::merton::PyMertonModel;
use crate::valuations::instruments::credit::toggle_exercise::PyToggleExerciseModel;
use finstack_valuations::instruments::fixed_income::bond::pricing::merton_mc_engine::{
    BarrierCrossing, CalibrationParameter, MertonMcCalibrationSpec,
    MertonMcConfig as RustMertonMcConfig, MertonMcResult as RustMertonMcResult, PikMode,
    PikSchedule,
};
use finstack_valuations::instruments::fixed_income::bond::pricing::quote_conversions::BondQuoteInput;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyModule;

// ---------------------------------------------------------------------------
// PIK schedule parsing
// ---------------------------------------------------------------------------

/// Parse a Python `pik_schedule` value into a Rust [`PikSchedule`].
///
/// Accepted forms:
/// - `None` → `Uniform(Cash)` (default)
/// - `"cash"` / `"pik"` / `"toggle"` → `Uniform(mode)`
/// - `[(0.0, "pik"), (2.0, "cash")]` → `Stepped([...])`
/// - `[(0.0, "toggle"), (3.0, {"cash": 0.5, "pik": 0.5})]` → mixed schedule
fn parse_pik_schedule(obj: &Bound<'_, pyo3::types::PyAny>) -> PyResult<PikSchedule> {
    if obj.is_none() {
        return Ok(PikSchedule::default());
    }
    if let Ok(s) = obj.extract::<String>() {
        return Ok(PikSchedule::Uniform(parse_pik_mode_str(&s)?));
    }
    // Try to iterate as a list of (float, mode) tuples
    if let Ok(iter) = obj.try_iter() {
        let mut steps = Vec::new();
        for item in iter {
            let item = item?;
            let tuple = item.extract::<(f64, Bound<'_, pyo3::types::PyAny>)>()?;
            let mode = parse_pik_mode(&tuple.1)?;
            steps.push((tuple.0, mode));
        }
        if !steps.is_empty() {
            return Ok(PikSchedule::Stepped(steps));
        }
    }
    Err(PyValueError::new_err(
        "pik_schedule must be None, a string ('cash'/'pik'/'toggle'), \
         or a list of (time, mode) tuples",
    ))
}

fn parse_pik_mode(obj: &Bound<'_, pyo3::types::PyAny>) -> PyResult<PikMode> {
    if let Ok(s) = obj.extract::<String>() {
        return parse_pik_mode_str(&s);
    }
    if let Ok(d) = obj.extract::<std::collections::HashMap<String, f64>>() {
        let cash = d.get("cash").copied().unwrap_or(0.0);
        let pik = d.get("pik").copied().unwrap_or(0.0);
        return Ok(PikMode::Split {
            cash_fraction: cash,
            pik_fraction: pik,
        });
    }
    Err(PyValueError::new_err(
        "PIK mode must be 'cash', 'pik', 'toggle', or {'cash': ..., 'pik': ...}",
    ))
}

fn parse_pik_mode_str(s: &str) -> PyResult<PikMode> {
    match s.to_lowercase().as_str() {
        "cash" => Ok(PikMode::Cash),
        "pik" => Ok(PikMode::Pik),
        "toggle" | "pik_toggle" => Ok(PikMode::Toggle),
        other => Err(PyValueError::new_err(format!(
            "Unknown PIK mode '{other}': use 'cash', 'pik', or 'toggle'"
        ))),
    }
}

// ---------------------------------------------------------------------------
// PyMertonMcConfig
// ---------------------------------------------------------------------------

/// Monte Carlo configuration for Merton structural credit pricing.
///
/// Bundles a :class:`MertonModel` with optional credit extensions
/// (endogenous hazard, dynamic recovery, toggle exercise) and simulation
/// parameters (paths, seed, antithetic, time steps).
///
/// Examples
/// --------
///     >>> from finstack.valuations.instruments.credit import MertonModel, MertonMcConfig
///     >>> m = MertonModel(asset_value=100, asset_vol=0.25, debt_barrier=80, risk_free_rate=0.04)
///     >>> config = MertonMcConfig(m, num_paths=5000, seed=123)
///     >>> config.num_paths
///     5000
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "MertonMcConfig",
    frozen
)]
pub struct PyMertonMcConfig {
    pub(crate) inner: RustMertonMcConfig,
}

#[pymethods]
impl PyMertonMcConfig {
    /// Construct a Monte Carlo configuration for Merton structural pricing.
    ///
    /// Parameters
    /// ----------
    /// merton : MertonModel
    ///     Structural credit model.
    /// pik_schedule : str | list[tuple[float, str | dict]] | None, optional
    ///     PIK schedule controlling per-coupon behavior. Accepted forms:
    ///
    ///     - ``None`` (default) — derived from the bond's coupon type
    ///     - ``"cash"`` / ``"pik"`` / ``"toggle"`` — uniform mode
    ///     - ``[(0.0, "pik"), (2.0, "cash")]`` — stepped schedule
    ///     - ``[(0.0, "toggle"), (3.0, {"cash": 0.5, "pik": 0.5})]`` — mixed
    /// endogenous_hazard : EndogenousHazardSpec | None, optional
    ///     Endogenous (leverage-dependent) hazard rate model.
    /// dynamic_recovery : DynamicRecoverySpec | None, optional
    ///     Dynamic (notional-dependent) recovery rate model.
    /// toggle_model : ToggleExerciseModel | None, optional
    ///     Toggle exercise model, active for ``PikMode::Toggle`` periods.
    /// num_paths : int, optional
    ///     Number of Monte Carlo paths (default: 10,000).
    /// seed : int, optional
    ///     RNG seed for reproducibility (default: 42).
    /// antithetic : bool, optional
    ///     Use antithetic variates for variance reduction (default: True).
    /// time_steps_per_year : int, optional
    ///     Simulation time steps per year (default: 12).
    /// barrier_crossing : str | None, optional
    ///     Barrier-crossing policy: ``"discrete"`` or ``"brownian_bridge"``.
    ///     When ``None`` (default), uses ``"brownian_bridge"`` for first-passage
    ///     barriers and ``"discrete"`` for terminal barriers.
    /// calibrate_to_z_spread : float | None, optional
    ///     When set, calibrate the debt barrier so the cash base-case MC
    ///     z-spread matches this target (decimal, e.g. 0.05 for 500 bp).
    /// calibrate_to_price : float | None, optional
    ///     When set, calibrate the debt barrier so the cash base-case MC
    ///     clean price (% of par) matches this target.
    /// calibration_parameter : str, optional
    ///     Which parameter to solve for: ``"barrier"`` (default) or ``"vol"``.
    /// calibration_low_paths : int, optional
    ///     Number of MC paths for the calibration pass (default: 2000).
    ///
    /// Returns
    /// -------
    /// MertonMcConfig
    #[new]
    #[pyo3(signature = (
        merton,
        *,
        pik_schedule = None,
        endogenous_hazard = None,
        dynamic_recovery = None,
        toggle_model = None,
        num_paths = 10_000,
        seed = 42,
        antithetic = true,
        time_steps_per_year = 12,
        barrier_crossing = None,
        calibrate_to_z_spread = None,
        calibrate_to_price = None,
        calibration_parameter = "barrier",
        calibration_low_paths = 2_000,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        merton: &PyMertonModel,
        pik_schedule: Option<&Bound<'_, pyo3::types::PyAny>>,
        endogenous_hazard: Option<&PyEndogenousHazardSpec>,
        dynamic_recovery: Option<&PyDynamicRecoverySpec>,
        toggle_model: Option<&PyToggleExerciseModel>,
        num_paths: usize,
        seed: u64,
        antithetic: bool,
        time_steps_per_year: usize,
        barrier_crossing: Option<&str>,
        calibrate_to_z_spread: Option<f64>,
        calibrate_to_price: Option<f64>,
        calibration_parameter: &str,
        calibration_low_paths: usize,
    ) -> PyResult<Self> {
        let rust_schedule = match pik_schedule {
            Some(obj) => parse_pik_schedule(obj)?,
            None => PikSchedule::default(),
        };
        let mut config = RustMertonMcConfig::new(merton.inner.clone())
            .pik_schedule(rust_schedule)
            .num_paths(num_paths)
            .seed(seed)
            .antithetic(antithetic)
            .time_steps_per_year(time_steps_per_year);

        if let Some(bc) = barrier_crossing {
            let policy = match bc.to_lowercase().as_str() {
                "discrete" => BarrierCrossing::Discrete,
                "brownian_bridge" | "bb" => BarrierCrossing::BrownianBridge,
                other => {
                    return Err(PyValueError::new_err(format!(
                        "Unknown barrier_crossing '{other}': use 'discrete' or 'brownian_bridge'"
                    )));
                }
            };
            config = config.barrier_crossing(policy);
        }

        if let Some(eh) = endogenous_hazard {
            config = config.endogenous_hazard(eh.inner.clone());
        }
        if let Some(dr) = dynamic_recovery {
            config = config.dynamic_recovery(dr.inner.clone());
        }
        if let Some(tm) = toggle_model {
            config = config.toggle_model(tm.inner.clone());
        }

        if calibrate_to_z_spread.is_some() && calibrate_to_price.is_some() {
            return Err(PyValueError::new_err(
                "Cannot set both calibrate_to_z_spread and calibrate_to_price",
            ));
        }
        let cal_param = match calibration_parameter.to_lowercase().as_str() {
            "barrier" | "debt_barrier" => CalibrationParameter::DebtBarrier,
            "vol" | "asset_vol" => CalibrationParameter::AssetVol,
            other => {
                return Err(PyValueError::new_err(format!(
                    "Unknown calibration_parameter '{other}': use 'barrier' or 'vol'"
                )));
            }
        };
        if let Some(z) = calibrate_to_z_spread {
            config = config.calibration(MertonMcCalibrationSpec {
                target: BondQuoteInput::ZSpread(z),
                parameter: cal_param,
                low_paths: calibration_low_paths,
                ..MertonMcCalibrationSpec::default()
            });
        }
        if let Some(px) = calibrate_to_price {
            config = config.calibration(MertonMcCalibrationSpec {
                target: BondQuoteInput::CleanPricePct(px),
                parameter: cal_param,
                low_paths: calibration_low_paths,
                ..MertonMcCalibrationSpec::default()
            });
        }

        Ok(Self { inner: config })
    }

    /// Number of Monte Carlo paths.
    #[getter]
    fn num_paths(&self) -> usize {
        self.inner.num_paths
    }

    /// RNG seed.
    #[getter]
    fn seed(&self) -> u64 {
        self.inner.seed
    }

    /// Whether antithetic variates are enabled.
    #[getter]
    fn antithetic(&self) -> bool {
        self.inner.antithetic
    }

    /// Number of time steps per year.
    #[getter]
    fn time_steps_per_year(&self) -> usize {
        self.inner.time_steps_per_year
    }

    /// Barrier-crossing policy: ``"discrete"`` or ``"brownian_bridge"``.
    #[getter]
    fn barrier_crossing(&self) -> &str {
        match self.inner.barrier_crossing {
            BarrierCrossing::Discrete => "discrete",
            BarrierCrossing::BrownianBridge => "brownian_bridge",
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "MertonMcConfig(num_paths={}, seed={}, antithetic={}, time_steps_per_year={})",
            self.inner.num_paths,
            self.inner.seed,
            self.inner.antithetic,
            self.inner.time_steps_per_year,
        )
    }
}

// ---------------------------------------------------------------------------
// PyMertonMcResult
// ---------------------------------------------------------------------------

/// Result from Monte Carlo Merton structural credit pricing.
///
/// Contains clean/dirty prices, loss metrics, spread, and path statistics.
/// All properties are read-only.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "MertonMcResult",
    frozen
)]
pub struct PyMertonMcResult {
    pub(crate) inner: RustMertonMcResult,
}

#[pymethods]
impl PyMertonMcResult {
    /// Clean price as percentage of par.
    #[getter]
    fn clean_price_pct(&self) -> f64 {
        self.inner.clean_price_pct
    }

    /// Dirty price as percentage of par.
    #[getter]
    fn dirty_price_pct(&self) -> f64 {
        self.inner.dirty_price_pct
    }

    /// Expected loss as fraction of risk-free present value.
    #[getter]
    fn expected_loss(&self) -> f64 {
        self.inner.expected_loss
    }

    /// Unexpected loss (standard deviation of path PVs / notional).
    #[getter]
    fn unexpected_loss(&self) -> f64 {
        self.inner.unexpected_loss
    }

    /// Expected shortfall at 95% confidence level.
    #[getter]
    fn expected_shortfall_95(&self) -> f64 {
        self.inner.expected_shortfall_95
    }

    /// Average PIK fraction across all coupon dates and paths.
    #[getter]
    fn average_pik_fraction(&self) -> f64 {
        self.inner.average_pik_fraction
    }

    /// Effective spread in basis points (spread implied by MC price vs risk-free).
    #[getter]
    fn effective_spread_bp(&self) -> f64 {
        self.inner.effective_spread_bp
    }

    /// Fraction of paths that defaulted.
    #[getter]
    fn default_rate(&self) -> f64 {
        self.inner.path_statistics.default_rate
    }

    /// Average default time (in years) among defaulted paths.
    #[getter]
    fn avg_default_time(&self) -> f64 {
        self.inner.path_statistics.avg_default_time
    }

    /// Average terminal notional (reflects PIK accrual).
    #[getter]
    fn avg_terminal_notional(&self) -> f64 {
        self.inner.path_statistics.avg_terminal_notional
    }

    /// Average recovery percentage among defaulted paths.
    #[getter]
    fn avg_recovery_pct(&self) -> f64 {
        self.inner.path_statistics.avg_recovery_pct
    }

    /// Fraction of coupon dates where PIK was elected.
    #[getter]
    fn pik_exercise_rate(&self) -> f64 {
        self.inner.path_statistics.pik_exercise_rate
    }

    /// Number of paths used.
    #[getter]
    fn num_paths(&self) -> usize {
        self.inner.num_paths
    }

    /// Standard error of the clean price estimate.
    #[getter]
    fn standard_error(&self) -> f64 {
        self.inner.standard_error
    }

    fn __repr__(&self) -> String {
        format!(
            "MertonMcResult(clean_price_pct={:.4}, default_rate={:.4}, num_paths={})",
            self.inner.clean_price_pct,
            self.inner.path_statistics.default_rate,
            self.inner.num_paths,
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
    module.add_class::<PyMertonMcConfig>()?;
    module.add_class::<PyMertonMcResult>()?;
    Ok(vec!["MertonMcConfig", "MertonMcResult"])
}
