use crate::instruments::common::{FxUnderlyingParams, OptionParams, PricingOverrides};
use crate::instruments::traits::Attributes;
use finstack_core::dates::DayCount;
use finstack_core::money::Money;
use finstack_core::F;

use super::types::FxOption;

/// Enhanced FX option builder using parameter groups.
///
/// Reduces complexity by grouping related parameters together.
#[derive(Default)]
pub struct FxOptionBuilder {
    // Core required parameters
    id: Option<String>,
    notional: Option<Money>,
    
    // Parameter groups (required)
    fx_underlying: Option<FxUnderlyingParams>,
    option_params: Option<OptionParams>,
    
    // Optional parameters
    day_count: Option<DayCount>,
    pricing_overrides: Option<PricingOverrides>,
}

impl FxOptionBuilder {
    /// Create a new FX option builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set instrument ID (required)
    pub fn id(mut self, value: impl Into<String>) -> Self {
        self.id = Some(value.into());
        self
    }

    /// Set notional amount (required)
    pub fn notional(mut self, value: Money) -> Self {
        self.notional = Some(value);
        self
    }

    /// Set FX underlying parameters (required)
    pub fn fx_underlying(mut self, value: FxUnderlyingParams) -> Self {
        self.fx_underlying = Some(value);
        self
    }

    /// Set option parameters (required)
    pub fn option_params(mut self, value: OptionParams) -> Self {
        self.option_params = Some(value);
        self
    }

    /// Set day count convention (optional, defaults to Act/365F)
    pub fn day_count(mut self, value: DayCount) -> Self {
        self.day_count = Some(value);
        self
    }

    /// Set pricing overrides (optional)
    pub fn pricing_overrides(mut self, value: PricingOverrides) -> Self {
        self.pricing_overrides = Some(value);
        self
    }

    /// Convenience: Set implied volatility override
    pub fn implied_vol(mut self, vol: F) -> Self {
        self.pricing_overrides = Some(
            self.pricing_overrides
                .unwrap_or_default()
                .with_implied_vol(vol)
        );
        self
    }

    /// Build the FX option
    pub fn build(self) -> finstack_core::Result<FxOption> {
        // Validate required fields
        let id = self.id.ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                id: "fx_option_id".to_string(),
            })
        })?;
        let notional = self.notional.ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                id: "fx_option_notional".to_string(),
            })
        })?;
        let fx_underlying = self.fx_underlying.ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                id: "fx_underlying_params".to_string(),
            })
        })?;
        let option_params = self.option_params.ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                id: "option_params".to_string(),
            })
        })?;

        // Apply pricing overrides if present
        let pricing = self.pricing_overrides.unwrap_or_default();

        Ok(FxOption {
            id,
            base_currency: fx_underlying.base_currency,
            quote_currency: fx_underlying.quote_currency,
            strike: option_params.strike,
            option_type: option_params.option_type,
            exercise_style: option_params.exercise_style,
            expiry: option_params.expiry,
            day_count: self.day_count.unwrap_or(DayCount::Act365F),
            notional,
            settlement: option_params.settlement,
            domestic_disc_id: fx_underlying.domestic_disc_id,
            foreign_disc_id: fx_underlying.foreign_disc_id,
            vol_id: "FX-VOL", // Standard FX volatility surface
            implied_vol: pricing.implied_volatility,
            attributes: Attributes::new(),
        })
    }
}