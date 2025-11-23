use crate::instruments::common::traits::Instrument;
use crate::instruments::swaption::Swaption;
use crate::pricer::{InstrumentType, ModelKey, Pricer, PricerKey, PricingError};
use crate::results::ValuationResult;
use finstack_core::market_data::MarketContext;

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
