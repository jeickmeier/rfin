//! Credit Default Swap (CDS) types and implementations.
use crate::cashflow::traits::DatedFlows;
use crate::instruments::common::traits::Attributes;
use crate::instruments::PricingOverrides;
use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency, StubKind};
use finstack_core::market_data::traits::Discounting;
use finstack_core::market_data::traits::Survival;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::InstrumentId;

use crate::instruments::cds::pricer::CDSPricer;

// Re-export PayReceive from common parameters (works for both IRS and CDS)
pub use crate::instruments::common::parameters::legs::PayReceive;

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
    /// Get the standard day count convention.
    ///
    /// Per ISDA standards:
    /// - North America/Europe: ACT/360
    /// - Asia: ACT/365F
    #[must_use]
    pub fn day_count(&self) -> DayCount {
        match self {
            CDSConvention::IsdaNa | CDSConvention::IsdaEu => DayCount::Act360,
            CDSConvention::IsdaAs => DayCount::Act365F,
            CDSConvention::Custom => DayCount::Act360, // Default
        }
    }

    /// Get the standard payment frequency (quarterly for all conventions).
    #[must_use]
    pub fn frequency(&self) -> Frequency {
        Frequency::quarterly()
    }

    /// Get the standard business day convention.
    ///
    /// Per ISDA 2014 Credit Derivatives Definitions Section 4.12, CDS payment
    /// dates use **Modified Following** to prevent dates from rolling into
    /// the next month.
    #[must_use]
    pub fn business_day_convention(&self) -> BusinessDayConvention {
        BusinessDayConvention::ModifiedFollowing
    }

    /// Get the standard stub convention.
    #[must_use]
    pub fn stub_convention(&self) -> StubKind {
        StubKind::ShortFront
    }

    /// Get the standard settlement delay in business days.
    ///
    /// Returns the number of business days between trade date and settlement
    /// for standard CDS conventions by region.
    #[must_use]
    pub fn settlement_delay(&self) -> u16 {
        match self {
            CDSConvention::IsdaNa | CDSConvention::IsdaEu => 3,
            CDSConvention::IsdaAs => 3, // Most Asian markets use 3 days (some use 2)
            CDSConvention::Custom => 3, // Default to 3 days
        }
    }

    /// Get the default holiday calendar identifier for this convention.
    ///
    /// Returns the standard calendar for business day adjustments:
    /// - North America: `nyse` (New York Stock Exchange)
    /// - Europe: `target` (TARGET2 / ECB)
    /// - Asia: `jpto` (Tokyo Stock Exchange)
    #[must_use]
    pub fn default_calendar(&self) -> &'static str {
        match self {
            CDSConvention::IsdaNa => "nyse",
            CDSConvention::IsdaEu => "target",
            CDSConvention::IsdaAs => "jpto",
            CDSConvention::Custom => "nyse", // Default to NYSE
        }
    }
}

// Re-export from common parameters
pub use crate::instruments::common::parameters::legs::{PremiumLegSpec, ProtectionLegSpec};

/// Credit Default Swap instrument.
///
/// # Market Standards & Citations (Week 5)
///
/// ## ISDA Standards
///
/// This implementation follows the **ISDA 2014 Credit Derivatives Definitions**:
/// - **Section 1.1:** General Terms and Credit Events
/// - **Section 3.2:** Fixed Payments (Premium Leg)
/// - **Section 3.3:** Floating Payments (Protection Leg)
/// - **Section 7.1:** Settlement Terms
///
/// ## ISDA CDS Standard Model
///
/// The pricing engine implements the **ISDA CDS Standard Model (2009)**:
/// - Quarterly premium payments (20th of Mar/Jun/Sep/Dec - IMM dates)
/// - ACT/360 day count
/// - Modified Following business day convention
/// - Accrual-on-default included in premium leg
/// - Settlement: T+3 (North America), T+1 (Europe post-2009)
///
/// ## Integration Methods
///
/// Multiple numerical integration methods available:
/// - **ISDA Exact:** Analytical integration at exact cashflow dates (default)
/// - **Gaussian Quadrature:** 8-point Gauss-Legendre for smooth integration
/// - **Adaptive Simpson:** Adaptive refinement for complex survival curves
///
/// ## References
///
/// - ISDA 2014 Credit Derivatives Definitions
/// - "Modelling Single-name and Multi-name Credit Derivatives" by O'Kane (2008)
/// - ISDA CDS Standard Model Implementation (Markit, 2009)
/// - Bloomberg CDSW function documentation
///
/// See unit tests and `examples/` for usage.
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
// Note: JsonSchema derive requires finstack-core types to implement JsonSchema
// #[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
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

// Implement HasCreditCurve for generic CS01 calculator
impl crate::metrics::HasCreditCurve for CreditDefaultSwap {
    fn credit_curve_id(&self) -> &finstack_core::types::CurveId {
        &self.protection.credit_curve_id
    }
}

impl CreditDefaultSwap {
    /// Create a canonical example CDS for testing and documentation.
    ///
    /// Returns a 5-year investment-grade CDS with standard ISDA conventions.
    pub fn example() -> Self {
        Self::buy_protection(
            "CDS-CORP-5Y",
            Money::new(10_000_000.0, Currency::USD),
            100.0, // 100 bps spread
            Date::from_calendar_date(2024, time::Month::March, 20).expect("Valid example date"),
            Date::from_calendar_date(2029, time::Month::March, 20).expect("Valid example date"),
            "USD-OIS",
            "CORP-HAZARD",
        )
    }

    /// Create a standard CDS with ISDA conventions (buy protection).
    #[allow(clippy::too_many_arguments)]
    pub fn buy_protection(
        id: impl Into<InstrumentId>,
        notional: Money,
        spread_bp: f64,
        start: Date,
        maturity: Date,
        discount_curve_id: impl Into<finstack_core::types::CurveId>,
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
            .side(PayReceive::PayFixed)
            .convention(convention)
            .premium(PremiumLegSpec {
                start,
                end: maturity,
                freq,
                stub,
                bdc,
                calendar_id: Some(convention.default_calendar().to_string()),
                dc,
                spread_bp,
                discount_curve_id: discount_curve_id.into(),
            })
            .protection(ProtectionLegSpec {
                credit_curve_id: credit_id.into(),
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
        discount_curve_id: impl Into<finstack_core::types::CurveId>,
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
            .side(PayReceive::ReceiveFixed)
            .convention(convention)
            .premium(PremiumLegSpec {
                start,
                end: maturity,
                freq,
                stub,
                bdc,
                calendar_id: Some(convention.default_calendar().to_string()),
                dc,
                spread_bp,
                discount_curve_id: discount_curve_id.into(),
            })
            .protection(ProtectionLegSpec {
                credit_curve_id: credit_id.into(),
                recovery_rate: crate::instruments::cds::parameters::RECOVERY_SENIOR_UNSECURED,
                settlement_delay: convention.settlement_delay(),
            })
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("CDS sell protection construction should not fail")
    }

    /// Create a new CDS with standard ISDA conventions using explicit inputs.
    ///
    /// This is an internal helper method used by synthetic CDS creation in
    /// cds_option and cds_index modules. For public API, use `buy_protection()`,
    /// `sell_protection()`, or `builder()`.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new_isda(
        id: impl Into<InstrumentId>,
        notional: Money,
        side: PayReceive,
        convention: CDSConvention,
        spread_bp: f64,
        start: finstack_core::dates::Date,
        end: finstack_core::dates::Date,
        recovery_rate: f64,
        discount_curve_id: impl Into<finstack_core::types::CurveId>,
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
                calendar_id: Some(convention.default_calendar().to_string()),
                dc,
                spread_bp,
                discount_curve_id: discount_curve_id.into(),
            },
            protection: ProtectionLegSpec {
                credit_curve_id: credit_id.into(),
                recovery_rate,
                settlement_delay: convention.settlement_delay(),
            },
            pricing_overrides: PricingOverrides::default(),
            attributes: Attributes::new(),
        }
    }

    /// Validate recovery rate is within valid bounds [0, 1].
    ///
    /// Returns an error if recovery rate is outside the valid range.
    pub fn validate_recovery_rate(recovery_rate: f64) -> finstack_core::Result<()> {
        if !(0.0..=1.0).contains(&recovery_rate) {
            return Err(finstack_core::Error::Validation(format!(
                "Recovery rate must be between 0.0 and 1.0, got {}",
                recovery_rate
            )));
        }
        Ok(())
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
            let amount = self.notional.amount() * (self.premium.spread_bp / 10000.0) * year_frac;
            flows.push((d, Money::new(amount, self.notional.currency())));
            prev = d;
        }

        Ok(flows)
    }

    /// Calculate premium leg PV (delegates to enhanced pricer)
    pub fn pv_premium_leg(
        &self,
        disc: &dyn Discounting,
        surv: &dyn Survival,
        as_of: Date,
    ) -> finstack_core::Result<Money> {
        let pricer = CDSPricer::new();
        pricer.pv_premium_leg(self, disc, surv, as_of)
    }

    /// Calculate protection leg PV (delegates to enhanced pricer)
    pub fn pv_protection_leg(
        &self,
        disc: &dyn Discounting,
        surv: &dyn Survival,
        as_of: Date,
    ) -> finstack_core::Result<Money> {
        let pricer = CDSPricer::new();
        pricer.pv_protection_leg(self, disc, surv, as_of)
    }

    /// Calculate par spread (spread that makes PV = 0) via enhanced pricer
    pub fn par_spread(
        &self,
        disc: &dyn Discounting,
        surv: &dyn Survival,
        as_of: Date,
    ) -> finstack_core::Result<f64> {
        let pricer = CDSPricer::new();
        pricer.par_spread(self, disc, surv, as_of)
    }

    /// Calculate risky annuity (premium leg PV per bp) via enhanced pricer
    pub fn risky_annuity(
        &self,
        disc: &dyn Discounting,
        surv: &dyn Survival,
        as_of: Date,
    ) -> finstack_core::Result<f64> {
        let pricer = CDSPricer::new();
        pricer.risky_annuity(self, disc, surv, as_of)
    }

    /// Calculate risky PV01 (change in PV for 1bp spread change)
    pub fn risky_pv01(
        &self,
        disc: &dyn Discounting,
        surv: &dyn Survival,
        as_of: Date,
    ) -> finstack_core::Result<f64> {
        let pricer = CDSPricer::new();
        pricer.risky_pv01(self, disc, surv, as_of)
    }

    /// Calculate the net present value of this CDS
    pub fn npv(
        &self,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<Money> {
        let disc = market.get_discount_ref(&self.premium.discount_curve_id)?;
        let surv = market.get_hazard_ref(&self.protection.credit_curve_id)?;
        let pricer = CDSPricer::new();

        // Calculate NPV as protection leg PV - premium leg PV (from buyer's perspective)
        let protection_pv = pricer.pv_protection_leg(self, disc, surv, as_of)?;
        let premium_pv = pricer.pv_premium_leg(self, disc, surv, as_of)?;

        // Apply sign convention based on side
        let npv_amount = match self.side {
            PayReceive::PayFixed => {
                // Protection buyer: pays premium, receives protection
                protection_pv.amount() - premium_pv.amount()
            }
            PayReceive::ReceiveFixed => {
                // Protection seller: receives premium, pays protection
                premium_pv.amount() - protection_pv.amount()
            }
        };

        Ok(Money::new(npv_amount, self.notional.currency()))
    }
}

impl crate::instruments::common::traits::Instrument for CreditDefaultSwap {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::CDS
    }

    fn as_any(&self) -> &dyn ::std::any::Any {
        self
    }

    fn attributes(&self) -> &crate::instruments::common::traits::Attributes {
        &self.attributes
    }

    fn attributes_mut(&mut self) -> &mut crate::instruments::common::traits::Attributes {
        &mut self.attributes
    }

    fn clone_box(&self) -> Box<dyn crate::instruments::common::traits::Instrument> {
        Box::new(self.clone())
    }

    fn value(
        &self,
        market: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        self.npv(market, as_of)
    }

    fn price_with_metrics(
        &self,
        market: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
        metrics: &[crate::metrics::MetricId],
    ) -> finstack_core::Result<crate::results::ValuationResult> {
        let base_value = self.value(market, as_of)?;
        crate::instruments::common::helpers::build_with_metrics_dyn(
            std::sync::Arc::new(self.clone()),
            std::sync::Arc::new(market.clone()),
            as_of,
            base_value,
            metrics,
        )
    }
}

impl crate::instruments::common::pricing::HasDiscountCurve for CreditDefaultSwap {
    fn discount_curve_id(&self) -> &finstack_core::types::CurveId {
        &self.premium.discount_curve_id
    }
}

impl crate::instruments::common::traits::CurveDependencies for CreditDefaultSwap {
    fn curve_dependencies(&self) -> crate::instruments::common::traits::InstrumentCurves {
        crate::instruments::common::traits::InstrumentCurves::builder()
            .discount(self.premium.discount_curve_id.clone())
            .credit(self.protection.credit_curve_id.clone())
            .build()
    }
}

impl crate::cashflow::traits::CashflowProvider for CreditDefaultSwap {
    fn build_schedule(
        &self,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<DatedFlows> {
        // For theta calculation, we only care about premium cashflows
        // Protection leg is continuous and doesn't have discrete cashflows
        self.build_premium_schedule(curves, as_of)
    }
}
