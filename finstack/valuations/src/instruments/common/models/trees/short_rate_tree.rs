//! Short-rate tree models for bond valuation with embedded options.
//!
//! Implements curve-consistent short-rate trees for pricing callable/putable bonds
//! and calculating Option-Adjusted Spread (OAS). Uses industry-standard models
//! like Ho-Lee and Black-Derman-Toy.
//!
//! # Volatility Conventions
//!
//! **Critical**: The volatility parameter interpretation depends on the model type:
//!
//! | Model | Vol Type | Parameter | Formula | Typical Range |
//! |-------|----------|-----------|---------|---------------|
//! | Ho-Lee | Normal/Absolute | σ (bps/yr) | dr = θdt + σdW | 50-150 bps (0.005-0.015) |
//! | BDT | Lognormal/Relative | σ (%) | dr/r = θdt + σdW | 15-30% (0.15-0.30) |
//!
//! ## Converting Between Conventions
//!
//! Use `finstack_core::math::volatility::convert_atm_volatility` to convert:
//!
//! ```rust,no_run
//! use finstack_core::math::volatility::{convert_atm_volatility, VolatilityConvention};
//!
//! let normal_vol = 0.01;
//! let rate_level = 0.05;
//!
//! let lognormal_vol = convert_atm_volatility(
//!     normal_vol,
//!     VolatilityConvention::Normal,
//!     VolatilityConvention::Lognormal,
//!     rate_level,
//!     1.0,
//! )?;
//! assert!(lognormal_vol > 0.15 && lognormal_vol < 0.25);
//!
//! let back_to_normal = convert_atm_volatility(
//!     lognormal_vol,
//!     VolatilityConvention::Lognormal,
//!     VolatilityConvention::Normal,
//!     rate_level,
//!     1.0,
//! )?;
//! assert!((back_to_normal - normal_vol).abs() < 1e-10);
//! # Ok::<(), finstack_core::Error>(())
//! ```
//!
//! ## Calibration Sources
//!
//! - **Swaption market**: ATM swaption vols are typically quoted in normal (bps)
//! - **Cap/floor market**: Often quoted in lognormal (Black vol)
//! - **Historical**: Calculate from rate time series

use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::traits::Discounting;
use finstack_core::types::CurveId;
use finstack_core::{Error, Result};

use super::tree_framework::{
    price_recombining_tree, state_keys, RecombiningInputs, StateGenerator, StateVariables,
    TreeBranching, TreeGreeks, TreeModel, TreeValuator,
};

/// Default normal (absolute) volatility for Ho-Lee model.
///
/// 100 basis points per year, typical for developed market government bonds
/// in a normal rate environment (2-5% rates).
pub const DEFAULT_NORMAL_VOL: f64 = 0.01; // 100 bps/yr

/// Default lognormal (relative) volatility for Black-Derman-Toy model.
///
/// 20% annualized, typical for developed market government bonds.
/// This corresponds to ~100 bps normal vol at a 5% rate level.
pub const DEFAULT_LOGNORMAL_VOL: f64 = 0.20; // 20%

// ============================================================================
// Short-Rate Model Types
// ============================================================================

/// Compounding convention for per-node discount factors in the short-rate tree.
///
/// | Convention | Formula | Use Case |
/// |------------|---------|----------|
/// | `Continuous` | `exp(-r * dt)` | Default; matches continuous short-rate dynamics |
/// | `Simple` | `1 / (1 + r * dt)` | Money-market / Bloomberg BDT convention |
/// | `SemiAnnual` | `(1 + r/2)^(-2 * dt)` | US bond market convention |
/// | `Quarterly` | `(1 + r/4)^(-4 * dt)` | Quarterly compounding |
/// | `Monthly` | `(1 + r/12)^(-12 * dt)` | Monthly compounding |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TreeCompounding {
    /// Continuous compounding: `df = exp(-r * dt)`.
    #[default]
    Continuous,
    /// Simple (money-market) compounding: `df = 1 / (1 + r * dt)`.
    Simple,
    /// Semi-annual compounding: `df = (1 + r/2)^(-2 * dt)`.
    SemiAnnual,
    /// Quarterly compounding: `df = (1 + r/4)^(-4 * dt)`.
    Quarterly,
    /// Monthly compounding: `df = (1 + r/12)^(-12 * dt)`.
    Monthly,
}

impl TreeCompounding {
    /// Compute the per-step discount factor for a given rate and time step.
    #[inline]
    pub fn df(self, rate: f64, dt: f64) -> f64 {
        match self {
            Self::Continuous => (-rate * dt).exp(),
            Self::Simple => 1.0 / (1.0 + rate * dt),
            Self::SemiAnnual => (1.0 + rate / 2.0).powf(-2.0 * dt),
            Self::Quarterly => (1.0 + rate / 4.0).powf(-4.0 * dt),
            Self::Monthly => (1.0 + rate / 12.0).powf(-12.0 * dt),
        }
    }

    /// Convert a rate under this convention to the equivalent continuous rate.
    ///
    /// Returns `r_cont` such that `exp(-r_cont * dt) = self.df(rate, dt)`.
    #[inline]
    pub fn to_continuous(self, rate: f64, dt: f64) -> f64 {
        if dt.abs() < f64::EPSILON {
            return rate;
        }
        let d = self.df(rate, dt);
        if d > 0.0 { -d.ln() / dt } else { rate }
    }
}

/// Short-rate tree model types.
///
/// Each model has distinct volatility conventions and mathematical properties:
///
/// | Model | Vol Type | Negative Rates | Mean Reversion | Use Case |
/// |-------|----------|----------------|----------------|----------|
/// | Ho-Lee | Normal | ✅ Yes | ❌ No | Low/negative rate environments |
/// | BDT | Lognormal | ❌ No | Not currently applied | Traditional positive rate environments |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortRateModel {
    /// Ho-Lee model: Gaussian/normal short rates.
    ///
    /// ## Rate Dynamics
    /// ```text
    /// dr = θ(t)dt + σdW
    /// ```
    /// where:
    /// - `θ(t)` is calibrated to match the discount curve
    /// - `σ` is the **normal volatility** (absolute, in rate units like 0.01 = 100 bps)
    ///
    /// ## Properties
    /// - ✅ Handles negative rates naturally
    /// - ❌ No mean reversion (rates can drift arbitrarily)
    /// - Analytically tractable
    ///
    /// ## Typical Volatility Range
    /// - Low rates (<2%): 50-80 bps (0.005-0.008)
    /// - Normal rates (2-5%): 80-120 bps (0.008-0.012)
    /// - High rates (>5%): 100-150 bps (0.010-0.015)
    /// - Crisis: 150-300 bps (0.015-0.030)
    HoLee,

    /// Black-Derman-Toy / Black-Karasinski model: Lognormal short rates.
    ///
    /// ## Rate Dynamics
    /// ```text
    /// d(ln r) = [θ(t) - κ ln r] dt + σ dW
    /// ```
    /// where:
    /// - `θ(t)` is calibrated to match the discount curve
    /// - `σ` is the **lognormal volatility** (relative, like 0.20 = 20%)
    /// - `κ` is the mean reversion speed (0 recovers standard BDT)
    ///
    /// ## Properties
    /// - ❌ Cannot handle negative rates (rates stay positive)
    /// - When `κ = 0`: standard BDT with constant lognormal volatility
    /// - When `κ > 0`: Black-Karasinski extension; rate dispersion is
    ///   tightened via the integrated variance `σ²(1-e^{-2κΔt})/(2κ)`
    /// - Lognormal distribution matches cap/floor market conventions
    ///
    /// ## Typical Volatility Range
    /// - Low vol environment: 10-15% (0.10-0.15)
    /// - Normal market: 15-25% (0.15-0.25)
    /// - High vol/stress: 25-40% (0.25-0.40)
    ///
    /// ## Important
    /// ⚠️ The default 1% volatility in older code is **far too low** for BDT.
    /// Use [`DEFAULT_LOGNORMAL_VOL`] (20%) or calibrate to swaption market.
    BlackDermanToy,
}

/// Configuration for short-rate tree construction.
///
/// # Volatility Convention
///
/// ⚠️ **Critical**: The `volatility` field has different interpretations depending on the model:
///
/// | Model | Volatility Type | Example |
/// |-------|-----------------|---------|
/// | [`ShortRateModel::HoLee`] | Normal (absolute) | 0.01 = 100 bps/yr |
/// | [`ShortRateModel::BlackDermanToy`] | Lognormal (relative) | 0.20 = 20%/yr |
///
/// Use the helper constructors ([`ShortRateTreeConfig::ho_lee`], [`ShortRateTreeConfig::bdt`])
/// or `finstack_core::math::volatility::convert_atm_volatility` to avoid convention errors.
///
/// # Examples
///
/// ```rust,ignore
/// use finstack_valuations::instruments::models::trees::short_rate_tree::{
///     ShortRateTreeConfig, ShortRateModel, DEFAULT_NORMAL_VOL, DEFAULT_LOGNORMAL_VOL,
/// };
///
/// // Ho-Lee with 100 bps normal vol (recommended for negative rate environments)
/// let ho_lee = ShortRateTreeConfig::ho_lee(100, 0.01);
/// assert_eq!(ho_lee.model, ShortRateModel::HoLee);
///
/// // BDT with 20% lognormal vol (recommended for positive rate environments)
/// let bdt = ShortRateTreeConfig::bdt(100, 0.20, 0.03);
/// assert_eq!(bdt.model, ShortRateModel::BlackDermanToy);
///
/// // Use defaults with model-appropriate volatility
/// let ho_lee_default = ShortRateTreeConfig::default_ho_lee(100);
/// assert_eq!(ho_lee_default.volatility, DEFAULT_NORMAL_VOL);
///
/// let bdt_default = ShortRateTreeConfig::default_bdt(100);
/// assert_eq!(bdt_default.volatility, DEFAULT_LOGNORMAL_VOL);
/// ```
#[derive(Debug, Clone)]
pub struct ShortRateTreeConfig {
    /// Number of time steps in the tree.
    ///
    /// More steps improve accuracy but increase computation time O(n²).
    /// Typical values: 50 (fast), 100 (standard), 200+ (high precision).
    pub steps: usize,

    /// Tree model type determining rate dynamics and volatility interpretation.
    pub model: ShortRateModel,

    /// Interest rate volatility (annualized).
    ///
    /// ⚠️ **Interpretation depends on model**:
    /// - **Ho-Lee**: Normal volatility in rate units (0.01 = 100 bps/yr)
    /// - **BDT**: Lognormal volatility as proportion (0.20 = 20%/yr)
    ///
    /// See [`ShortRateModel`] for typical ranges per model type.
    pub volatility: f64,

    /// Mean reversion parameter.
    ///
    /// Controls how quickly rates revert to the long-term mean.
    /// - Typical values: 0.01-0.10 (1-10% per year)
    /// - Higher values = faster reversion, less rate dispersion
    /// - Ho-Lee/Hull-White: explicit mean reversion in the drift
    /// - BDT/Black-Karasinski: tightens the per-step lognormal spread
    ///   via integrated variance; 0 recovers standard BDT
    pub mean_reversion: Option<f64>,

    /// Tree branching type (binomial or trinomial).
    ///
    /// - **Binomial**: Standard two-branch tree (up/down)
    /// - **Trinomial**: Three-branch tree (up/mid/down) for models with
    ///   trinomial calibration support
    ///
    /// Default: Binomial. Use trinomial only with a matching calibrated lattice.
    pub branching: TreeBranching,

    /// Per-node discount factor convention.
    ///
    /// Controls whether calibration and pricing use continuous `exp(-r*dt)` or
    /// simple `1/(1+r*dt)` compounding. Bloomberg's lognormal OAS model uses
    /// simple compounding; the default is continuous for backward compatibility.
    pub compounding: TreeCompounding,
}

impl Default for ShortRateTreeConfig {
    /// Default configuration using Ho-Lee model with appropriate normal volatility.
    ///
    /// For BDT model, use [`ShortRateTreeConfig::default_bdt`] instead.
    fn default() -> Self {
        Self::default_ho_lee(100)
    }
}

impl ShortRateTreeConfig {
    /// Create a Ho-Lee configuration with specified normal volatility.
    ///
    /// Uses binomial branching by default. For trinomial branching,
    /// use [`with_trinomial`](Self::with_trinomial) after construction.
    ///
    /// # Arguments
    ///
    /// * `steps` - Number of tree steps (50-200 typical)
    /// * `normal_vol` - Normal volatility in rate units (e.g., 0.01 = 100 bps/yr)
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use finstack_valuations::instruments::models::trees::short_rate_tree::ShortRateTreeConfig;
    ///
    /// // 100 steps, 80 bps normal vol
    /// let config = ShortRateTreeConfig::ho_lee(100, 0.008);
    /// ```
    pub fn ho_lee(steps: usize, normal_vol: f64) -> Self {
        Self {
            steps,
            model: ShortRateModel::HoLee,
            volatility: normal_vol,
            mean_reversion: None,
            branching: TreeBranching::Binomial,
            compounding: TreeCompounding::default(),
        }
    }

    /// Create a Black-Derman-Toy / Black-Karasinski configuration.
    ///
    /// Uses binomial branching with state-price recursion calibration.
    ///
    /// # Arguments
    ///
    /// * `steps` - Number of tree steps (50-200 typical)
    /// * `lognormal_vol` - Lognormal volatility (e.g., 0.20 = 20%/yr)
    /// * `mean_reversion` - Mean reversion speed (0.0 = standard BDT, 0.03 = Bloomberg default)
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use finstack_valuations::instruments::models::trees::short_rate_tree::ShortRateTreeConfig;
    ///
    /// // 100 steps, 20% lognormal vol
    /// let config = ShortRateTreeConfig::bdt(100, 0.20, 0.0);
    /// ```
    pub fn bdt(steps: usize, lognormal_vol: f64, mean_reversion: f64) -> Self {
        Self {
            steps,
            model: ShortRateModel::BlackDermanToy,
            volatility: lognormal_vol,
            mean_reversion: Some(mean_reversion),
            branching: TreeBranching::Binomial,
            compounding: TreeCompounding::default(),
        }
    }

    /// Set the per-node compounding convention.
    #[must_use]
    pub fn with_compounding(mut self, compounding: TreeCompounding) -> Self {
        self.compounding = compounding;
        self
    }

    /// Create Ho-Lee configuration with default normal volatility (100 bps).
    ///
    /// Suitable for developed market government bonds in normal rate environments.
    pub fn default_ho_lee(steps: usize) -> Self {
        Self::ho_lee(steps, DEFAULT_NORMAL_VOL)
    }

    /// Create BDT configuration with default lognormal volatility (20%).
    ///
    /// Suitable for developed market government bonds with positive rates.
    /// Uses the current non-mean-reverting binomial BDT calibration.
    pub fn default_bdt(steps: usize) -> Self {
        Self::bdt(steps, DEFAULT_LOGNORMAL_VOL, 0.0)
    }

    /// Set trinomial branching.
    ///
    /// The selected model must calibrate a matching `2 * step + 1` lattice.
    #[must_use]
    pub fn with_trinomial(mut self) -> Self {
        self.branching = TreeBranching::Trinomial;
        self
    }

    /// Set binomial branching (standard two-branch tree).
    #[must_use]
    pub fn with_binomial(mut self) -> Self {
        self.branching = TreeBranching::Binomial;
        self
    }

    /// Create configuration from normal volatility, automatically selecting
    /// the appropriate model based on rate environment.
    ///
    /// # Arguments
    ///
    /// * `steps` - Number of tree steps
    /// * `normal_vol` - Normal volatility in rate units (e.g., 0.01 = 100 bps)
    /// * `rate_level` - Current/reference rate level for model selection
    ///
    /// # Model Selection
    ///
    /// - If `rate_level < 0.01` (1%): Uses Ho-Lee (handles negative rates)
    /// - Otherwise: Uses BDT with converted lognormal vol
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use finstack_valuations::instruments::models::trees::short_rate_tree::{
    ///     ShortRateTreeConfig, ShortRateModel,
    /// };
    ///
    /// // Low rate environment → Ho-Lee
    /// let config = ShortRateTreeConfig::from_normal_vol(100, 0.008, 0.005)?;
    /// assert_eq!(config.model, ShortRateModel::HoLee);
    ///
    /// // Normal rate environment → BDT with converted vol
    /// let config = ShortRateTreeConfig::from_normal_vol(100, 0.01, 0.05)?;
    /// assert_eq!(config.model, ShortRateModel::BlackDermanToy);
    /// // Vol should be approximately 20% (price-matching conversion)
    /// assert!(config.volatility > 0.15 && config.volatility < 0.25);
    /// # Ok::<(), finstack_core::Error>(())
    /// ```
    pub fn from_normal_vol(steps: usize, normal_vol: f64, rate_level: f64) -> Result<Self> {
        if rate_level < 0.01 {
            // Low/negative rate environment: use Ho-Lee
            Ok(Self::ho_lee(steps, normal_vol))
        } else {
            // Positive rate environment: use BDT with converted vol
            let lognormal_vol = finstack_core::math::volatility::convert_atm_volatility(
                normal_vol,
                finstack_core::math::volatility::VolatilityConvention::Normal,
                finstack_core::math::volatility::VolatilityConvention::Lognormal,
                rate_level,
                1.0,
            )?;
            Ok(Self::bdt(steps, lognormal_vol, 0.0))
        }
    }
}

use std::sync::Arc;

/// Result of short-rate tree calibration with quality metrics.
///
/// Provides diagnostic information about calibration quality, allowing
/// users to assess whether the tree is suitable for their use case.
#[derive(Debug, Clone, Default)]
pub struct CalibrationResult {
    /// Maximum calibration error in basis points.
    pub max_error_bps: f64,
    /// Step at which maximum error occurred.
    pub max_error_step: usize,
    /// Number of steps where the solver failed and fallback was used.
    pub fallback_count: usize,
    /// Whether calibration completed successfully.
    pub converged: bool,
}

impl CalibrationResult {
    /// Returns true if calibration quality is acceptable (max error < 1bp, no fallbacks).
    #[must_use]
    pub fn is_acceptable(&self) -> bool {
        self.converged && self.max_error_bps < 1.0 && self.fallback_count == 0
    }

    /// Returns true if calibration quality is good (max error < 0.1bp).
    #[must_use]
    pub fn is_good(&self) -> bool {
        self.converged && self.max_error_bps < 0.1 && self.fallback_count == 0
    }
}

/// Short-rate tree for valuing bonds with embedded options
#[derive(Debug, Clone)]
pub struct ShortRateTree {
    config: ShortRateTreeConfig,
    /// Calibrated short rates at each node: rates[step][node]
    rates: Arc<Vec<Vec<f64>>>,
    /// Transition probabilities: probs[step] gives (p_up, p_down) for that step
    probs: Vec<(f64, f64)>,
    /// Time steps in years
    time_steps: Vec<f64>,
    /// Discount curve used for calibration
    calibration_curve_id: CurveId,
    /// Calibration quality metrics (populated after calibration).
    calibration_quality: Option<CalibrationResult>,
}

impl ShortRateTree {
    /// Create a new short-rate tree with the given configuration.
    pub fn new(config: ShortRateTreeConfig) -> Self {
        Self {
            config,
            rates: Arc::new(Vec::new()),
            probs: Vec::new(),
            time_steps: Vec::new(),
            calibration_curve_id: CurveId::new(""),
            calibration_quality: None,
        }
    }

    /// Returns the calibration result if calibration has been performed.
    ///
    /// # Returns
    ///
    /// - `Some(CalibrationResult)` with quality metrics if calibrated
    /// - `None` if not yet calibrated
    #[must_use]
    pub fn calibration_result(&self) -> Option<&CalibrationResult> {
        self.calibration_quality.as_ref()
    }

    /// Create a Ho-Lee tree with specified normal (absolute) volatility.
    ///
    /// # Arguments
    ///
    /// * `steps` - Number of tree steps (50-200 typical)
    /// * `normal_vol` - Normal volatility in rate units (e.g., 0.01 = 100 bps/yr)
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use finstack_valuations::instruments::models::trees::short_rate_tree::ShortRateTree;
    ///
    /// // Ho-Lee with 100 bps annual volatility
    /// let tree = ShortRateTree::ho_lee(100, 0.01);
    /// ```
    pub fn ho_lee(steps: usize, normal_vol: f64) -> Self {
        Self::new(ShortRateTreeConfig::ho_lee(steps, normal_vol))
    }

    /// Create a Black-Derman-Toy tree with specified lognormal (relative) volatility.
    ///
    /// # Arguments
    ///
    /// * `steps` - Number of tree steps (50-200 typical)
    /// * `lognormal_vol` - Lognormal volatility (e.g., 0.20 = 20%/yr)
    /// * `mean_reversion` - Must be zero for the current non-mean-reverting BDT calibration
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use finstack_valuations::instruments::models::trees::short_rate_tree::ShortRateTree;
    ///
    /// // BDT with 20% lognormal volatility
    /// let tree = ShortRateTree::black_derman_toy(100, 0.20, 0.0);
    /// ```
    ///
    /// # Warning
    ///
    /// ⚠️ The volatility parameter is **lognormal** (relative), not normal (absolute).
    /// A value of 0.20 means 20% annual rate volatility, not 20 bps.
    /// Use `finstack_core::math::volatility::convert_atm_volatility` to convert from normal if needed.
    pub fn black_derman_toy(steps: usize, lognormal_vol: f64, mean_reversion: f64) -> Self {
        Self::new(ShortRateTreeConfig::bdt(
            steps,
            lognormal_vol,
            mean_reversion,
        ))
    }

    /// Create a Ho-Lee tree with default normal volatility (100 bps).
    pub fn default_ho_lee(steps: usize) -> Self {
        Self::new(ShortRateTreeConfig::default_ho_lee(steps))
    }

    /// Create a BDT tree with default lognormal volatility (20%).
    pub fn default_bdt(steps: usize) -> Self {
        Self::new(ShortRateTreeConfig::default_bdt(steps))
    }

    /// Calibrate the tree to match a given discount curve
    pub fn calibrate(
        &mut self,
        discount_curve: &dyn Discounting,
        time_to_maturity: f64,
    ) -> Result<()> {
        self.calibration_curve_id = CurveId::new("CALIBRATED");

        // Build time grid
        let dt = time_to_maturity / self.config.steps as f64;
        self.time_steps = (0..=self.config.steps).map(|i| i as f64 * dt).collect();

        // Initialize data structures
        let mut rates = vec![Vec::new(); self.config.steps + 1];
        self.probs = vec![(0.5, 0.5); self.config.steps]; // Default to equal probabilities

        match self.config.model {
            ShortRateModel::HoLee => self.calibrate_ho_lee(&mut rates, discount_curve, dt)?,
            ShortRateModel::BlackDermanToy => self.calibrate_bdt(&mut rates, discount_curve, dt)?,
        }

        self.rates = Arc::new(rates);

        Ok(())
    }

    /// Calibrate Ho-Lee model parameters
    fn calibrate_ho_lee(
        &mut self,
        rates: &mut [Vec<f64>],
        discount_curve: &dyn Discounting,
        dt: f64,
    ) -> Result<()> {
        let sigma = self.config.volatility;

        // Initialize first step with current short rate
        // r0 should match P(0, T1) = exp(-r0 * T1)
        // r0 = -ln(P(0, T1)) / T1
        let r0 = if self.time_steps[1] > 0.0 {
            -discount_curve.df(self.time_steps[1]).ln() / self.time_steps[1]
        } else {
            0.03 // Fallback rate
        };

        rates[0] = vec![r0];

        // State prices (Arrow-Debreu prices) for the current step
        let mut state_prices = vec![1.0]; // Q[0] = 1.0

        // Build tree forward
        for step in 0..self.config.steps {
            // We are at step i (time t_i). We have rates[i] and state_prices[i].
            // We want to determine rates[i+1] such that the bond price P(0, t_{i+2}) matches market.
            // Note: rates[i] determines discounting from t_{i+1} to t_i.
            // Wait, rates[i] applies to [t_i, t_{i+1}].
            // So rates[i] determines P(0, t_{i+1}) given P(0, t_i).
            // But rates[i] is already fixed!
            // We are determining rates[i+1] which applies to [t_{i+1}, t_{i+2}].
            // So we calibrate rates[i+1] to match P(0, t_{i+2}).

            let next_next_time = if step + 2 < self.time_steps.len() {
                self.time_steps[step + 2]
            } else {
                // End of tree, no need to calibrate further rates (they won't be used for discounting)
                // But we still need to populate the vector to avoid index errors
                0.0
            };

            let next_nodes = step + 2;
            let mut next_rates_base = vec![0.0; next_nodes];
            let mut next_state_prices = vec![0.0; next_nodes];

            // 1. Calculate next state prices and base rates (without theta)
            //
            // Hull-White extension: when mean_reversion `a` is set, the drift
            // includes a pull toward zero: dr = [theta(t) - a*r] dt + sigma dW
            // This reduces rate dispersion at long maturities compared to Ho-Lee.
            let mr_drift = self.config.mean_reversion.unwrap_or(0.0);
            for (i, &current_rate) in rates[step].iter().enumerate() {
                let q = state_prices[i];
                let df = (-current_rate * dt).exp();
                let mean_rev_adj = mr_drift * current_rate * dt;

                // Up move (to i+1)
                let r_up_base = current_rate + sigma * dt.sqrt() - mean_rev_adj;
                if i + 1 < next_nodes {
                    next_rates_base[i + 1] = r_up_base;
                    next_state_prices[i + 1] += q * df * 0.5;
                }

                // Down move (to i)
                let r_down_base = current_rate - sigma * dt.sqrt() - mean_rev_adj;
                if i < next_nodes {
                    next_rates_base[i] = r_down_base;
                    next_state_prices[i] += q * df * 0.5;
                }
            }

            // 2. Solve for theta (drift adjustment to match discount curve)
            //
            // Ho-Lee calibration: r_next[j] = r_base[j] + θ
            // Discount factor: exp(-r_next[j] * dt) = exp(-(r_base[j] + θ) * dt)
            //                = exp(-r_base[j]*dt) * exp(-θ*dt)
            // Model price: P_model = Σ Q_next[j] * exp(-r_next[j] * dt)
            //            = exp(-θ*dt) * Σ Q_next[j] * exp(-r_base[j]*dt)
            //            = exp(-θ*dt) * P_model_base
            // Target: P_target = exp(-θ*dt) * P_model_base
            // ⇒ θ = -ln(P_target / P_model_base) / dt
            let theta = if next_next_time > 0.0 {
                let p_target = discount_curve.df(next_next_time);
                let mut p_model_base = 0.0;
                for (j, &q_next) in next_state_prices.iter().enumerate() {
                    let r_base = next_rates_base[j];
                    // Discount from t_{i+2} to t_{i+1} using r_{i+1}
                    p_model_base += q_next * (-r_base * dt).exp();
                }

                if p_model_base > 0.0 && p_target > 0.0 {
                    -(p_target / p_model_base).ln() / dt
                } else {
                    0.0
                }
            } else {
                0.0
            };

            // 3. Apply theta directly to get final rates (θ is the rate adjustment)
            let mut next_rates = vec![0.0; next_nodes];
            for j in 0..next_nodes {
                next_rates[j] = next_rates_base[j] + theta;
            }

            rates[step + 1] = next_rates;
            state_prices = next_state_prices;
        }

        // Measure actual calibration error (floating-point accumulation)
        let mut max_error_bps = 0.0_f64;
        let mut max_error_step = 0_usize;
        {
            let mut q = vec![1.0_f64]; // Arrow-Debreu prices
            for (step, rates_step) in rates.iter().enumerate().take(self.config.steps) {
                let next_nodes = step + 2;
                let mut next_q = vec![0.0; next_nodes];
                for (i, &rate_i) in rates_step.iter().enumerate() {
                    let df_i = (-rate_i * dt).exp();
                    if i + 1 < next_nodes {
                        next_q[i + 1] += q[i] * df_i * 0.5;
                    }
                    if i < next_nodes {
                        next_q[i] += q[i] * df_i * 0.5;
                    }
                }
                let model_df: f64 = next_q.iter().sum();
                let t_next = self.time_steps[step + 1];
                let target_df = discount_curve.df(t_next);
                if target_df > 0.0 {
                    let err = ((model_df - target_df) / target_df).abs() * 10_000.0;
                    if err > max_error_bps {
                        max_error_bps = err;
                        max_error_step = step;
                    }
                }
                q = next_q;
            }
        }

        self.calibration_quality = Some(CalibrationResult {
            max_error_bps,
            max_error_step,
            fallback_count: 0,
            converged: true,
        });

        Ok(())
    }

    /// Calibrate Black-Derman-Toy / Black-Karasinski model using state-price recursion.
    ///
    /// When `mean_reversion` is zero, this is standard BDT with constant lognormal
    /// volatility. When positive, it extends to Black-Karasinski: the per-step
    /// lognormal spread uses the integrated variance `σ² (1 - e^{-2κΔt}) / (2κ)`
    /// instead of `σ²Δt`, tightening the rate distribution at longer horizons.
    ///
    /// Bloomberg's OAS1 "L=Lognormal" model defaults to κ = 0.03.
    fn calibrate_bdt(
        &mut self,
        rates: &mut [Vec<f64>],
        discount_curve: &dyn Discounting,
        dt: f64,
    ) -> Result<()> {
        use finstack_core::math::{BrentSolver, Solver};

        let sigma = self.config.volatility;
        let kappa = self.config.mean_reversion.unwrap_or(0.0);
        let solver = BrentSolver::new();

        // Black-Karasinski / BDT: lognormal rates with optional mean reversion.
        // The up multiplier uses the integrated lognormal standard deviation
        // per step. For κ = 0 this reduces to σ√dt (standard BDT).
        let step_vol = if kappa.abs() < 1e-12 {
            sigma * dt.sqrt()
        } else {
            sigma * ((1.0 - (-2.0 * kappa * dt).exp()) / (2.0 * kappa)).sqrt()
        };
        let u = step_vol.exp();
        let p = 0.5;

        // Bounds for alpha solver (reasonable rate range: 0bp to 5000bp = 50%)
        let alpha_lb = 1e-6;
        let alpha_ub = 0.50;

        // Initialize first step with initial short rate
        let r0 = if self.time_steps[1] > 0.0 {
            // Use initial forward rate from discount curve
            -discount_curve.df(self.time_steps[1]).ln() / self.time_steps[1]
        } else {
            0.03 // Fallback rate
        };

        rates[0] = vec![r0.clamp(alpha_lb, alpha_ub)]; // Ensure within bounds
        let mut state_prices = vec![vec![1.0]]; // Q[0] = [1.0]

        // Set transition probabilities (constant for BDT)
        for i in 0..self.config.steps {
            self.probs[i] = (p, 1.0 - p);
        }

        // Track calibration quality for diagnostics
        let mut max_error_bps = 0.0_f64;
        let mut max_error_step = 0_usize;
        let mut fallback_count = 0_usize;

        // Build tree forward, calibrating drift at each step
        for step in 0..self.config.steps {
            let current_time = self.time_steps[step + 1];
            let target_df = discount_curve.df(current_time);

            if target_df <= 0.0 {
                return Err(Error::Validation(format!(
                    "BDT calibration: non-positive discount factor {} at time {}",
                    target_df, current_time
                )));
            }

            let num_nodes = step + 1;
            let current_state_prices = &state_prices[step];
            let current_rates = &rates[step];

            // Solve for drift parameter alpha such that model ZCB price matches market
            let comp = self.config.compounding;
            let objective = |alpha: f64| -> f64 {
                let mut model_price = 0.0;

                for (j, &state_price) in current_state_prices.iter().enumerate().take(num_nodes) {
                    let rate = alpha * u.powf(num_nodes as f64 - 1.0 - 2.0 * j as f64);
                    let rate_clamped = rate.clamp(alpha_lb, alpha_ub);
                    model_price += state_price * comp.df(rate_clamped, dt);
                }

                model_price - target_df
            };

            // Initial guess for alpha based on previous step or forward rate
            let initial_alpha = if step == 0 {
                r0.clamp(alpha_lb, alpha_ub)
            } else {
                // Use geometric mean of previous step rates as initial guess
                let mean_rate =
                    current_rates.iter().map(|&r| r.ln()).sum::<f64>() / current_rates.len() as f64;
                mean_rate.exp().clamp(alpha_lb, alpha_ub)
            };

            // Solve for alpha with convergence tracking
            let (alpha, used_fallback) = match solver.solve(objective, initial_alpha) {
                Ok(a) => (a.clamp(alpha_lb, alpha_ub), false),
                Err(_) => {
                    // Solver failed - use fallback based on market rate
                    let market_rate = if current_time > 0.0 {
                        -target_df.ln() / current_time
                    } else {
                        0.03
                    };
                    fallback_count += 1;
                    (market_rate.clamp(alpha_lb, alpha_ub), true)
                }
            };

            let current_step_rates: Vec<f64> = (0..num_nodes)
                .map(|j| {
                    let rate = alpha * u.powf(num_nodes as f64 - 1.0 - 2.0 * j as f64);
                    rate.clamp(alpha_lb, alpha_ub)
                })
                .collect();
            rates[step] = current_step_rates.clone();

            let model_df = {
                let mut model_price = 0.0;
                for (j, &state_price) in current_state_prices.iter().enumerate().take(num_nodes) {
                    model_price += state_price * comp.df(current_step_rates[j], dt);
                }
                model_price
            };
            let error_bps = ((model_df - target_df) / target_df).abs() * 10000.0;

            if error_bps > max_error_bps {
                max_error_bps = error_bps;
                max_error_step = step;
            }

            // Log warning if calibration error is significant (>1bp) or fallback was used
            if error_bps > 1.0 || used_fallback {
                tracing::warn!(
                    "BDT calibration step {}: error={:.2}bp, target_df={:.6}, model_df={:.6}{}",
                    step,
                    error_bps,
                    target_df,
                    model_df,
                    if used_fallback {
                        " (FALLBACK USED)"
                    } else {
                        ""
                    }
                );
            }

            // Build next step rates using calibrated alpha
            let next_nodes = num_nodes + 1;
            let mut next_rates = vec![0.0; next_nodes];
            let mut next_state_prices = vec![0.0; next_nodes];

            for (j, &state_price) in current_state_prices.iter().enumerate().take(num_nodes) {
                let discount_factor = comp.df(current_step_rates[j], dt);
                let state_price_contribution = state_price * discount_factor;

                // Up move: j -> j+1
                if j + 1 < next_nodes {
                    let up_rate = alpha * u.powf(next_nodes as f64 - 1.0 - 2.0 * (j + 1) as f64);
                    next_rates[j + 1] = up_rate.clamp(alpha_lb, alpha_ub);
                    next_state_prices[j + 1] += state_price_contribution * p;
                }

                // Down move: j -> j
                if j < next_nodes {
                    let down_rate = alpha * u.powf(next_nodes as f64 - 1.0 - 2.0 * j as f64);
                    next_rates[j] = down_rate.clamp(alpha_lb, alpha_ub);
                    next_state_prices[j] += state_price_contribution * (1.0 - p);
                }
            }

            rates[step + 1] = next_rates;
            state_prices.push(next_state_prices);
        }

        // Log calibration summary
        if max_error_bps > 1.0 || fallback_count > 0 {
            tracing::warn!(
                "BDT calibration completed: max error={:.2}bp at step {}, fallbacks={} (target: <1bp, 0 fallbacks)",
                max_error_bps,
                max_error_step,
                fallback_count
            );
        } else {
            tracing::debug!(
                "BDT calibration completed: max error={:.4}bp at step {}",
                max_error_bps,
                max_error_step
            );
        }

        // Store calibration result for user inspection
        self.calibration_quality = Some(CalibrationResult {
            max_error_bps,
            max_error_step,
            fallback_count,
            converged: true,
        });

        Ok(())
    }

    /// Get the short rate at a specific node
    pub fn rate_at_node(&self, step: usize, node: usize) -> Result<f64> {
        if step >= self.rates.len() || node >= self.rates[step].len() {
            return Err(Error::internal(format!(
                "short-rate tree node out of bounds: step={step}, node={node}"
            )));
        }
        Ok(self.rates[step][node])
    }

    /// Get transition probabilities at a step
    pub fn probabilities(&self, step: usize) -> Result<(f64, f64)> {
        if step >= self.probs.len() {
            return Err(Error::internal(format!(
                "short-rate tree probability row out of bounds: step={step}"
            )));
        }
        Ok(self.probs[step])
    }

    /// Get time at step
    pub fn time_at_step(&self, step: usize) -> Result<f64> {
        if step >= self.time_steps.len() {
            return Err(Error::internal(format!(
                "short-rate tree time step out of bounds: step={step}"
            )));
        }
        Ok(self.time_steps[step])
    }

    fn expected_nodes_at_step(branching: TreeBranching, step: usize) -> usize {
        match branching {
            TreeBranching::Binomial => step + 1,
            TreeBranching::Trinomial => 2 * step + 1,
        }
    }

    fn validate_lattice_geometry(&self) -> Result<()> {
        if self.rates.len() != self.config.steps + 1 {
            return Err(Error::internal(format!(
                "short-rate tree lattice geometry mismatch: expected {} rate rows, got {}",
                self.config.steps + 1,
                self.rates.len()
            )));
        }

        for (step, rates_at_step) in self.rates.iter().enumerate() {
            let expected = Self::expected_nodes_at_step(self.config.branching, step);
            if rates_at_step.len() != expected {
                return Err(Error::internal(format!(
                    "short-rate tree lattice geometry mismatch for {:?}: step {} expected {} nodes, got {}",
                    self.config.branching,
                    step,
                    expected,
                    rates_at_step.len()
                )));
            }
        }

        Ok(())
    }
}

impl TreeModel for ShortRateTree {
    fn price<V: TreeValuator>(
        &self,
        mut initial_vars: StateVariables,
        time_to_maturity: f64,
        market_context: &MarketContext,
        valuator: &V,
    ) -> Result<f64> {
        if self.rates.is_empty() {
            tracing::debug!("ShortRateTree::price called before calibration (rates is empty)");
            return Err(Error::internal(
                "short-rate tree must be calibrated before pricing",
            ));
        }
        self.validate_lattice_geometry()?;

        // Ensure initial rate is present
        if !initial_vars.contains_key(state_keys::INTEREST_RATE) {
            if let Some(row) = self.rates.first() {
                if let Some(&r0) = row.first() {
                    initial_vars.insert(state_keys::INTEREST_RATE, r0);
                }
            }
        }

        // Get OAS from initial variables (default to 0)
        let oas = initial_vars.get("oas").copied().unwrap_or(0.0);

        // Create custom state generator that uses pre-calibrated rates
        // Clone rates (cheap Arc clone) to avoid lifetime issues with closures
        let rates_clone = self.rates.clone();
        let state_gen: StateGenerator = Box::new(move |step: usize, node: usize| -> f64 {
            if step < rates_clone.len() && node < rates_clone[step].len() {
                rates_clone[step][node]
            } else {
                0.0 // Fallback
            }
        });

        let rates_clone2 = self.rates.clone();
        let compounding = self.config.compounding;
        let dt_pricing = time_to_maturity / self.config.steps as f64;
        let rate_gen: StateGenerator = Box::new(move |step: usize, node: usize| -> f64 {
            let r = if step < rates_clone2.len() && node < rates_clone2[step].len() {
                rates_clone2[step][node] + oas / 10000.0
            } else {
                return 0.0;
            };
            compounding.to_continuous(r, dt_pricing)
        });

        // Set up branching probabilities based on tree type
        let (p_up, p_down, p_middle) = match self.config.branching {
            TreeBranching::Trinomial => {
                // Trinomial: equal probabilities for up/mid/down
                // This provides better numerical stability for mean-reverting models
                (1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0)
            }
            TreeBranching::Binomial => {
                // Binomial: use calibrated probabilities if available, else 50/50
                let (pu, pd) = self.probs.first().copied().unwrap_or((0.5, 0.5));
                (pu, pd, 0.0)
            }
        };

        price_recombining_tree(RecombiningInputs {
            branching: self.config.branching,
            steps: self.config.steps,
            initial_vars,
            time_to_maturity,
            market_context,
            valuator,
            up_factor: 1.0,   // Not used with custom_state_generator
            down_factor: 1.0, // Not used with custom_state_generator
            middle_factor: if self.config.branching == TreeBranching::Trinomial {
                Some(1.0)
            } else {
                None
            },
            prob_up: p_up,
            prob_down: p_down,
            prob_middle: Some(p_middle),
            interest_rate: 0.0, // Not used with custom_rate_generator
            barrier: None,
            custom_state_generator: Some(&state_gen),
            custom_rate_generator: Some(&rate_gen),
        })
    }

    fn calculate_greeks<V: TreeValuator>(
        &self,
        initial_vars: StateVariables,
        time_to_maturity: f64,
        market_context: &MarketContext,
        valuator: &V,
        bump_size: Option<f64>,
    ) -> Result<TreeGreeks> {
        // Base price
        let base_price = self.price(
            initial_vars.clone(),
            time_to_maturity,
            market_context,
            valuator,
        )?;

        let mut greeks = TreeGreeks {
            price: base_price,
            delta: 0.0, // Not applicable for bond vs rates
            gamma: 0.0, // Not applicable for bond vs rates
            vega: 0.0,  // Volatility sensitivity
            theta: 0.0, // Time decay
            rho: 0.0,   // Interest rate sensitivity
        };

        let vol_bump = bump_size.unwrap_or(0.01); // Default 1% vol bump (100bp for normal, 1% for lognormal)

        // Calculate Vega (volatility sensitivity) using central difference
        // This requires rebuilding trees with bumped volatility
        // Try to get the discount curve from market context for recalibration
        if let Ok(discount_curve) = market_context.get_discount(&self.calibration_curve_id) {
            // Build tree with vol + bump
            let mut config_up = self.config.clone();
            config_up.volatility += vol_bump;
            let mut tree_up = ShortRateTree::new(config_up);
            if tree_up
                .calibrate(discount_curve.as_ref(), time_to_maturity)
                .is_ok()
            {
                let price_up = tree_up.price(
                    initial_vars.clone(),
                    time_to_maturity,
                    market_context,
                    valuator,
                )?;

                // Build tree with vol - bump
                let mut config_down = self.config.clone();
                config_down.volatility = (config_down.volatility - vol_bump).max(1e-6);
                let mut tree_down = ShortRateTree::new(config_down);
                if tree_down
                    .calibrate(discount_curve.as_ref(), time_to_maturity)
                    .is_ok()
                {
                    let price_down = tree_down.price(
                        initial_vars.clone(),
                        time_to_maturity,
                        market_context,
                        valuator,
                    )?;

                    // Central difference vega per 1% vol
                    greeks.vega = (price_up - price_down) / 2.0;
                } else {
                    // Fallback to one-sided difference
                    greeks.vega = price_up - base_price;
                }
            }
        } else {
            // No discount curve available - vega cannot be computed accurately
            tracing::debug!(
                "ShortRateTree::calculate_greeks: discount curve '{}' not found, vega set to 0",
                self.calibration_curve_id.as_str()
            );
        }

        // Calculate Rho (interest rate sensitivity)
        // Approximate using finite differences on OAS (per 1bp)
        let mut bumped_vars = initial_vars.clone();
        let base_oas = initial_vars.get("oas").copied().unwrap_or(0.0);
        bumped_vars.insert("oas", base_oas + 1.0); // 1bp bump

        let bumped_price = self.price(bumped_vars, time_to_maturity, market_context, valuator)?;
        greeks.rho = bumped_price - base_price;

        // Calculate Theta (time decay) - 1 day bump
        let dt = 1.0 / 365.25;
        if time_to_maturity > dt {
            let price_tomorrow = self.price(
                initial_vars,
                time_to_maturity - dt,
                market_context,
                valuator,
            )?;
            greeks.theta = -(base_price - price_tomorrow) / dt;
        }

        Ok(greeks)
    }
}

/// State variable keys specific to short-rate trees
pub mod short_rate_keys {
    /// Short rate at the current node
    pub const SHORT_RATE: &str = "interest_rate";
    /// Option-Adjusted Spread added to the short rate
    pub const OAS: &str = "oas";
    /// Current tree step
    pub const STEP: &str = "step";
    /// Current node index
    pub const NODE: &str = "node";
    /// Time from valuation date
    pub const TIME: &str = "time";
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::common_impl::models::trees::tree_framework::NodeState;
    use finstack_core::market_data::term_structures::DiscountCurve;
    use finstack_core::math::interp::InterpStyle;
    use finstack_core::math::volatility::{convert_atm_volatility, VolatilityConvention};
    use time::Month;

    fn create_test_curve() -> DiscountCurve {
        DiscountCurve::builder("USD-OIS")
            .base_date(
                finstack_core::dates::Date::from_calendar_date(2025, Month::January, 1)
                    .expect("should succeed"),
            )
            .knots([(0.0, 1.0), (1.0, 0.97), (2.0, 0.94), (5.0, 0.85)])
            .interp(InterpStyle::LogLinear)
            .build()
            .expect("should succeed")
    }

    struct ConstantValuator;

    impl TreeValuator for ConstantValuator {
        fn value_at_maturity(&self, _state: &NodeState) -> Result<f64> {
            Ok(1.0)
        }

        fn value_at_node(
            &self,
            _state: &NodeState,
            continuation_value: f64,
            _dt: f64,
        ) -> Result<f64> {
            Ok(continuation_value)
        }
    }

    #[test]
    fn test_ho_lee_tree_creation() {
        let tree = ShortRateTree::ho_lee(50, 0.01);
        assert_eq!(tree.config.steps, 50);
        assert_eq!(tree.config.model, ShortRateModel::HoLee);
        assert_eq!(tree.config.volatility, 0.01);
    }

    #[test]
    fn test_tree_calibration() {
        let mut tree = ShortRateTree::ho_lee(10, 0.015);
        let curve = create_test_curve();

        let result = tree.calibrate(&curve, 2.0);
        assert!(result.is_ok());

        // Tree should have rates at each step
        assert_eq!(tree.rates.len(), 11); // 0 to 10 steps
        assert_eq!(tree.rates[0].len(), 1); // First step has one node
        assert_eq!(tree.rates[10].len(), 11); // Last step has 11 nodes
    }

    #[test]
    fn test_rate_access() {
        let mut tree = ShortRateTree::ho_lee(5, 0.01);
        let curve = create_test_curve();
        tree.calibrate(&curve, 1.0).expect("should succeed");

        // Should be able to access rates at valid nodes
        let r0 = tree.rate_at_node(0, 0).expect("should succeed");
        assert!(r0 > 0.0);

        let r_final = tree.rate_at_node(5, 2).expect("should succeed");
        assert!(r_final.is_finite());

        // Invalid access should error
        assert!(tree.rate_at_node(10, 0).is_err());
        assert!(tree.rate_at_node(0, 5).is_err());
    }

    #[test]
    fn test_bdt_tree_creation() {
        // BDT with realistic 20% lognormal volatility
        let tree = ShortRateTree::black_derman_toy(25, 0.20, 0.03);
        assert_eq!(tree.config.model, ShortRateModel::BlackDermanToy);
        assert_eq!(tree.config.volatility, 0.20);
        assert_eq!(tree.config.mean_reversion, Some(0.03));
    }

    #[test]
    fn test_bdt_calibration_populates_quality_metrics() {
        let mut tree = ShortRateTree::black_derman_toy(6, 0.20, 0.0);
        let curve = create_test_curve();

        tree.calibrate(&curve, 2.0).expect("should succeed");

        assert_eq!(tree.rates.len(), 7);
        assert_eq!(tree.probs.len(), 6);
        assert!(tree.probabilities(0).expect("probabilities").0.is_finite());
        let quality = tree.calibration_result().expect("calibration result");
        assert!(quality.converged);
        assert!(quality.max_error_bps.is_finite());
    }

    #[test]
    fn test_bdt_stored_lattice_prices_zero_coupon_to_calibration_curve() {
        let steps = 8;
        let maturity = 2.0;
        let mut tree = ShortRateTree::black_derman_toy(steps, 0.20, 0.0);
        let curve = create_test_curve();
        tree.calibrate(&curve, maturity).expect("BDT calibration");

        let mut vars = StateVariables::default();
        vars.insert(
            short_rate_keys::SHORT_RATE,
            tree.rate_at_node(0, 0).expect("root rate"),
        );
        let market = MarketContext::new();
        let actual = tree
            .price(vars, maturity, &market, &ConstantValuator)
            .expect("BDT zero coupon price");
        let expected = curve.df(maturity);

        assert!(
            (actual - expected).abs() < 1e-8,
            "BDT stored lattice should price a zero coupon to the calibration curve: actual={actual}, expected={expected}"
        );
    }

    #[test]
    fn test_bdt_config_uses_binomial_branching_matching_calibration_geometry() {
        let config = ShortRateTreeConfig::bdt(6, 0.20, 0.0);
        assert_eq!(config.branching, TreeBranching::Binomial);

        let mut tree = ShortRateTree::new(config);
        let curve = create_test_curve();
        tree.calibrate(&curve, 2.0).expect("BDT calibration");

        for step in 0..=6 {
            assert_eq!(
                tree.rates[step].len(),
                step + 1,
                "BDT calibration is binomial-width at step {step}"
            );
        }
    }

    #[test]
    fn test_short_rate_tree_rejects_branching_geometry_mismatch() {
        let mut tree = ShortRateTree::new(ShortRateTreeConfig::bdt(6, 0.20, 0.0).with_trinomial());
        let curve = create_test_curve();
        tree.calibrate(&curve, 2.0).expect("BDT calibration");

        let mut vars = StateVariables::default();
        vars.insert(
            short_rate_keys::SHORT_RATE,
            tree.rate_at_node(0, 0).expect("root rate"),
        );
        vars.insert(short_rate_keys::OAS, 0.0);
        let market = MarketContext::new();
        let err = tree
            .price(vars, 2.0, &market, &ConstantValuator)
            .expect_err("pricing must reject missing trinomial nodes instead of using zero rates");

        assert!(
            err.to_string().contains("lattice geometry"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn test_bdt_mean_reversion_calibrates_and_tightens_rate_dispersion() {
        let steps = 10;
        let mut tree_no_mr = ShortRateTree::new(ShortRateTreeConfig::bdt(steps, 0.20, 0.0));
        let mut tree_mr = ShortRateTree::new(ShortRateTreeConfig::bdt(steps, 0.20, 0.05));
        let curve = create_test_curve();

        tree_no_mr.calibrate(&curve, 2.0).expect("BDT(κ=0)");
        tree_mr.calibrate(&curve, 2.0).expect("BDT(κ=0.05)");

        let quality = tree_mr.calibration_result().expect("quality");
        assert!(quality.is_acceptable(), "BDT(κ=0.05) calibration: max_error={:.2}bp", quality.max_error_bps);

        let max_rate_no_mr = tree_no_mr.rate_at_node(steps, 0).expect("top node no MR");
        let max_rate_mr = tree_mr.rate_at_node(steps, 0).expect("top node MR");
        assert!(
            max_rate_mr < max_rate_no_mr,
            "mean reversion should tighten rate dispersion: no_mr_max={max_rate_no_mr:.6}, mr_max={max_rate_mr:.6}"
        );

        let market = MarketContext::new();
        let mut vars = StateVariables::default();
        vars.insert(short_rate_keys::SHORT_RATE, tree_mr.rate_at_node(0, 0).expect("root"));
        let zcb = tree_mr.price(vars, 2.0, &market, &ConstantValuator).expect("ZCB price");
        let target = curve.df(2.0);
        assert!(
            (zcb - target).abs() < 1e-6,
            "BDT(κ=0.05) should still price ZCBs to curve: got={zcb:.8}, target={target:.8}"
        );
    }

    // ========================================================================
    // Volatility Conversion Tests
    // ========================================================================

    #[test]
    fn test_normal_to_lognormal_vol_conversion() {
        // Test that conversion produces reasonable lognormal vol and round-trips correctly
        let normal_vol = 0.01; // 100 bps
        let rate_level = 0.05; // 5%

        let lognormal = convert_atm_volatility(
            normal_vol,
            VolatilityConvention::Normal,
            VolatilityConvention::Lognormal,
            rate_level,
            1.0,
        )
        .expect("valid conversion");

        // Lognormal vol should be in a reasonable range (roughly normal_vol / rate_level)
        assert!(
            lognormal > 0.15 && lognormal < 0.25,
            "lognormal vol {lognormal} out of range"
        );

        // Round-trip should recover original
        let recovered = convert_atm_volatility(
            lognormal,
            VolatilityConvention::Lognormal,
            VolatilityConvention::Normal,
            rate_level,
            1.0,
        )
        .expect("valid conversion");
        assert!(
            (recovered - normal_vol).abs() < 1e-10,
            "Round-trip failed: got {recovered}, expected {normal_vol}"
        );
    }

    #[test]
    fn test_lognormal_to_normal_vol_conversion() {
        // Test that conversion produces reasonable normal vol and round-trips correctly
        let lognormal_vol = 0.20; // 20%
        let rate_level = 0.05; // 5%

        let normal = convert_atm_volatility(
            lognormal_vol,
            VolatilityConvention::Lognormal,
            VolatilityConvention::Normal,
            rate_level,
            1.0,
        )
        .expect("valid conversion");

        // Normal vol should be in a reasonable range (roughly lognormal_vol * rate_level)
        assert!(
            normal > 0.005 && normal < 0.015,
            "normal vol {normal} out of range"
        );

        // Round-trip should recover original
        let recovered = convert_atm_volatility(
            normal,
            VolatilityConvention::Normal,
            VolatilityConvention::Lognormal,
            rate_level,
            1.0,
        )
        .expect("valid conversion");
        assert!(
            (recovered - lognormal_vol).abs() < 1e-10,
            "Round-trip failed: got {recovered}, expected {lognormal_vol}"
        );
    }

    #[test]
    fn test_vol_conversion_roundtrip() {
        let original_normal = 0.012; // 120 bps
        let rate_level = 0.045; // 4.5%

        let lognormal = convert_atm_volatility(
            original_normal,
            VolatilityConvention::Normal,
            VolatilityConvention::Lognormal,
            rate_level,
            1.0,
        )
        .expect("valid conversion");
        let back_to_normal = convert_atm_volatility(
            lognormal,
            VolatilityConvention::Lognormal,
            VolatilityConvention::Normal,
            rate_level,
            1.0,
        )
        .expect("valid conversion");

        assert!(
            (back_to_normal - original_normal).abs() < 1e-6,
            "Roundtrip conversion should be exact"
        );
    }

    #[test]
    fn test_normal_to_lognormal_errors_on_zero_rate() {
        let err = convert_atm_volatility(
            0.01,
            VolatilityConvention::Normal,
            VolatilityConvention::Lognormal,
            0.0,
            1.0,
        )
        .expect_err("should error");
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn test_normal_to_lognormal_errors_on_negative_rate() {
        let err = convert_atm_volatility(
            0.01,
            VolatilityConvention::Normal,
            VolatilityConvention::Lognormal,
            -0.01,
            1.0,
        )
        .expect_err("should error");
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn test_calibration_result_quality_helpers_cover_thresholds() {
        let good = CalibrationResult {
            max_error_bps: 0.05,
            max_error_step: 2,
            fallback_count: 0,
            converged: true,
        };
        assert!(good.is_good());
        assert!(good.is_acceptable());

        let acceptable_only = CalibrationResult {
            max_error_bps: 0.5,
            max_error_step: 3,
            fallback_count: 0,
            converged: true,
        };
        assert!(!acceptable_only.is_good());
        assert!(acceptable_only.is_acceptable());

        let poor = CalibrationResult {
            max_error_bps: 2.0,
            max_error_step: 1,
            fallback_count: 1,
            converged: true,
        };
        assert!(!poor.is_good());
        assert!(!poor.is_acceptable());
    }

    // ========================================================================
    // Config Factory Tests
    // ========================================================================

    #[test]
    fn test_config_ho_lee_factory() {
        let config = ShortRateTreeConfig::ho_lee(100, 0.008);
        assert_eq!(config.steps, 100);
        assert_eq!(config.model, ShortRateModel::HoLee);
        assert_eq!(config.volatility, 0.008);
        assert_eq!(config.mean_reversion, None);
    }

    #[test]
    fn test_config_bdt_factory() {
        let config = ShortRateTreeConfig::bdt(100, 0.20, 0.03);
        assert_eq!(config.steps, 100);
        assert_eq!(config.model, ShortRateModel::BlackDermanToy);
        assert_eq!(config.volatility, 0.20);
        assert_eq!(config.mean_reversion, Some(0.03));
    }

    #[test]
    fn test_config_from_normal_vol_factory() {
        let config = ShortRateTreeConfig::from_normal_vol(100, 0.008, 0.005).expect("valid config");
        assert_eq!(config.model, ShortRateModel::HoLee);

        let config = ShortRateTreeConfig::from_normal_vol(100, 0.01, 0.05).expect("valid config");
        assert_eq!(config.model, ShortRateModel::BlackDermanToy);
        // Vol should be in reasonable range (roughly normal_vol / rate_level ≈ 0.20)
        assert!(
            config.volatility > 0.15 && config.volatility < 0.25,
            "volatility {} out of expected range",
            config.volatility
        );
    }

    #[test]
    fn test_config_default_ho_lee() {
        let config = ShortRateTreeConfig::default_ho_lee(50);
        assert_eq!(config.steps, 50);
        assert_eq!(config.model, ShortRateModel::HoLee);
        assert_eq!(config.volatility, DEFAULT_NORMAL_VOL);
    }

    #[test]
    fn test_config_default_bdt() {
        let config = ShortRateTreeConfig::default_bdt(50);
        assert_eq!(config.steps, 50);
        assert_eq!(config.model, ShortRateModel::BlackDermanToy);
        assert_eq!(config.volatility, DEFAULT_LOGNORMAL_VOL);
    }

    #[test]
    fn test_config_from_normal_vol_low_rates() {
        // Low rate environment → should use Ho-Lee
        let config = ShortRateTreeConfig::from_normal_vol(100, 0.008, 0.005).expect("valid config");
        assert_eq!(config.model, ShortRateModel::HoLee);
        assert_eq!(config.volatility, 0.008); // Unchanged
    }

    #[test]
    fn test_config_from_normal_vol_normal_rates() {
        // Normal rate environment → should use BDT with converted vol
        let config = ShortRateTreeConfig::from_normal_vol(100, 0.01, 0.05).expect("valid config");
        assert_eq!(config.model, ShortRateModel::BlackDermanToy);
        // Vol should be in reasonable range (roughly normal_vol / rate_level ≈ 0.20)
        assert!(
            config.volatility > 0.15 && config.volatility < 0.25,
            "volatility {} out of expected range",
            config.volatility
        );
    }

    #[test]
    fn test_config_branching_helpers_and_normal_vol_boundary() {
        let binomial = ShortRateTreeConfig::bdt(50, 0.20, 0.03).with_binomial();
        assert_eq!(binomial.branching, TreeBranching::Binomial);

        let trinomial = ShortRateTreeConfig::ho_lee(50, 0.01).with_trinomial();
        assert_eq!(trinomial.branching, TreeBranching::Trinomial);

        let boundary = ShortRateTreeConfig::from_normal_vol(50, 0.01, 0.01).expect("valid config");
        assert_eq!(boundary.model, ShortRateModel::BlackDermanToy);
    }

    // ========================================================================
    // Tree Factory Tests
    // ========================================================================

    #[test]
    fn test_tree_default_ho_lee() {
        let tree = ShortRateTree::default_ho_lee(75);
        assert_eq!(tree.config.steps, 75);
        assert_eq!(tree.config.model, ShortRateModel::HoLee);
        assert_eq!(tree.config.volatility, DEFAULT_NORMAL_VOL);
    }

    #[test]
    fn test_tree_default_bdt() {
        let tree = ShortRateTree::default_bdt(75);
        assert_eq!(tree.config.steps, 75);
        assert_eq!(tree.config.model, ShortRateModel::BlackDermanToy);
        assert_eq!(tree.config.volatility, DEFAULT_LOGNORMAL_VOL);
    }

    #[test]
    fn test_probability_and_time_accessors_validate_bounds() {
        let mut tree = ShortRateTree::ho_lee(5, 0.01);
        let curve = create_test_curve();
        tree.calibrate(&curve, 1.0).expect("should succeed");

        assert_eq!(tree.probabilities(0).expect("probabilities"), (0.5, 0.5));
        assert_eq!(tree.time_at_step(0).expect("time"), 0.0);
        assert!(tree.time_at_step(5).expect("time").is_finite());
        assert!(tree.probabilities(10).is_err());
        assert!(tree.time_at_step(10).is_err());
    }

    #[test]
    fn test_price_rejects_uncalibrated_tree() {
        let tree = ShortRateTree::ho_lee(5, 0.01);
        let err = tree
            .price(
                StateVariables::default(),
                1.0,
                &MarketContext::new(),
                &ConstantValuator,
            )
            .expect_err("uncalibrated tree should error");
        assert!(err.to_string().contains("must be calibrated"));
    }
}
