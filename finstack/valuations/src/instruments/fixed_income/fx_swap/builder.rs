use finstack_core::prelude::*;
use finstack_core::F;

use super::types::FxSwap;

// Builder pattern using simple struct for clarity (avoids too_many_arguments for new)
#[derive(Default)]
pub struct FxSwapBuilder {
    id: Option<String>,
    base_currency: Option<Currency>,
    quote_currency: Option<Currency>,
    near_date: Option<Date>,
    far_date: Option<Date>,
    base_notional: Option<Money>,
    domestic_disc_id: Option<&'static str>,
    foreign_disc_id: Option<&'static str>,
    near_rate: Option<F>,
    far_rate: Option<F>,
}

impl FxSwapBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn id(mut self, value: impl Into<String>) -> Self {
        self.id = Some(value.into());
        self
    }
    pub fn base_currency(mut self, value: Currency) -> Self {
        self.base_currency = Some(value);
        self
    }
    pub fn quote_currency(mut self, value: Currency) -> Self {
        self.quote_currency = Some(value);
        self
    }
    pub fn near_date(mut self, value: Date) -> Self {
        self.near_date = Some(value);
        self
    }
    pub fn far_date(mut self, value: Date) -> Self {
        self.far_date = Some(value);
        self
    }
    pub fn base_notional(mut self, value: Money) -> Self {
        self.base_notional = Some(value);
        self
    }
    pub fn domestic_disc_id(mut self, value: &'static str) -> Self {
        self.domestic_disc_id = Some(value);
        self
    }
    pub fn foreign_disc_id(mut self, value: &'static str) -> Self {
        self.foreign_disc_id = Some(value);
        self
    }
    pub fn near_rate(mut self, value: F) -> Self {
        self.near_rate = Some(value);
        self
    }
    pub fn far_rate(mut self, value: F) -> Self {
        self.far_rate = Some(value);
        self
    }

    pub fn build(self) -> finstack_core::Result<FxSwap> {
        Ok(FxSwap {
            id: self.id.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            base_currency: self.base_currency.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            quote_currency: self.quote_currency.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            near_date: self.near_date.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            far_date: self.far_date.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            base_notional: self.base_notional.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            domestic_disc_id: self.domestic_disc_id.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            foreign_disc_id: self.foreign_disc_id.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            near_rate: self.near_rate,
            far_rate: self.far_rate,
            attributes: crate::instruments::traits::Attributes::new(),
        })
    }
}
