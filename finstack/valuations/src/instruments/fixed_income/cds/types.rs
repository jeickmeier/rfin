//! Credit Default Swap (CDS) types and implementations.

use crate::cashflow::traits::DatedFlows;
use crate::instruments::common::{CDSConstructionParams, CreditParams, DateRange, MarketRefs, PricingOverrides};
use crate::instruments::traits::Attributes;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency, StubKind};
use finstack_core::market_data::traits::{Discount, Survival};
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
    /// Pricing overrides (including upfront payment)
    pub pricing_overrides: PricingOverrides,
    /// Additional attributes
    pub attributes: Attributes,
}

impl CreditDefaultSwap {
    /// Create a new CDS builder.
    pub fn builder() -> crate::instruments::fixed_income::cds::mod_cds::CDSBuilder {
        crate::instruments::fixed_income::cds::mod_cds::CDSBuilder::new()
    }

    /// Create a standard CDS with ISDA conventions (buy protection).
    pub fn buy_protection(
        id: impl Into<String>,
        reference_entity: impl Into<String>,
        notional: Money,
        spread_bp: F,
        start: Date,
        maturity: Date,
    ) -> Self {
        use crate::instruments::common::{CreditParams, DateRange, MarketRefs};

        let credit_params = CreditParams::investment_grade(reference_entity, "CREDIT-CURVE");
        let date_range = DateRange::new(start, maturity);
        let market_refs = MarketRefs::credit("USD-OIS", "CREDIT-CURVE");

        Self::builder()
            .id(id)
            .notional(notional)
            .side(PayReceive::PayProtection)
            .spread_bp(spread_bp)
            .credit_params(credit_params)
            .date_range(date_range)
            .market_refs(market_refs)
            .build()
            .expect("CDS buy protection construction should not fail")
    }

    /// Create a standard CDS with ISDA conventions (sell protection).
    pub fn sell_protection(
        id: impl Into<String>,
        reference_entity: impl Into<String>,
        notional: Money,
        spread_bp: F,
        start: Date,
        maturity: Date,
    ) -> Self {
        use crate::instruments::common::{CreditParams, DateRange, MarketRefs};

        let credit_params = CreditParams::investment_grade(reference_entity, "CREDIT-CURVE");
        let date_range = DateRange::new(start, maturity);
        let market_refs = MarketRefs::credit("USD-OIS", "CREDIT-CURVE");

        Self::builder()
            .id(id)
            .notional(notional)
            .side(PayReceive::ReceiveProtection)
            .spread_bp(spread_bp)
            .credit_params(credit_params)
            .date_range(date_range)
            .market_refs(market_refs)
            .build()
            .expect("CDS sell protection construction should not fail")
    }

    /// Create a high-yield CDS with tighter recovery assumptions.
    pub fn high_yield(
        id: impl Into<String>,
        reference_entity: impl Into<String>,
        notional: Money,
        spread_bp: F,
        start: Date,
        maturity: Date,
        side: PayReceive,
    ) -> Self {
        use crate::instruments::common::{CreditParams, DateRange, MarketRefs};

        let credit_params = CreditParams::high_yield(reference_entity, "CREDIT-CURVE");
        let date_range = DateRange::new(start, maturity);
        let market_refs = MarketRefs::credit("USD-OIS", "CREDIT-CURVE");

        Self::builder()
            .id(id)
            .notional(notional)
            .side(side)
            .spread_bp(spread_bp)
            .credit_params(credit_params)
            .date_range(date_range)
            .market_refs(market_refs)
            .build()
            .expect("High yield CDS construction should not fail")
    }

    /// Create a new CDS with standard ISDA conventions using parameter structs
    pub fn new_isda(
        id: impl Into<String>,
        construction_params: &CDSConstructionParams,
        date_range: &DateRange,
        credit_params: &CreditParams,
        market_refs: &MarketRefs,
    ) -> Self {
        let dc = construction_params.convention.day_count();
        let freq = construction_params.convention.frequency();
        let bdc = construction_params.convention.business_day_convention();
        let stub = construction_params.convention.stub_convention();

        let credit_id = market_refs
            .credit_id
            .as_ref()
            .expect("Credit curve required for CDS");

        Self {
            id: id.into(),
            notional: construction_params.notional,
            reference_entity: credit_params.reference_entity.clone(),
            side: construction_params.side,
            convention: construction_params.convention,
            premium: PremiumLegSpec {
                start: date_range.start,
                end: date_range.end,
                freq,
                stub,
                bdc,
                calendar_id: None,
                dc,
                spread_bp: construction_params.spread_bp,
                disc_id: Box::leak(market_refs.disc_id.to_string().into_boxed_str()),
            },
            protection: ProtectionLegSpec {
                credit_id: Box::leak(credit_id.to_string().into_boxed_str()),
                recovery_rate: credit_params.recovery_rate,
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
        disc: &dyn Discount,
        surv: &dyn Survival,
    ) -> finstack_core::Result<Money> {
        let pricer = cds_pricer::CDSPricer::new();
        let as_of = disc.base_date();
        pricer.pv_premium_leg(self, disc, surv, as_of)
    }

    /// Calculate protection leg PV (delegates to enhanced pricer)
    pub fn pv_protection_leg(
        &self,
        disc: &dyn Discount,
        surv: &dyn Survival,
    ) -> finstack_core::Result<Money> {
        let pricer = cds_pricer::CDSPricer::new();
        let as_of = disc.base_date();
        pricer.pv_protection_leg(self, disc, surv, as_of)
    }

    /// Calculate par spread (spread that makes PV = 0) via enhanced pricer
    pub fn par_spread(&self, disc: &dyn Discount, surv: &dyn Survival) -> finstack_core::Result<F> {
        let pricer = cds_pricer::CDSPricer::new();
        let as_of = disc.base_date();
        pricer.par_spread(self, disc, surv, as_of)
    }

    /// Calculate risky annuity (premium leg PV per bp) via enhanced pricer
    pub fn risky_annuity(
        &self,
        disc: &dyn Discount,
        surv: &dyn Survival,
    ) -> finstack_core::Result<F> {
        let pricer = cds_pricer::CDSPricer::new();
        let as_of = disc.base_date();
        pricer.risky_annuity(self, disc, surv, as_of)
    }

    /// Calculate risky PV01 (change in PV for 1bp spread change)
    pub fn risky_pv01(&self, disc: &dyn Discount, surv: &dyn Survival) -> finstack_core::Result<F> {
        let pricer = cds_pricer::CDSPricer::new();
        let as_of = disc.base_date();
        pricer.risky_pv01(self, disc, surv, as_of)
    }

    /// Calculate CS01 (change in PV for 1bp credit spread change) via enhanced pricer
    pub fn cs01(&self, curves: &MarketContext) -> finstack_core::Result<F> {
        let pricer = cds_pricer::CDSPricer::new();
        pricer.cs01(self, curves, curves.disc(self.premium.disc_id)?.base_date())
    }
}

impl_instrument!(
    CreditDefaultSwap,
    "CreditDefaultSwap",
    pv = |s, curves, _as_of| {
        let disc = curves.disc(s.premium.disc_id)?;
        let surv = curves.hazard(s.protection.credit_id)?;
        let pv_premium = s.pv_premium_leg(&*disc, surv.as_ref())?;
        let pv_protection = s.pv_protection_leg(&*disc, surv.as_ref())?;
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
