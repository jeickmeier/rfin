use crate::instruments::common::{MarketRefs, PricingOverrides};
use crate::instruments::options::cap_floor::InterestRateOption;
use crate::instruments::options::cap_floor::RateOptionType;
use crate::instruments::traits::Attributes;
use finstack_core::dates::{Date, DayCount, Frequency};
use finstack_core::money::Money;
use finstack_core::F;

/// Builder for interest rate options (caps/floors/caplets/floorlets) using MarketRefs.
#[derive(Default)]
pub struct IrOptionBuilder {
    // Required
    id: Option<String>,
    notional: Option<Money>,
    rate_option_type: Option<RateOptionType>,
    strike_rate: Option<F>,

    // Dates
    start_date: Option<Date>,
    end_date: Option<Date>,

    // Schedule/daycount
    frequency: Option<Frequency>,
    day_count: Option<DayCount>,

    // Market links
    market_refs: Option<MarketRefs>,

    // Optional overrides
    pricing_overrides: Option<PricingOverrides>,
}

impl IrOptionBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn id(mut self, value: impl Into<String>) -> Self {
        self.id = Some(value.into());
        self
    }

    pub fn notional(mut self, value: Money) -> Self {
        self.notional = Some(value);
        self
    }

    pub fn rate_option_type(mut self, value: RateOptionType) -> Self {
        self.rate_option_type = Some(value);
        self
    }

    pub fn strike_rate(mut self, value: F) -> Self {
        self.strike_rate = Some(value);
        self
    }

    pub fn start_date(mut self, value: Date) -> Self {
        self.start_date = Some(value);
        self
    }

    pub fn end_date(mut self, value: Date) -> Self {
        self.end_date = Some(value);
        self
    }

    pub fn frequency(mut self, value: Frequency) -> Self {
        self.frequency = Some(value);
        self
    }

    pub fn day_count(mut self, value: DayCount) -> Self {
        self.day_count = Some(value);
        self
    }

    /// Provide discount/forward/vol links
    pub fn market_refs(mut self, refs: MarketRefs) -> Self {
        self.market_refs = Some(refs);
        self
    }

    pub fn pricing_overrides(mut self, value: PricingOverrides) -> Self {
        self.pricing_overrides = Some(value);
        self
    }

    pub fn build(self) -> finstack_core::Result<InterestRateOption> {
        let id = self.id.ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                id: "ir_option_id".to_string(),
            })
        })?;
        let notional = self.notional.ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                id: "ir_option_notional".to_string(),
            })
        })?;
        let rate_option_type = self.rate_option_type.ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                id: "ir_option_type".to_string(),
            })
        })?;
        let strike_rate = self.strike_rate.ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                id: "ir_option_strike".to_string(),
            })
        })?;
        let start_date = self.start_date.ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                id: "ir_option_start".to_string(),
            })
        })?;
        let end_date = self.end_date.ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                id: "ir_option_end".to_string(),
            })
        })?;
        let refs = self.market_refs.ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                id: "market_refs".to_string(),
            })
        })?;

        let fwd_id = refs.fwd_id.ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                id: "forward_curve_id".to_string(),
            })
        })?;
        let vol_id = refs.vol_id.ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                id: "vol_surface_id".to_string(),
            })
        })?;

        let instrument = InterestRateOption::new(
            id,
            rate_option_type,
            notional,
            strike_rate,
            start_date,
            end_date,
            self.frequency.unwrap_or(Frequency::quarterly()),
            self.day_count.unwrap_or(DayCount::Act360),
            Box::leak(refs.disc_id.into_string().into_boxed_str()),
            Box::leak(fwd_id.into_string().into_boxed_str()),
            Box::leak(vol_id.into_string().into_boxed_str()),
        );

        let mut instrument = instrument;
        if let Some(po) = self.pricing_overrides {
            instrument.pricing_overrides = po;
        }
        instrument.attributes = Attributes::new();
        Ok(instrument)
    }
}


