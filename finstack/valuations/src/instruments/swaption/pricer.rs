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
    pub fn new() -> Self {
        Self {
            model: ModelKey::Black76,
        }
    }

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

        // Use the provided as_of date for consistency
        // Compute present value using the instrument's value method
        let pv = swaption
            .value(market, as_of)
            .map_err(|e| PricingError::model_failure(e.to_string()))?;

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
        // TODO: Implement full LSMC pricing with Bermudan exercise
        // This requires building the swap schedule and using the LSMC payoff
        let pv = swaption
            .value(market, as_of)
            .map_err(|e| PricingError::model_failure(e.to_string()))?;

        Ok(ValuationResult::stamped(swaption.id(), as_of, pv))
    }
}
