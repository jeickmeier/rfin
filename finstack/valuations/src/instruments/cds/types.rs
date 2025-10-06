//! Credit Default Swap (CDS) types and implementations.
use crate::cashflow::traits::DatedFlows;
use crate::instruments::common::traits::Attributes;
use crate::instruments::PricingOverrides;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency, StubKind};
use finstack_core::market_data::traits::Discounting;
use finstack_core::market_data::traits::Survival;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::InstrumentId;

use crate::instruments::cds::pricer::CDSPricer;

/// CDS payment types
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PayReceive {
    /// Protection buyer pays premium leg
    PayProtection,
    /// Protection seller receives premium leg
    ReceiveProtection,
}

impl std::fmt::Display for PayReceive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PayReceive::PayProtection => write!(f, "pay_protection"),
            PayReceive::ReceiveProtection => write!(f, "receive_protection"),
        }
    }
}

impl std::str::FromStr for PayReceive {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let normalized = s.to_ascii_lowercase().replace('-', "_");
        match normalized.as_str() {
            "pay_protection" | "buyer" | "buy" => Ok(PayReceive::PayProtection),
            "receive_protection" | "seller" | "sell" => Ok(PayReceive::ReceiveProtection),
            other => Err(format!("Unknown CDS pay/receive: {}", other)),
        }
    }
}

/// ISDA CDS conventions
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

    /// Get the standard settlement delay in business days
    ///
    /// Returns the number of business days between trade date and settlement
    /// for standard CDS conventions by region.
    pub fn settlement_delay(&self) -> u16 {
        match self {
            CDSConvention::IsdaNa | CDSConvention::IsdaEu => 3,
            CDSConvention::IsdaAs => 3, // Most Asian markets use 3 days (some use 2)
            CDSConvention::Custom => 3, // Default to 3 days
        }
    }
}

// Re-export from common parameters
pub use crate::instruments::common::parameters::legs::{PremiumLegSpec, ProtectionLegSpec};

// Removed legacy spec structs with string ids; use types from parameters::legs with CurveId.

/// Credit Default Swap instrument
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CreditDefaultSwap {
    /// Unique instrument identifier
    pub id: InstrumentId,
    /// Notional amount
    pub notional: Money,
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
        id: impl Into<InstrumentId>,
        notional: Money,
        spread_bp: f64,
        start: Date,
        maturity: Date,
        disc_id: impl Into<finstack_core::types::CurveId>,
        credit_id: impl Into<finstack_core::types::CurveId>,
    ) -> Self {
        let convention = CDSConvention::IsdaNa;
        let dc = convention.day_count();
        let freq = convention.frequency();
        let bdc = convention.business_day_convention();
        let stub = convention.stub_convention();

        CreditDefaultSwapBuilder::new()
            .id(id.into())
            .notional(notional)
            .side(PayReceive::PayProtection)
            .convention(convention)
            .premium(PremiumLegSpec {
                start,
                end: maturity,
                freq,
                stub,
                bdc,
                calendar_id: None,
                dc,
                spread_bp,
                disc_id: disc_id.into(),
            })
            .protection(ProtectionLegSpec {
                credit_id: credit_id.into(),
                recovery_rate: crate::instruments::cds::parameters::RECOVERY_SENIOR_UNSECURED,
                settlement_delay: convention.settlement_delay(),
            })
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("CDS buy protection construction should not fail")
    }

    /// Create a standard CDS with ISDA conventions (sell protection).
    #[allow(clippy::too_many_arguments)]
    pub fn sell_protection(
        id: impl Into<InstrumentId>,
        notional: Money,
        spread_bp: f64,
        start: Date,
        maturity: Date,
        disc_id: impl Into<finstack_core::types::CurveId>,
        credit_id: impl Into<finstack_core::types::CurveId>,
    ) -> Self {
        let convention = CDSConvention::IsdaNa;
        let dc = convention.day_count();
        let freq = convention.frequency();
        let bdc = convention.business_day_convention();
        let stub = convention.stub_convention();

        CreditDefaultSwapBuilder::new()
            .id(id.into())
            .notional(notional)
            .side(PayReceive::ReceiveProtection)
            .convention(convention)
            .premium(PremiumLegSpec {
                start,
                end: maturity,
                freq,
                stub,
                bdc,
                calendar_id: None,
                dc,
                spread_bp,
                disc_id: disc_id.into(),
            })
            .protection(ProtectionLegSpec {
                credit_id: credit_id.into(),
                recovery_rate: crate::instruments::cds::parameters::RECOVERY_SENIOR_UNSECURED,
                settlement_delay: convention.settlement_delay(),
            })
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("CDS sell protection construction should not fail")
    }

    /// Create a high-yield CDS with tighter recovery assumptions.
    #[allow(clippy::too_many_arguments)]
    pub fn high_yield(
        id: impl Into<InstrumentId>,
        notional: Money,
        spread_bp: f64,
        start: Date,
        maturity: Date,
        side: PayReceive,
        disc_id: impl Into<finstack_core::types::CurveId>,
        credit_id: impl Into<finstack_core::types::CurveId>,
    ) -> Self {
        let convention = CDSConvention::IsdaNa;
        let dc = convention.day_count();
        let freq = convention.frequency();
        let bdc = convention.business_day_convention();
        let stub = convention.stub_convention();

        CreditDefaultSwapBuilder::new()
            .id(id.into())
            .notional(notional)
            .side(side)
            .convention(convention)
            .premium(PremiumLegSpec {
                start,
                end: maturity,
                freq,
                stub,
                bdc,
                calendar_id: None,
                dc,
                spread_bp,
                disc_id: disc_id.into(),
            })
            .protection(ProtectionLegSpec {
                credit_id: credit_id.into(),
                recovery_rate: crate::instruments::cds::parameters::RECOVERY_HIGH_YIELD_DEFAULT,
                settlement_delay: convention.settlement_delay(),
            })
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("High yield CDS construction should not fail")
    }

    /// Create a new CDS with standard ISDA conventions using explicit inputs.
    #[allow(clippy::too_many_arguments)]
    pub fn new_isda(
        id: impl Into<InstrumentId>,
        notional: Money,
        side: PayReceive,
        convention: CDSConvention,
        spread_bp: f64,
        start: finstack_core::dates::Date,
        end: finstack_core::dates::Date,
        recovery_rate: f64,
        disc_id: impl Into<finstack_core::types::CurveId>,
        credit_id: impl Into<finstack_core::types::CurveId>,
    ) -> Self {
        let dc = convention.day_count();
        let freq = convention.frequency();
        let bdc = convention.business_day_convention();
        let stub = convention.stub_convention();

        Self {
            id: id.into(),
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
                spread_bp,
                disc_id: disc_id.into(),
            },
            protection: ProtectionLegSpec {
                credit_id: credit_id.into(),
                recovery_rate,
                settlement_delay: convention.settlement_delay(),
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
            self.premium.calendar_id.as_deref(),
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
        let pricer = CDSPricer::new();
        let as_of = disc.base_date();
        pricer.pv_premium_leg(self, disc, surv, as_of)
    }

    /// Calculate protection leg PV (delegates to enhanced pricer)
    pub fn pv_protection_leg(
        &self,
        disc: &dyn Discounting,
        surv: &dyn Survival,
    ) -> finstack_core::Result<Money> {
        let pricer = CDSPricer::new();
        let as_of = disc.base_date();
        pricer.pv_protection_leg(self, disc, surv, as_of)
    }

    /// Calculate par spread (spread that makes PV = 0) via enhanced pricer
    pub fn par_spread(
        &self,
        disc: &dyn Discounting,
        surv: &dyn Survival,
    ) -> finstack_core::Result<f64> {
        let pricer = CDSPricer::new();
        let as_of = disc.base_date();
        pricer.par_spread(self, disc, surv, as_of)
    }

    /// Calculate risky annuity (premium leg PV per bp) via enhanced pricer
    pub fn risky_annuity(
        &self,
        disc: &dyn Discounting,
        surv: &dyn Survival,
    ) -> finstack_core::Result<f64> {
        let pricer = CDSPricer::new();
        let as_of = disc.base_date();
        pricer.risky_annuity(self, disc, surv, as_of)
    }

    /// Calculate risky PV01 (change in PV for 1bp spread change)
    pub fn risky_pv01(
        &self,
        disc: &dyn Discounting,
        surv: &dyn Survival,
    ) -> finstack_core::Result<f64> {
        let pricer = CDSPricer::new();
        let as_of = disc.base_date();
        pricer.risky_pv01(self, disc, surv, as_of)
    }

    /// Calculate CS01 (change in PV for 1bp credit spread change) via enhanced pricer
    pub fn cs01(&self, curves: &MarketContext) -> finstack_core::Result<f64> {
        let pricer = CDSPricer::new();
        pricer.cs01(
            self,
            curves,
            curves
                .get_discount_ref(self.premium.disc_id.clone())?
                .base_date(),
        )
    }

    /// Calculate the net present value of this CDS
    pub fn npv(
        &self,
        curves: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<Money> {
        let disc = curves.get_discount_ref(self.premium.disc_id.clone())?;
        let surv = curves.get_hazard_ref(self.protection.credit_id.clone())?;
        let pricer = CDSPricer::new();
        pricer.npv_with_upfront(self, disc, surv, as_of)
    }
}

impl_instrument!(
    CreditDefaultSwap,
    crate::pricer::InstrumentType::CDS,
    "CreditDefaultSwap",
    pv = |s, curves, as_of| {
        // Call the instrument's own NPV method
        s.npv(curves, as_of)
    }
);

impl crate::instruments::common::HasDiscountCurve for CreditDefaultSwap {
    fn discount_curve_id(&self) -> &finstack_core::types::CurveId {
        &self.premium.disc_id
    }
}
