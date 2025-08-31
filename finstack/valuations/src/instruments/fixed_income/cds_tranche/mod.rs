//! CDS Tranche instrument (boilerplate implementation).
//!
//! A CDS tranche references a standardized credit index (e.g., CDX IG/HY, iTraxx)
//! and a loss layer defined by attachment/detachment points. This module provides
//! a minimal scaffold for the instrument type and wiring to the pricing/metrics
//! framework. Valuation logic is intentionally minimal and returns zero PV in the
//! instrument currency until tranche pricing models are implemented.

pub mod metrics;

use crate::impl_attributable;
use crate::metrics::MetricId;
use crate::results::ValuationResult;
use crate::traits::{Attributes, Priceable};
use finstack_core::prelude::*;
use finstack_core::dates::{BusinessDayConvention, DayCount, Frequency};
use finstack_core::F;

/// Buyer/seller perspective for CDS tranche premium/protection
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrancheSide {
    /// Buy protection on the tranche (pay running, receive protection)
    BuyProtection,
    /// Sell protection on the tranche (receive running, pay protection)
    SellProtection,
}

/// CDS Tranche instrument definition (boilerplate)
#[derive(Clone, Debug)]
pub struct CdsTranche {
    /// Unique instrument identifier
    pub id: String,
    /// Index name (e.g., "CDX.NA.IG", "CDX.NA.HY", "iTraxx EUR")
    pub index_name: String,
    /// Series number (e.g., 37)
    pub series: u16,
    /// Attachment point in percent (e.g., 0.0 for equity)
    pub attach_pct: F,
    /// Detachment point in percent (e.g., 3.0 for 0-3% tranche)
    pub detach_pct: F,
    /// Notional amount of the tranche
    pub notional: Money,
    /// Maturity date of the tranche
    pub maturity: Date,
    /// Running coupon in basis points (e.g., 100 = 1.00%)
    pub running_coupon_bp: F,
    /// Payment frequency (typically quarterly)
    pub payment_frequency: Frequency,
    /// Day count (typically Act/360)
    pub day_count: DayCount,
    /// Business day convention
    pub business_day_convention: BusinessDayConvention,
    /// Optional holiday calendar id
    pub calendar_id: Option<&'static str>,
    /// Discount curve identifier (by quote currency)
    pub disc_id: &'static str,
    /// Credit index identifier for survival/loss modeling (placeholder)
    pub credit_index_id: &'static str,
    /// Tranche side (buy/sell protection)
    pub side: TrancheSide,
    /// Attributes for tagging and selection
    pub attributes: Attributes,
}

impl CdsTranche {
    /// Create a new CDS tranche with required fields
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: impl Into<String>,
        index_name: impl Into<String>,
        series: u16,
        attach_pct: F,
        detach_pct: F,
        notional: Money,
        maturity: Date,
        running_coupon_bp: F,
        payment_frequency: Frequency,
        day_count: DayCount,
        business_day_convention: BusinessDayConvention,
        calendar_id: Option<&'static str>,
        disc_id: &'static str,
        credit_index_id: &'static str,
        side: TrancheSide,
    ) -> Self {
        Self {
            id: id.into(),
            index_name: index_name.into(),
            series,
            attach_pct,
            detach_pct,
            notional,
            maturity,
            running_coupon_bp,
            payment_frequency,
            day_count,
            business_day_convention,
            calendar_id,
            disc_id,
            credit_index_id,
            side,
            attributes: Attributes::new(),
        }
    }

    /// Builder entrypoint
    pub fn builder() -> CdsTrancheBuilder { CdsTrancheBuilder::new() }
}

impl Priceable for CdsTranche {
    /// Minimal PV implementation: returns 0.0 in instrument currency as placeholder
    fn value(&self, _curves: &finstack_core::market_data::multicurve::CurveSet, _as_of: Date) -> finstack_core::Result<Money> {
        Ok(Money::new(0.0, self.notional.currency()))
    }

    fn price_with_metrics(
        &self,
        curves: &finstack_core::market_data::multicurve::CurveSet,
        as_of: Date,
        metrics: &[MetricId],
    ) -> finstack_core::Result<ValuationResult> {
        let base_value = self.value(curves, as_of)?;
        let instrument: crate::instruments::Instrument = crate::instruments::Instrument::CDSTranche(self.clone());
        crate::instruments::build_with_metrics(instrument, curves, as_of, base_value, metrics)
    }

    fn price(
        &self,
        curves: &finstack_core::market_data::multicurve::CurveSet,
        as_of: Date,
    ) -> finstack_core::Result<ValuationResult> {
        // Provide a small standard set of tranche metrics by default
        let standard_metrics = vec![
            MetricId::custom("upfront"),
            MetricId::custom("spread_dv01"),
            MetricId::ExpectedLoss,
            MetricId::JumpToDefault,
        ];
        self.price_with_metrics(curves, as_of, &standard_metrics)
    }
}

// Generate standard Attributable implementation using macro
impl_attributable!(CdsTranche);

// Builder pattern for CdsTranche
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
    side: Option<TrancheSide>,
}

impl CdsTrancheBuilder {
    pub fn new() -> Self { Self::default() }

    pub fn id(mut self, value: impl Into<String>) -> Self { self.id = Some(value.into()); self }
    pub fn index_name(mut self, value: impl Into<String>) -> Self { self.index_name = Some(value.into()); self }
    pub fn series(mut self, value: u16) -> Self { self.series = Some(value); self }
    pub fn attach_pct(mut self, value: F) -> Self { self.attach_pct = Some(value); self }
    pub fn detach_pct(mut self, value: F) -> Self { self.detach_pct = Some(value); self }
    pub fn notional(mut self, value: Money) -> Self { self.notional = Some(value); self }
    pub fn maturity(mut self, value: Date) -> Self { self.maturity = Some(value); self }
    pub fn running_coupon_bp(mut self, value: F) -> Self { self.running_coupon_bp = Some(value); self }
    pub fn payment_frequency(mut self, value: Frequency) -> Self { self.payment_frequency = Some(value); self }
    pub fn day_count(mut self, value: DayCount) -> Self { self.day_count = Some(value); self }
    pub fn business_day_convention(mut self, value: BusinessDayConvention) -> Self { self.business_day_convention = Some(value); self }
    pub fn calendar_id(mut self, value: &'static str) -> Self { self.calendar_id = Some(value); self }
    pub fn disc_id(mut self, value: &'static str) -> Self { self.disc_id = Some(value); self }
    pub fn credit_index_id(mut self, value: &'static str) -> Self { self.credit_index_id = Some(value); self }
    pub fn side(mut self, value: TrancheSide) -> Self { self.side = Some(value); self }

    pub fn build(self) -> finstack_core::Result<CdsTranche> {
        Ok(CdsTranche {
            id: self.id.ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?,
            index_name: self.index_name.ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?,
            series: self.series.ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?,
            attach_pct: self.attach_pct.ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?,
            detach_pct: self.detach_pct.ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?,
            notional: self.notional.ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?,
            maturity: self.maturity.ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?,
            running_coupon_bp: self.running_coupon_bp.ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?,
            payment_frequency: self.payment_frequency.unwrap_or_else(Frequency::quarterly),
            day_count: self.day_count.unwrap_or(DayCount::Act360),
            business_day_convention: self.business_day_convention.unwrap_or(BusinessDayConvention::Following),
            calendar_id: self.calendar_id,
            disc_id: self.disc_id.ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?,
            credit_index_id: self.credit_index_id.ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?,
            side: self.side.ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?,
            attributes: Attributes::new(),
        })
    }
}

impl From<CdsTranche> for crate::instruments::Instrument {
    fn from(value: CdsTranche) -> Self {
        crate::instruments::Instrument::CDSTranche(value)
    }
}

impl std::convert::TryFrom<crate::instruments::Instrument> for CdsTranche {
    type Error = finstack_core::Error;

    fn try_from(value: crate::instruments::Instrument) -> finstack_core::Result<Self> {
        match value {
            crate::instruments::Instrument::CDSTranche(v) => Ok(v),
            _ => Err(finstack_core::Error::from(
                finstack_core::error::InputError::Invalid,
            )),
        }
    }
}


