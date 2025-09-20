//! Credit Default Swap (CDS) types and implementations.
use crate::cashflow::traits::DatedFlows;
use crate::instruments::traits::Attributes;
use crate::instruments::PricingOverrides;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency, StubKind};
use finstack_core::market_data::traits::Discounting;
use finstack_core::market_data::traits::Survival;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::F;

pub use super::cds_pricer;

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
#[derive(Clone, Debug, finstack_macros::FinancialBuilder)]
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
    /// Pricing overrides (including upfront payment)
    pub pricing_overrides: PricingOverrides,
    /// Additional attributes
    pub attributes: Attributes,
}

impl CreditDefaultSwap {
    /// Create a standard CDS with ISDA conventions (buy protection).
    #[allow(clippy::too_many_arguments)]
    pub fn buy_protection(
        id: impl Into<String>,
        reference_entity: impl Into<String>,
        notional: Money,
        spread_bp: F,
        start: Date,
        maturity: Date,
        disc_id: &'static str,
        credit_id: &'static str,
    ) -> Self {
        let reference_entity: String = reference_entity.into();

        let dc = CDSConvention::IsdaNa.day_count();
        let freq = CDSConvention::IsdaNa.frequency();
        let bdc = CDSConvention::IsdaNa.business_day_convention();
        let stub = CDSConvention::IsdaNa.stub_convention();

        CreditDefaultSwapBuilder::new()
            .id(id.into())
            .notional(notional)
            .reference_entity(reference_entity)
            .side(PayReceive::PayProtection)
            .convention(CDSConvention::IsdaNa)
            .premium(PremiumLegSpec {
                start,
                end: maturity,
                freq,
                stub,
                bdc,
                calendar_id: None,
                dc,
                spread_bp,
                disc_id,
            })
            .protection(ProtectionLegSpec {
                credit_id,
                recovery_rate: crate::instruments::cds::parameters::RECOVERY_SENIOR_UNSECURED,
                settlement: SettlementType::Cash,
                settlement_delay: 3,
            })
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("CDS buy protection construction should not fail")
    }

    /// Create a standard CDS with ISDA conventions (sell protection).
    #[allow(clippy::too_many_arguments)]
    pub fn sell_protection(
        id: impl Into<String>,
        reference_entity: impl Into<String>,
        notional: Money,
        spread_bp: F,
        start: Date,
        maturity: Date,
        disc_id: &'static str,
        credit_id: &'static str,
    ) -> Self {
        let reference_entity: String = reference_entity.into();

        let dc = CDSConvention::IsdaNa.day_count();
        let freq = CDSConvention::IsdaNa.frequency();
        let bdc = CDSConvention::IsdaNa.business_day_convention();
        let stub = CDSConvention::IsdaNa.stub_convention();

        CreditDefaultSwapBuilder::new()
            .id(id.into())
            .notional(notional)
            .reference_entity(reference_entity)
            .side(PayReceive::ReceiveProtection)
            .convention(CDSConvention::IsdaNa)
            .premium(PremiumLegSpec {
                start,
                end: maturity,
                freq,
                stub,
                bdc,
                calendar_id: None,
                dc,
                spread_bp,
                disc_id,
            })
            .protection(ProtectionLegSpec {
                credit_id,
                recovery_rate: crate::instruments::cds::parameters::RECOVERY_SENIOR_UNSECURED,
                settlement: SettlementType::Cash,
                settlement_delay: 3,
            })
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("CDS sell protection construction should not fail")
    }

    /// Create a high-yield CDS with tighter recovery assumptions.
    #[allow(clippy::too_many_arguments)]
    pub fn high_yield(
        id: impl Into<String>,
        reference_entity: impl Into<String>,
        notional: Money,
        spread_bp: F,
        start: Date,
        maturity: Date,
        side: PayReceive,
        disc_id: &'static str,
        credit_id: &'static str,
    ) -> Self {
        let reference_entity: String = reference_entity.into();

        let dc = CDSConvention::IsdaNa.day_count();
        let freq = CDSConvention::IsdaNa.frequency();
        let bdc = CDSConvention::IsdaNa.business_day_convention();
        let stub = CDSConvention::IsdaNa.stub_convention();

        CreditDefaultSwapBuilder::new()
            .id(id.into())
            .notional(notional)
            .reference_entity(reference_entity)
            .side(side)
            .convention(CDSConvention::IsdaNa)
            .premium(PremiumLegSpec {
                start,
                end: maturity,
                freq,
                stub,
                bdc,
                calendar_id: None,
                dc,
                spread_bp,
                disc_id,
            })
            .protection(ProtectionLegSpec {
                credit_id,
                recovery_rate: crate::instruments::cds::parameters::RECOVERY_HIGH_YIELD_DEFAULT,
                settlement: SettlementType::Cash,
                settlement_delay: 3,
            })
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("High yield CDS construction should not fail")
    }

    /// Create a new CDS with standard ISDA conventions using explicit inputs.
    #[allow(clippy::too_many_arguments)]
    pub fn new_isda(
        id: impl Into<String>,
        notional: Money,
        side: PayReceive,
        convention: CDSConvention,
        spread_bp: F,
        start: finstack_core::dates::Date,
        end: finstack_core::dates::Date,
        reference_entity: impl Into<String>,
        recovery_rate: F,
        disc_id: &'static str,
        credit_id: &'static str,
    ) -> Self {
        let reference_entity: String = reference_entity.into();
        let dc = convention.day_count();
        let freq = convention.frequency();
        let bdc = convention.business_day_convention();
        let stub = convention.stub_convention();

        Self {
            id: id.into(),
            notional,
            reference_entity,
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
            pricing_overrides: PricingOverrides::default(),
            attributes: Attributes::new(),
        }
    }

    /// Build premium leg cashflows
    pub fn build_premium_schedule(
        &self,
        _curves: &MarketContext,
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
            let year_frac = self.premium.dc.year_fraction(
                prev,
                d,
                finstack_core::dates::DayCountCtx::default(),
            )?;
            let amount = self.notional * (self.premium.spread_bp / 10000.0) * year_frac;
            flows.push((d, amount));
            prev = d;
        }

        Ok(flows)
    }

    /// Calculate premium leg PV (delegates to enhanced pricer)
    pub fn pv_premium_leg(
        &self,
        disc: &dyn Discounting,
        surv: &dyn Survival,
    ) -> finstack_core::Result<Money> {
        let pricer = cds_pricer::CDSPricer::new();
        let as_of = disc.base_date();
        pricer.pv_premium_leg(self, disc, surv, as_of)
    }

    /// Calculate protection leg PV (delegates to enhanced pricer)
    pub fn pv_protection_leg(
        &self,
        disc: &dyn Discounting,
        surv: &dyn Survival,
    ) -> finstack_core::Result<Money> {
        let pricer = cds_pricer::CDSPricer::new();
        let as_of = disc.base_date();
        pricer.pv_protection_leg(self, disc, surv, as_of)
    }

    /// Calculate par spread (spread that makes PV = 0) via enhanced pricer
    pub fn par_spread(
        &self,
        disc: &dyn Discounting,
        surv: &dyn Survival,
    ) -> finstack_core::Result<F> {
        let pricer = cds_pricer::CDSPricer::new();
        let as_of = disc.base_date();
        pricer.par_spread(self, disc, surv, as_of)
    }

    /// Calculate risky annuity (premium leg PV per bp) via enhanced pricer
    pub fn risky_annuity(
        &self,
        disc: &dyn Discounting,
        surv: &dyn Survival,
    ) -> finstack_core::Result<F> {
        let pricer = cds_pricer::CDSPricer::new();
        let as_of = disc.base_date();
        pricer.risky_annuity(self, disc, surv, as_of)
    }

    /// Calculate risky PV01 (change in PV for 1bp spread change)
    pub fn risky_pv01(
        &self,
        disc: &dyn Discounting,
        surv: &dyn Survival,
    ) -> finstack_core::Result<F> {
        let pricer = cds_pricer::CDSPricer::new();
        let as_of = disc.base_date();
        pricer.risky_pv01(self, disc, surv, as_of)
    }

    /// Calculate CS01 (change in PV for 1bp credit spread change) via enhanced pricer
    pub fn cs01(&self, curves: &MarketContext) -> finstack_core::Result<F> {
        let pricer = cds_pricer::CDSPricer::new();
        pricer.cs01(
            self,
            curves,
            curves
                .get_ref::<finstack_core::market_data::term_structures::discount_curve::DiscountCurve>(
                    self.premium.disc_id,
                )?
                .base_date(),
        )
    }
}

impl_instrument!(
    CreditDefaultSwap,
    "CreditDefaultSwap",
    pv = |s, curves, _as_of| {
        let disc = curves
            .get_ref::<finstack_core::market_data::term_structures::discount_curve::DiscountCurve>(
            s.premium.disc_id,
        )?;
        let surv = curves
            .get_ref::<finstack_core::market_data::term_structures::hazard_curve::HazardCurve>(
                s.protection.credit_id,
            )?;
        let pv_premium = s.pv_premium_leg(disc, surv)?;
        let pv_protection = s.pv_protection_leg(disc, surv)?;
        let pv = match s.side {
            PayReceive::PayProtection => (pv_protection - pv_premium)?,
            PayReceive::ReceiveProtection => (pv_premium - pv_protection)?,
        };
        if let Some(upfront) = s.pricing_overrides.upfront_payment {
            pv + upfront
        } else {
            Ok(pv)
        }
    }
);
