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
//! use finstack_valuations::instruments::bond::Bond;
//! use finstack_valuations::instruments::bond::pricing::tree_engine::TreePricer;
//! use finstack_core::market_data::context::MarketContext;
//! use finstack_core::dates::Date;
//!
//! # let bond = Bond::example();
//! # let market = MarketContext::new();
//! # let as_of = Date::from_calendar_date(2024, time::Month::January, 15).unwrap();
//! let pricer = TreePricer::new();
//! let oas_bp = pricer.calculate_oas(&bond, &market, as_of, 98.5)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # See Also
//!
//! - [`TreePricer`] for OAS calculation
//! - [`BondValuator`] for tree valuator implementation
//! - [`TreePricerConfig`] for configuration options

#![allow(clippy::module_inception)]

use super::super::types::Bond;

#[cfg(test)]
use super::super::types::CallPut;
use crate::cashflow::traits::CashflowProvider;
use crate::instruments::common::models::trees::tree_framework::state_keys as tf_keys;
use crate::instruments::common::models::trees::two_factor_rates_credit::{
    RatesCreditConfig, RatesCreditTree,
};
use crate::instruments::common::models::{
    short_rate_keys, NodeState, ShortRateTree, ShortRateTreeConfig, StateVariables, TreeModel,
    TreeValuator,
};
#[cfg(test)]
use crate::instruments::PricingOverrides;
use finstack_core::collections::HashMap;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::math::solver::{BrentSolver, Solver};
use finstack_core::Result;

#[cfg(test)]
use finstack_core::money::Money;

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
/// use finstack_valuations::instruments::bond::pricing::tree_engine::TreePricerConfig;
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
/// // Custom configuration
/// let custom = TreePricerConfig {
///     tree_steps: 200,
///     volatility: 0.015,  // 150 bps normal vol for Ho-Lee
///     tolerance: 1e-8,
///     max_iterations: 100,
///     initial_bracket_size_bp: Some(2000.0),
/// };
/// ```
#[derive(Clone, Debug)]
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
        }
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
    /// use finstack_valuations::instruments::bond::pricing::tree_engine::TreePricerConfig;
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
    /// use finstack_valuations::instruments::bond::pricing::tree_engine::TreePricerConfig;
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
    /// use finstack_valuations::instruments::bond::pricing::tree_engine::TreePricerConfig;
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
    /// use finstack_valuations::instruments::bond::pricing::tree_engine::TreePricerConfig;
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
        }
    }
}

/// Bond valuator for tree-based pricing of callable/putable bonds.
///
/// Implements [`TreeValuator`] trait for backward induction pricing with embedded options.
/// Maps bond cashflows and call/put schedules to tree time steps and handles
/// exercise decisions during backward induction.
///
/// # Examples
///
/// ```rust,no_run
/// use finstack_valuations::instruments::bond::Bond;
/// use finstack_valuations::instruments::bond::pricing::tree_engine::BondValuator;
/// use finstack_core::market_data::context::MarketContext;
/// use finstack_core::dates::Date;
///
/// # let bond = Bond::example();
/// # let market = MarketContext::new();
/// # let as_of = Date::from_calendar_date(2024, time::Month::January, 15).unwrap();
/// let valuator = BondValuator::new(bond, &market, as_of, 5.0, 100)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct BondValuator {
    bond: Bond,
    /// Coupon amounts indexed by time step
    coupon_map: HashMap<usize, f64>,
    /// Call prices indexed by time step (if callable)
    call_map: HashMap<usize, f64>,
    /// Put prices indexed by time step (if putable)
    put_map: HashMap<usize, f64>,
    /// Time steps for tree pricing
    time_steps: Vec<f64>,
    /// Optional recovery rate sourced from a hazard curve in MarketContext
    recovery_rate: Option<f64>,
}

impl BondValuator {
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
    /// use finstack_valuations::instruments::bond::Bond;
    /// use finstack_valuations::instruments::bond::pricing::tree_engine::BondValuator;
    /// use finstack_core::market_data::context::MarketContext;
    /// use finstack_core::dates::Date;
    ///
    /// # let bond = Bond::example();
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
        let dt = time_to_maturity / tree_steps as f64;
        let time_steps: Vec<f64> = (0..=tree_steps).map(|i| i as f64 * dt).collect();

        let curves = market_context;
        let discount_curve = market_context.get_discount(&bond.discount_curve_id)?;
        let dc_curve = discount_curve.day_count();
        let flows = bond.build_schedule(curves, as_of)?;

        let mut coupon_map = HashMap::default();
        for (date, amount) in &flows {
            if *date > as_of {
                let time_frac = dc_curve.year_fraction(
                    as_of,
                    *date,
                    finstack_core::dates::DayCountCtx::default(),
                )?;
                // Distributed mapping: spread cashflow between two nearest time steps
                // to reduce discretization error and improve convergence.
                let raw = (time_frac / time_to_maturity) * tree_steps as f64;

                // Ensure we don't go out of bounds
                let raw_clamped = raw.clamp(0.0, tree_steps as f64);

                // Lower step index
                let step_idx = raw_clamped.floor() as usize;

                // Weight for the upper step (fractional part)
                let weight = raw_clamped - step_idx as f64;

                // Distribute to step_idx (weight: 1.0 - weight)
                if step_idx > 0 {
                    *coupon_map.entry(step_idx).or_insert(0.0) += amount.amount() * (1.0 - weight);
                }

                // Distribute to step_idx + 1 (weight: weight)
                if step_idx < tree_steps {
                    *coupon_map.entry(step_idx + 1).or_insert(0.0) += amount.amount() * weight;
                }
            }
        }

        let mut call_map = HashMap::default();
        let mut put_map = HashMap::default();
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
                    if step > tree_steps {
                        step = tree_steps;
                    }
                    let call_price = bond.notional.amount() * (call.price_pct_of_par / 100.0);
                    call_map.insert(step, call_price);
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
                    if step > tree_steps {
                        step = tree_steps;
                    }
                    let put_price = bond.notional.amount() * (put.price_pct_of_par / 100.0);
                    put_map.insert(step, put_price);
                }
            }
        }

        // Source recovery rate from the hazard curve identified by the bond's credit_curve_id
        // (if present), otherwise try the discount curve ID with fallback "-CREDIT" suffix.
        // This ensures consistency with the hazard curve used for rates+credit tree calibration.
        let mut recovery_rate: Option<f64> = None;
        if let Some(ref credit_id) = bond.credit_curve_id {
            if let Ok(hc) = market_context.get_hazard(credit_id.as_str()) {
                recovery_rate = Some(hc.recovery_rate());
            }
        }

        Ok(Self {
            bond,
            coupon_map,
            call_map,
            put_map,
            time_steps,
            recovery_rate,
        })
    }
}

impl TreeValuator for BondValuator {
    fn value_at_maturity(&self, _state: &NodeState) -> Result<f64> {
        let final_step = self.time_steps.len() - 1;
        let cashflow = self.coupon_map.get(&final_step).copied().unwrap_or(0.0);
        Ok(cashflow)
    }

    fn value_at_node(&self, state: &NodeState, continuation_value: f64, dt: f64) -> Result<f64> {
        let step = state.step;
        let coupon = self.coupon_map.get(&step).copied().unwrap_or(0.0);

        // Alive (no default) value at end of the step including coupon, with call/put decisions
        let mut alive_value = continuation_value + coupon;
        if let Some(&put_price) = self.put_map.get(&step) {
            alive_value = alive_value.max(put_price);
        }
        if let Some(&call_price) = self.call_map.get(&step) {
            alive_value = alive_value.min(call_price);
        }

        // Default handling: if hazard rate is present, compute survival/default weighting
        // Use cached fields instead of hash lookups for performance
        if let Some(hazard) = state.hazard_rate {
            let df = state.df.unwrap_or(1.0);
            let p_surv = (-hazard.max(0.0) * dt).exp();
            let default_prob = (1.0 - p_surv).clamp(0.0, 1.0);
            let recovery = self
                .recovery_rate
                .map(|rr| rr.clamp(0.0, 1.0) * self.bond.notional.amount())
                .unwrap_or(0.0);
            let node_value = p_surv * alive_value + default_prob * df * recovery;
            Ok(node_value)
        } else {
            // No hazard info at this node; return alive path value
            Ok(alive_value)
        }
    }
}

/// Tree-based pricer for bonds with embedded options and OAS calculations.
///
/// Provides methods for calculating option-adjusted spread (OAS) for bonds with
/// embedded call/put options. Automatically selects between short-rate and
/// rates+credit tree models based on available market data.
///
/// # Examples
///
/// ```rust,no_run
/// use finstack_valuations::instruments::bond::Bond;
/// use finstack_valuations::instruments::bond::pricing::tree_engine::TreePricer;
/// use finstack_core::market_data::context::MarketContext;
/// use finstack_core::dates::Date;
///
/// # let bond = Bond::example();
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
    /// use finstack_valuations::instruments::bond::pricing::tree_engine::TreePricer;
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
    /// use finstack_valuations::instruments::bond::pricing::tree_engine::{TreePricer, TreePricerConfig};
    ///
    /// let config = TreePricerConfig {
    ///     tree_steps: 200,
    ///     volatility: 0.015,
    ///     tolerance: 1e-8,
    ///     max_iterations: 100,
    ///     initial_bracket_size_bp: Some(2000.0),
    /// };
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
    /// use finstack_valuations::instruments::bond::Bond;
    /// use finstack_valuations::instruments::bond::pricing::tree_engine::TreePricer;
    /// use finstack_core::market_data::context::MarketContext;
    /// use finstack_core::dates::Date;
    ///
    /// # let bond = Bond::example();
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
        // clean_price_pct_of_par is expected to be the CLEAN price quoted in percent of par.
        // Convert to currency and add accrued interest (currency) to form the dirty target.
        let accrued_ccy = self.calculate_accrued_interest(bond, market_context, as_of)?;
        let dirty_target = (clean_price_pct_of_par * bond.notional.amount() / 100.0) + accrued_ccy;
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
                ..Default::default()
            };
            let mut tree = RatesCreditTree::new(cfg);
            let _recovery = tree.align_hazard_from_curve(hc);
            rc_tree = Some(tree);
            use_rates_credit = true;
        }

        let mut sr_tree: Option<ShortRateTree> = None;
        if !use_rates_credit {
            let tree_config = ShortRateTreeConfig {
                steps: self.config.tree_steps,
                volatility: self.config.volatility,
                ..Default::default()
            };
            let mut tree = ShortRateTree::new(tree_config);
            tree.calibrate(discount_curve.as_ref(), time_to_maturity)?;
            sr_tree = Some(tree);
        }

        let valuator = BondValuator::new(
            bond.clone(),
            market_context,
            as_of,
            time_to_maturity,
            self.config.tree_steps,
        )?;

        // Get initial short rate for state variables (needed by tree framework)
        let initial_rate = if let Some(tree) = sr_tree.as_ref() {
            tree.rate_at_node(0, 0).unwrap_or(0.03)
        } else {
            discount_curve.zero(0.0)
        };

        let objective_fn = |oas: f64| -> f64 {
            // `oas` is treated in basis points (bp) to match `short_rate_keys::OAS`
            // semantics in the short-rate tree. When using the rates+credit tree,
            // we convert bp → decimal and add it to the short rate passed via
            // `INTEREST_RATE`.
            let mut vars = StateVariables::default();
            if use_rates_credit {
                let base_rate = discount_curve.zero(0.0);
                let oas_bp = oas;
                let rate_with_oas = base_rate + oas_bp / 10_000.0;
                vars.insert(tf_keys::INTEREST_RATE, rate_with_oas);
                if let Some(hc) = hazard_curve.as_ref() {
                    // Use first knot hazard as base
                    if let Some((_, lambda0)) = hc.knot_points().next() {
                        vars.insert(tf_keys::HAZARD_RATE, lambda0.max(0.0));
                    } else {
                        vars.insert(tf_keys::HAZARD_RATE, 0.01);
                    }
                } else {
                    vars.insert(tf_keys::HAZARD_RATE, 0.01);
                }
                // Let valuator handle call/put; OAS is not used here (credit spread embedded via hazard)
                if let Some(tree) = rc_tree.as_ref() {
                    match tree.price(vars, time_to_maturity, market_context, &valuator) {
                        Ok(model_price) => model_price - dirty_target,
                        Err(_) => 1.0e6,
                    }
                } else {
                    1.0e6
                }
            } else {
                // Set both the initial rate and OAS for the tree framework
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
            .with_tolerance(self.config.tolerance)
            .with_initial_bracket_size(self.config.initial_bracket_size_bp);
        // Respect the configured maximum iteration cap for OAS root-finding.
        solver.max_iterations = self.config.max_iterations;
        let initial_guess = 0.0;
        let oas_bp = solver.solve(objective_fn, initial_guess)?;
        Ok(oas_bp)
    }

    fn calculate_accrued_interest(
        &self,
        bond: &Bond,
        _market_context: &MarketContext,
        as_of: Date,
    ) -> Result<f64> {
        // Build full schedule with market context and use generic accrual engine
        let schedule = bond.get_full_schedule(_market_context)?;
        crate::cashflow::accrual::accrued_interest_amount(&schedule, as_of, &bond.accrual_config())
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
/// use finstack_valuations::instruments::bond::Bond;
/// use finstack_valuations::instruments::bond::pricing::tree_engine::calculate_oas;
/// use finstack_core::market_data::context::MarketContext;
/// use finstack_core::dates::Date;
///
/// # let bond = Bond::example();
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
    use crate::instruments::bond::CallPutSchedule;
    use finstack_core::math::interp::InterpStyle;
    use time::Month;
    fn create_test_bond() -> Bond {
        use crate::instruments::bond::CashflowSpec;

        let issue = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let maturity = Date::from_calendar_date(2030, Month::January, 1).expect("Valid test date");

        Bond::builder()
            .id("TEST_BOND".into())
            .notional(Money::new(1000.0, finstack_core::currency::Currency::USD))
            .issue(issue)
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
            .settlement_days_opt(Some(2))
            .ex_coupon_days_opt(Some(0))
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
        });
        bond.call_put = Some(call_put);
        bond
    }
    fn create_test_market_context() -> MarketContext {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let discount_curve =
            finstack_core::market_data::term_structures::discount_curve::DiscountCurve::builder(
                "USD-OIS",
            )
            .base_date(base_date)
            .knots([(0.0, 1.0), (1.0, 0.96), (5.0, 0.85), (10.0, 0.70)])
            .set_interp(InterpStyle::LogLinear)
            .build()
            .expect("DiscountCurve builder should succeed with valid test data");
        MarketContext::new().insert_discount(discount_curve)
    }
    #[test]
    fn test_bond_valuator_creation() {
        let bond = create_test_bond();
        let market_context = create_test_market_context();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let valuator = BondValuator::new(bond, &market_context, as_of, 5.0, 50);
        assert!(valuator.is_ok());
        let valuator = valuator.expect("BondValuator creation should succeed in test");
        assert!(!valuator.coupon_map.is_empty());
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
        assert!(!valuator.call_map.is_empty());
        assert!(valuator.put_map.is_empty());
    }

    #[test]
    #[cfg(feature = "slow")]
    fn test_rates_credit_default_lowers_price() {
        use crate::instruments::common::models::trees::tree_framework::state_keys as tf_keys;
        use crate::instruments::common::models::trees::two_factor_rates_credit::{
            RatesCreditConfig, RatesCreditTree,
        };
        use finstack_core::market_data::term_structures::hazard_curve::HazardCurve;

        let bond = create_test_bond();
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");

        // Curves: discount + two hazard scenarios
        let discount_curve =
            finstack_core::market_data::term_structures::discount_curve::DiscountCurve::builder(
                "USD-OIS",
            )
            .base_date(base_date)
            .knots([(0.0, 1.0), (5.0, 0.85)])
            .set_interp(InterpStyle::LogLinear)
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
            .insert_discount(discount_curve)
            .insert_hazard(low_hazard);
        // Recreate for high scenario to avoid cloning requirements
        let discount_curve2 =
            finstack_core::market_data::term_structures::discount_curve::DiscountCurve::builder(
                "USD-OIS",
            )
            .base_date(base_date)
            .knots([(0.0, 1.0), (5.0, 0.85)])
            .set_interp(InterpStyle::LogLinear)
            .build()
            .expect("Curve builder should succeed with valid test data");
        let high_hazard2 =
            finstack_core::market_data::term_structures::hazard_curve::HazardCurve::builder(
                "HAZ-HIGH",
            )
            .base_date(base_date)
            .recovery_rate(0.4)
            .knots([(0.0, 0.05), (5.0, 0.05)])
            .build()
            .expect("Curve builder should succeed with valid test data");
        let ctx_high = MarketContext::new()
            .insert_discount(discount_curve2)
            .insert_hazard(high_hazard2);

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

        // Two-factor rates+credit trees aligned to each hazard curve
        let mut tree_low = RatesCreditTree::new(RatesCreditConfig {
            steps,
            ..Default::default()
        });
        // Align to the hazard curve stored in the context
        let low_hc_ref = ctx_low
            .get_hazard_ref("HAZ-LOW")
            .expect("Hazard curve should exist in test context");
        tree_low.align_hazard_from_curve(low_hc_ref);
        let mut tree_high = RatesCreditTree::new(RatesCreditConfig {
            steps,
            ..Default::default()
        });
        let high_hc_ref = ctx_high
            .get_hazard_ref("HAZ-HIGH")
            .expect("Hazard curve should exist in test context");
        tree_high.align_hazard_from_curve(high_hc_ref);

        // Initial state
        let mut vars = StateVariables::default();
        vars.insert(tf_keys::INTEREST_RATE, 0.03);
        vars.insert(tf_keys::HAZARD_RATE, 0.01);

        let pv_low = tree_low
            .price(vars.clone(), time_to_maturity, &ctx_low, &valuator_low)
            .expect("price low");

        // Use higher base hazard for the high scenario
        let mut vars_high = vars.clone();
        vars_high.insert(tf_keys::HAZARD_RATE, 0.05);
        let pv_high = tree_high
            .price(vars_high, time_to_maturity, &ctx_high, &valuator_high)
            .expect("price high");

        // With higher hazard, price should be lower (all else equal)
        assert!(pv_high < pv_low, "pv_high={} pv_low={}", pv_high, pv_low);
    }
    #[test]
    fn test_accrued_interest_calculation() {
        let bond = create_test_bond();
        let market_context = create_test_market_context();
        let calculator = TreePricer::new();
        let coupon_date = Date::from_calendar_date(2025, Month::July, 1).expect("Valid test date");
        let accrued = calculator
            .calculate_accrued_interest(&bond, &market_context, coupon_date)
            .expect("Accrued interest calculation should succeed in test");
        assert!(accrued.abs() < 1e-6);
        let mid_period = Date::from_calendar_date(2025, Month::April, 1).expect("Valid test date");
        let accrued_mid = calculator
            .calculate_accrued_interest(&bond, &market_context, mid_period)
            .expect("Accrued interest calculation should succeed in test");
        assert!(accrued_mid > 0.0);
    }
}
