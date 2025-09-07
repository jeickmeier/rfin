use super::types::{InflationSwap, PayReceiveInflation};
use finstack_core::dates::{Date, DayCount};
use finstack_core::money::Money;
use finstack_core::F;

/// Builder for `InflationSwap`
#[derive(Default)]
pub struct InflationSwapBuilder {
    id: Option<String>,
    notional: Option<Money>,
    start: Option<Date>,
    maturity: Option<Date>,
    fixed_rate: Option<F>,
    inflation_id: Option<&'static str>,
    disc_id: Option<&'static str>,
    dc: Option<DayCount>,
    side: Option<PayReceiveInflation>,
}

impl InflationSwapBuilder {
    pub fn new() -> Self { Self::default() }

    pub fn id(mut self, value: impl Into<String>) -> Self { self.id = Some(value.into()); self }
    pub fn notional(mut self, value: Money) -> Self { self.notional = Some(value); self }
    pub fn start(mut self, value: Date) -> Self { self.start = Some(value); self }
    pub fn maturity(mut self, value: Date) -> Self { self.maturity = Some(value); self }
    pub fn fixed_rate(mut self, value: F) -> Self { self.fixed_rate = Some(value); self }
    pub fn inflation_id(mut self, value: &'static str) -> Self { self.inflation_id = Some(value); self }
    pub fn disc_id(mut self, value: &'static str) -> Self { self.disc_id = Some(value); self }
    pub fn dc(mut self, value: DayCount) -> Self { self.dc = Some(value); self }
    pub fn side(mut self, value: PayReceiveInflation) -> Self { self.side = Some(value); self }

    pub fn build(self) -> finstack_core::Result<InflationSwap> {
        Ok(InflationSwap {
            id: self.id.ok_or_else(|| finstack_core::Error::from(
                finstack_core::error::InputError::Invalid))?,
            notional: self.notional.ok_or_else(|| finstack_core::Error::from(
                finstack_core::error::InputError::Invalid))?,
            start: self.start.ok_or_else(|| finstack_core::Error::from(
                finstack_core::error::InputError::Invalid))?,
            maturity: self.maturity.ok_or_else(|| finstack_core::Error::from(
                finstack_core::error::InputError::Invalid))?,
            fixed_rate: self.fixed_rate.ok_or_else(|| finstack_core::Error::from(
                finstack_core::error::InputError::Invalid))?,
            inflation_id: self.inflation_id.ok_or_else(|| finstack_core::Error::from(
                finstack_core::error::InputError::Invalid))?,
            disc_id: self.disc_id.ok_or_else(|| finstack_core::Error::from(
                finstack_core::error::InputError::Invalid))?,
            dc: self.dc.unwrap_or(DayCount::ActAct),
            side: self.side.unwrap_or(PayReceiveInflation::PayFixed),
            attributes: crate::instruments::traits::Attributes::new(),
        })
    }
}


