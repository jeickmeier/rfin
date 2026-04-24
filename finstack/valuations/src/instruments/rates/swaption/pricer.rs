use crate::instruments::common_impl::helpers::year_fraction;
use crate::instruments::common_impl::models::trees::{HullWhiteTree, HullWhiteTreeConfig};
use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::rates::swaption::pricing::BermudanSwaptionTreeValuator;
use crate::instruments::rates::swaption::{BermudanSwaption, Swaption};
use crate::pricer::{
    InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingErrorContext,
};
use crate::results::ValuationResult;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::traits::Discounting;
use finstack_core::money::Money;
use std::sync::Arc;

// LSMC imports (gated by feature)
use crate::instruments::common_impl::parameters::OptionType;
use crate::instruments::rates::swaption::pricing::monte_carlo_lsmc::{
    SwaptionLsmcConfig, SwaptionLsmcPricer as SharedSwaptionLsmcPricer,
};
use crate::instruments::rates::swaption::pricing::monte_carlo_payoff::{
    BermudanSwaptionPayoff, SwapSchedule, SwaptionType,
};
use finstack_monte_carlo::pricer::basis::PolynomialBasis;
use finstack_monte_carlo::process::ou::{calibrate_theta_from_curve, HullWhite1FProcess};

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
                // Use SABR if available (implies Black vol in this library), otherwise look up surface
                if swaption.sabr_params.is_some() {
                    swaption.price_sabr(market, as_of).map_err(|e| {
                        PricingError::model_failure_with_context(
                            e.to_string(),
                            PricingErrorContext::default(),
                        )
                    })?
                } else {
                    let strike = swaption.strike_f64().map_err(|e| {
                        PricingError::model_failure_with_context(
                            e.to_string(),
                            PricingErrorContext::default(),
                        )
                    })?;
                    let time_to_expiry = year_fraction(swaption.day_count, as_of, swaption.expiry)
                        .map_err(|e| {
                            PricingError::model_failure_with_context(
                                e.to_string(),
                                PricingErrorContext::default(),
                            )
                        })?;

                    let vol_provider = market
                        .get_vol_provider(swaption.vol_surface_id.as_str())
                        .map_err(|e| {
                            PricingError::missing_market_data_with_context(
                                e.to_string(),
                                PricingErrorContext::default(),
                            )
                        })?;

                    let underlying_tenor =
                        year_fraction(swaption.day_count, swaption.expiry, swaption.swap_end)
                            .map_err(|e| {
                                PricingError::model_failure_with_context(
                                    e.to_string(),
                                    PricingErrorContext::default(),
                                )
                            })?;

                    let vol = if let Some(impl_vol) =
                        swaption.pricing_overrides.market_quotes.implied_volatility
                    {
                        impl_vol
                    } else {
                        vol_provider.vol_clamped(time_to_expiry, underlying_tenor, strike)
                    };

                    swaption.price_black(market, vol, as_of).map_err(|e| {
                        PricingError::model_failure_with_context(
                            e.to_string(),
                            PricingErrorContext::default(),
                        )
                    })?
                }
            }
            // For Discounting or other models, fallback to instrument's internal preference
            // (which might be Normal/Bachelier)
            _ => swaption.value(market, as_of).map_err(|e| {
                PricingError::model_failure_with_context(
                    e.to_string(),
                    PricingErrorContext::default(),
                )
            })?,
        };

        // Return stamped result
        Ok(ValuationResult::stamped(swaption.id(), as_of, pv))
    }
}

// ========================= BERMUDAN SWAPTION PRICER =========================

/// Pricing method for Bermudan swaptions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BermudanPricingMethod {
    /// Hull-White trinomial tree (industry standard, faster)
    #[default]
    HullWhiteTree,
    /// Longstaff-Schwartz Monte Carlo (more flexible)
    LSMC,
}

impl std::fmt::Display for BermudanPricingMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BermudanPricingMethod::HullWhiteTree => write!(f, "hull_white_tree"),
            BermudanPricingMethod::LSMC => write!(f, "lsmc"),
        }
    }
}

impl std::str::FromStr for BermudanPricingMethod {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let normalized = s.to_ascii_lowercase().replace('-', "_");
        match normalized.as_str() {
            "hull_white_tree" | "tree" | "hw" => Ok(Self::HullWhiteTree),
            "lsmc" | "monte_carlo" | "mc" => Ok(Self::LSMC),
            other => Err(format!(
                "Unknown Bermudan pricing method: '{}'. Valid: hull_white_tree, lsmc",
                other
            )),
        }
    }
}

/// Hull-White model parameters for Bermudan swaption pricing.
///
/// # Calibration Requirements
///
/// **Important**: The default parameters (κ=3%, σ=1%) are generic starting values
/// and should **not** be used directly for production pricing. For accurate
/// Bermudan swaption valuation:
///
/// 1. **Calibrate to co-terminal Europeans**: Fit κ and σ to match co-terminal
///    European swaption prices from the volatility surface. The co-terminal
///    swaptions share the same underlying swap end date as the Bermudan.
///
/// 2. **Mean reversion (κ)**: Controls the term structure of volatility. Higher κ
///    reduces volatility at longer tenors. Typical calibrated values: 1-10%.
///
/// 3. **Volatility (σ)**: Short rate volatility. Should be calibrated to match
///    ATM swaption implied volatilities. Typical values: 50-200 bps.
///
/// # Impact of Uncalibrated Parameters
///
/// Using uncalibrated defaults can produce Bermudan premium errors of 10-30%
/// of the early exercise value, which may be material for risk management.
///
/// # Example: Calibration Workflow
///
/// ```text
/// // 1. Get European swaption prices from vol surface for co-terminal expiries
/// // 2. Use optimizer to find κ, σ that minimize pricing error
/// // 3. Create calibrated HullWhiteParams
/// let calibrated_params = HullWhiteParams::new(calibrated_kappa, calibrated_sigma);
/// let pricer = BermudanSwaptionPricer::tree_pricer(calibrated_params);
/// ```
///
/// # References
///
/// - Hull, J. & White, A. (1990). "Pricing Interest-Rate-Derivative Securities."
///   *Review of Financial Studies*, 3(4), 573-592.
/// - Brigo, D. & Mercurio, F. (2006). *Interest Rate Models - Theory and Practice*.
///   Chapter 4: One-factor Short-Rate Models.
#[derive(Debug, Clone)]
pub struct HullWhiteParams {
    /// Mean reversion speed (κ).
    ///
    /// Controls how quickly short rates revert to the long-term mean.
    /// Higher values reduce volatility at longer maturities.
    /// Typical calibrated values: 0.01 to 0.10 (1% to 10%).
    pub kappa: f64,
    /// Short rate volatility (σ).
    ///
    /// Instantaneous volatility of the short rate process.
    /// Should be calibrated to match ATM swaption implied volatilities.
    /// Typical calibrated values: 0.005 to 0.02 (50 to 200 bps).
    pub sigma: f64,
}

impl Default for HullWhiteParams {
    /// Returns generic default parameters for testing and initialization.
    ///
    /// **Warning**: These defaults (κ=3%, σ=1%) are not calibrated and should
    /// not be used for production pricing. See struct-level documentation
    /// for calibration requirements.
    fn default() -> Self {
        Self {
            kappa: 0.03, // 3% mean reversion (uncalibrated default)
            sigma: 0.01, // 100 bps volatility (uncalibrated default)
        }
    }
}

impl HullWhiteParams {
    /// Create new Hull-White parameters with specified values.
    ///
    /// For production use, these should be calibrated to co-terminal
    /// European swaption prices from the volatility surface.
    pub fn new(kappa: f64, sigma: f64) -> Self {
        Self { kappa, sigma }
    }

    /// Returns true when these parameters are the generic uncalibrated defaults.
    pub fn is_uncalibrated_default(&self) -> bool {
        (self.kappa - 0.03).abs() < f64::EPSILON && (self.sigma - 0.01).abs() < f64::EPSILON
    }

    /// Create tree configuration with specified number of steps.
    pub(crate) fn to_tree_config(&self, steps: usize) -> HullWhiteTreeConfig {
        HullWhiteTreeConfig::new(self.kappa, self.sigma, steps)
    }
}

/// Opaque calibrated Hull-White model for Bermudan swaption pricing.
#[derive(Debug, Clone)]
pub struct CalibratedHullWhiteModel {
    tree: Arc<HullWhiteTree>,
}

impl CalibratedHullWhiteModel {
    /// Calibrate a Hull-White tree model from a discount curve and horizon.
    pub fn calibrate(
        params: HullWhiteParams,
        steps: usize,
        disc: &dyn Discounting,
        ttm: f64,
    ) -> Result<Self, PricingError> {
        if steps == 0 {
            return Err(PricingError::model_failure_with_context(
                "Tree steps must be positive".to_string(),
                PricingErrorContext::default(),
            ));
        }
        let config = params.to_tree_config(steps);
        let tree = HullWhiteTree::calibrate(config, disc, ttm).map_err(|e| {
            PricingError::model_failure_with_context(e.to_string(), PricingErrorContext::default())
        })?;
        Ok(Self {
            tree: Arc::new(tree),
        })
    }

    pub(crate) fn tree(&self) -> &Arc<HullWhiteTree> {
        &self.tree
    }
}

/// Pricer for Bermudan swaptions using Hull-White tree or LSMC.
///
/// # Model Reuse
///
/// For portfolio pricing, calibrate the Hull-White model once and reuse it
/// across multiple instruments using `with_calibrated_model`:
///
/// ```text
/// use finstack_valuations::instruments::rates::swaption::pricer::{
///     BermudanSwaptionPricer, HullWhiteParams,
/// };
/// use finstack_valuations::instruments::rates::swaption::pricer::CalibratedHullWhiteModel;
/// use finstack_core::market_data::traits::Discounting;
///
/// # fn main() -> finstack_core::Result<()> {
/// // Calibrate once (discount curve and horizon omitted here)
/// # let disc: &dyn Discounting = todo!("provide a discount curve from MarketContext");
/// let ttm = 5.0;
/// let tree = CalibratedHullWhiteModel::calibrate(
///     HullWhiteParams::default(),
///     100,
///     disc,
///     ttm,
/// )?;
///
/// // Reuse across many instruments
/// let pricer = BermudanSwaptionPricer::tree_pricer(HullWhiteParams::default())
///     .with_calibrated_model(tree.clone());
/// # let _ = pricer;
/// # Ok(())
/// # }
/// ```
///
/// # Example
///
/// ```text
/// use finstack_valuations::instruments::rates::swaption::pricer::{
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
    pre_calibrated_model: Option<CalibratedHullWhiteModel>,
    /// When true, refuse to price with uncalibrated default HW parameters.
    ///
    /// Set via [`require_calibration`](Self::require_calibration). The
    /// pricer registry (`finstack_valuations::pricer::exotics`) sets
    /// this on all registered Bermudan pricers so that callers reaching
    /// the registry with uncalibrated params receive a clear error
    /// rather than a silently-wrong price. Direct constructor callers
    /// retain the permissive default (`false`) for testing and bespoke
    /// workflows.
    enforce_calibration: bool,
}

impl BermudanSwaptionPricer {
    /// Default number of Monte Carlo paths.
    ///
    /// 100,000 paths balances accuracy and performance for typical Bermudan
    /// swaptions. For production pricing requiring tight standard errors
    /// (<0.05% of option value), increase to 500,000 paths.
    pub const DEFAULT_MC_PATHS: usize = 100_000;

    /// Create a Hull-White tree pricer.
    pub fn tree_pricer(hw_params: HullWhiteParams) -> Self {
        Self {
            method: BermudanPricingMethod::HullWhiteTree,
            hw_params,
            tree_steps: 100,
            mc_paths: Self::DEFAULT_MC_PATHS,
            mc_seed: 42,
            pre_calibrated_model: None,
            enforce_calibration: false,
        }
    }

    /// Create an LSMC pricer.
    ///
    /// Uses 100,000 paths by default. For 10M notional Bermudan swaptions,
    /// this typically produces standard errors of ~0.1-0.5% of the option value.
    /// Increase to 500,000 paths for production-grade accuracy (<0.05% SE).
    pub fn lsmc_pricer(hw_params: HullWhiteParams) -> Self {
        Self {
            method: BermudanPricingMethod::LSMC,
            hw_params,
            tree_steps: 100,
            mc_paths: Self::DEFAULT_MC_PATHS,
            mc_seed: 42,
            pre_calibrated_model: None,
            enforce_calibration: false,
        }
    }

    /// Require calibrated Hull-White parameters before pricing.
    ///
    /// When set, [`pricing a Bermudan swaption`](Self::price) returns
    /// `Err(PricingError::ModelFailure { .. })` if the pricer holds the
    /// uncalibrated `HullWhiteParams::default()` (κ=3%, σ=1%) *and* no
    /// pre-calibrated model has been supplied via
    /// [`with_calibrated_model`](Self::with_calibrated_model).
    ///
    /// The legacy permissive behaviour — emitting a `tracing::warn!`
    /// and proceeding to price with uncalibrated defaults — caused
    /// silent 10–30% mispricing of early-exercise premia. The pricer
    /// registry in `finstack_valuations::pricer::exotics` sets this on
    /// every registered Bermudan pricer; direct constructor callers
    /// who need permissive behaviour (tests, bespoke backtests) can
    /// omit the call.
    pub fn require_calibration(mut self) -> Self {
        self.enforce_calibration = true;
        self
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
    /// ```text
    /// use finstack_valuations::instruments::rates::swaption::pricer::{
    ///     BermudanSwaptionPricer, CalibratedHullWhiteModel, HullWhiteParams,
    /// };
    /// use finstack_core::market_data::traits::Discounting;
    ///
    /// # fn main() -> finstack_core::Result<()> {
    /// // Calibrate once
    /// # let disc: &dyn Discounting = todo!("provide a discount curve from MarketContext");
    /// let ttm = 5.0;
    /// let tree = CalibratedHullWhiteModel::calibrate(
    ///     HullWhiteParams::default(),
    ///     100,
    ///     disc,
    ///     ttm,
    /// )?;
    ///
    /// // Price portfolio with reused model
    /// let pricer = BermudanSwaptionPricer::tree_pricer(HullWhiteParams::default())
    ///     .with_calibrated_model(tree);
    /// # let _ = pricer;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_calibrated_model(mut self, model: CalibratedHullWhiteModel) -> Self {
        self.pre_calibrated_model = Some(model);
        self
    }

    /// Get the pre-calibrated model, if set.
    pub fn calibrated_model(&self) -> Option<&CalibratedHullWhiteModel> {
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
        if swaption.forward_curve_id != swaption.discount_curve_id {
            return Err(PricingError::model_failure_with_context(
                "Bermudan tree pricing is currently single-curve only. \
                 Set forward_curve_id equal to discount_curve_id or use a multi-curve-capable engine."
                    .to_string(),
                PricingErrorContext::default(),
            ));
        }

        // Get discount curve
        let disc = market
            .get_discount(swaption.discount_curve_id.as_str())
            .map_err(|e| {
                PricingError::missing_market_data_with_context(
                    e.to_string(),
                    PricingErrorContext::default(),
                )
            })?;

        // Calculate time to maturity
        let ttm = swaption.time_to_maturity(as_of).map_err(|e| {
            PricingError::model_failure_with_context(e.to_string(), PricingErrorContext::default())
        })?;

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
            let valuator =
                BermudanSwaptionTreeValuator::new(swaption, cached_tree, disc.as_ref(), as_of)
                    .map_err(|e| {
                        PricingError::model_failure_with_context(
                            e.to_string(),
                            PricingErrorContext::default(),
                        )
                    })?;
            (valuator.price(), true)
        } else {
            // Calibrate new model (O(Steps × Time) per instrument)
            if self.hw_params.is_uncalibrated_default() {
                if self.enforce_calibration {
                    return Err(PricingError::model_failure_with_context(
                        format!(
                            "Bermudan swaption {} received uncalibrated HullWhiteParams::default() \
                             (κ={:.3}, σ={:.3}) and no pre-calibrated model. Supply calibrated \
                             params via `HullWhiteParams::new(κ, σ)` or a pre-calibrated tree via \
                             `.with_calibrated_model(…)`.",
                            swaption.id, self.hw_params.kappa, self.hw_params.sigma,
                        ),
                        PricingErrorContext::default(),
                    ));
                }
                tracing::warn!(
                    instrument_id = %swaption.id,
                    kappa = self.hw_params.kappa,
                    sigma = self.hw_params.sigma,
                    "Pricing Bermudan swaption with uncalibrated HullWhiteParams::default(); calibrate to co-terminal swaptions for production use"
                );
            }
            let model = CalibratedHullWhiteModel::calibrate(
                self.hw_params.clone(),
                self.tree_steps,
                disc.as_ref(),
                ttm,
            )?;

            let valuator =
                BermudanSwaptionTreeValuator::new(swaption, &model, disc.as_ref(), as_of).map_err(
                    |e| {
                        PricingError::model_failure_with_context(
                            e.to_string(),
                            PricingErrorContext::default(),
                        )
                    },
                )?;
            (valuator.price(), false)
        };

        let mut result = ValuationResult::stamped(
            swaption.id.as_str(),
            as_of,
            Money::new(pv, swaption.notional.currency()),
        );

        // Record whether cached model was used (1.0 = true, 0.0 = false)
        result.measures.insert(
            crate::metrics::MetricId::custom("used_cached_model"),
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
    fn price_lsmc(
        &self,
        swaption: &BermudanSwaption,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> Result<ValuationResult, PricingError> {
        // Refuse uncalibrated defaults when enforcement is enabled (as
        // the pricer registry does). LSMC has no cached model path, so
        // this guard runs before any curve loading.
        if self.enforce_calibration
            && self.pre_calibrated_model.is_none()
            && self.hw_params.is_uncalibrated_default()
        {
            return Err(PricingError::model_failure_with_context(
                format!(
                    "Bermudan swaption {} LSMC received uncalibrated \
                     HullWhiteParams::default() (κ={:.3}, σ={:.3}). Supply \
                     calibrated params via `HullWhiteParams::new(κ, σ)` or a \
                     pre-calibrated tree via `.with_calibrated_model(…)`.",
                    swaption.id, self.hw_params.kappa, self.hw_params.sigma,
                ),
                PricingErrorContext::default(),
            ));
        }

        if swaption.forward_curve_id != swaption.discount_curve_id {
            return Err(PricingError::model_failure_with_context(
                "Bermudan Hull-White pricing is currently single-curve only. \
                 Set forward_curve_id equal to discount_curve_id or use a multi-curve-capable engine."
                    .to_string(),
                PricingErrorContext::default(),
            ));
        }

        // Get discount curve
        let disc = market
            .get_discount(swaption.discount_curve_id.as_str())
            .map_err(|e| {
                PricingError::missing_market_data_with_context(
                    e.to_string(),
                    PricingErrorContext::default(),
                )
            })?;

        // Calculate time to maturity
        let ttm = swaption.time_to_maturity(as_of).map_err(|e| {
            PricingError::model_failure_with_context(e.to_string(), PricingErrorContext::default())
        })?;

        if ttm <= 0.0 {
            // Expired - return zero
            return Ok(ValuationResult::stamped(
                swaption.id.as_str(),
                as_of,
                Money::new(0.0, swaption.notional.currency()),
            ));
        }

        // Get exercise times in years
        let exercise_times = swaption.exercise_times(as_of).map_err(|e| {
            PricingError::model_failure_with_context(e.to_string(), PricingErrorContext::default())
        })?;

        if exercise_times.is_empty() {
            return Err(PricingError::model_failure_with_context(
                "No valid exercise dates for Bermudan swaption".to_string(),
                PricingErrorContext::default(),
            ));
        }

        // Filter exercise times to be within [0, ttm]
        let valid_exercise_times: Vec<f64> = exercise_times
            .into_iter()
            .filter(|&t| t > 0.0 && t <= ttm)
            .collect();

        if valid_exercise_times.is_empty() {
            return Err(PricingError::model_failure_with_context(
                "No exercise dates before maturity".to_string(),
                PricingErrorContext::default(),
            ));
        }

        // Build swap schedule (payment times and accrual fractions)
        let (payment_dates, accrual_fractions) =
            swaption.build_swap_schedule(as_of).map_err(|e| {
                PricingError::model_failure_with_context(
                    e.to_string(),
                    PricingErrorContext::default(),
                )
            })?;

        // Convert payment dates to year fractions
        let ctx = finstack_core::dates::DayCountContext::default();
        let payment_times: Vec<f64> = payment_dates
            .iter()
            .map(|&d| swaption.day_count.year_fraction(as_of, d, ctx))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| {
                PricingError::model_failure_with_context(
                    e.to_string(),
                    PricingErrorContext::default(),
                )
            })?;

        // Create swap schedule for MC pricer
        let swap_start_time = swaption
            .day_count
            .year_fraction(as_of, swaption.swap_start, ctx)
            .map_err(|e| {
                PricingError::model_failure_with_context(
                    e.to_string(),
                    PricingErrorContext::default(),
                )
            })?;

        let swap_schedule =
            SwapSchedule::new(swap_start_time, ttm, payment_times, accrual_fractions);

        // Determine option type for payoff
        let option_type = match swaption.option_type {
            OptionType::Call => SwaptionType::Payer,
            OptionType::Put => SwaptionType::Receiver,
        };
        let strike = swaption.strike_f64().map_err(|e| {
            PricingError::model_failure_with_context(e.to_string(), PricingErrorContext::default())
        })?;

        // Create Bermudan payoff
        let payoff = BermudanSwaptionPayoff::new(
            valid_exercise_times.clone(),
            swap_schedule,
            strike,
            option_type,
            swaption.notional.amount(),
        );

        // Build exercise-aligned time grid
        let (time_grid, exercise_indices) = SwaptionLsmcConfig::build_exercise_aligned_grid(
            &valid_exercise_times,
            ttm,
            2, // Minimum steps between exercise dates
        )
        .map_err(|e| {
            PricingError::model_failure_with_context(e.to_string(), PricingErrorContext::default())
        })?;

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
            .map_err(|e| {
                PricingError::model_failure_with_context(
                    e.to_string(),
                    PricingErrorContext::default(),
                )
            })?;

        // Build result with diagnostics
        let mut result = ValuationResult::stamped(swaption.id.as_str(), as_of, estimate.mean);

        // Add LSMC diagnostics to measures
        result.measures.insert(
            crate::metrics::MetricId::custom("mc_stderr"),
            estimate.stderr,
        );
        result.measures.insert(
            crate::metrics::MetricId::custom("lsmc_num_paths"),
            self.mc_paths as f64,
        );
        result.measures.insert(
            crate::metrics::MetricId::custom("lsmc_seed"),
            self.mc_seed as f64,
        );
        let (ci_low, ci_high) = estimate.ci_95;
        result.measures.insert(
            crate::metrics::MetricId::custom("lsmc_ci95_low"),
            ci_low.amount(),
        );
        result.measures.insert(
            crate::metrics::MetricId::custom("lsmc_ci95_high"),
            ci_high.amount(),
        );

        Ok(result)
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
