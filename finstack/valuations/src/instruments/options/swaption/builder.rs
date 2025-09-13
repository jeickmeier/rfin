use crate::instruments::common::{MarketRefs, PricingOverrides};
use crate::instruments::options::swaption::{Swaption, SwaptionExercise, SwaptionSettlement};
use crate::instruments::options::OptionType;
use finstack_core::dates::{Date, DayCount, Frequency};
use finstack_core::money::Money;
use finstack_core::F;

/// Builder for Swaption using MarketRefs for curve/surface links.
#[derive(Default)]
pub struct SwaptionBuilder {
    id: Option<String>,
    option_type: Option<OptionType>,
    notional: Option<Money>,
    strike_rate: Option<F>,
    expiry: Option<Date>,
    swap_start: Option<Date>,
    swap_end: Option<Date>,
    fixed_freq: Option<Frequency>,
    float_freq: Option<Frequency>,
    day_count: Option<DayCount>,
    exercise: Option<SwaptionExercise>,
    settlement: Option<SwaptionSettlement>,
    market_refs: Option<MarketRefs>,
    pricing_overrides: Option<PricingOverrides>,
}

impl SwaptionBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn id(mut self, value: impl Into<String>) -> Self {
        self.id = Some(value.into());
        self
    }
    pub fn payer(mut self) -> Self {
        self.option_type = Some(OptionType::Call);
        self
    }
    pub fn receiver(mut self) -> Self {
        self.option_type = Some(OptionType::Put);
        self
    }
    pub fn notional(mut self, value: Money) -> Self { self.notional = Some(value); self }
    pub fn strike_rate(mut self, value: F) -> Self { self.strike_rate = Some(value); self }
    pub fn expiry(mut self, value: Date) -> Self { self.expiry = Some(value); self }
    pub fn swap_dates(mut self, start: Date, end: Date) -> Self { self.swap_start = Some(start); self.swap_end = Some(end); self }
    pub fn fixed_freq(mut self, value: Frequency) -> Self { self.fixed_freq = Some(value); self }
    pub fn float_freq(mut self, value: Frequency) -> Self { self.float_freq = Some(value); self }
    pub fn day_count(mut self, value: DayCount) -> Self { self.day_count = Some(value); self }
    pub fn exercise(mut self, value: SwaptionExercise) -> Self { self.exercise = Some(value); self }
    pub fn settlement(mut self, value: SwaptionSettlement) -> Self { self.settlement = Some(value); self }
    pub fn market_refs(mut self, refs: MarketRefs) -> Self { self.market_refs = Some(refs); self }
    pub fn pricing_overrides(mut self, val: PricingOverrides) -> Self { self.pricing_overrides = Some(val); self }

    pub fn build(self) -> finstack_core::Result<Swaption> {
        let id = self.id.ok_or_else(|| finstack_core::error::InputError::NotFound { id: "swaption_id".to_string() })?;
        let option_type = self.option_type.ok_or_else(|| finstack_core::error::InputError::NotFound { id: "swaption_type".to_string() })?;
        let notional = self.notional.ok_or_else(|| finstack_core::error::InputError::NotFound { id: "swaption_notional".to_string() })?;
        let strike = self.strike_rate.ok_or_else(|| finstack_core::error::InputError::NotFound { id: "swaption_strike".to_string() })?;
        let expiry = self.expiry.ok_or_else(|| finstack_core::error::InputError::NotFound { id: "swaption_expiry".to_string() })?;
        let swap_start = self.swap_start.ok_or_else(|| finstack_core::error::InputError::NotFound { id: "swap_start".to_string() })?;
        let swap_end = self.swap_end.ok_or_else(|| finstack_core::error::InputError::NotFound { id: "swap_end".to_string() })?;
        let refs = self.market_refs.ok_or_else(|| finstack_core::error::InputError::NotFound { id: "market_refs".to_string() })?;
        let fwd_id = refs.fwd_id.ok_or_else(|| finstack_core::error::InputError::NotFound { id: "forward_curve_id".to_string() })?;
        let vol_id = refs.vol_id.ok_or_else(|| finstack_core::error::InputError::NotFound { id: "vol_surface_id".to_string() })?;

        let mut s = match option_type {
            OptionType::Call => Swaption::new_payer(
                id,
                notional,
                strike,
                expiry,
                swap_start,
                swap_end,
                Box::leak(refs.disc_id.into_string().into_boxed_str()),
                Box::leak(fwd_id.into_string().into_boxed_str()),
                Box::leak(vol_id.into_string().into_boxed_str()),
            ),
            OptionType::Put => Swaption::new_receiver(
                id,
                notional,
                strike,
                expiry,
                swap_start,
                swap_end,
                Box::leak(refs.disc_id.into_string().into_boxed_str()),
                Box::leak(fwd_id.into_string().into_boxed_str()),
                Box::leak(vol_id.into_string().into_boxed_str()),
            ),
        };

        // overrides
        if let Some(po) = self.pricing_overrides { s.pricing_overrides = po; }
        if let Some(ff) = self.fixed_freq { s.fixed_freq = ff; }
        if let Some(fl) = self.float_freq { s.float_freq = fl; }
        if let Some(dc) = self.day_count { s.day_count = dc; }
        if let Some(ex) = self.exercise { s.exercise = ex; }
        if let Some(set) = self.settlement { s.settlement = set; }

        Ok(s)
    }
}


