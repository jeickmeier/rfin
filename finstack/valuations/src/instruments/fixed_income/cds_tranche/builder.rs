use crate::instruments::common::MarketRefs;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency};
use finstack_core::prelude::*;
use finstack_core::F;

use super::types::{CdsTranche, TrancheSide};

/// Builder pattern for CdsTranche
#[derive(Default)]
pub struct CdsTrancheBuilder {
    id: Option<String>,
    index_name: Option<String>,
    series: Option<u16>,
    attach_pct: Option<F>,
    detach_pct: Option<F>,
    notional: Option<Money>,
    maturity: Option<Date>,
    running_coupon_bp: Option<F>,
    payment_frequency: Option<Frequency>,
    day_count: Option<DayCount>,
    business_day_convention: Option<BusinessDayConvention>,
    calendar_id: Option<&'static str>,
    disc_id: Option<&'static str>,
    credit_index_id: Option<&'static str>,
    market_refs: Option<MarketRefs>,
    side: Option<TrancheSide>,
    effective_date: Option<Date>,
}

impl CdsTrancheBuilder {
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
    pub fn attach_pct(mut self, value: F) -> Self {
        self.attach_pct = Some(value);
        self
    }
    pub fn detach_pct(mut self, value: F) -> Self {
        self.detach_pct = Some(value);
        self
    }
    pub fn notional(mut self, value: Money) -> Self {
        self.notional = Some(value);
        self
    }
    pub fn maturity(mut self, value: Date) -> Self {
        self.maturity = Some(value);
        self
    }
    pub fn running_coupon_bp(mut self, value: F) -> Self {
        self.running_coupon_bp = Some(value);
        self
    }
    pub fn payment_frequency(mut self, value: Frequency) -> Self {
        self.payment_frequency = Some(value);
        self
    }
    pub fn day_count(mut self, value: DayCount) -> Self {
        self.day_count = Some(value);
        self
    }
    pub fn business_day_convention(mut self, value: BusinessDayConvention) -> Self {
        self.business_day_convention = Some(value);
        self
    }
    pub fn calendar_id(mut self, value: &'static str) -> Self {
        self.calendar_id = Some(value);
        self
    }
    pub fn disc_id(mut self, value: &'static str) -> Self {
        self.disc_id = Some(value);
        self
    }
    pub fn credit_index_id(mut self, value: &'static str) -> Self {
        self.credit_index_id = Some(value);
        self
    }
    pub fn market_refs(mut self, refs: MarketRefs) -> Self {
        self.market_refs = Some(refs);
        self
    }
    pub fn side(mut self, value: TrancheSide) -> Self {
        self.side = Some(value);
        self
    }
    pub fn effective_date(mut self, value: Date) -> Self {
        self.effective_date = Some(value);
        self
    }

    pub fn build(self) -> finstack_core::Result<CdsTranche> {
        // Prefer MarketRefs for discount id if provided
        let disc_id = if let Some(refs) = &self.market_refs {
            self.disc_id.unwrap_or_else(|| Box::leak(refs.disc_id.as_str().to_string().into_boxed_str()))
        } else {
            self.disc_id.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?
        };

        Ok(CdsTranche {
            id: self.id.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            index_name: self.index_name.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            series: self.series.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            attach_pct: self.attach_pct.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            detach_pct: self.detach_pct.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            notional: self.notional.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            maturity: self.maturity.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            running_coupon_bp: self.running_coupon_bp.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            payment_frequency: self.payment_frequency.unwrap_or_else(Frequency::quarterly),
            day_count: self.day_count.unwrap_or(DayCount::Act360),
            business_day_convention: self
                .business_day_convention
                .unwrap_or(BusinessDayConvention::Following),
            calendar_id: self.calendar_id,
            disc_id,
            credit_index_id: self.credit_index_id.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            side: self.side.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            effective_date: self.effective_date,
            attributes: crate::instruments::traits::Attributes::new(),
        })
    }
}
