//! Credit Default Swap (CDS) instrument implementation with ISDA conventions.
//!
//! Provides comprehensive CDS valuation including par spread calculation,
//! risky PV01, CS01, and protection leg valuation.

use crate::metrics::MetricId;
// use crate::results::ValuationResult; // not needed with macro-based impl
use crate::cashflow::traits::DatedFlows;
use crate::instruments::traits::Attributes;

use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency, StubKind};
use finstack_core::market_data::multicurve::CurveSet;
use finstack_core::market_data::term_structures::credit_curve::CreditCurve;
use finstack_core::market_data::traits::Discount;
use finstack_core::money::Money;
use finstack_core::F;

pub mod cds_pricer;
pub mod metrics;

/// CDS payment types
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PayReceive {
    /// Protection buyer pays premium leg
    PayProtection,
    /// Protection seller receives premium leg
    ReceiveProtection,
}

/// ISDA CDS conventions
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CDSConvention {
    /// Standard North American convention (quarterly, Act/360)
    IsdaNa,
    /// Standard European convention (quarterly, Act/360)
    IsdaEu,
    /// Standard Asian convention (quarterly, Act/365)
    IsdaAs,
    /// Custom convention
    Custom,
}

impl CDSConvention {
    /// Get the standard day count convention
    pub fn day_count(&self) -> DayCount {
        match self {
            CDSConvention::IsdaNa | CDSConvention::IsdaEu => DayCount::Act360,
            CDSConvention::IsdaAs => DayCount::Act365F,
            CDSConvention::Custom => DayCount::Act360, // Default
        }
    }

    /// Get the standard payment frequency
    pub fn frequency(&self) -> Frequency {
        Frequency::quarterly()
    }

    /// Get the standard business day convention
    pub fn business_day_convention(&self) -> BusinessDayConvention {
        BusinessDayConvention::Following
    }

    /// Get the standard stub convention
    pub fn stub_convention(&self) -> StubKind {
        StubKind::ShortFront
    }
}

/// Settlement type for CDS protection payment
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettlementType {
    /// Physical delivery of defaulted bonds
    Physical,
    /// Cash settlement based on recovery rate
    Cash,
    /// Auction-based settlement
    Auction,
}

/// Premium leg specification
#[derive(Clone, Debug)]
pub struct PremiumLegSpec {
    /// Start date of protection
    pub start: Date,
    /// End date of protection
    pub end: Date,
    /// Payment frequency
    pub freq: Frequency,
    /// Stub convention
    pub stub: StubKind,
    /// Business day convention
    pub bdc: BusinessDayConvention,
    /// Holiday calendar identifier
    pub calendar_id: Option<&'static str>,
    /// Day count convention
    pub dc: DayCount,
    /// Fixed spread in basis points
    pub spread_bp: F,
    /// Discount curve identifier
    pub disc_id: &'static str,
}

/// Protection leg specification
#[derive(Clone, Debug)]
pub struct ProtectionLegSpec {
    /// Credit curve identifier for default probabilities
    pub credit_id: &'static str,
    /// Recovery rate (0.0 to 1.0)
    pub recovery_rate: F,
    /// Settlement type on default
    pub settlement: SettlementType,
    /// Settlement delay in business days
    pub settlement_delay: u16,
}

/// Credit Default Swap instrument
#[derive(Clone, Debug)]
pub struct CreditDefaultSwap {
    /// Unique instrument identifier
    pub id: String,
    /// Notional amount
    pub notional: Money,
    /// Reference entity (issuer being protected)
    pub reference_entity: String,
    /// Buyer/seller perspective
    pub side: PayReceive,
    /// ISDA convention
    pub convention: CDSConvention,
    /// Premium leg specification
    pub premium: PremiumLegSpec,
    /// Protection leg specification
    pub protection: ProtectionLegSpec,
    /// Upfront payment (if any)
    pub upfront: Option<Money>,
    /// Additional attributes
    pub attributes: Attributes,
}

impl CreditDefaultSwap {
    /// Create a new CDS builder.
    pub fn builder() -> CDSBuilder {
        CDSBuilder::new()
    }

    /// Create a new CDS with standard ISDA conventions
    #[allow(clippy::too_many_arguments)]
    pub fn new_isda(
        id: impl Into<String>,
        notional: Money,
        reference_entity: impl Into<String>,
        side: PayReceive,
        convention: CDSConvention,
        start: Date,
        end: Date,
        spread_bp: F,
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
            notional,
            reference_entity: reference_entity.into(),
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
                spread_bp,
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

    /// Build premium leg cashflows
    pub fn build_premium_schedule(
        &self,
        _curves: &CurveSet,
        _as_of: Date,
    ) -> finstack_core::Result<DatedFlows> {
        // Use centralized schedule builder and standard DayCount accrual
        let sched = crate::cashflow::builder::build_dates(
            self.premium.start,
            self.premium.end,
            self.premium.freq,
            self.premium.stub,
            self.premium.bdc,
            self.premium.calendar_id,
        );
        let dates = sched.dates;
        if dates.len() < 2 {
            return Ok(vec![]);
        }

        let mut flows = Vec::with_capacity(dates.len() - 1);
        let mut prev = dates[0];
        for &d in &dates[1..] {
            let year_frac = self.premium.dc.year_fraction(prev, d)?;
            let amount = self.notional * (self.premium.spread_bp / 10000.0) * year_frac;
            flows.push((d, amount));
            prev = d;
        }

        Ok(flows)
    }

    /// Calculate premium leg PV (delegates to enhanced pricer)
    pub fn pv_premium_leg(
        &self,
        disc: &dyn Discount,
        surv: &CreditCurve,
    ) -> finstack_core::Result<Money> {
        let pricer = cds_pricer::CDSPricer::new();
        let as_of = disc.base_date();
        pricer.pv_premium_leg(self, disc, surv, as_of)
    }

    /// Calculate protection leg PV (delegates to enhanced pricer)
    pub fn pv_protection_leg(
        &self,
        disc: &dyn Discount,
        credit: &CreditCurve,
    ) -> finstack_core::Result<Money> {
        let pricer = cds_pricer::CDSPricer::new();
        let as_of = disc.base_date();
        pricer.pv_protection_leg(self, disc, credit, as_of)
    }

    /// Calculate par spread (spread that makes PV = 0) via enhanced pricer
    pub fn par_spread(
        &self,
        disc: &dyn Discount,
        credit: &CreditCurve,
    ) -> finstack_core::Result<F> {
        let pricer = cds_pricer::CDSPricer::new();
        let as_of = disc.base_date();
        pricer.par_spread(self, disc, credit, as_of)
    }

    /// Calculate risky annuity (premium leg PV per bp) via enhanced pricer
    pub fn risky_annuity(
        &self,
        disc: &dyn Discount,
        credit: &CreditCurve,
    ) -> finstack_core::Result<F> {
        let pricer = cds_pricer::CDSPricer::new();
        let as_of = disc.base_date();
        pricer.risky_annuity(self, disc, credit, as_of)
    }

    /// Calculate risky PV01 (change in PV for 1bp spread change)
    pub fn risky_pv01(
        &self,
        disc: &dyn Discount,
        credit: &CreditCurve,
    ) -> finstack_core::Result<F> {
        let pricer = cds_pricer::CDSPricer::new();
        let as_of = disc.base_date();
        pricer.risky_pv01(self, disc, credit, as_of)
    }

    /// Calculate CS01 (change in PV for 1bp credit spread change) via enhanced pricer
    pub fn cs01(&self, curves: &CurveSet) -> finstack_core::Result<F> {
        let pricer = cds_pricer::CDSPricer::new();
        pricer.cs01(self, curves, curves.discount(self.premium.disc_id)?.base_date())
    }
}

// Custom Priceable implementation for CDS (has nested fields like premium.disc_id)
impl_instrument!(
    CreditDefaultSwap, CDS,
    pv = |s, curves, _as_of| {
        let disc = curves.discount(s.premium.disc_id)?;
        let credit = curves.credit(s.protection.credit_id)?;
        let pv_premium = s.pv_premium_leg(&*disc, &credit)?;
        let pv_protection = s.pv_protection_leg(&*disc, &credit)?;
        let pv = match s.side {
            PayReceive::PayProtection => (pv_protection - pv_premium)?,
            PayReceive::ReceiveProtection => (pv_premium - pv_protection)?,
        };
        if let Some(upfront) = s.upfront { pv + upfront } else { Ok(pv) }
    },
    metrics = |_s| {
        vec![
            MetricId::ParSpread,
            MetricId::RiskyPv01,
            MetricId::Cs01,
            MetricId::ProtectionLegPv,
            MetricId::PremiumLegPv,
        ]
    }
);

/// Builder pattern for CDS instruments
#[derive(Default)]
pub struct CDSBuilder {
    id: Option<String>,
    notional: Option<Money>,
    reference_entity: Option<String>,
    side: Option<PayReceive>,
    convention: Option<CDSConvention>,
    start: Option<Date>,
    end: Option<Date>,
    spread_bp: Option<F>,
    credit_id: Option<&'static str>,
    recovery_rate: Option<F>,
    disc_id: Option<&'static str>,
    upfront: Option<Money>,
}

impl CDSBuilder {
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

    pub fn reference_entity(mut self, value: impl Into<String>) -> Self {
        self.reference_entity = Some(value.into());
        self
    }

    pub fn side(mut self, value: PayReceive) -> Self {
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

    pub fn spread_bp(mut self, value: F) -> Self {
        self.spread_bp = Some(value);
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

    pub fn build(self) -> finstack_core::Result<CreditDefaultSwap> {
        let id = self
            .id
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        let notional = self
            .notional
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        let reference_entity = self
            .reference_entity
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
        let spread_bp = self
            .spread_bp
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

        // Use the new_isda method for proper construction
        let mut cds = CreditDefaultSwap::new_isda(
            id,
            notional,
            reference_entity,
            side,
            convention,
            start,
            end,
            spread_bp,
            credit_id,
            recovery_rate,
            disc_id,
        );

        // Set optional upfront payment
        cds.upfront = self.upfront;

        Ok(cds)
    }
}

// Conversions and Attributable provided by macro

// (Removed legacy survival_probability helper; Enhanced pricer handles this.)

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use time::Month;

    #[test]
    fn test_cds_creation() {
        let notional = Money::new(10_000_000.0, Currency::USD);
        let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let end = Date::from_calendar_date(2030, Month::January, 1).unwrap();

        let cds = CreditDefaultSwap::new_isda(
            "CDS001",
            notional,
            "ABC Corp",
            PayReceive::PayProtection,
            CDSConvention::IsdaNa,
            start,
            end,
            100.0, // 100bp spread
            "ABC-SENIOR",
            0.4, // 40% recovery
            "USD-OIS",
        );

        assert_eq!(cds.id, "CDS001");
        assert_eq!(cds.reference_entity, "ABC Corp");
        assert_eq!(cds.premium.spread_bp, 100.0);
        assert_eq!(cds.protection.recovery_rate, 0.4);
    }

    #[test]
    fn test_isda_conventions() {
        assert_eq!(CDSConvention::IsdaNa.day_count(), DayCount::Act360);
        assert_eq!(CDSConvention::IsdaEu.day_count(), DayCount::Act360);
        assert_eq!(CDSConvention::IsdaAs.day_count(), DayCount::Act365F);
        assert_eq!(CDSConvention::IsdaNa.frequency(), Frequency::quarterly());
    }

    #[test]
    fn test_cds_builder_pattern() {
        let notional = Money::new(10_000_000.0, Currency::USD);
        let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let end = Date::from_calendar_date(2030, Month::January, 1).unwrap();

        let cds = CreditDefaultSwap::builder()
            .id("CDS002")
            .notional(notional)
            .reference_entity("XYZ Corp")
            .side(PayReceive::PayProtection)
            .convention(CDSConvention::IsdaNa)
            .start(start)
            .end(end)
            .spread_bp(150.0)
            .credit_id("XYZ-SENIOR")
            .recovery_rate(0.35)
            .disc_id("USD-OIS")
            .upfront(Money::new(50_000.0, Currency::USD))
            .build()
            .unwrap();

        assert_eq!(cds.id, "CDS002");
        assert_eq!(cds.reference_entity, "XYZ Corp");
        assert_eq!(cds.premium.spread_bp, 150.0);
        assert_eq!(cds.protection.recovery_rate, 0.35);
        assert_eq!(cds.upfront, Some(Money::new(50_000.0, Currency::USD)));
        assert_eq!(cds.side, PayReceive::PayProtection);
        assert_eq!(cds.convention, CDSConvention::IsdaNa);
    }
}
