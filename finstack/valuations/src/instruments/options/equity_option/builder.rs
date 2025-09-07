use crate::instruments::options::{ExerciseStyle, OptionType, SettlementType};
use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_core::F;
use crate::instruments::traits::Attributes;

use super::types::EquityOption;

/// Builder pattern for EquityOption instruments
#[derive(Default)]
pub struct EquityOptionBuilder {
    id: Option<String>,
    underlying_ticker: Option<String>,
    strike: Option<Money>,
    option_type: Option<OptionType>,
    exercise_style: Option<ExerciseStyle>,
    expiry: Option<Date>,
    contract_size: Option<F>,
    day_count: Option<finstack_core::dates::DayCount>,
    settlement: Option<SettlementType>,
    disc_id: Option<&'static str>,
    spot_id: Option<&'static str>,
    vol_id: Option<&'static str>,
    div_yield_id: Option<&'static str>,
    implied_vol: Option<F>,
}

impl EquityOptionBuilder {
    pub fn new() -> Self { Self::default() }
    pub fn id(mut self, value: impl Into<String>) -> Self { self.id = Some(value.into()); self }
    pub fn underlying_ticker(mut self, value: impl Into<String>) -> Self { self.underlying_ticker = Some(value.into()); self }
    pub fn strike(mut self, value: Money) -> Self { self.strike = Some(value); self }
    pub fn option_type(mut self, value: OptionType) -> Self { self.option_type = Some(value); self }
    pub fn exercise_style(mut self, value: ExerciseStyle) -> Self { self.exercise_style = Some(value); self }
    pub fn expiry(mut self, value: Date) -> Self { self.expiry = Some(value); self }
    pub fn contract_size(mut self, value: F) -> Self { self.contract_size = Some(value); self }
    pub fn day_count(mut self, value: finstack_core::dates::DayCount) -> Self { self.day_count = Some(value); self }
    pub fn settlement(mut self, value: SettlementType) -> Self { self.settlement = Some(value); self }
    pub fn disc_id(mut self, value: &'static str) -> Self { self.disc_id = Some(value); self }
    pub fn spot_id(mut self, value: &'static str) -> Self { self.spot_id = Some(value); self }
    pub fn vol_id(mut self, value: &'static str) -> Self { self.vol_id = Some(value); self }
    pub fn div_yield_id(mut self, value: &'static str) -> Self { self.div_yield_id = Some(value); self }
    pub fn implied_vol(mut self, value: F) -> Self { self.implied_vol = Some(value); self }

    pub fn build(self) -> finstack_core::Result<EquityOption> {
        Ok(EquityOption {
            id: self.id.ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?,
            underlying_ticker: self.underlying_ticker.ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?,
            strike: self.strike.ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?,
            option_type: self.option_type.ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?,
            exercise_style: self.exercise_style.unwrap_or(ExerciseStyle::European),
            expiry: self.expiry.ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?,
            contract_size: self.contract_size.unwrap_or(1.0),
            day_count: self.day_count.unwrap_or(finstack_core::dates::DayCount::Act365F),
            settlement: self.settlement.unwrap_or(SettlementType::Physical),
            disc_id: self.disc_id.ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?,
            spot_id: self.spot_id.ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?,
            vol_id: self.vol_id.ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?,
            div_yield_id: self.div_yield_id,
            implied_vol: self.implied_vol,
            attributes: Attributes::new(),
        })
    }
}


