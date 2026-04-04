//! Tree-based bond pricing bindings (OAS, callable/putable bonds).
//!
//! Exposes `TreeModelChoice`, `TreePricerConfig`, and `TreePricer` to Python
//! for full Bloomberg-comparable OAS and tree-price workflows.

use crate::core::dates::utils::py_to_date;
use crate::core::market_data::PyMarketContext;
use crate::errors::{core_to_py, PyContext};
use pyo3::prelude::*;
use pyo3::types::{PyModule, PyType};

use finstack_valuations::instruments::fixed_income::bond::pricing::tree_engine::{
    TreeModelChoice, TreePricer as RustTreePricer, TreePricerConfig as RustTreePricerConfig,
};

use super::PyBond;

/// Choice of short-rate model for the bond pricing tree.
///
/// Controls which interest rate tree is used for backward induction.
///
/// Variants
/// --------
/// - ``TreeModelChoice.ho_lee()`` — Ho-Lee / BDT model (default)
/// - ``TreeModelChoice.hull_white(kappa, sigma)`` — Hull-White 1-factor
/// - ``TreeModelChoice.hull_white_calibrated(surface_id)`` — Hull-White calibrated to swaptions
///
/// Examples
/// --------
///     >>> m = TreeModelChoice.ho_lee()
///     >>> m.model_type
///     'HoLee'
///     >>> m = TreeModelChoice.hull_white(0.03, 0.01)
///     >>> m.model_type
///     'HullWhite'
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "TreeModelChoice",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyTreeModelChoice {
    pub(crate) inner: TreeModelChoice,
}

#[pymethods]
impl PyTreeModelChoice {
    /// Ho-Lee / BDT model (default).
    #[classmethod]
    fn ho_lee(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: TreeModelChoice::HoLee,
        }
    }

    /// Hull-White 1-factor with user-specified parameters.
    ///
    /// Parameters
    /// ----------
    /// kappa : float
    ///     Mean reversion speed (e.g., 0.03 for 3%).
    /// sigma : float
    ///     Short rate volatility (e.g., 0.01 for 100 bp).
    #[classmethod]
    fn hull_white(_cls: &Bound<'_, PyType>, kappa: f64, sigma: f64) -> Self {
        Self {
            inner: TreeModelChoice::HullWhite { kappa, sigma },
        }
    }

    /// Hull-White 1-factor calibrated to co-terminal swaptions.
    ///
    /// Parameters
    /// ----------
    /// swaption_vol_surface_id : str
    ///     ID of the swaption volatility surface in the market context.
    #[classmethod]
    fn hull_white_calibrated(_cls: &Bound<'_, PyType>, swaption_vol_surface_id: String) -> Self {
        Self {
            inner: TreeModelChoice::HullWhiteCalibratedToSwaptions {
                swaption_vol_surface_id,
            },
        }
    }

    /// Model type as a string: ``"HoLee"``, ``"HullWhite"``, or ``"HullWhiteCalibratedToSwaptions"``.
    #[getter]
    fn model_type(&self) -> &'static str {
        match &self.inner {
            TreeModelChoice::HoLee => "HoLee",
            TreeModelChoice::HullWhite { .. } => "HullWhite",
            TreeModelChoice::HullWhiteCalibratedToSwaptions { .. } => {
                "HullWhiteCalibratedToSwaptions"
            }
        }
    }

    /// Mean reversion speed (Hull-White only, ``None`` otherwise).
    #[getter]
    fn kappa(&self) -> Option<f64> {
        match &self.inner {
            TreeModelChoice::HullWhite { kappa, .. } => Some(*kappa),
            _ => None,
        }
    }

    /// Short rate volatility (Hull-White only, ``None`` otherwise).
    #[getter]
    fn sigma(&self) -> Option<f64> {
        match &self.inner {
            TreeModelChoice::HullWhite { sigma, .. } => Some(*sigma),
            _ => None,
        }
    }

    /// Swaption vol surface ID (calibrated Hull-White only, ``None`` otherwise).
    #[getter]
    fn swaption_vol_surface_id(&self) -> Option<String> {
        match &self.inner {
            TreeModelChoice::HullWhiteCalibratedToSwaptions {
                swaption_vol_surface_id,
            } => Some(swaption_vol_surface_id.clone()),
            _ => None,
        }
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            TreeModelChoice::HoLee => "TreeModelChoice.HoLee".to_string(),
            TreeModelChoice::HullWhite { kappa, sigma } => {
                format!("TreeModelChoice.HullWhite(kappa={kappa}, sigma={sigma})")
            }
            TreeModelChoice::HullWhiteCalibratedToSwaptions {
                swaption_vol_surface_id,
            } => {
                format!(
                    "TreeModelChoice.HullWhiteCalibratedToSwaptions('{swaption_vol_surface_id}')"
                )
            }
        }
    }
}

/// Configuration for tree-based bond pricing (callable/putable bonds, OAS).
///
/// Controls the tree structure, convergence settings, and solver parameters
/// for option-adjusted spread calculations.
///
/// The default constructor uses Ho-Lee model with 100 bps normal volatility
/// and 100 tree steps.
///
/// Examples
/// --------
///     >>> config = TreePricerConfig()
///     >>> config.tree_steps
///     100
///     >>> config = TreePricerConfig.hull_white(0.03, 0.01)
///     >>> config.tree_model.model_type
///     'HullWhite'
///     >>> config = TreePricerConfig.high_precision(0.012)
///     >>> config.tree_steps
///     200
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "TreePricerConfig",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyTreePricerConfig {
    pub(crate) inner: RustTreePricerConfig,
}

#[pymethods]
impl PyTreePricerConfig {
    /// Create a tree pricer configuration.
    ///
    /// Parameters
    /// ----------
    /// tree_steps : int, optional
    ///     Number of time steps (default: 100).
    /// volatility : float, optional
    ///     Short rate volatility (default: 0.01 = 100 bps normal vol).
    /// tolerance : float, optional
    ///     OAS convergence tolerance (default: 1e-6).
    /// max_iterations : int, optional
    ///     Maximum OAS solver iterations (default: 50).
    /// initial_bracket_size_bp : float | None, optional
    ///     Initial bracket for Brent solver (default: 1000 bp).
    /// mean_reversion : float | None, optional
    ///     Mean reversion speed for Hull-White extension.
    /// tree_model : TreeModelChoice | None, optional
    ///     Short-rate model choice (default: Ho-Lee).
    #[new]
    #[pyo3(signature = (
        *,
        tree_steps=100,
        volatility=0.01,
        tolerance=1e-6,
        max_iterations=50,
        initial_bracket_size_bp=Some(1000.0),
        mean_reversion=None,
        tree_model=None,
    ))]
    fn new_py(
        tree_steps: usize,
        volatility: f64,
        tolerance: f64,
        max_iterations: usize,
        initial_bracket_size_bp: Option<f64>,
        mean_reversion: Option<f64>,
        tree_model: Option<&PyTreeModelChoice>,
    ) -> Self {
        Self {
            inner: RustTreePricerConfig {
                tree_steps,
                volatility,
                tolerance,
                max_iterations,
                initial_bracket_size_bp,
                mean_reversion,
                tree_model: tree_model.map(|m| m.inner.clone()).unwrap_or_default(),
            },
        }
    }

    /// Production Ho-Lee configuration with normal volatility.
    ///
    /// Parameters
    /// ----------
    /// normal_vol : float
    ///     Normal (absolute) volatility in rate units (e.g., 0.01 = 100 bps/yr).
    #[classmethod]
    fn production_ho_lee(_cls: &Bound<'_, PyType>, normal_vol: f64) -> Self {
        Self {
            inner: RustTreePricerConfig::production_ho_lee(normal_vol),
        }
    }

    /// Production BDT configuration with lognormal volatility.
    ///
    /// Parameters
    /// ----------
    /// lognormal_vol : float
    ///     Lognormal (relative) volatility as proportion (e.g., 0.20 = 20%/yr).
    #[classmethod]
    fn production_bdt(_cls: &Bound<'_, PyType>, lognormal_vol: f64) -> Self {
        Self {
            inner: RustTreePricerConfig::production_bdt(lognormal_vol),
        }
    }

    /// Default BDT configuration with 20% lognormal volatility.
    #[classmethod]
    fn default_bdt(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: RustTreePricerConfig::default_bdt(),
        }
    }

    /// High-precision configuration for regulatory/audit (200 steps, < 0.5 bp).
    ///
    /// Parameters
    /// ----------
    /// calibrated_vol : float
    ///     Annualized short rate volatility from market calibration.
    #[classmethod]
    fn high_precision(_cls: &Bound<'_, PyType>, calibrated_vol: f64) -> Self {
        Self {
            inner: RustTreePricerConfig::high_precision(calibrated_vol),
        }
    }

    /// Fast configuration for portfolio screening (50 steps, ~2-5 bp accuracy).
    ///
    /// Parameters
    /// ----------
    /// calibrated_vol : float
    ///     Annualized short rate volatility.
    #[classmethod]
    fn fast(_cls: &Bound<'_, PyType>, calibrated_vol: f64) -> Self {
        Self {
            inner: RustTreePricerConfig::fast(calibrated_vol),
        }
    }

    /// Hull-White 1-factor configuration with user-specified parameters.
    ///
    /// Parameters
    /// ----------
    /// kappa : float
    ///     Mean reversion speed (e.g., 0.03 for 3%).
    /// sigma : float
    ///     Short rate volatility (e.g., 0.01 for 100 bp).
    #[classmethod]
    fn hull_white(_cls: &Bound<'_, PyType>, kappa: f64, sigma: f64) -> Self {
        Self {
            inner: RustTreePricerConfig::hull_white(kappa, sigma),
        }
    }

    /// Hull-White 1-factor calibrated to swaption volatilities.
    ///
    /// Parameters
    /// ----------
    /// swaption_vol_surface_id : str
    ///     ID of the swaption vol surface in the market context.
    #[classmethod]
    fn hull_white_calibrated(_cls: &Bound<'_, PyType>, swaption_vol_surface_id: String) -> Self {
        Self {
            inner: RustTreePricerConfig::hull_white_calibrated(swaption_vol_surface_id),
        }
    }

    /// Number of time steps in the interest rate tree.
    #[getter]
    fn tree_steps(&self) -> usize {
        self.inner.tree_steps
    }

    /// Short rate volatility (annualized).
    #[getter]
    fn volatility(&self) -> f64 {
        self.inner.volatility
    }

    /// Convergence tolerance for OAS root finding.
    #[getter]
    fn tolerance(&self) -> f64 {
        self.inner.tolerance
    }

    /// Maximum iterations for root finding.
    #[getter]
    fn max_iterations(&self) -> usize {
        self.inner.max_iterations
    }

    /// Initial bracket size (bp) for the OAS root solver.
    #[getter]
    fn initial_bracket_size_bp(&self) -> Option<f64> {
        self.inner.initial_bracket_size_bp
    }

    /// Mean reversion speed (``None`` for pure Ho-Lee).
    #[getter]
    fn mean_reversion(&self) -> Option<f64> {
        self.inner.mean_reversion
    }

    /// Short-rate model choice.
    #[getter]
    fn tree_model(&self) -> PyTreeModelChoice {
        PyTreeModelChoice {
            inner: self.inner.tree_model.clone(),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "TreePricerConfig(tree_steps={}, volatility={}, model={})",
            self.inner.tree_steps,
            self.inner.volatility,
            PyTreeModelChoice {
                inner: self.inner.tree_model.clone()
            }
            .__repr__()
        )
    }
}

/// Tree-based pricer for bonds with embedded options and OAS calculations.
///
/// Provides methods for calculating option-adjusted spread (OAS) and
/// pricing bonds at a given OAS. Automatically selects between short-rate
/// and rates+credit tree models based on available market data.
///
/// Examples
/// --------
///     >>> pricer = TreePricer()
///     >>> oas_bp = pricer.calculate_oas(bond, market, as_of, clean_price_pct=98.5)
///     >>> pricer = TreePricer(TreePricerConfig.hull_white(0.03, 0.01))
///     >>> oas_bp = pricer.calculate_oas(bond, market, as_of, clean_price_pct=98.5)
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "TreePricer",
    frozen
)]
pub struct PyTreePricer {
    config: Option<RustTreePricerConfig>,
}

#[pymethods]
impl PyTreePricer {
    /// Create a tree pricer.
    ///
    /// Parameters
    /// ----------
    /// config : TreePricerConfig | None, optional
    ///     Custom configuration. If ``None``, uses the default (100 steps, Ho-Lee, 100 bps vol).
    #[new]
    #[pyo3(signature = (config=None))]
    fn new_py(config: Option<&PyTreePricerConfig>) -> Self {
        Self {
            config: config.map(|c| c.inner.clone()),
        }
    }

    /// Calculate option-adjusted spread (OAS) for a bond.
    ///
    /// Solves for the constant spread (in basis points) that equates
    /// the tree-model price to the market price. Uses Brent's method.
    ///
    /// Parameters
    /// ----------
    /// bond : Bond
    ///     Bond instrument (may have call/put options).
    /// market : MarketContext
    ///     Market data including discount and optionally hazard curves.
    /// as_of : datetime.date
    ///     Valuation date.
    /// clean_price_pct : float
    ///     Market clean price as percentage of par (e.g., 98.5).
    ///
    /// Returns
    /// -------
    /// float
    ///     OAS in basis points (e.g., 150.0 means 150 bp).
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the discount curve is missing, calibration fails, or the
    ///     solver does not converge.
    #[pyo3(signature = (bond, market, as_of, clean_price_pct))]
    fn calculate_oas(
        &self,
        bond: &PyBond,
        market: &PyMarketContext,
        as_of: &Bound<'_, pyo3::types::PyAny>,
        clean_price_pct: f64,
    ) -> PyResult<f64> {
        let date = py_to_date(as_of).context("as_of")?;
        let pricer = match &self.config {
            Some(c) => RustTreePricer::with_config(c.clone()),
            None => RustTreePricer::new(),
        };
        pricer
            .calculate_oas(&bond.inner, &market.inner, date, clean_price_pct)
            .map_err(core_to_py)
    }

    /// Price a bond at a given OAS using the short-rate tree.
    ///
    /// Builds a short-rate tree calibrated to the discount curve and
    /// performs backward induction with the specified OAS applied as
    /// a parallel shift.
    ///
    /// Parameters
    /// ----------
    /// bond : Bond
    ///     Bond instrument.
    /// market : MarketContext
    ///     Market data.
    /// as_of : datetime.date
    ///     Valuation date.
    /// oas_decimal : float
    ///     OAS in decimal form (e.g., 0.015 = 150 bp).
    ///
    /// Returns
    /// -------
    /// float
    ///     Dirty price in currency units.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the discount curve is missing or tree calibration fails.
    #[pyo3(signature = (bond, market, as_of, oas_decimal))]
    #[staticmethod]
    fn price_from_oas(
        bond: &PyBond,
        market: &PyMarketContext,
        as_of: &Bound<'_, pyo3::types::PyAny>,
        oas_decimal: f64,
    ) -> PyResult<f64> {
        let date = py_to_date(as_of).context("as_of")?;
        finstack_valuations::instruments::fixed_income::bond::pricing::quote_conversions::price_from_oas(
            &bond.inner,
            &market.inner,
            date,
            oas_decimal,
        )
        .map_err(core_to_py)
    }

    fn __repr__(&self) -> String {
        "TreePricer(...)".to_string()
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyTreeModelChoice>()?;
    module.add_class::<PyTreePricerConfig>()?;
    module.add_class::<PyTreePricer>()?;
    Ok(vec!["TreeModelChoice", "TreePricerConfig", "TreePricer"])
}
