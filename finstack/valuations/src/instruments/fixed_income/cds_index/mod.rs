//! CDS Index instrument (boilerplate implementation).
//!
//! Provides a scaffold for standardized CDS indices such as CDX IG/HY and
//! iTraxx Europe. This mirrors the single-name CDS structure and reuses the
//! CDS pricer by mapping the index to a synthetic single-name CDS for
//! valuation and metrics.

pub mod metrics;

use crate::impl_attributable;
use crate::metrics::MetricId;
use crate::results::ValuationResult;
use crate::instruments::traits::{Attributes, Priceable};
use finstack_core::dates::Date;
use finstack_core::market_data::multicurve::CurveSet;
use finstack_core::money::Money;
use finstack_core::F;

// Reuse CDS components for conventions and legs
use super::cds::{CDSConvention, CreditDefaultSwap, PayReceive as CdsPayReceive, PremiumLegSpec, ProtectionLegSpec, SettlementType};

/// CDS Index instrument definition
#[derive(Clone, Debug)]
pub struct CDSIndex {
    /// Unique instrument identifier
    pub id: String,
    /// Index name, e.g., "CDX.NA.IG", "CDX.NA.HY", "iTraxx Europe"
    pub index_name: String,
    /// Series number (e.g., 42)
    pub series: u16,
    /// Version number within series
    pub version: u16,
    /// Notional amount of the index
    pub notional: Money,
    /// Protection buyer/seller perspective
    pub side: CdsPayReceive,
    /// Regional ISDA convention
    pub convention: CDSConvention,
    /// Premium leg specification (coupon schedule and discounting)
    pub premium: PremiumLegSpec,
    /// Protection leg specification (credit curve and settlement)
    pub protection: ProtectionLegSpec,
    /// Optional upfront payment
    pub upfront: Option<Money>,
    /// Attributes for tagging and selection
    pub attributes: Attributes,
}

impl CDSIndex {
    /// Create a new CDS Index builder
    pub fn builder() -> CDSIndexBuilder { CDSIndexBuilder::new() }

    /// Convenience constructor using standard ISDA conventions
    #[allow(clippy::too_many_arguments)]
    pub fn new_standard(
        id: impl Into<String>,
        index_name: impl Into<String>,
        series: u16,
        version: u16,
        notional: Money,
        side: CdsPayReceive,
        convention: CDSConvention,
        start: Date,
        end: Date,
        fixed_coupon_bp: F,
        credit_id: &'static str,
        recovery_rate: F,
        disc_id: &'static str,
    ) -> Self {
        let dc = convention.day_count();
        let freq = convention.frequency();
        let bdc = convention.business_day_convention();
        let stub = convention.stub_convention();

        Self {
            id: id.into(),
            index_name: index_name.into(),
            series,
            version,
            notional,
            side,
            convention,
            premium: PremiumLegSpec {
                start,
                end,
                freq,
                stub,
                bdc,
                calendar_id: None,
                dc,
                spread_bp: fixed_coupon_bp,
                disc_id,
            },
            protection: ProtectionLegSpec {
                credit_id,
                recovery_rate,
                settlement: SettlementType::Cash,
                settlement_delay: 3,
            },
            upfront: None,
            attributes: Attributes::new(),
        }
    }

    /// Map this index to a synthetic single-name CDS for valuation reuse
    pub fn to_synthetic_cds(&self) -> CreditDefaultSwap {
        CreditDefaultSwap {
            id: self.id.clone(),
            notional: self.notional,
            reference_entity: self.index_name.clone(),
            side: self.side,
            convention: self.convention,
            premium: self.premium.clone(),
            protection: self.protection.clone(),
            upfront: self.upfront,
            attributes: self.attributes.clone(),
        }
    }
}

impl Priceable for CDSIndex {
    fn value(&self, curves: &CurveSet, as_of: Date) -> finstack_core::Result<Money> {
        // Delegate to synthetic CDS valuation
        let cds = self.to_synthetic_cds();
        cds.value(curves, as_of)
    }

    fn price_with_metrics(
        &self,
        curves: &CurveSet,
        as_of: Date,
        metrics: &[MetricId],
    ) -> finstack_core::Result<ValuationResult> {
        // Compute base value
        let base_value = self.value(curves, as_of)?;

        crate::instruments::build_with_metrics(
            crate::instruments::Instrument::CDSIndex(self.clone()),
            curves,
            as_of,
            base_value,
            metrics,
        )
    }

    fn price(&self, curves: &CurveSet, as_of: Date) -> finstack_core::Result<ValuationResult> {
        // Standard CDS metrics apply to CDS indices as well
        let standard_metrics = [
            MetricId::ParSpread,
            MetricId::RiskyPv01,
            MetricId::Cs01,
            MetricId::ProtectionLegPv,
            MetricId::PremiumLegPv,
        ];
        self.price_with_metrics(curves, as_of, &standard_metrics)
    }
}

// Generate standard Attributable implementation using macro
impl_attributable!(CDSIndex);

impl From<CDSIndex> for crate::instruments::Instrument {
    fn from(value: CDSIndex) -> Self {
        crate::instruments::Instrument::CDSIndex(value)
    }
}

impl std::convert::TryFrom<crate::instruments::Instrument> for CDSIndex {
    type Error = finstack_core::Error;

    fn try_from(value: crate::instruments::Instrument) -> finstack_core::Result<Self> {
        match value {
            crate::instruments::Instrument::CDSIndex(v) => Ok(v),
            _ => Err(finstack_core::Error::from(
                finstack_core::error::InputError::Invalid,
            )),
        }
    }
}

/// Builder pattern for CDS Index instruments
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
    pub fn new() -> Self { Self::default() }

    pub fn id(mut self, value: impl Into<String>) -> Self { self.id = Some(value.into()); self }
    pub fn index_name(mut self, value: impl Into<String>) -> Self { self.index_name = Some(value.into()); self }
    pub fn series(mut self, value: u16) -> Self { self.series = Some(value); self }
    pub fn version(mut self, value: u16) -> Self { self.version = Some(value); self }
    pub fn notional(mut self, value: Money) -> Self { self.notional = Some(value); self }
    pub fn side(mut self, value: CdsPayReceive) -> Self { self.side = Some(value); self }
    pub fn convention(mut self, value: CDSConvention) -> Self { self.convention = Some(value); self }
    pub fn start(mut self, value: Date) -> Self { self.start = Some(value); self }
    pub fn end(mut self, value: Date) -> Self { self.end = Some(value); self }
    pub fn fixed_coupon_bp(mut self, value: F) -> Self { self.fixed_coupon_bp = Some(value); self }
    pub fn credit_id(mut self, value: &'static str) -> Self { self.credit_id = Some(value); self }
    pub fn recovery_rate(mut self, value: F) -> Self { self.recovery_rate = Some(value); self }
    pub fn disc_id(mut self, value: &'static str) -> Self { self.disc_id = Some(value); self }
    pub fn upfront(mut self, value: Money) -> Self { self.upfront = Some(value); self }

    pub fn build(self) -> finstack_core::Result<CDSIndex> {
        let id = self.id.ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        let index_name = self.index_name.ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        let series = self.series.ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        let version = self.version.ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        let notional = self.notional.ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        let side = self.side.ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        let convention = self.convention.ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        let start = self.start.ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        let end = self.end.ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        let fixed_coupon_bp = self.fixed_coupon_bp.ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        let credit_id = self.credit_id.ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        let recovery_rate = self.recovery_rate.ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        let disc_id = self.disc_id.ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use time::Month;

    #[test]
    fn test_cds_index_builder() {
        let start = Date::from_calendar_date(2025, Month::March, 20).unwrap();
        let end = Date::from_calendar_date(2030, Month::June, 20).unwrap();
        let idx = CDSIndex::builder()
            .id("CDX-IG-S42-V1")
            .index_name("CDX.NA.IG")
            .series(42)
            .version(1)
            .notional(Money::new(10_000_000.0, Currency::USD))
            .side(CdsPayReceive::PayProtection)
            .convention(CDSConvention::IsdaNa)
            .start(start)
            .end(end)
            .fixed_coupon_bp(100.0)
            .credit_id("CDX-NA-IG")
            .recovery_rate(0.40)
            .disc_id("USD-OIS")
            .build()
            .unwrap();

        assert_eq!(idx.index_name, "CDX.NA.IG");
        assert_eq!(idx.series, 42);
        assert_eq!(idx.version, 1);
        assert_eq!(idx.premium.spread_bp, 100.0);
        assert_eq!(idx.protection.recovery_rate, 0.40);
    }
}


