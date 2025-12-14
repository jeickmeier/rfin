use crate::instruments::common::models::trees::{HullWhiteTree, HullWhiteTreeConfig};
use crate::instruments::common::traits::Instrument;
use crate::instruments::pricing_overrides::VolSurfaceExtrapolation;
use crate::instruments::swaption::pricing::BermudanSwaptionTreeValuator;
use crate::instruments::swaption::{BermudanSwaption, Swaption};
use crate::pricer::{InstrumentType, ModelKey, Pricer, PricerKey, PricingError};
use crate::results::ValuationResult;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use std::sync::Arc;

// LSMC imports (gated by feature)
#[cfg(feature = "mc")]
use crate::instruments::common::mc::process::ou::{calibrate_theta_from_curve, HullWhite1FProcess};
#[cfg(feature = "mc")]
use crate::instruments::common::models::monte_carlo::payoff::swaption::{
    BermudanSwaptionPayoff, SwapSchedule, SwaptionType,
};
#[cfg(feature = "mc")]
use crate::instruments::common::models::monte_carlo::pricer::basis::PolynomialBasis;
#[cfg(feature = "mc")]
use crate::instruments::common::models::monte_carlo::pricer::swaption_lsmc::{
    SwaptionLsmcConfig, SwaptionLsmcPricer as SharedSwaptionLsmcPricer,
};
#[cfg(feature = "mc")]
use crate::instruments::common::parameters::OptionType;

// ========================= NEW SIMPLIFIED PRICER =========================

/// New simplified Swaption pricer supporting multiple models.
pub struct SimpleSwaptionBlackPricer {
    model: ModelKey,
}

impl SimpleSwaptionBlackPricer {
    /// Create a new swaption pricer with default Black76 model
    pub fn new() -> Self {
        Self {
            model: ModelKey::Black76,
        }
    }

    /// Create a swaption pricer with specified model key
    pub fn with_model(model: ModelKey) -> Self {
        Self { model }
    }
}

impl Default for SimpleSwaptionBlackPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for SimpleSwaptionBlackPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::Swaption, self.model)
    }

    #[allow(unused_variables)]
    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> Result<ValuationResult, PricingError> {
        // Type-safe downcasting
        let swaption = instrument
            .as_any()
            .downcast_ref::<Swaption>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::Swaption, instrument.key())
            })?;

        // Explicit dispatch based on PRICER configuration
        // If model is Black76, we enforce Black pricing regardless of instrument preference
        let pv = match self.model {
            ModelKey::Black76 => {
                let disc = market
                    .get_discount_ref(swaption.discount_curve_id.as_ref())
                    .map_err(|e| PricingError::model_failure(e.to_string()))?;

                // Use SABR if available (implies Black vol in this library), otherwise look up surface
                if swaption.sabr_params.is_some() {
                    swaption
                        .price_sabr(disc, as_of)
                        .map_err(|e| PricingError::model_failure(e.to_string()))?
                } else {
                    let time_to_expiry = swaption
                        .year_fraction(as_of, swaption.expiry, swaption.day_count)
                        .map_err(|e| PricingError::model_failure(e.to_string()))?;

                    let vol_surface = market
                        .surface_ref(swaption.vol_surface_id.as_str())
                        .map_err(|e| PricingError::missing_market_data(e.to_string()))?;

                    let vol = if let Some(impl_vol) = swaption.pricing_overrides.implied_volatility
                    {
                        impl_vol
                    } else {
                        match swaption.pricing_overrides.vol_surface_extrapolation {
                            VolSurfaceExtrapolation::Clamp => {
                                vol_surface.value_clamped(time_to_expiry, swaption.strike_rate)
                            }
                            VolSurfaceExtrapolation::Error => vol_surface
                                .value_checked(time_to_expiry, swaption.strike_rate)
                                .map_err(|e| PricingError::missing_market_data(e.to_string()))?,
                        }
                    };

                    swaption
                        .price_black(disc, vol, as_of)
                        .map_err(|e| PricingError::model_failure(e.to_string()))?
                }
            }
            // For Discounting or other models, fallback to instrument's internal preference
            // (which might be Normal/Bachelier)
            _ => swaption
                .value(market, as_of)
                .map_err(|e| PricingError::model_failure(e.to_string()))?,
        };

        // Return stamped result
        Ok(ValuationResult::stamped(swaption.id(), as_of, pv))
    }
}

// ========================= BERMUDAN SWAPTION PRICER =========================

/// Pricing method for Bermudan swaptions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BermudanPricingMethod {
    /// Hull-White trinomial tree (industry standard, faster)
    #[default]
    HullWhiteTree,
    /// Longstaff-Schwartz Monte Carlo (more flexible)
    LSMC,
}

/// Hull-White model parameters for Bermudan swaption pricing.
#[derive(Clone, Debug)]
pub struct HullWhiteParams {
    /// Mean reversion speed (κ)
    pub kappa: f64,
    /// Short rate volatility (σ)
    pub sigma: f64,
}

impl Default for HullWhiteParams {
    fn default() -> Self {
        Self {
            kappa: 0.03, // 3% mean reversion
            sigma: 0.01, // 100 bps volatility
        }
    }
}

impl HullWhiteParams {
    /// Create new Hull-White parameters.
    pub fn new(kappa: f64, sigma: f64) -> Self {
        Self { kappa, sigma }
    }

    /// Create tree configuration.
    pub fn to_tree_config(&self, steps: usize) -> HullWhiteTreeConfig {
        HullWhiteTreeConfig::new(self.kappa, self.sigma, steps)
    }
}

/// Pricer for Bermudan swaptions using Hull-White tree or LSMC.
///
/// # Model Reuse
///
/// For portfolio pricing, calibrate the Hull-White model once and reuse it
/// across multiple instruments using [`with_calibrated_model`]:
///
/// ```rust,ignore
/// use finstack_valuations::instruments::swaption::pricer::{
///     BermudanSwaptionPricer, HullWhiteParams,
/// };
/// use finstack_valuations::instruments::common::models::trees::{HullWhiteTree, HullWhiteTreeConfig};
/// use std::sync::Arc;
///
/// // Calibrate once
/// let config = HullWhiteParams::default().to_tree_config(100);
/// let tree = Arc::new(HullWhiteTree::calibrate(config, &disc, ttm).unwrap());
///
/// // Reuse for portfolio
/// for swaption in portfolio {
///     let pricer = BermudanSwaptionPricer::tree_pricer(HullWhiteParams::default())
///         .with_calibrated_model(tree.clone());
///     pricer.price_dyn(&swaption, &market, as_of);
/// }
/// ```
///
/// # Example
///
/// ```rust,no_run
/// use finstack_valuations::instruments::swaption::pricer::{
///     BermudanSwaptionPricer, BermudanPricingMethod, HullWhiteParams,
/// };
///
/// // Create tree-based pricer with default parameters
/// let pricer = BermudanSwaptionPricer::tree_pricer(HullWhiteParams::default());
///
/// // Create LSMC pricer
/// let lsmc_pricer = BermudanSwaptionPricer::lsmc_pricer(HullWhiteParams::default());
/// ```
pub struct BermudanSwaptionPricer {
    /// Pricing method
    method: BermudanPricingMethod,
    /// Hull-White parameters
    hw_params: HullWhiteParams,
    /// Number of tree steps (for tree method)
    tree_steps: usize,
    /// Number of MC paths (for LSMC method)
    mc_paths: usize,
    /// Random seed (for LSMC method)
    mc_seed: u64,
    /// Pre-calibrated Hull-White tree for model reuse.
    ///
    /// When set, the pricer skips calibration and uses this model directly.
    /// This enables O(1) pricing per instrument instead of O(Steps × Time) calibration.
    pre_calibrated_model: Option<Arc<HullWhiteTree>>,
}

impl BermudanSwaptionPricer {
    /// Create a Hull-White tree pricer.
    pub fn tree_pricer(hw_params: HullWhiteParams) -> Self {
        Self {
            method: BermudanPricingMethod::HullWhiteTree,
            hw_params,
            tree_steps: 100,
            mc_paths: 50_000,
            mc_seed: 42,
            pre_calibrated_model: None,
        }
    }

    /// Create an LSMC pricer.
    pub fn lsmc_pricer(hw_params: HullWhiteParams) -> Self {
        Self {
            method: BermudanPricingMethod::LSMC,
            hw_params,
            tree_steps: 100,
            mc_paths: 50_000,
            mc_seed: 42,
            pre_calibrated_model: None,
        }
    }

    /// Set number of tree steps.
    pub fn with_tree_steps(mut self, steps: usize) -> Self {
        self.tree_steps = steps;
        self
    }

    /// Set number of Monte Carlo paths.
    pub fn with_mc_paths(mut self, paths: usize) -> Self {
        self.mc_paths = paths;
        self
    }

    /// Set random seed.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.mc_seed = seed;
        self
    }

    /// Use a pre-calibrated Hull-White model for pricing.
    ///
    /// This enables model reuse across a portfolio, eliminating the need
    /// to re-calibrate for each instrument. The model should be calibrated
    /// with appropriate parameters for the instruments being priced.
    ///
    /// # Performance
    ///
    /// Using a pre-calibrated model reduces pricing complexity from
    /// O(Steps × Time) per instrument to O(1) per instrument.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use std::sync::Arc;
    ///
    /// // Calibrate once
    /// let config = HullWhiteParams::default().to_tree_config(100);
    /// let tree = Arc::new(HullWhiteTree::calibrate(config, &disc, ttm)?);
    ///
    /// // Price portfolio with reused model
    /// let pricer = BermudanSwaptionPricer::tree_pricer(HullWhiteParams::default())
    ///     .with_calibrated_model(tree);
    /// ```
    pub fn with_calibrated_model(mut self, model: Arc<HullWhiteTree>) -> Self {
        self.pre_calibrated_model = Some(model);
        self
    }

    /// Get the pre-calibrated model, if set.
    pub fn calibrated_model(&self) -> Option<&Arc<HullWhiteTree>> {
        self.pre_calibrated_model.as_ref()
    }

    /// Price using Hull-White tree.
    ///
    /// If a pre-calibrated model is set via [`with_calibrated_model`], it will
    /// be used directly, skipping the calibration step.
    fn price_tree(
        &self,
        swaption: &BermudanSwaption,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> Result<ValuationResult, PricingError> {
        // Get discount curve
        let disc = market
            .get_discount_ref(swaption.discount_curve_id.as_str())
            .map_err(|e| PricingError::missing_market_data(e.to_string()))?;

        // Calculate time to maturity
        let ttm = swaption
            .time_to_maturity(as_of)
            .map_err(|e| PricingError::model_failure(e.to_string()))?;

        if ttm <= 0.0 {
            // Expired - return zero
            return Ok(ValuationResult::stamped(
                swaption.id.as_str(),
                as_of,
                Money::new(0.0, swaption.notional.currency()),
            ));
        }

        // Use pre-calibrated model if available, otherwise calibrate a new one
        let (pv, used_cached_model) = if let Some(ref cached_tree) = self.pre_calibrated_model {
            // Use pre-calibrated model (O(1) per instrument)
            let valuator = BermudanSwaptionTreeValuator::new(swaption, cached_tree, disc, as_of)
                .map_err(|e| PricingError::model_failure(e.to_string()))?;
            (valuator.price(), true)
        } else {
            // Calibrate new model (O(Steps × Time) per instrument)
            let tree_config = self.hw_params.to_tree_config(self.tree_steps);
            let tree = HullWhiteTree::calibrate(tree_config, disc, ttm)
                .map_err(|e| PricingError::model_failure(e.to_string()))?;

            let valuator = BermudanSwaptionTreeValuator::new(swaption, &tree, disc, as_of)
                .map_err(|e| PricingError::model_failure(e.to_string()))?;
            (valuator.price(), false)
        };

        let mut result = ValuationResult::stamped(
            swaption.id.as_str(),
            as_of,
            Money::new(pv, swaption.notional.currency()),
        );

        // Record whether cached model was used (1.0 = true, 0.0 = false)
        result.measures.insert(
            "used_cached_model".to_string(),
            if used_cached_model { 1.0 } else { 0.0 },
        );

        Ok(result)
    }

    /// Price using LSMC (Longstaff-Schwartz Monte Carlo).
    ///
    /// Uses Hull-White 1F simulation with curve-calibrated θ(t) and
    /// Longstaff-Schwartz backward induction for optimal exercise decisions.
    ///
    /// # Features
    ///
    /// - Hull-White 1F short rate simulation with exact discretization
    /// - Curve-derived piecewise θ(t) for initial curve consistency
    /// - Polynomial basis functions for regression
    /// - Antithetic variates for variance reduction
    /// - Standard error estimation in results
    #[cfg(feature = "mc")]
    fn price_lsmc(
        &self,
        swaption: &BermudanSwaption,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> Result<ValuationResult, PricingError> {
        // Get discount curve
        let disc = market
            .get_discount_ref(swaption.discount_curve_id.as_str())
            .map_err(|e| PricingError::missing_market_data(e.to_string()))?;

        // Calculate time to maturity
        let ttm = swaption
            .time_to_maturity(as_of)
            .map_err(|e| PricingError::model_failure(e.to_string()))?;

        if ttm <= 0.0 {
            // Expired - return zero
            return Ok(ValuationResult::stamped(
                swaption.id.as_str(),
                as_of,
                Money::new(0.0, swaption.notional.currency()),
            ));
        }

        // Get exercise times in years
        let exercise_times = swaption
            .exercise_times(as_of)
            .map_err(|e| PricingError::model_failure(e.to_string()))?;

        if exercise_times.is_empty() {
            return Err(PricingError::model_failure(
                "No valid exercise dates for Bermudan swaption".to_string(),
            ));
        }

        // Filter exercise times to be within [0, ttm]
        let valid_exercise_times: Vec<f64> = exercise_times
            .into_iter()
            .filter(|&t| t > 0.0 && t <= ttm)
            .collect();

        if valid_exercise_times.is_empty() {
            return Err(PricingError::model_failure(
                "No exercise dates before maturity".to_string(),
            ));
        }

        // Build swap schedule (payment times and accrual fractions)
        let (payment_dates, accrual_fractions) = swaption
            .build_swap_schedule(as_of)
            .map_err(|e| PricingError::model_failure(e.to_string()))?;

        // Convert payment dates to year fractions
        let ctx = finstack_core::dates::DayCountCtx::default();
        let payment_times: Vec<f64> = payment_dates
            .iter()
            .map(|&d| swaption.day_count.year_fraction(as_of, d, ctx))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| PricingError::model_failure(e.to_string()))?;

        // Create swap schedule for MC pricer
        let swap_start_time = swaption
            .day_count
            .year_fraction(as_of, swaption.swap_start, ctx)
            .map_err(|e| PricingError::model_failure(e.to_string()))?;

        let swap_schedule =
            SwapSchedule::new(swap_start_time, ttm, payment_times, accrual_fractions);

        // Determine option type for payoff
        let option_type = match swaption.option_type {
            OptionType::Call => SwaptionType::Payer,
            OptionType::Put => SwaptionType::Receiver,
        };

        // Create Bermudan payoff
        let payoff = BermudanSwaptionPayoff::new(
            valid_exercise_times.clone(),
            swap_schedule,
            swaption.strike_rate,
            option_type,
            swaption.notional.amount(),
            swaption.notional.currency(),
        );

        // Build exercise-aligned time grid
        let (time_grid, exercise_indices) = SwaptionLsmcConfig::build_exercise_aligned_grid(
            &valid_exercise_times,
            ttm,
            2, // Minimum steps between exercise dates
        )
        .map_err(|e| PricingError::model_failure(e.to_string()))?;

        // Build θ(t) times for calibration (use grid times)
        let theta_times: Vec<f64> = (0..=time_grid.num_steps())
            .map(|i| time_grid.time(i.min(time_grid.num_steps() - 1)))
            .filter(|&t| t <= ttm)
            .collect();

        // Create discount curve function
        let discount_fn = |t: f64| disc.df(t);

        // Calibrate Hull-White parameters from discount curve
        let hw_params = calibrate_theta_from_curve(
            self.hw_params.kappa,
            self.hw_params.sigma,
            discount_fn,
            &theta_times,
        );

        // Get initial short rate from discount curve
        let dt_small = 0.01; // Small time step for initial rate
        let initial_rate = if dt_small > 0.0 {
            -disc.df(dt_small).ln() / dt_small
        } else {
            0.03
        };

        // Create Hull-White process
        let hw_process = HullWhite1FProcess::new(hw_params);

        // Create LSMC config
        let lsmc_config = SwaptionLsmcConfig::new(self.mc_paths, self.mc_seed)
            .with_basis_degree(3)
            .with_antithetic(true);

        // Create the shared LSMC pricer
        let lsmc_pricer = SharedSwaptionLsmcPricer::with_config(lsmc_config, hw_process);

        // Create basis functions
        let basis = PolynomialBasis::new(3);

        // Price using the shared LSMC engine with custom grid
        let estimate = lsmc_pricer
            .price_bermudan_with_grid(
                &payoff,
                initial_rate,
                &time_grid,
                &exercise_indices,
                &basis,
                discount_fn,
                swaption.notional.currency(),
            )
            .map_err(|e| PricingError::model_failure(e.to_string()))?;

        // Build result with diagnostics
        let mut result = ValuationResult::stamped(swaption.id.as_str(), as_of, estimate.mean);

        // Add LSMC diagnostics to measures
        result
            .measures
            .insert("lsmc_stderr".to_string(), estimate.stderr);
        result
            .measures
            .insert("lsmc_num_paths".to_string(), self.mc_paths as f64);
        result
            .measures
            .insert("lsmc_seed".to_string(), self.mc_seed as f64);
        let (ci_low, ci_high) = estimate.ci_95;
        result
            .measures
            .insert("lsmc_ci95_low".to_string(), ci_low.amount());
        result
            .measures
            .insert("lsmc_ci95_high".to_string(), ci_high.amount());

        Ok(result)
    }

    /// Fallback for when MC feature is disabled.
    #[cfg(not(feature = "mc"))]
    fn price_lsmc(
        &self,
        swaption: &BermudanSwaption,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> Result<ValuationResult, PricingError> {
        // Fall back to tree pricing when MC is disabled
        self.price_tree(swaption, market, as_of)
    }
}

impl Default for BermudanSwaptionPricer {
    fn default() -> Self {
        Self::tree_pricer(HullWhiteParams::default())
    }
}

impl Pricer for BermudanSwaptionPricer {
    fn key(&self) -> PricerKey {
        match self.method {
            BermudanPricingMethod::HullWhiteTree => {
                PricerKey::new(InstrumentType::BermudanSwaption, ModelKey::HullWhite1F)
            }
            BermudanPricingMethod::LSMC => PricerKey::new(
                InstrumentType::BermudanSwaption,
                ModelKey::MonteCarloHullWhite1F,
            ),
        }
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> Result<ValuationResult, PricingError> {
        // Type-safe downcasting
        let swaption = instrument
            .as_any()
            .downcast_ref::<BermudanSwaption>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::BermudanSwaption, instrument.key())
            })?;

        match self.method {
            BermudanPricingMethod::HullWhiteTree => self.price_tree(swaption, market, as_of),
            BermudanPricingMethod::LSMC => self.price_lsmc(swaption, market, as_of),
        }
    }
}
