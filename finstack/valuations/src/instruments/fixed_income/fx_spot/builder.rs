use crate::instruments::traits::Attributes;
use finstack_core::dates::BusinessDayConvention;
use finstack_core::prelude::*;
use finstack_core::F;

use super::types::FxSpot;

#[derive(Default)]
pub struct FxSpotBuilder {
    pub(crate) id: Option<String>,
    pub(crate) base: Option<Currency>,
    pub(crate) quote: Option<Currency>,
    pub(crate) settlement: Option<Date>,
    pub(crate) spot_rate: Option<F>,
    pub(crate) notional: Option<Money>,
    pub(crate) bdc: Option<BusinessDayConvention>,
    pub(crate) calendar_id: Option<&'static str>,
}

impl FxSpotBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn id(mut self, value: impl Into<String>) -> Self {
        self.id = Some(value.into());
        self
    }

    pub fn base(mut self, value: Currency) -> Self {
        self.base = Some(value);
        self
    }

    pub fn quote(mut self, value: Currency) -> Self {
        self.quote = Some(value);
        self
    }

    pub fn settlement(mut self, value: Date) -> Self {
        self.settlement = Some(value);
        self
    }

    pub fn spot_rate(mut self, value: F) -> Self {
        self.spot_rate = Some(value);
        self
    }

    pub fn notional(mut self, value: Money) -> Self {
        self.notional = Some(value);
        self
    }

    pub fn bdc(mut self, value: BusinessDayConvention) -> Self {
        self.bdc = Some(value);
        self
    }

    pub fn calendar_id(mut self, value: &'static str) -> Self {
        self.calendar_id = Some(value);
        self
    }

    pub fn build(self) -> finstack_core::Result<FxSpot> {
        Ok(FxSpot {
            id: self.id.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            base: self.base.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            quote: self.quote.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            settlement: self.settlement,
            spot_rate: self.spot_rate,
            notional: self.notional,
            bdc: self.bdc.unwrap_or(BusinessDayConvention::Following),
            calendar_id: self.calendar_id,
            attributes: Attributes::new(),
        })
    }
}
