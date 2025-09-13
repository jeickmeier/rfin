use crate::instruments::common::{
    EquityUnderlyingParams, MarketRefs, OptionParams, PricingOverrides,
};
use crate::instruments::traits::Attributes;
use finstack_core::dates::DayCount;
use finstack_core::money::Money;
use finstack_core::F;

use super::types::EquityOption;
use finstack_core::types::InstrumentId;

/// Enhanced equity option builder using parameter groups.
///
/// Reduces 12 optional fields to 4 required parameter groups plus optional overrides.
#[derive(Default)]
pub struct EquityOptionBuilder {
    // Core required parameters
    id: Option<InstrumentId>,
    notional: Option<Money>,

    // Parameter groups (required)
    underlying: Option<EquityUnderlyingParams>,
    option_params: Option<OptionParams>,
    market_refs: Option<MarketRefs>,

    // Optional parameters
    day_count: Option<DayCount>,
    pricing_overrides: Option<PricingOverrides>,
}

impl EquityOptionBuilder {
    /// Create a new equity option builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set instrument ID (required)
    pub fn id(mut self, value: impl Into<String>) -> Self {
        self.id = Some(InstrumentId::new(value.into()));
        self
    }

    /// Set notional amount (required)
    pub fn notional(mut self, value: Money) -> Self {
        self.notional = Some(value);
        self
    }

    /// Set underlying equity parameters (required)
    pub fn underlying(mut self, value: EquityUnderlyingParams) -> Self {
        self.underlying = Some(value);
        self
    }

    /// Set option parameters (required)  
    pub fn option_params(mut self, value: OptionParams) -> Self {
        self.option_params = Some(value);
        self
    }

    /// Set market data references (required)
    pub fn market_refs(mut self, value: MarketRefs) -> Self {
        self.market_refs = Some(value);
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
                .with_implied_vol(vol),
        );
        self
    }

    /// Build the equity option
    pub fn build(self) -> finstack_core::Result<EquityOption> {
        // Validate required fields
        let id = self.id.ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                id: "option_id".to_string(),
            })
        })?;
        let notional = self.notional.ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                id: "option_notional".to_string(),
            })
        })?;
        let underlying = self.underlying.ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                id: "underlying_params".to_string(),
            })
        })?;
        let option_params = self.option_params.ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                id: "option_params".to_string(),
            })
        })?;
        let market_refs = self.market_refs.ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                id: "market_refs".to_string(),
            })
        })?;

        // Apply pricing overrides if present
        let pricing = self.pricing_overrides.unwrap_or_default();

        // Convert strike from rate to Money format if needed
        let strike_money = Money::new(option_params.strike, notional.currency());

        Ok(EquityOption {
            id,
            underlying_ticker: underlying.ticker,
            strike: strike_money,
            option_type: option_params.option_type,
            exercise_style: option_params.exercise_style,
            expiry: option_params.expiry,
            contract_size: underlying.contract_size,
            day_count: self.day_count.unwrap_or(DayCount::Act365F),
            settlement: option_params.settlement,
            disc_id: market_refs.disc_id,
            spot_id: underlying.spot_id,
            vol_id: market_refs
                .vol_id
                .ok_or_else(|| {
                    finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                        id: "volatility_surface_id".to_string(),
                    })
                })?
                ,
            div_yield_id: underlying.dividend_yield_id,
            pricing_overrides: pricing,
            attributes: Attributes::new(),
        })
    }
}
