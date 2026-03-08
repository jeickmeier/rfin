//! Tree-based pricing engine for bonds with embedded options and OAS calculations.
//!
//! This module provides tree-based pricing for callable/putable bonds and option-adjusted
//! spread (OAS) calculations using either:
//! - **Short-rate tree**: For bonds without credit risk
//! - **Rates+credit tree**: For bonds with credit risk (two-factor model)
//!
//! # Pricing Models
//!
//! ## Short-Rate Tree
//! Used for bonds without embedded credit risk. The tree models interest rate evolution
//! and applies call/put constraints via backward induction.
//!
//! ## Rates+Credit Tree
//! Used when a hazard curve is present in the market context. Models both interest rate
//! and credit risk evolution, with default events and recovery payments.
//!
//! # Examples
//!
//! ```rust,no_run
//! use finstack_valuations::instruments::fixed_income::bond::Bond;
//! use finstack_valuations::instruments::fixed_income::bond::pricing::tree_engine::TreePricer;
//! use finstack_core::market_data::context::MarketContext;
//! use finstack_core::dates::Date;
//!
//! # let bond = Bond::example().unwrap();
//! # let market = MarketContext::new();
//! # let as_of = Date::from_calendar_date(2024, time::Month::January, 15).unwrap();
//! let pricer = TreePricer::new();
//! let oas_bp = pricer.calculate_oas(&bond, &market, as_of, 98.5)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # See Also
//!
//! - `TreePricer` for OAS calculation
//! - tree-valuator implementation details in this module
//! - `TreePricerConfig` for configuration options

#![allow(clippy::module_inception)]
#![allow(dead_code)] // Public API items may be used by external bindings or tests
use super::super::super::types::Bond;

#[cfg(test)]
use super::super::super::types::CallPut;
use crate::cashflow::traits::CashflowProvider;
use crate::instruments::common_impl::models::trees::hull_white_tree::{
    HullWhiteTree, HullWhiteTreeConfig,
};
use crate::instruments::common_impl::models::trees::two_factor_rates_credit::{
    RatesCreditConfig, RatesCreditTree,
};
use crate::instruments::common_impl::models::{
    short_rate_keys, NodeState, ShortRateTree, ShortRateTreeConfig, StateVariables, TreeModel,
    TreeValuator,
};
#[cfg(test)]
use crate::instruments::PricingOverrides;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::math::solver::{BrentSolver, Solver};
use finstack_core::types::Percentage;
use finstack_core::Result;

#[cfg(test)]
use finstack_core::money::Money;

/// Choice of short-rate model for the bond pricing tree.
///
/// Controls which interest rate tree is used for backward induction. The default
/// `HoLee` model is a simple parallel-shift tree appropriate for quick estimates.
/// For production callable bond OAS, prefer `HullWhite` with calibrated parameters
/// or `HullWhiteCalibratedToSwaptions` for automatic calibration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum TreeModelChoice {
    /// Ho-Lee / BDT model (current default) with exogenous volatility.
    HoLee,
    /// Hull-White 1-factor with user-specified parameters.
    HullWhite {
        /// Mean reversion speed (e.g., 0.03 for 3%)
        kappa: f64,
        /// Short rate volatility (e.g., 0.01 for 100bp)
        sigma: f64,
    },
    /// Hull-White 1-factor calibrated to co-terminal swaptions.
    ///
    /// Extracts relevant swaption quotes from the market context and calibrates
    /// (kappa, sigma) automatically. This is the recommended choice for
    /// production callable bond OAS.
    HullWhiteCalibratedToSwaptions {
        /// ID of the swaption volatility surface in the market context
        swaption_vol_surface_id: String,
    },
}

impl Default for TreeModelChoice {
    fn default() -> Self {
        Self::HoLee
    }
}

/// Configuration for tree-based bond pricing (callable/putable bonds, OAS).
///
/// Controls the tree structure, convergence settings, and solver parameters
/// for option-adjusted spread calculations.
///
/// # Volatility Convention
///
/// ⚠️ **Critical**: The volatility interpretation depends on the underlying model:
///
/// | Model | Vol Type | Parameter | Typical Range |
/// |-------|----------|-----------|---------------|
/// | Ho-Lee (default) | Normal/Absolute | σ (rate units) | 50-150 bps (0.005-0.015) |
/// | BDT | Lognormal/Relative | σ (proportion) | 15-30% (0.15-0.30) |
///
/// The default configuration uses Ho-Lee with **normal volatility**.
///
/// ## Volatility Ranges by Model Type
///
/// ### Ho-Lee (Normal Volatility - Default)
///
/// | Rate Environment | Typical Vol Range | Example |
/// |------------------|-------------------|---------|
/// | Low rates (< 2%) | 50-80 bps | 0.005-0.008 |
/// | Normal rates (2-5%) | 80-120 bps | 0.008-0.012 |
/// | High rates (> 5%) | 100-150 bps | 0.010-0.015 |
/// | Crisis/stress | 150-300 bps | 0.015-0.030 |
///
/// ### Black-Derman-Toy (Lognormal Volatility)
///
/// | Market Condition | Typical Vol Range | Example |
/// |------------------|-------------------|---------|
/// | Low volatility | 10-15% | 0.10-0.15 |
/// | Normal market | 15-25% | 0.15-0.25 |
/// | High vol/stress | 25-40% | 0.25-0.40 |
///
/// ## Calibration Approaches
///
/// | Approach | Description | When to Use |
/// |----------|-------------|-------------|
/// | **Swaption-implied** | Calibrate to ATM swaption vol at bond's maturity | Institutional trading |
/// | **Historical** | Rolling 1Y historical rate vol | Quick estimates |
/// | **Model-implied** | Hull-White or BDT calibration | Full term structure |
///
/// ## Converting Between Conventions
///
/// Use `finstack_core::math::volatility::convert_atm_volatility`:
///
/// ```rust,no_run
/// use finstack_core::math::volatility::{convert_atm_volatility, VolatilityConvention};
///
/// // Normal vol (100 bps) at 5% rate → lognormal vol (20%)
/// let lognormal = convert_atm_volatility(
///     0.01,
///     VolatilityConvention::Normal,
///     VolatilityConvention::Lognormal,
///     0.05,
///     1.0,
/// )?;
///
/// // Lognormal vol (20%) at 5% rate → normal vol (100 bps)
/// let normal = convert_atm_volatility(
///     0.20,
///     VolatilityConvention::Lognormal,
///     VolatilityConvention::Normal,
///     0.05,
///     1.0,
/// )?;
/// # Ok::<(), finstack_core::Error>(())
/// ```
///
/// # Tree Resolution
///
/// The `tree_steps` parameter controls pricing accuracy vs computation time:
///
/// | Steps | Accuracy | Use Case |
/// |-------|----------|----------|
/// | 50 | ~2-5 bp | Quick screening |
/// | 100 | ~1 bp | Default, most trading |
/// | 200 | < 0.5 bp | Risk reports |
/// | 500 | < 0.2 bp | Regulatory/audit |
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::instruments::fixed_income::bond::pricing::tree_engine::TreePricerConfig;
///
/// // Default configuration using Ho-Lee with 100 bps normal vol
/// let default = TreePricerConfig::default();
///
/// // Production configuration with calibrated normal volatility
/// let production = TreePricerConfig::production_ho_lee(0.01); // 100 bps
///
/// // BDT model with lognormal volatility
/// let bdt = TreePricerConfig::production_bdt(0.20); // 20% lognormal
///
/// // High-precision configuration for regulatory reporting
/// let audit = TreePricerConfig::high_precision(0.01);
///
/// // Hull-White model (production recommended for callable bonds)
/// let hw = TreePricerConfig::hull_white(0.03, 0.01);
/// ```
#[derive(Debug, Clone)]
pub struct TreePricerConfig {
    /// Number of time steps in the interest rate tree.
    ///
    /// Higher values improve accuracy but increase computation time quadratically.
    /// Recommended: 100 for trading, 200+ for risk reports.
    pub tree_steps: usize,

    /// Short rate volatility (annualized).
    ///
    /// ⚠️ **Interpretation depends on model type**:
    /// - **Ho-Lee (default)**: Normal volatility in rate units (0.01 = 100 bps)
    /// - **BDT**: Lognormal volatility as proportion (0.20 = 20%)
    ///
    /// The default value of 100 bps (0.01) is appropriate for Ho-Lee model
    /// in normal rate environments. For BDT, use 15-25% (0.15-0.25).
    ///
    /// See struct-level documentation for calibration guidance and typical ranges.
    pub volatility: f64,

    /// Convergence tolerance for iterative solvers (OAS root finding).
    ///
    /// Default: `1e-6` (0.01 bp precision on OAS).
    /// Tighter tolerances increase iterations but improve precision.
    pub tolerance: f64,

    /// Maximum iterations for root finding algorithms.
    ///
    /// The OAS solver uses Brent's method which typically converges
    /// in 10-20 iterations. The cap prevents infinite loops on
    /// pathological inputs.
    pub max_iterations: usize,

    /// Initial bracket size (in basis points) for the OAS root solver.
    ///
    /// Wider brackets handle distressed/high-spread bonds but may
    /// slow convergence for tight spreads. Default: 1000 bp.
    pub initial_bracket_size_bp: Option<f64>,

    /// Mean reversion speed for Hull-White extension (annualized).
    ///
    /// When set with Ho-Lee model, transforms the tree into Hull-White 1F:
    /// `dr = [theta(t) - a*r] dt + sigma dW`
    ///
    /// - `None` (default): pure Ho-Lee (no mean reversion)
    /// - `Some(0.03)`: 3% annual mean reversion (moderate)
    /// - `Some(0.10)`: 10% annual mean reversion (strong)
    pub mean_reversion: Option<f64>,

    /// Short-rate model for the pricing tree.
    ///
    /// - `HoLee` (default): Uses the existing `ShortRateTree` path.
    /// - `HullWhite { kappa, sigma }`: Uses a calibrated HW trinomial tree.
    /// - `HullWhiteCalibratedToSwaptions { .. }`: Auto-calibrates HW params
    ///   from swaption vol data (preferred for production callable bond OAS).
    pub tree_model: TreeModelChoice,
}

impl Default for TreePricerConfig {
    /// Default configuration using Ho-Lee model with 100 bps normal volatility.
    ///
    /// This is appropriate for normal rate environments (2-5% rates).
    /// For low/negative rate environments, consider lower volatility.
    /// For BDT model, use [`TreePricerConfig::default_bdt`] instead.
    fn default() -> Self {
        Self {
            tree_steps: 100,
            volatility: 0.01, // 100 bps normal vol - appropriate for Ho-Lee
            tolerance: 1e-6,
            max_iterations: 50,
            initial_bracket_size_bp: Some(1000.0),
            mean_reversion: None,
            tree_model: TreeModelChoice::default(),
        }
    }
}

/// Get the tree pricer configuration for a bond.
///
/// This centralized function sources tree config from `bond.pricing_overrides`
/// when present, otherwise returns defaults. Use this instead of constructing
/// `TreePricerConfig::default()` directly to ensure consistent configuration
/// across all tree-based pricing paths (OAS metric, price_from_oas, embedded
/// option value, etc.).
///
/// # Arguments
///
/// * `bond` - The bond to get tree config for
///
/// # Returns
///
/// A `TreePricerConfig` with values from pricing_overrides or defaults.
///
/// # Examples
///
/// ```rust,no_run
/// use finstack_valuations::instruments::fixed_income::bond::Bond;
/// use finstack_valuations::instruments::fixed_income::bond::pricing::tree_engine::bond_tree_config;
///
/// let bond = Bond::example().unwrap();
/// let config = bond_tree_config(&bond);
/// ```
pub fn bond_tree_config(bond: &Bond) -> TreePricerConfig {
    // For callable/putable bonds, default to Hull-White with reasonable parameters.
    // HullWhiteCalibratedToSwaptions should be preferred when swaption vol data
    // is available in the market context.
    let tree_model = if bond.call_put.is_some() {
        TreeModelChoice::HullWhite {
            kappa: bond
                .pricing_overrides
                .model_config
                .mean_reversion
                .unwrap_or(0.03),
            sigma: bond
                .pricing_overrides
                .model_config
                .tree_volatility
                .unwrap_or(0.01),
        }
    } else {
        TreeModelChoice::HoLee
    };

    TreePricerConfig {
        tree_steps: bond
            .pricing_overrides
            .model_config
            .tree_steps
            .unwrap_or(100),
        volatility: bond
            .pricing_overrides
            .model_config
            .tree_volatility
            .unwrap_or(0.01),
        tolerance: 1e-6,
        max_iterations: 50,
        initial_bracket_size_bp: Some(1000.0),
        mean_reversion: bond.pricing_overrides.model_config.mean_reversion,
        tree_model,
    }
}

impl TreePricerConfig {
    // ========================================================================
    // Model-Specific Factory Methods
    // ========================================================================

    /// Create a production configuration for Ho-Lee model with normal volatility.
    ///
    /// Uses 100 tree steps which provides ~1 bp OAS accuracy for most bonds.
    /// Suitable for trading and daily risk reporting.
    ///
    /// # Arguments
    ///
    /// * `normal_vol` - Normal (absolute) volatility in rate units
    ///   (e.g., 0.01 = 100 bps/yr)
    ///
    /// # Typical Values
    ///
    /// - Low rates (<2%): 50-80 bps (0.005-0.008)
    /// - Normal rates (2-5%): 80-120 bps (0.008-0.012)
    /// - High rates (>5%): 100-150 bps (0.010-0.015)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::instruments::fixed_income::bond::pricing::tree_engine::TreePricerConfig;
    ///
    /// // Use 100 bps normal vol calibrated from swaption market
    /// let config = TreePricerConfig::production_ho_lee(0.01);
    /// ```
    pub fn production_ho_lee(normal_vol: f64) -> Self {
        Self {
            tree_steps: 100,
            volatility: normal_vol,
            tolerance: 1e-6,
            max_iterations: 50,
            initial_bracket_size_bp: Some(1000.0),
            mean_reversion: None,
            tree_model: TreeModelChoice::HoLee,
        }
    }

    /// Create a production configuration for Ho-Lee model with typed volatility.
    pub fn production_ho_lee_pct(normal_vol: Percentage) -> Self {
        Self {
            tree_steps: 100,
            volatility: normal_vol.as_decimal(),
            tolerance: 1e-6,
            max_iterations: 50,
            initial_bracket_size_bp: Some(1000.0),
            mean_reversion: None,
            tree_model: TreeModelChoice::HoLee,
        }
    }

    /// Create a production configuration for BDT model with lognormal volatility.
    ///
    /// Uses 100 tree steps which provides ~1 bp OAS accuracy for most bonds.
    /// BDT is preferred for positive rate environments where lognormal
    /// distribution better matches market conventions.
    ///
    /// # Arguments
    ///
    /// * `lognormal_vol` - Lognormal (relative) volatility as proportion
    ///   (e.g., 0.20 = 20%/yr)
    ///
    /// # Typical Values
    ///
    /// - Low volatility: 10-15% (0.10-0.15)
    /// - Normal market: 15-25% (0.15-0.25)
    /// - High vol/stress: 25-40% (0.25-0.40)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::instruments::fixed_income::bond::pricing::tree_engine::TreePricerConfig;
    ///
    /// // Use 20% lognormal vol (equivalent to ~100 bps at 5% rates)
    /// let config = TreePricerConfig::production_bdt(0.20);
    /// ```
    pub fn production_bdt(lognormal_vol: f64) -> Self {
        Self {
            tree_steps: 100,
            volatility: lognormal_vol,
            tolerance: 1e-6,
            max_iterations: 50,
            initial_bracket_size_bp: Some(1000.0),
            mean_reversion: None,
            tree_model: TreeModelChoice::HoLee,
        }
    }

    /// Create a production configuration for BDT model with typed volatility.
    pub fn production_bdt_pct(lognormal_vol: Percentage) -> Self {
        Self {
            tree_steps: 100,
            volatility: lognormal_vol.as_decimal(),
            tolerance: 1e-6,
            max_iterations: 50,
            initial_bracket_size_bp: Some(1000.0),
            mean_reversion: None,
            tree_model: TreeModelChoice::HoLee,
        }
    }

    /// Create default configuration for BDT model with 20% lognormal volatility.
    ///
    /// This is appropriate for normal positive rate environments.
    pub fn default_bdt() -> Self {
        Self::production_bdt(0.20)
    }

    /// Create a high-precision configuration for regulatory/audit purposes.
    ///
    /// Uses 200 tree steps for < 0.5 bp OAS accuracy and tighter convergence
    /// tolerance. Approximately 4x slower than production configuration.
    ///
    /// # Arguments
    ///
    /// * `calibrated_vol` - Annualized short rate volatility from market calibration
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::instruments::fixed_income::bond::pricing::tree_engine::TreePricerConfig;
    ///
    /// // High precision for regulatory reporting
    /// let config = TreePricerConfig::high_precision(0.012);
    /// ```
    pub fn high_precision(calibrated_vol: f64) -> Self {
        Self {
            tree_steps: 200,
            volatility: calibrated_vol,
            tolerance: 1e-8,
            max_iterations: 100,
            initial_bracket_size_bp: Some(1500.0),
            mean_reversion: None,
            tree_model: TreeModelChoice::HoLee,
        }
    }

    /// Create a high-precision configuration using typed volatility.
    pub fn high_precision_pct(calibrated_vol: Percentage) -> Self {
        Self {
            tree_steps: 200,
            volatility: calibrated_vol.as_decimal(),
            tolerance: 1e-8,
            max_iterations: 100,
            initial_bracket_size_bp: Some(1500.0),
            mean_reversion: None,
            tree_model: TreeModelChoice::HoLee,
        }
    }

    /// Create a fast configuration for screening large portfolios.
    ///
    /// Uses 50 tree steps for ~2-5 bp accuracy. Approximately 4x faster
    /// than production configuration. Suitable for quick screening and
    /// relative value analysis where precision is less critical.
    ///
    /// # Arguments
    ///
    /// * `calibrated_vol` - Annualized short rate volatility from market calibration
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::instruments::fixed_income::bond::pricing::tree_engine::TreePricerConfig;
    ///
    /// // Fast screening of 10,000 bond universe
    /// let config = TreePricerConfig::fast(0.012);
    /// ```
    pub fn fast(calibrated_vol: f64) -> Self {
        Self {
            tree_steps: 50,
            volatility: calibrated_vol,
            tolerance: 1e-4,
            max_iterations: 30,
            initial_bracket_size_bp: Some(1000.0),
            mean_reversion: None,
            tree_model: TreeModelChoice::HoLee,
        }
    }

    /// Create a fast configuration using typed volatility.
    pub fn fast_pct(calibrated_vol: Percentage) -> Self {
        Self {
            tree_steps: 50,
            volatility: calibrated_vol.as_decimal(),
            tolerance: 1e-4,
            max_iterations: 30,
            initial_bracket_size_bp: Some(1000.0),
            mean_reversion: None,
            tree_model: TreeModelChoice::HoLee,
        }
    }

    /// Create a configuration using a Hull-White 1-factor tree.
    ///
    /// Recommended for production callable bond OAS when swaption calibration
    /// is not available. Uses the specified (kappa, sigma) directly.
    ///
    /// # Arguments
    ///
    /// * `kappa` - Mean reversion speed (e.g., 0.03 for 3%)
    /// * `sigma` - Short rate volatility (e.g., 0.01 for 100bp)
    pub fn hull_white(kappa: f64, sigma: f64) -> Self {
        Self {
            tree_steps: 100,
            volatility: sigma,
            tolerance: 1e-6,
            max_iterations: 50,
            initial_bracket_size_bp: Some(1000.0),
            mean_reversion: Some(kappa),
            tree_model: TreeModelChoice::HullWhite { kappa, sigma },
        }
    }

    /// Create a configuration using a Hull-White 1-factor tree calibrated
    /// to swaption volatilities from the market context.
    ///
    /// This is the recommended choice for production callable bond OAS.
    ///
    /// # Arguments
    ///
    /// * `swaption_vol_surface_id` - ID of the swaption vol surface in market context
    pub fn hull_white_calibrated(swaption_vol_surface_id: String) -> Self {
        Self {
            tree_steps: 100,
            volatility: 0.01, // placeholder; overridden by calibrated sigma
            tolerance: 1e-6,
            max_iterations: 50,
            initial_bracket_size_bp: Some(1000.0),
            mean_reversion: None,
            tree_model: TreeModelChoice::HullWhiteCalibratedToSwaptions {
                swaption_vol_surface_id,
            },
        }
    }
}

/// Bond valuator for tree-based pricing of callable/putable bonds.
///
/// Implements [`TreeValuator`] trait for backward induction pricing with embedded options.
/// Maps bond cashflows and call/put schedules to tree time steps and handles
/// exercise decisions during backward induction.
///
/// # Call/Put Redemption Convention
///
/// Call/put redemption prices are computed as `outstanding_principal × (price_pct_of_par / 100)`,
/// where `outstanding_principal` is the remaining principal at the exercise date after
/// any amortization. This correctly handles amortizing callable bonds.
///
/// # Performance
///
/// Uses `Vec` instead of `HashMap` for step-indexed lookups to eliminate hashing
/// overhead in the backward induction hot path. For a 200-step tree, this provides
/// significant speedup over hash-based lookups.
///
/// # Thread Safety
///
/// `BondValuator` is `Send + Sync` (all fields are owned data or primitives),
/// making it safe to share across threads for parallel portfolio pricing.
///
/// # Examples
///
/// ```rust,no_run
/// use finstack_valuations::instruments::fixed_income::bond::Bond;
/// use finstack_valuations::instruments::fixed_income::bond::pricing::tree_engine::BondValuator;
/// use finstack_core::market_data::context::MarketContext;
/// use finstack_core::dates::Date;
///
/// # let bond = Bond::example().unwrap();
/// # let market = MarketContext::new();
/// # let as_of = Date::from_calendar_date(2024, time::Month::January, 15).unwrap();
/// let valuator = BondValuator::new(bond, &market, as_of, 5.0, 100)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct BondValuator {
    bond: Bond,
    /// Holder-view cashflow amounts indexed by time step (dense vector for O(1) access).
    /// Includes coupons, amortization, and final redemption — all positive receipts
    /// from the holder's perspective. Index `i` corresponds to time step `i`.
    /// Default value is 0.0.
    cashflow_vec: Vec<f64>,
    /// Call prices indexed by time step (sparse via Option for memory efficiency).
    /// `Some(price)` indicates a call option is exercisable at that step.
    /// Price is computed as `outstanding_principal × (price_pct / 100)`.
    call_vec: Vec<Option<f64>>,
    /// Put prices indexed by time step (sparse via Option for memory efficiency).
    /// `Some(price)` indicates a put option is exercisable at that step.
    /// Price is computed as `outstanding_principal × (price_pct / 100)`.
    put_vec: Vec<Option<f64>>,
    /// Outstanding principal indexed by time step for amortizing bonds.
    /// Used for call/put redemption and recovery calculations.
    outstanding_principal_vec: Vec<f64>,
    /// Time steps for tree pricing
    time_steps: Vec<f64>,
    /// Optional recovery rate sourced from a hazard curve in MarketContext
    recovery_rate: Option<f64>,
    /// Issuer call exercise friction in **cents per 100** of outstanding principal.
    ///
    /// This raises the exercise threshold (issuer calls only when continuation exceeds
    /// `call_price + friction_amount`), but redemption still occurs at `call_price`.
    call_friction_cents: f64,
}

impl BondValuator {
    fn make_whole_call_price(
        call: &crate::instruments::fixed_income::bond::CallPut,
        reference_curve: &dyn finstack_core::market_data::traits::Discounting,
        time_steps: &[f64],
        cashflow_vec: &[f64],
        step: usize,
        floor_price: f64,
    ) -> f64 {
        let call_time = *time_steps.get(step).unwrap_or(&0.0);
        let spread = call
            .make_whole
            .as_ref()
            .map(|spec| spec.spread_bps / 10_000.0)
            .unwrap_or(0.0);

        let mut pv_remaining = 0.0;
        for (future_step, amount) in cashflow_vec.iter().enumerate().skip(step + 1) {
            let amount = *amount;
            if amount.abs() <= f64::EPSILON {
                continue;
            }
            let future_time = *time_steps.get(future_step).unwrap_or(&call_time);
            if future_time <= call_time {
                continue;
            }

            let tau = future_time - call_time;
            let df_ratio = reference_curve.df(future_time) / reference_curve.df(call_time);
            pv_remaining += amount * df_ratio * (-spread * tau).exp();
        }

        floor_price.max(pv_remaining)
    }

    /// Create a new bond valuator for tree pricing.
    ///
    /// Builds maps of coupons, call prices, and put prices indexed by tree step.
    /// Cashflows and option exercise dates are mapped to the nearest tree step
    /// using the discount curve's day-count convention.
    ///
    /// # Arguments
    ///
    /// * `bond` - The bond to value
    /// * `market_context` - Market data including curves
    /// * `as_of` - Valuation date (time origin for the tree)
    /// * `time_to_maturity` - Time from `as_of` to maturity in years
    /// * `tree_steps` - Number of tree steps
    ///
    /// # Returns
    ///
    /// A `BondValuator` instance ready for tree-based pricing.
    ///
    /// # Errors
    ///
    /// Returns `Err` when:
    /// - Discount curve is not found
    /// - Cashflow schedule building fails
    /// - Time fraction calculations fail
    ///
    /// # Time Axis Consistency
    ///
    /// The `as_of` date defines the time origin (t=0) for the tree. All cashflow
    /// times and option exercise times are measured from `as_of` using the discount
    /// curve's day-count convention to ensure consistency with tree calibration.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use finstack_valuations::instruments::fixed_income::bond::Bond;
    /// use finstack_valuations::instruments::fixed_income::bond::pricing::tree_engine::BondValuator;
    /// use finstack_core::market_data::context::MarketContext;
    /// use finstack_core::dates::Date;
    ///
    /// # let bond = Bond::example().unwrap();
    /// # let market = MarketContext::new();
    /// # let as_of = Date::from_calendar_date(2024, time::Month::January, 15).unwrap();
    /// let valuator = BondValuator::new(bond, &market, as_of, 5.0, 100)?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new(
        bond: Bond,
        market_context: &MarketContext,
        as_of: Date,
        time_to_maturity: f64,
        tree_steps: usize,
    ) -> Result<Self> {
        use crate::cashflow::primitives::CFKind;

        let dt = time_to_maturity / tree_steps as f64;
        let time_steps: Vec<f64> = (0..=tree_steps).map(|i| i as f64 * dt).collect();
        let num_steps = tree_steps + 1; // Include step 0

        let curves = market_context;
        let discount_curve = market_context.get_discount(&bond.discount_curve_id)?;
        let dc_curve = discount_curve.day_count();
        let flows = bond.build_dated_flows(curves, as_of)?;

        // Build outstanding principal schedule from the full cashflow schedule.
        // This tracks notional minus cumulative amortization at each step for
        // correct call/put redemption pricing on amortizing bonds.
        let full_schedule = bond.get_full_schedule(market_context)?;
        let mut outstanding_principal_vec = vec![bond.notional.amount(); num_steps];

        // Collect amortization events sorted by date
        let mut amort_events: Vec<(Date, f64)> = full_schedule
            .flows
            .iter()
            .filter(|cf| matches!(cf.kind, CFKind::Amortization | CFKind::Notional))
            .filter(|cf| cf.date > as_of && cf.amount.amount() > 0.0)
            .map(|cf| (cf.date, cf.amount.amount()))
            .collect();
        amort_events.sort_by_key(|(d, _)| *d);

        // Track cumulative amortization and map to time steps
        let mut cumulative_amort = 0.0;
        let initial_notional = bond.notional.amount();
        let mut amort_idx = 0;

        for step in 0..num_steps {
            let step_time = time_steps[step];

            // Process any amortization events that occur at or before this step time
            while amort_idx < amort_events.len() {
                let (amort_date, amort_amt) = amort_events[amort_idx];
                let amort_time = dc_curve
                    .year_fraction(
                        as_of,
                        amort_date,
                        finstack_core::dates::DayCountCtx::default(),
                    )
                    .unwrap_or(0.0);

                if amort_time <= step_time + dt / 2.0 {
                    // This amortization has occurred by this step
                    cumulative_amort += amort_amt;
                    amort_idx += 1;
                } else {
                    break;
                }
            }

            outstanding_principal_vec[step] = (initial_notional - cumulative_amort).max(0.0);
        }

        // Collect exercise dates so we can snap coincident coupons to the same
        // tree step used for the call/put (ceil mapping), preventing timing
        // mismatches between coupon receipt and exercise decision.
        let mut exercise_dates = std::collections::HashSet::new();
        if let Some(ref call_put) = bond.call_put {
            for call in &call_put.calls {
                if call.date > as_of && call.date <= bond.maturity {
                    exercise_dates.insert(call.date);
                }
            }
            for put in &call_put.puts {
                if put.date > as_of && put.date <= bond.maturity {
                    exercise_dates.insert(put.date);
                }
            }
        }

        // Pre-allocate vectors for O(1) access during backward induction
        let mut cashflow_vec = vec![0.0; num_steps];
        for (date, amount) in &flows {
            if *date > as_of {
                let time_frac = dc_curve.year_fraction(
                    as_of,
                    *date,
                    finstack_core::dates::DayCountCtx::default(),
                )?;
                let raw = (time_frac / time_to_maturity) * tree_steps as f64;

                // Ensure we don't go out of bounds
                let raw_clamped = raw.clamp(0.0, tree_steps as f64);

                // When a cashflow date matches an exercise date, snap to the
                // exercise step (ceil) to prevent timing mismatches between
                // coupon receipt and exercise decision.
                if exercise_dates.contains(date) {
                    let mut step = raw_clamped.ceil() as usize;
                    if step == 0 {
                        step = 1;
                    }
                    if step >= num_steps {
                        step = num_steps - 1;
                    }
                    cashflow_vec[step] += amount.amount();
                } else {
                    // Distributed mapping: spread cashflow between two nearest time steps
                    // to reduce discretization error and improve convergence.

                    // Lower step index
                    let step_idx = raw_clamped.floor() as usize;

                    // Weight for the upper step (fractional part)
                    let weight = raw_clamped - step_idx as f64;

                    // Distribute to step_idx (weight: 1.0 - weight)
                    if step_idx > 0 && step_idx < num_steps {
                        cashflow_vec[step_idx] += amount.amount() * (1.0 - weight);
                    }

                    // Distribute to step_idx + 1 (weight: weight)
                    if step_idx + 1 < num_steps {
                        cashflow_vec[step_idx + 1] += amount.amount() * weight;
                    }
                }
            }
        }

        // Sparse vectors for call/put (most steps have no option)
        // Call/put redemption uses outstanding principal at exercise date, not original notional.
        let mut call_vec: Vec<Option<f64>> = vec![None; num_steps];
        let mut put_vec: Vec<Option<f64>> = vec![None; num_steps];
        if let Some(ref call_put) = bond.call_put {
            for call in &call_put.calls {
                if call.date > as_of && call.date <= bond.maturity {
                    let time_frac = dc_curve.year_fraction(
                        as_of,
                        call.date,
                        finstack_core::dates::DayCountCtx::default(),
                    )?;
                    let raw = (time_frac / time_to_maturity) * tree_steps as f64;
                    let mut step = raw.ceil() as usize;
                    if step == 0 {
                        step = 1;
                    }
                    if step >= num_steps {
                        step = num_steps - 1;
                    }
                    let outstanding = outstanding_principal_vec[step];
                    let floor_price = outstanding * (call.price_pct_of_par / 100.0);
                    let call_price = if let Some(spec) = &call.make_whole {
                        let reference_curve =
                            market_context.get_discount(&spec.reference_curve_id)?;
                        Self::make_whole_call_price(
                            call,
                            reference_curve.as_ref(),
                            &time_steps,
                            &cashflow_vec,
                            step,
                            floor_price,
                        )
                    } else {
                        floor_price
                    };
                    call_vec[step] = Some(call_price);
                }
            }
            for put in &call_put.puts {
                if put.date > as_of && put.date <= bond.maturity {
                    let time_frac = dc_curve.year_fraction(
                        as_of,
                        put.date,
                        finstack_core::dates::DayCountCtx::default(),
                    )?;
                    let raw = (time_frac / time_to_maturity) * tree_steps as f64;
                    let mut step = raw.ceil() as usize;
                    if step == 0 {
                        step = 1;
                    }
                    if step >= num_steps {
                        step = num_steps - 1;
                    }
                    // Use outstanding principal at exercise step, not original notional
                    let outstanding = outstanding_principal_vec[step];
                    let put_price = outstanding * (put.price_pct_of_par / 100.0);
                    put_vec[step] = Some(put_price);
                }
            }
        }

        // Source recovery rate from hazard curve using the same precedence as
        // HazardBondEngine and TreePricer::calculate_oas:
        // 1. credit_curve_id (if present)
        // 2. discount_curve_id
        // 3. discount_curve_id with "-CREDIT" suffix
        // This ensures consistency across all credit-aware pricing paths.
        let recovery_rate = Self::resolve_recovery_rate(&bond, market_context);
        let call_friction_cents = bond
            .pricing_overrides
            .model_config
            .call_friction_cents
            .unwrap_or(0.0);

        Ok(Self {
            bond,
            cashflow_vec,
            call_vec,
            put_vec,
            outstanding_principal_vec,
            time_steps,
            recovery_rate,
            call_friction_cents,
        })
    }

    /// Get the total holder-view cashflow amount at this time step.
    ///
    /// This includes coupons, amortization, and final redemption — all positive
    /// receipts from the holder's perspective.
    #[inline]
    fn cashflow_at(&self, step: usize) -> f64 {
        self.cashflow_vec.get(step).copied().unwrap_or(0.0)
    }

    /// Check if there's a call option at this time step.
    #[inline]
    fn call_at(&self, step: usize) -> Option<f64> {
        self.call_vec.get(step).copied().flatten()
    }

    /// Check if there's a put option at this time step.
    #[inline]
    fn put_at(&self, step: usize) -> Option<f64> {
        self.put_vec.get(step).copied().flatten()
    }

    /// Get outstanding principal at this time step.
    ///
    /// For bullet bonds, this returns the original notional.
    /// For amortizing bonds, this returns the remaining principal after amortization.
    #[inline]
    fn outstanding_principal_at(&self, step: usize) -> f64 {
        self.outstanding_principal_vec
            .get(step)
            .copied()
            .unwrap_or(self.bond.notional.amount())
    }

    /// Price the bond using a calibrated Hull-White trinomial tree with OAS.
    ///
    /// Uses `HullWhiteTree::backward_induction` with the bond's cashflow and
    /// call/put schedules applied at each node. The OAS is applied as an
    /// additional parallel shift to the short rate when discounting.
    ///
    /// # Arguments
    ///
    /// * `hw_tree` - Calibrated Hull-White tree
    /// * `oas_bp` - Option-adjusted spread in basis points
    ///
    /// # Returns
    ///
    /// Model dirty price of the bond.
    pub(crate) fn price_with_hw_tree(&self, hw_tree: &HullWhiteTree, oas_bp: f64) -> f64 {
        let oas_decimal = oas_bp / 10_000.0;
        let dt = hw_tree.dt();
        let final_step = hw_tree.num_steps();

        let terminal_cf = self.cashflow_at(final_step);
        let terminal_values = vec![terminal_cf; hw_tree.num_nodes(final_step)];

        hw_tree.backward_induction(&terminal_values, |step, _node_idx, continuation| {
            // The HW tree's backward_induction already discounts by the short
            // rate r(step, node). Apply the OAS as additional discounting.
            let oas_discount = (-oas_decimal * dt).exp();
            let oas_adjusted = continuation * oas_discount;

            let coupon = self.cashflow_at(step);
            let mut principal_value = oas_adjusted;

            if let Some(put_price) = self.put_at(step) {
                principal_value = principal_value.max(put_price);
            }

            if let Some(call_price) = self.call_at(step) {
                let outstanding = self.outstanding_principal_at(step);
                let friction_amount = outstanding * (self.call_friction_cents / 10_000.0);
                let threshold = call_price + friction_amount;
                if principal_value > threshold {
                    principal_value = principal_value.min(call_price);
                }
            }

            coupon + principal_value
        })
    }

    /// Resolve recovery rate from hazard curve using the same precedence as
    /// HazardBondEngine and TreePricer::calculate_oas.
    ///
    /// Precedence:
    /// 1. `credit_curve_id` if present
    /// 2. `discount_curve_id`
    /// 3. `discount_curve_id` with "-CREDIT" suffix
    ///
    /// Returns `None` if no hazard curve can be resolved.
    fn resolve_recovery_rate(bond: &Bond, market: &MarketContext) -> Option<f64> {
        // Try credit_curve_id first
        if let Some(ref credit_id) = bond.credit_curve_id {
            if let Ok(hc) = market.get_hazard(credit_id.as_str()) {
                return Some(hc.recovery_rate());
            }
        }

        // Try discount_curve_id
        if let Ok(hc) = market.get_hazard(bond.discount_curve_id.as_str()) {
            return Some(hc.recovery_rate());
        }

        // Try discount_curve_id with "-CREDIT" suffix
        let credit_id = format!("{}-CREDIT", bond.discount_curve_id.as_str());
        if let Ok(hc) = market.get_hazard(&credit_id) {
            return Some(hc.recovery_rate());
        }

        None
    }
}

impl TreeValuator for BondValuator {
    fn value_at_maturity(&self, _state: &NodeState) -> Result<f64> {
        let final_step = self.time_steps.len() - 1;
        let cashflow = self.cashflow_at(final_step);
        Ok(cashflow)
    }

    fn value_at_node(&self, state: &NodeState, continuation_value: f64, dt: f64) -> Result<f64> {
        let step = state.step;
        let coupon = self.cashflow_at(step);

        // Call/put exercise logic:
        // - Coupon is ALWAYS paid on coupon dates regardless of exercise decision
        // - Call/put redemption is principal-only (price_pct_of_par × outstanding)
        // - Exercise decision compares continuation vs redemption value
        //
        // Formula: value = coupon + min(max(continuation, put_redemption), call_redemption)
        //
        // This ensures:
        // 1. Coupon is received regardless of exercise
        // 2. Put floor: holder can demand redemption if continuation < put_price
        // 3. Call cap: issuer can redeem if continuation > call_price

        // Start with continuation value (principal path if not exercised)
        let mut principal_value = continuation_value;

        // Put option: holder can exercise if redemption > continuation
        if let Some(put_price) = self.put_at(step) {
            principal_value = principal_value.max(put_price);
        }

        // Call option: issuer can exercise if redemption < continuation, subject to friction.
        //
        // With friction, the issuer only calls when continuation exceeds:
        //   call_price + (outstanding_principal × call_friction_cents / 10_000)
        //
        // (because 1 cent per 100 of par = 0.0001 of notional).
        if let Some(call_price) = self.call_at(step) {
            let outstanding = self.outstanding_principal_at(step);
            let friction_amount = outstanding * (self.call_friction_cents / 10_000.0);
            let threshold = call_price + friction_amount;
            if principal_value > threshold {
                principal_value = principal_value.min(call_price);
            }
        }

        // Coupon is added after exercise decision (coupon is paid regardless)
        let alive_value = coupon + principal_value;

        // Default handling: if hazard rate is present, compute survival/default weighting.
        // Use cached fields instead of hash lookups for performance.
        //
        // Recovery convention: recovery is received at the *current* node upon
        // default (standard Hull/Brigo-Mercurio convention). No additional one-
        // period discounting is applied — `alive_value` and `recovery` are both
        // in PV-at-this-node terms.
        if let Some(hazard) = state.hazard_rate {
            let p_surv = (-hazard.max(0.0) * dt).exp();
            let default_prob = (1.0 - p_surv).clamp(0.0, 1.0);
            // Use outstanding principal at this step for recovery (FRP convention)
            let outstanding = self.outstanding_principal_at(step);
            let recovery = self
                .recovery_rate
                .map(|rr| rr.clamp(0.0, 1.0) * outstanding)
                .unwrap_or(0.0);
            let node_value = p_surv * alive_value + default_prob * recovery;
            Ok(node_value)
        } else {
            // No hazard info at this node; return alive path value
            Ok(alive_value)
        }
    }
}

const _: () = {
    fn _assert_send<T: Send>() {}
    fn _assert_sync<T: Sync>() {}
    fn _assertions() {
        _assert_send::<BondValuator>();
        _assert_sync::<BondValuator>();
        _assert_send::<TreePricer>();
        _assert_sync::<TreePricer>();
    }
};

/// Tree-based pricer for bonds with embedded options and OAS calculations.
///
/// Provides methods for calculating option-adjusted spread (OAS) for bonds with
/// embedded call/put options. Automatically selects between short-rate and
/// rates+credit tree models based on available market data.
///
/// # Examples
///
/// ```rust,no_run
/// use finstack_valuations::instruments::fixed_income::bond::Bond;
/// use finstack_valuations::instruments::fixed_income::bond::pricing::tree_engine::TreePricer;
/// use finstack_core::market_data::context::MarketContext;
/// use finstack_core::dates::Date;
///
/// # let bond = Bond::example().unwrap();
/// # let market = MarketContext::new();
/// # let as_of = Date::from_calendar_date(2024, time::Month::January, 15).unwrap();
/// let pricer = TreePricer::new();
/// // OAS in basis points
/// let oas_bp = pricer.calculate_oas(&bond, &market, as_of, 98.5)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct TreePricer {
    /// Pricer configuration (tree steps, volatility, convergence settings)
    config: TreePricerConfig,
}

impl TreePricer {
    /// Create a new tree pricer with default configuration.
    ///
    /// # Returns
    ///
    /// A `TreePricer` with default configuration (100 steps, 1% volatility).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::instruments::fixed_income::bond::pricing::tree_engine::TreePricer;
    ///
    /// let pricer = TreePricer::new();
    /// ```
    pub fn new() -> Self {
        Self {
            config: TreePricerConfig::default(),
        }
    }

    /// Create a tree pricer with custom configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Custom tree pricer configuration
    ///
    /// # Returns
    ///
    /// A `TreePricer` with the specified configuration.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::instruments::fixed_income::bond::pricing::tree_engine::{TreePricer, TreePricerConfig};
    ///
    /// let config = TreePricerConfig::high_precision(0.015);
    /// let pricer = TreePricer::with_config(config);
    /// ```
    pub fn with_config(config: TreePricerConfig) -> Self {
        Self { config }
    }

    /// Calculate option-adjusted spread (OAS) for a bond.
    ///
    /// Solves for the constant spread that equates the tree price to the market price.
    /// Uses Brent's method for root finding, automatically selecting between short-rate
    /// and rates+credit tree models based on available market data.
    ///
    /// # OAS Convention
    ///
    /// Under either model the OAS is a **parallel shift to the calibrated risk-free
    /// short rate lattice** (in basis points). When the rates+credit two-factor tree
    /// is used, the hazard tree captures the credit spread independently, so the OAS
    /// represents the option-adjusted spread **over the risk-free curve** — consistent
    /// with the Bloomberg OAS convention for risky bonds.
    ///
    /// # Arguments
    ///
    /// * `bond` - The bond to calculate OAS for (must have call/put options)
    /// * `market_context` - Market context with discount and optionally hazard curves
    /// * `as_of` - Valuation date
    /// * `clean_price_pct_of_par` - Market clean price as percentage of par (e.g., 98.5)
    ///
    /// # Returns
    ///
    /// OAS in basis points (e.g., 150.0 means 150 basis points).
    ///
    /// # Errors
    ///
    /// Returns `Err` when:
    /// - Discount curve is not found
    /// - Tree calibration fails
    /// - Root finding fails to converge
    /// - Bond is already matured
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use finstack_valuations::instruments::fixed_income::bond::Bond;
    /// use finstack_valuations::instruments::fixed_income::bond::pricing::tree_engine::TreePricer;
    /// use finstack_core::market_data::context::MarketContext;
    /// use finstack_core::dates::Date;
    ///
    /// # let bond = Bond::example().unwrap();
    /// # let market = MarketContext::new();
    /// # let as_of = Date::from_calendar_date(2024, time::Month::January, 15).unwrap();
    /// let pricer = TreePricer::new();
    /// let oas_bp = pricer.calculate_oas(&bond, &market, as_of, 98.5)?;
    /// // oas_bp is in basis points
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn calculate_oas(
        &self,
        bond: &Bond,
        market_context: &MarketContext,
        as_of: Date,
        clean_price_pct_of_par: f64,
    ) -> Result<f64> {
        use crate::instruments::fixed_income::bond::pricing::settlement::QuoteDateContext;

        // Dirty target must use accrued at the quote/settlement date to match
        // the market convention used by YTM, Z-spread, and the quote engine.
        let quote_ctx = QuoteDateContext::new(bond, market_context, as_of)?;
        let dirty_target =
            quote_ctx.dirty_from_clean_pct(clean_price_pct_of_par, bond.notional.amount());
        // Choose model: if a hazard curve is present in MarketContext whose ID matches the bond's
        // discount ID (preferred) or the fallback pattern "{discount_curve_id}-CREDIT", use the rates+credit
        // two-factor tree; otherwise, fall back to short-rate.
        let mut use_rates_credit = false;
        let mut rc_tree: Option<RatesCreditTree> = None;
        let discount_curve = market_context.get_discount(&bond.discount_curve_id)?;
        // Align tree time basis with the discount curve's own day-count.
        if as_of >= bond.maturity {
            return Ok(0.0);
        }
        let dc_curve = discount_curve.day_count();
        let time_to_maturity = dc_curve.year_fraction(
            as_of,
            bond.maturity,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        if time_to_maturity <= 0.0 {
            return Ok(0.0);
        }
        let hazard_curve = if let Some(hid) = bond.credit_curve_id.as_ref() {
            market_context.get_hazard(hid.as_str()).ok()
        } else {
            market_context
                .get_hazard(bond.discount_curve_id.as_str())
                .ok()
                .or_else(|| {
                    market_context
                        .get_hazard(format!("{}-CREDIT", bond.discount_curve_id.as_str()))
                        .ok()
                })
        };
        if let Some(hc) = hazard_curve.as_ref() {
            let cfg = RatesCreditConfig {
                steps: self.config.tree_steps,
                rate_vol: self.config.volatility,
                ..Default::default()
            };
            let mut tree = RatesCreditTree::new(cfg);
            tree.calibrate(discount_curve.as_ref(), hc.as_ref(), time_to_maturity)?;
            rc_tree = Some(tree);
            use_rates_credit = true;
        }

        // Resolve the effective HW parameters when using HullWhite model variants.
        // For HullWhiteCalibratedToSwaptions, attempt swaption calibration;
        // on failure, log a warning and fall back to HoLee.
        let effective_model = match &self.config.tree_model {
            TreeModelChoice::HullWhiteCalibratedToSwaptions {
                swaption_vol_surface_id,
            } if !use_rates_credit => Self::resolve_hw_calibrated(
                market_context,
                &discount_curve,
                swaption_vol_surface_id,
                time_to_maturity,
            ),
            other => other.clone(),
        };

        let mut sr_tree: Option<ShortRateTree> = None;
        let mut hw_tree: Option<HullWhiteTree> = None;

        if !use_rates_credit {
            match &effective_model {
                TreeModelChoice::HullWhite { kappa, sigma } => {
                    let hw_config = HullWhiteTreeConfig {
                        kappa: *kappa,
                        sigma: *sigma,
                        steps: self.config.tree_steps,
                        max_nodes: None,
                    };
                    hw_tree = Some(HullWhiteTree::calibrate(
                        hw_config,
                        discount_curve.as_ref(),
                        time_to_maturity,
                    )?);
                }
                TreeModelChoice::HoLee | TreeModelChoice::HullWhiteCalibratedToSwaptions { .. } => {
                    let tree_config = ShortRateTreeConfig {
                        steps: self.config.tree_steps,
                        volatility: self.config.volatility,
                        mean_reversion: self.config.mean_reversion,
                        ..Default::default()
                    };
                    let mut tree = ShortRateTree::new(tree_config);
                    tree.calibrate(discount_curve.as_ref(), time_to_maturity)?;
                    sr_tree = Some(tree);
                }
            }
        }

        let valuator = BondValuator::new(
            bond.clone(),
            market_context,
            as_of,
            time_to_maturity,
            self.config.tree_steps,
        )?;

        // Get initial short rate for state variables (needed by short-rate tree)
        let initial_rate = if let Some(tree) = sr_tree.as_ref() {
            tree.rate_at_node(0, 0).unwrap_or(0.03)
        } else {
            0.0 // Not used for rates+credit or HW tree
        };

        let objective_fn = |oas: f64| -> f64 {
            if use_rates_credit {
                let mut vars = StateVariables::default();
                vars.insert("oas", oas);
                if let Some(tree) = rc_tree.as_ref() {
                    match tree.price(vars, time_to_maturity, market_context, &valuator) {
                        Ok(model_price) => model_price - dirty_target,
                        Err(_) => 1.0e6,
                    }
                } else {
                    1.0e6
                }
            } else if let Some(ref tree) = hw_tree {
                // Hull-White trinomial tree: OAS applied inside backward induction
                let model_price = valuator.price_with_hw_tree(tree, oas);
                model_price - dirty_target
            } else {
                let mut vars = StateVariables::default();
                vars.insert(short_rate_keys::SHORT_RATE, initial_rate);
                vars.insert(short_rate_keys::OAS, oas);
                if let Some(tree) = sr_tree.as_ref() {
                    match tree.price(vars, time_to_maturity, market_context, &valuator) {
                        Ok(model_price) => model_price - dirty_target,
                        Err(_) => {
                            if oas > 0.0 {
                                1.0e6
                            } else {
                                -1.0e6
                            }
                        }
                    }
                } else {
                    1.0e6
                }
            }
        };

        let mut solver = BrentSolver::new()
            .tolerance(self.config.tolerance)
            .initial_bracket_size(self.config.initial_bracket_size_bp);
        // Respect the configured maximum iteration cap for OAS root-finding.
        solver.max_iterations = self.config.max_iterations;
        let initial_guess = 0.0;
        let oas_bp = solver.solve(objective_fn, initial_guess)?;
        Ok(oas_bp)
    }

    /// Attempt swaption-calibrated Hull-White. On failure, fall back to HoLee.
    ///
    /// Reads the swaption vol surface from the market context, converts grid
    /// points into `SwaptionQuote`s, and runs Levenberg-Marquardt calibration.
    fn resolve_hw_calibrated(
        market_context: &MarketContext,
        discount_curve: &std::sync::Arc<finstack_core::market_data::term_structures::DiscountCurve>,
        swaption_vol_surface_id: &str,
        time_to_maturity: f64,
    ) -> TreeModelChoice {
        use crate::calibration::hull_white::{
            calibrate_hull_white_to_swaptions_with_frequency, SwapFrequency, SwaptionQuote,
        };

        let surface = match market_context.get_surface(swaption_vol_surface_id) {
            Ok(s) => s,
            Err(_) => {
                tracing::warn!(
                    surface_id = swaption_vol_surface_id,
                    "Swaption vol surface not found in market context; \
                     falling back to HoLee tree model"
                );
                return TreeModelChoice::HoLee;
            }
        };

        // Build SwaptionQuote list from the surface grid.
        // Convention: expiries axis = swaption expiry (years),
        //             strikes axis = underlying swap tenor (years).
        // Each grid point is an ATM normal vol.
        let expiries = surface.expiries();
        let tenors = surface.strikes();
        let mut quotes = Vec::with_capacity(expiries.len() * tenors.len());
        for &expiry in expiries {
            // Only use swaptions expiring before the bond maturity
            if expiry > time_to_maturity || expiry <= 0.0 {
                continue;
            }
            for &tenor in tenors {
                if tenor <= 0.0 {
                    continue;
                }
                let vol = surface.value_clamped(expiry, tenor);
                if vol > 0.0 && vol.is_finite() {
                    quotes.push(SwaptionQuote {
                        expiry,
                        tenor,
                        volatility: vol,
                        is_normal_vol: true,
                    });
                }
            }
        }

        if quotes.len() < 2 {
            tracing::warn!(
                surface_id = swaption_vol_surface_id,
                n_valid = quotes.len(),
                "Insufficient swaption quotes from vol surface; \
                 falling back to HoLee tree model"
            );
            return TreeModelChoice::HoLee;
        }

        let dc = discount_curve.clone();
        let df_fn = move |t: f64| dc.df(t);

        match calibrate_hull_white_to_swaptions_with_frequency(
            &df_fn,
            &quotes,
            SwapFrequency::SemiAnnual,
        ) {
            Ok((params, report)) => {
                if report.success {
                    tracing::info!(
                        kappa = params.kappa,
                        sigma = params.sigma,
                        n_quotes = quotes.len(),
                        "Hull-White calibrated to swaptions"
                    );
                    TreeModelChoice::HullWhite {
                        kappa: params.kappa,
                        sigma: params.sigma,
                    }
                } else {
                    tracing::warn!(
                        reason = report.convergence_reason.as_str(),
                        "Swaption calibration did not converge; \
                         falling back to HoLee tree model"
                    );
                    TreeModelChoice::HoLee
                }
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "Swaption calibration failed; falling back to HoLee tree model"
                );
                TreeModelChoice::HoLee
            }
        }
    }
}

impl Default for TreePricer {
    fn default() -> Self {
        Self::new()
    }
}

/// Calculate option-adjusted spread for a bond given market price.
///
/// Convenience function using default tree configuration. This is a wrapper
/// around `TreePricer::new().calculate_oas()` for simple use cases.
///
/// # Arguments
///
/// * `bond` - The bond to calculate OAS for
/// * `market_context` - Market context with curves
/// * `as_of` - Valuation date
/// * `clean_price` - Market clean price as percentage of par
///
/// # Returns
///
/// OAS in basis points.
///
/// # Examples
///
/// ```rust,no_run
/// use finstack_valuations::instruments::fixed_income::bond::Bond;
/// use finstack_valuations::instruments::fixed_income::bond::pricing::tree_engine::calculate_oas;
/// use finstack_core::market_data::context::MarketContext;
/// use finstack_core::dates::Date;
///
/// # let bond = Bond::example().unwrap();
/// # let market = MarketContext::new();
/// # let as_of = Date::from_calendar_date(2024, time::Month::January, 15).unwrap();
/// let oas_bp = calculate_oas(&bond, &market, as_of, 98.5)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn calculate_oas(
    bond: &Bond,
    market_context: &MarketContext,
    as_of: Date,
    clean_price: f64,
) -> Result<f64> {
    let calculator = TreePricer::new();
    calculator.calculate_oas(bond, market_context, as_of, clean_price)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::instruments::fixed_income::bond::CallPutSchedule;
    use finstack_core::math::interp::InterpStyle;
    use finstack_core::types::CurveId;
    use time::Month;
    fn create_test_bond() -> Bond {
        use crate::instruments::fixed_income::bond::CashflowSpec;

        let issue = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let maturity = Date::from_calendar_date(2030, Month::January, 1).expect("Valid test date");

        Bond::builder()
            .id("TEST_BOND".into())
            .notional(Money::new(1000.0, finstack_core::currency::Currency::USD))
            .issue_date(issue)
            .maturity(maturity)
            .cashflow_spec(CashflowSpec::fixed(
                0.05,
                finstack_core::dates::Tenor::semi_annual(),
                finstack_core::dates::DayCount::Act365F,
            ))
            .discount_curve_id("USD-OIS".into())
            .credit_curve_id_opt(None)
            .pricing_overrides(PricingOverrides::default().with_clean_price(98.5))
            .call_put_opt(None)
            .custom_cashflows_opt(None)
            .attributes(Default::default())
            .settlement_convention_opt(Some(
                crate::instruments::fixed_income::bond::BondSettlementConvention {
                    settlement_days: 2,
                    ..Default::default()
                },
            ))
            .build()
            .expect("Bond builder should succeed with valid test data")
    }
    fn create_callable_bond() -> Bond {
        let mut bond = create_test_bond();
        let call_date = Date::from_calendar_date(2027, Month::January, 1).expect("Valid test date");
        let mut call_put = CallPutSchedule::default();
        call_put.calls.push(CallPut {
            date: call_date,
            price_pct_of_par: 102.0,
            end_date: None,
            make_whole: None,
        });
        bond.call_put = Some(call_put);
        bond
    }
    fn create_make_whole_callable_bond() -> Bond {
        let mut bond = create_test_bond();
        let call_date = Date::from_calendar_date(2027, Month::January, 1).expect("Valid test date");
        let mut call_put = CallPutSchedule::default();
        call_put.calls.push(CallPut {
            date: call_date,
            price_pct_of_par: 102.0,
            end_date: None,
            make_whole: Some(crate::instruments::fixed_income::bond::MakeWholeSpec {
                reference_curve_id: CurveId::from("USD-TSY"),
                spread_bps: 25.0,
            }),
        });
        bond.call_put = Some(call_put);
        bond
    }
    fn create_test_market_context() -> MarketContext {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let discount_curve =
            finstack_core::market_data::term_structures::DiscountCurve::builder("USD-OIS")
                .base_date(base_date)
                .knots([(0.0, 1.0), (1.0, 0.96), (5.0, 0.85), (10.0, 0.70)])
                .interp(InterpStyle::LogLinear)
                .build()
                .expect("DiscountCurve builder should succeed with valid test data");
        let treasury_curve =
            finstack_core::market_data::term_structures::DiscountCurve::builder("USD-TSY")
                .base_date(base_date)
                .knots([(0.0, 1.0), (1.0, 0.985), (5.0, 0.93), (10.0, 0.86)])
                .interp(InterpStyle::LogLinear)
                .build()
                .expect("Treasury curve should build");
        MarketContext::new()
            .insert(discount_curve)
            .insert(treasury_curve)
    }
    #[test]
    fn test_bond_valuator_creation() {
        let bond = create_test_bond();
        let market_context = create_test_market_context();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let valuator = BondValuator::new(bond, &market_context, as_of, 5.0, 50);
        assert!(valuator.is_ok());
        let valuator = valuator.expect("BondValuator creation should succeed in test");
        // Verify coupons were distributed across the vector
        assert!(valuator.cashflow_vec.iter().any(|&c| c > 0.0));
        assert!(market_context.get_discount("USD-OIS").is_ok());
    }
    #[test]
    #[cfg(feature = "slow")]
    fn test_oas_calculator_plain_bond() {
        let bond = create_test_bond();
        let market_context = create_test_market_context();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let calculator = TreePricer::new();
        let oas = calculator.calculate_oas(&bond, &market_context, as_of, 98.5);
        assert!(oas.is_ok());
        let oas_bp = oas.expect("OAS calculation should succeed in test");
        assert!(oas_bp > 0.0);
        assert!(oas_bp < 5000.0);
    }
    #[test]
    #[cfg(feature = "slow")]
    fn test_oas_calculator_callable_bond() {
        let bond = create_callable_bond();
        let market_context = create_test_market_context();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let calculator = TreePricer::new();
        let oas = calculator.calculate_oas(&bond, &market_context, as_of, 98.5);
        assert!(oas.is_ok());
        let oas_bp = oas.expect("OAS calculation should succeed in test");
        assert!(oas_bp > 0.0);
    }
    #[test]
    fn test_bond_valuator_with_calls() {
        let bond = create_callable_bond();
        let market_context = create_test_market_context();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let valuator = BondValuator::new(bond, &market_context, as_of, 5.0, 50)
            .expect("BondValuator creation should succeed in test");
        // Verify call option was populated in the vector
        assert!(valuator.call_vec.iter().any(|c| c.is_some()));
        // Verify no put options
        assert!(valuator.put_vec.iter().all(|p| p.is_none()));
    }

    #[test]
    fn test_bond_valuator_make_whole_call_exceeds_floor_when_reference_curve_is_low() {
        let bond = create_make_whole_callable_bond();
        let market_context = create_test_market_context();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let valuator = BondValuator::new(bond, &market_context, as_of, 5.0, 50)
            .expect("BondValuator creation should succeed in test");

        let (call_step, call_price) = valuator
            .call_vec
            .iter()
            .enumerate()
            .find_map(|(idx, price)| price.map(|value| (idx, value)))
            .expect("call price should be present");
        let floor_price = valuator.outstanding_principal_vec[call_step] * 1.02;

        assert!(
            call_price >= floor_price,
            "make-whole call price should never fall below floor: call_price={call_price}, floor={floor_price}"
        );
        assert!(
            call_price > floor_price,
            "make-whole call price should exceed floor with lower treasury curve: call_price={call_price}, floor={floor_price}"
        );
    }

    #[test]
    #[cfg(feature = "slow")]
    fn test_rates_credit_default_lowers_price() {
        use crate::instruments::common_impl::models::trees::two_factor_rates_credit::{
            RatesCreditConfig, RatesCreditTree,
        };
        use finstack_core::market_data::term_structures::HazardCurve;

        let bond = create_test_bond();
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");

        // Curves: discount + two hazard scenarios
        let discount_curve =
            finstack_core::market_data::term_structures::DiscountCurve::builder("USD-OIS")
                .base_date(base_date)
                .knots([(0.0, 1.0), (5.0, 0.85)])
                .interp(InterpStyle::LogLinear)
                .build()
                .expect("Curve builder should succeed with valid test data");

        let low_hazard = HazardCurve::builder("HAZ-LOW")
            .base_date(base_date)
            .recovery_rate(0.4)
            .knots([(0.0, 0.01), (5.0, 0.01)])
            .build()
            .expect("Curve builder should succeed with valid test data");
        let _high_hazard = HazardCurve::builder("HAZ-HIGH")
            .base_date(base_date)
            .recovery_rate(0.4)
            .knots([(0.0, 0.05), (5.0, 0.05)])
            .build()
            .expect("Curve builder should succeed with valid test data");

        let ctx_low = MarketContext::new()
            .insert(discount_curve)
            .insert(low_hazard);
        // Recreate for high scenario to avoid cloning requirements
        let discount_curve2 =
            finstack_core::market_data::term_structures::DiscountCurve::builder("USD-OIS")
                .base_date(base_date)
                .knots([(0.0, 1.0), (5.0, 0.85)])
                .interp(InterpStyle::LogLinear)
                .build()
                .expect("Curve builder should succeed with valid test data");
        let high_hazard2 =
            finstack_core::market_data::term_structures::HazardCurve::builder("HAZ-HIGH")
                .base_date(base_date)
                .recovery_rate(0.4)
                .knots([(0.0, 0.05), (5.0, 0.05)])
                .build()
                .expect("Curve builder should succeed with valid test data");
        let ctx_high = MarketContext::new()
            .insert(discount_curve2)
            .insert(high_hazard2);

        // Time grid
        let as_of = base_date;
        let time_to_maturity = bond
            .cashflow_spec
            .day_count()
            .year_fraction(
                as_of,
                bond.maturity,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);
        let steps = 40usize;

        // Valuator
        let valuator_low =
            BondValuator::new(bond.clone(), &ctx_low, as_of, time_to_maturity, steps)
                .expect("valuator");
        let valuator_high =
            BondValuator::new(bond.clone(), &ctx_high, as_of, time_to_maturity, steps)
                .expect("valuator");

        // Two-factor rates+credit trees calibrated to each hazard curve
        let disc_low = ctx_low
            .get_discount("USD-OIS")
            .expect("Discount curve should exist");
        let low_hc_ref = ctx_low
            .get_hazard("HAZ-LOW")
            .expect("Hazard curve should exist in test context");
        let mut tree_low = RatesCreditTree::new(RatesCreditConfig {
            steps,
            ..Default::default()
        });
        tree_low
            .calibrate(disc_low.as_ref(), low_hc_ref.as_ref(), time_to_maturity)
            .expect("calibration low");

        let disc_high = ctx_high
            .get_discount("USD-OIS")
            .expect("Discount curve should exist");
        let high_hc_ref = ctx_high
            .get_hazard("HAZ-HIGH")
            .expect("Hazard curve should exist in test context");
        let mut tree_high = RatesCreditTree::new(RatesCreditConfig {
            steps,
            ..Default::default()
        });
        tree_high
            .calibrate(disc_high.as_ref(), high_hc_ref.as_ref(), time_to_maturity)
            .expect("calibration high");

        let vars = StateVariables::default();

        let pv_low = tree_low
            .price(vars.clone(), time_to_maturity, &ctx_low, &valuator_low)
            .expect("price low");

        let pv_high = tree_high
            .price(vars, time_to_maturity, &ctx_high, &valuator_high)
            .expect("price high");

        // With higher hazard, price should be lower (all else equal)
        assert!(pv_high < pv_low, "pv_high={} pv_low={}", pv_high, pv_low);
    }
    #[test]
    fn test_accrued_interest_via_quote_context() {
        use crate::instruments::fixed_income::bond::pricing::settlement::QuoteDateContext;

        let bond = create_test_bond();
        let market_context = create_test_market_context();

        // At issue date, accrued at quote_date (= issue + settlement_days)
        // may be small but non-negative.
        let issue = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let ctx_issue = QuoteDateContext::new(&bond, &market_context, issue)
            .expect("QuoteDateContext should succeed in test");
        assert!(
            ctx_issue.accrued_at_quote_date >= 0.0,
            "Accrued at issue quote_date should be non-negative"
        );

        // Mid-period: accrued should be positive
        let mid_period = Date::from_calendar_date(2025, Month::April, 1).expect("Valid test date");
        let ctx_mid = QuoteDateContext::new(&bond, &market_context, mid_period)
            .expect("QuoteDateContext should succeed in test");
        assert!(
            ctx_mid.accrued_at_quote_date > 0.0,
            "Accrued mid-period should be positive"
        );
    }
}
