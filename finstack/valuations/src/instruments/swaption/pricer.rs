use crate::instruments::common::models::trees::{HullWhiteTree, HullWhiteTreeConfig};
use crate::instruments::common::traits::Instrument;
use crate::instruments::swaption::pricing::BermudanSwaptionTreeValuator;
use crate::instruments::swaption::{BermudanSwaption, Swaption};
use crate::pricer::{InstrumentType, ModelKey, Pricer, PricerKey, PricingError};
use crate::results::ValuationResult;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;

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
                        vol_surface.value_clamped(time_to_expiry, swaption.strike_rate)
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

        // For now, delegate to existing pricer
        // TODO: Implement full LSMC pricing for Bermudan swaptions.
        // This requires:
        // 1. Constructing the underlying swap schedule with all coupon dates.
        // 2. Simulating interest rate paths (e.g., Hull-White 1F/2F or LMM).
        // 3. Implementing Longstaff-Schwartz regression to estimate continuation value.
        // 4. Handling exercise opportunities at each reset date.
        let pv = swaption
            .value(market, as_of)
            .map_err(|e| PricingError::model_failure(e.to_string()))?;

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

    /// Price using Hull-White tree.
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

        // Build Hull-White tree
        let tree_config = self.hw_params.to_tree_config(self.tree_steps);
        let tree = HullWhiteTree::calibrate(tree_config, disc, ttm)
            .map_err(|e| PricingError::model_failure(e.to_string()))?;

        // Create valuator and price
        let valuator = BermudanSwaptionTreeValuator::new(swaption, &tree, disc, as_of)
            .map_err(|e| PricingError::model_failure(e.to_string()))?;

        let pv = valuator.price();

        Ok(ValuationResult::stamped(
            swaption.id.as_str(),
            as_of,
            Money::new(pv, swaption.notional.currency()),
        ))
    }

    /// Price using LSMC.
    fn price_lsmc(
        &self,
        swaption: &BermudanSwaption,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> Result<ValuationResult, PricingError> {
        // For now, fall back to tree pricing
        // Full LSMC implementation would use SwaptionLsmcPricer from mc module
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
