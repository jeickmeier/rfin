use crate::instruments::common::models::trees::{HullWhiteTree, HullWhiteTreeConfig};
use crate::instruments::common::traits::Instrument;
use crate::instruments::swaption::pricing::BermudanSwaptionTreeValuator;
use crate::instruments::swaption::{BermudanSwaption, Swaption};
use crate::instruments::pricing_overrides::VolSurfaceExtrapolation;
use crate::pricer::{InstrumentType, ModelKey, Pricer, PricerKey, PricingError};
use crate::results::ValuationResult;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use std::sync::Arc;

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
                            VolSurfaceExtrapolation::Error => {
                                vol_surface
                                    .value_checked(time_to_expiry, swaption.strike_rate)
                                    .map_err(|e| PricingError::missing_market_data(e.to_string()))?
                            }
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

// ========================= LSMC PRICER FOR BERMUDAN EXERCISE =========================

#[cfg(feature = "mc")]
/// Longstaff-Schwartz Monte Carlo pricer for Bermudan swaptions.
pub struct SwaptionLsmcPricer {
    #[allow(dead_code)] // Will be used when full LSMC implementation is added
    num_paths: usize,
    #[allow(dead_code)] // Will be used when full LSMC implementation is added
    seed: u64,
}

#[cfg(feature = "mc")]
impl SwaptionLsmcPricer {
    /// Create a new LSMC pricer with default config.
    pub fn new() -> Self {
        Self {
            num_paths: 100_000,
            seed: 42,
        }
    }

    /// Create with custom configuration.
    pub fn with_config(num_paths: usize, seed: u64) -> Self {
        Self { num_paths, seed }
    }
}

#[cfg(feature = "mc")]
impl Default for SwaptionLsmcPricer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "mc")]
impl Pricer for SwaptionLsmcPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::Swaption, ModelKey::MonteCarloGBM)
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

        // LSMC (Longstaff-Schwartz Monte Carlo) is not yet implemented.
        // This is an experimental feature placeholder.
        //
        // TODO(LSMC): Full implementation requires the following components:
        //
        // 1. **Path Generation**: Simulate short-rate or forward-rate paths using:
        //    - Hull-White 1F/2F model (for consistency with tree pricer)
        //    - Or LIBOR Market Model (LMM) for multi-factor dynamics
        //    Use `num_paths` and `seed` from self for reproducibility.
        //
        // 2. **Exercise Schedule**: Build exercise dates from the underlying swap:
        //    - Extract all coupon reset dates as potential exercise points
        //    - Map dates to time grid for simulation
        //
        // 3. **Payoff Calculation**: At each exercise date t_i:
        //    - Compute swap NPV conditional on rate path
        //    - Payoff = max(0, swap_npv) for receiver, or max(0, -swap_npv) for payer
        //
        // 4. **Regression (Longstaff-Schwartz)**:
        //    - At each exercise date (backward from expiry):
        //      a) Select in-the-money paths
        //      b) Regress discounted future cashflows on basis functions of state
        //         (e.g., polynomial in short rate: 1, r, r², ...)
        //      c) Compare regression estimate (continuation value) with exercise value
        //      d) Update optimal exercise decision
        //
        // 5. **Final Valuation**: Average discounted payoffs across paths.
        //
        // Reference: Longstaff, F.A. & Schwartz, E.S. (2001). "Valuing American Options
        // by Simulation: A Simple Least-Squares Approach." Review of Financial Studies.
        Err(PricingError::model_failure(
            "LSMC pricing not implemented (experimental). Use HullWhiteTree for Bermudan swaptions.",
        ))
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
    #[allow(dead_code)]
    mc_paths: usize,
    /// Random seed (for LSMC method)
    #[allow(dead_code)]
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
        result
            .measures
            .insert("used_cached_model".to_string(), if used_cached_model { 1.0 } else { 0.0 });

        Ok(result)
    }

    /// Price using LSMC.
    /// Price using LSMC (Longstaff-Schwartz Monte Carlo).
    ///
    /// # Current Status
    ///
    /// LSMC is not yet implemented and currently falls back to tree pricing.
    /// This fallback ensures API stability while the LSMC implementation is developed.
    ///
    /// # TODO: LSMC Implementation
    ///
    /// The full LSMC implementation (in progress) will provide:
    /// - **Path generation**: Hull-White or LMM rate simulation
    /// - **Regression**: Polynomial basis functions for continuation value
    /// - **Variance reduction**: Control variates, antithetic sampling
    /// - **Convergence diagnostics**: Standard error estimation
    ///
    /// For production Bermudan swaption pricing, use `HullWhiteTree` method
    /// which is fully implemented and industry-standard.
    fn price_lsmc(
        &self,
        swaption: &BermudanSwaption,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> Result<ValuationResult, PricingError> {
        // TODO(LSMC): Implement full Longstaff-Schwartz Monte Carlo.
        // Currently falls back to tree pricing for API stability.
        // See SwaptionLsmcPricer::price_dyn for detailed implementation roadmap.
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
            BermudanPricingMethod::LSMC => {
                PricerKey::new(InstrumentType::BermudanSwaption, ModelKey::MonteCarloGBM)
            }
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
