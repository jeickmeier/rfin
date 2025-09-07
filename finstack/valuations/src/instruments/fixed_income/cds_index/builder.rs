use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_core::F;

use super::types::CDSIndex;
use crate::instruments::fixed_income::cds::{CDSConvention, PayReceive as CdsPayReceive};

#[derive(Default)]
pub struct CDSIndexBuilder {
    id: Option<String>,
    index_name: Option<String>,
    series: Option<u16>,
    version: Option<u16>,
    notional: Option<Money>,
    side: Option<CdsPayReceive>,
    convention: Option<CDSConvention>,
    start: Option<Date>,
    end: Option<Date>,
    fixed_coupon_bp: Option<F>,
    credit_id: Option<&'static str>,
    recovery_rate: Option<F>,
    disc_id: Option<&'static str>,
    upfront: Option<Money>,
}

impl CDSIndexBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn id(mut self, value: impl Into<String>) -> Self {
        self.id = Some(value.into());
        self
    }
    pub fn index_name(mut self, value: impl Into<String>) -> Self {
        self.index_name = Some(value.into());
        self
    }
    pub fn series(mut self, value: u16) -> Self {
        self.series = Some(value);
        self
    }
    pub fn version(mut self, value: u16) -> Self {
        self.version = Some(value);
        self
    }
    pub fn notional(mut self, value: Money) -> Self {
        self.notional = Some(value);
        self
    }
    pub fn side(mut self, value: CdsPayReceive) -> Self {
        self.side = Some(value);
        self
    }
    pub fn convention(mut self, value: CDSConvention) -> Self {
        self.convention = Some(value);
        self
    }
    pub fn start(mut self, value: Date) -> Self {
        self.start = Some(value);
        self
    }
    pub fn end(mut self, value: Date) -> Self {
        self.end = Some(value);
        self
    }
    pub fn fixed_coupon_bp(mut self, value: F) -> Self {
        self.fixed_coupon_bp = Some(value);
        self
    }
    pub fn credit_id(mut self, value: &'static str) -> Self {
        self.credit_id = Some(value);
        self
    }
    pub fn recovery_rate(mut self, value: F) -> Self {
        self.recovery_rate = Some(value);
        self
    }
    pub fn disc_id(mut self, value: &'static str) -> Self {
        self.disc_id = Some(value);
        self
    }
    pub fn upfront(mut self, value: Money) -> Self {
        self.upfront = Some(value);
        self
    }

    pub fn build(self) -> finstack_core::Result<CDSIndex> {
        let id = self
            .id
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        let index_name = self
            .index_name
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        let series = self
            .series
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        let version = self
            .version
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        let notional = self
            .notional
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        let side = self
            .side
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        let convention = self
            .convention
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        let start = self
            .start
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        let end = self
            .end
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        let fixed_coupon_bp = self
            .fixed_coupon_bp
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        let credit_id = self
            .credit_id
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        let recovery_rate = self
            .recovery_rate
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        let disc_id = self
            .disc_id
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;

        let mut index = CDSIndex::new_standard(
            id,
            index_name,
            series,
            version,
            notional,
            side,
            convention,
            start,
            end,
            fixed_coupon_bp,
            credit_id,
            recovery_rate,
            disc_id,
        );
        index.upfront = self.upfront;
        Ok(index)
    }
}


