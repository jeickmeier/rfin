//! Credit Default Swap (CDS) types and implementations.
use crate::cashflow::traits::DatedFlows;
use crate::instruments::common::traits::Attributes;
use crate::instruments::PricingOverrides;
use crate::margin::types::OtcMarginSpec;
use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, StubKind, Tenor};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::traits::Discounting;
use finstack_core::market_data::traits::Survival;
use finstack_core::money::Money;
use finstack_core::types::InstrumentId;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use time::macros::date;

use crate::instruments::cds::pricer::CDSPricer;
use std::sync::OnceLock;

// Re-export PayReceive from common parameters (works for both IRS and CDS)
pub use crate::instruments::common::parameters::legs::PayReceive;

/// ISDA CDS conventions
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
    fn registry_id(&self) -> &'static str {
        match self {
            CDSConvention::IsdaNa => "ANY:IsdaNa",
            CDSConvention::IsdaEu => "ANY:IsdaEu",
            CDSConvention::IsdaAs => "ANY:IsdaAs",
            CDSConvention::Custom => "ANY:Custom",
        }
    }

    /// Look up resolved conventions from registry.
    ///
    /// Returns an error if the registry entry is missing (configuration error).
    fn try_registry(&self) -> finstack_core::Result<&'static CdsConventionResolved> {
        cds_conventions_registry()
            .get(self.registry_id())
            .ok_or_else(|| {
                finstack_core::Error::Validation(format!(
                    "Missing CDS conventions registry entry for '{}'. \
                     This indicates a configuration error in the embedded CDS conventions data.",
                    self.registry_id()
                ))
            })
    }

    /// Get the standard day count convention.
    ///
    /// Per ISDA standards:
    /// - North America/Europe: ACT/360
    /// - Asia: ACT/365F
    ///
    /// Returns ACT/360 as fallback if registry lookup fails.
    #[must_use]
    pub fn day_count(&self) -> DayCount {
        self.try_registry()
            .map(|r| r.day_count)
            .unwrap_or(DayCount::Act360)
    }

    /// Get the standard payment frequency (quarterly for all conventions).
    ///
    /// Returns quarterly as fallback if registry lookup fails.
    #[must_use]
    pub fn frequency(&self) -> Tenor {
        self.try_registry()
            .map(|r| r.frequency)
            .unwrap_or_else(|_| Tenor::quarterly())
    }

    /// Get the standard business day convention.
    ///
    /// Per ISDA 2014 Credit Derivatives Definitions Section 4.12, CDS payment
    /// dates use **Modified Following** to prevent dates from rolling into
    /// the next month.
    ///
    /// Returns ModifiedFollowing as fallback if registry lookup fails.
    #[must_use]
    pub fn business_day_convention(&self) -> BusinessDayConvention {
        self.try_registry()
            .map(|r| r.business_day_convention)
            .unwrap_or(BusinessDayConvention::ModifiedFollowing)
    }

    /// Get the standard stub convention.
    ///
    /// Returns ShortFront as fallback if registry lookup fails.
    #[must_use]
    pub fn stub_convention(&self) -> StubKind {
        self.try_registry()
            .map(|r| r.stub_convention)
            .unwrap_or(StubKind::ShortFront)
    }

    /// Get the standard settlement delay in business days.
    ///
    /// Returns the number of business days between trade date and settlement
    /// for standard CDS conventions by region.
    ///
    /// Returns 3 (T+3) as fallback if registry lookup fails.
    #[must_use]
    pub fn settlement_delay(&self) -> u16 {
        self.try_registry()
            .map(|r| r.settlement_delay_days)
            .unwrap_or(3)
    }

    /// Get the default holiday calendar identifier for this convention.
    ///
    /// Returns the standard calendar for business day adjustments:
    /// - North America: `nyse` (New York Stock Exchange)
    /// - Europe: `target2` (TARGET2 / ECB)
    /// - Asia: `jpto` (Tokyo Stock Exchange)
    ///
    /// Returns "nyse" as fallback if registry lookup fails.
    #[must_use]
    pub fn default_calendar(&self) -> &'static str {
        self.try_registry()
            .map(|r| r.default_calendar_id.as_str())
            .unwrap_or("nyse")
    }
}

pub(crate) fn resolve_market_conventions(
    currency: Currency,
    doc_clause: Option<&str>,
) -> finstack_core::Result<&'static CdsConventionResolved> {
    let ccy = currency.to_string();

    let normalize_clause = |s: &str| {
        let t = s.trim();
        if t.eq_ignore_ascii_case("default") {
            return "DEFAULT".to_string();
        }
        let canon = t.to_ascii_lowercase().replace('-', "_");
        match canon.as_str() {
            "isdana" | "isda_na" => "IsdaNa".to_string(),
            "isdaeu" | "isda_eu" => "IsdaEu".to_string(),
            "isdaas" | "isda_as" => "IsdaAs".to_string(),
            "custom" => "Custom".to_string(),
            _ => t.to_string(),
        }
    };

    let key = if let Some(clause) = doc_clause {
        format!("{}:{}", ccy, normalize_clause(clause))
    } else {
        format!("{}:DEFAULT", ccy)
    };

    // If caller specified a doc clause, do not silently change it. Fall back only for the
    // "no clause provided" case, or for missing currency defaults.
    if let Some(found) = cds_conventions_registry().get(&key) {
        return Ok(found);
    }

    if doc_clause.is_some() {
        return Err(finstack_core::Error::Validation(format!(
            "Unknown CDS market conventions key '{}'. Add it to finstack/valuations/data/conventions/cds_conventions.json",
            key
        )));
    }

    // Currency default missing: fall back to global default.
    cds_conventions_registry()
        .get("DEFAULT:DEFAULT")
        .ok_or_else(|| {
            finstack_core::Error::Validation(
                "Missing CDS market conventions entry 'DEFAULT:DEFAULT'".to_string(),
            )
        })
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CdsConventionResolved {
    pub doc_clause: CDSConvention,
    pub day_count: DayCount,
    pub frequency: Tenor,
    pub business_day_convention: BusinessDayConvention,
    pub stub_convention: StubKind,
    pub settlement_delay_days: u16,
    pub default_calendar_id: String,
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CdsConventionRecord {
    doc_clause: CDSConvention,
    day_count: DayCount,
    payment_frequency: String,
    business_day_convention: BusinessDayConvention,
    stub_convention: StubKind,
    settlement_days: u16,
    calendar_id: String,
}

impl CdsConventionRecord {
    fn try_into_resolved(self) -> finstack_core::Result<CdsConventionResolved> {
        let frequency = Tenor::parse(&self.payment_frequency).map_err(|e| {
            finstack_core::Error::Validation(format!(
                "Invalid `payment_frequency` in CDS conventions registry: '{}': {}",
                self.payment_frequency, e
            ))
        })?;
        Ok(CdsConventionResolved {
            doc_clause: self.doc_clause,
            day_count: self.day_count,
            frequency,
            business_day_convention: self.business_day_convention,
            stub_convention: self.stub_convention,
            settlement_delay_days: self.settlement_days,
            default_calendar_id: self.calendar_id,
        })
    }
}

fn normalize_cds_key(id: &str) -> String {
    id.trim().to_string()
}

/// Returns the global CDS conventions registry, lazily initialized from embedded JSON.
///
/// # Panics
///
/// Panics if the embedded `cds_conventions.json` file is corrupted or malformed.
/// This is intentional: corrupted embedded data represents a build/packaging error
/// that cannot be recovered at runtime and should fail fast during startup.
#[allow(clippy::expect_used)]
fn cds_conventions_registry(
) -> &'static finstack_core::collections::HashMap<String, CdsConventionResolved> {
    static REGISTRY: OnceLock<finstack_core::collections::HashMap<String, CdsConventionResolved>> =
        OnceLock::new();
    REGISTRY.get_or_init(|| {
        let json = include_str!("../../../../data/conventions/cds_conventions.json");
        let file: crate::market::conventions::loaders::json::RegistryFile<CdsConventionRecord> =
            serde_json::from_str(json)
                .expect("Failed to parse embedded CDS conventions registry JSON");

        // Build the registry, converting each record to resolved form
        let mut map = finstack_core::collections::HashMap::default();
        for entry in file.entries {
            // Each entry can have multiple alias IDs
            match entry.record.clone().try_into_resolved() {
                Ok(resolved) => {
                    for id in &entry.ids {
                        let key = normalize_cds_key(id);
                        map.insert(key, resolved.clone());
                    }
                }
                Err(e) => {
                    // Log warning but continue - this is startup-time validation
                    // The entry will be missing and lookups will fall back to defaults
                    eprintln!(
                        "Warning: Failed to load CDS convention '{:?}': {}",
                        entry.ids, e
                    );
                }
            }
        }
        map
    })
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
    /// Upfront payment (Date, Money).
    ///
    /// The amount is defined as a payment from Protection Buyer to Protection Seller.
    /// - If positive: Buyer pays Seller.
    /// - If negative: Seller pays Buyer.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub upfront: Option<(Date, Money)>,
    /// Optional OTC margin specification for VM/IM.
    ///
    /// For cleared CDS (e.g., via ICE Clear Credit), use
    /// `OtcMarginSpec::ice_clear_credit()`. For bilateral CDS,
    /// use `OtcMarginSpec::bilateral_simm()`.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub margin_spec: Option<OtcMarginSpec>,
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
    #[allow(clippy::expect_used)] // Example uses hardcoded valid values
    pub fn example() -> Self {
        Self::buy_protection(
            "CDS-CORP-5Y",
            Money::new(10_000_000.0, Currency::USD),
            100.0, // 100 bps spread
            date!(2024 - 03 - 20),
            date!(2029 - 03 - 20),
            "USD-OIS",
            "CORP-HAZARD",
        )
        .expect("Example CDS construction should not fail")
    }

    /// Create a standard CDS with ISDA conventions (buy protection).
    ///
    /// # Arguments
    ///
    /// * `spread_bp` - Spread in basis points (e.g., 100.0 = 100bp = 1%)
    ///
    /// # Errors
    ///
    /// Returns an error if the builder fails validation or spread_bp cannot be represented as Decimal.
    #[allow(clippy::too_many_arguments)]
    pub fn buy_protection(
        id: impl Into<InstrumentId>,
        notional: Money,
        spread_bp: f64,
        start: Date,
        maturity: Date,
        discount_curve_id: impl Into<finstack_core::types::CurveId>,
        credit_id: impl Into<finstack_core::types::CurveId>,
    ) -> finstack_core::Result<Self> {
        let convention = CDSConvention::IsdaNa;
        let dc = convention.day_count();
        let freq = convention.frequency();
        let bdc = convention.business_day_convention();
        let stub = convention.stub_convention();

        let spread_bp_decimal = Decimal::try_from(spread_bp).map_err(|e| {
            finstack_core::Error::Validation(format!(
                "spread_bp {} cannot be represented as Decimal: {}",
                spread_bp, e
            ))
        })?;

        let cds = CreditDefaultSwapBuilder::new()
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
                spread_bp: spread_bp_decimal,
                discount_curve_id: discount_curve_id.into(),
            })
            .protection(ProtectionLegSpec {
                credit_curve_id: credit_id.into(),
                recovery_rate: crate::instruments::cds::parameters::RECOVERY_SENIOR_UNSECURED,
                settlement_delay: convention.settlement_delay(),
            })
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()?;

        // Validate all parameters before returning
        cds.validate()?;
        Ok(cds)
    }

    /// Create a standard CDS with ISDA conventions (sell protection).
    ///
    /// # Arguments
    ///
    /// * `spread_bp` - Spread in basis points (e.g., 100.0 = 100bp = 1%)
    ///
    /// # Errors
    ///
    /// Returns an error if the builder fails validation or spread_bp cannot be represented as Decimal.
    #[allow(clippy::too_many_arguments)]
    pub fn sell_protection(
        id: impl Into<InstrumentId>,
        notional: Money,
        spread_bp: f64,
        start: Date,
        maturity: Date,
        discount_curve_id: impl Into<finstack_core::types::CurveId>,
        credit_id: impl Into<finstack_core::types::CurveId>,
    ) -> finstack_core::Result<Self> {
        let convention = CDSConvention::IsdaNa;
        let dc = convention.day_count();
        let freq = convention.frequency();
        let bdc = convention.business_day_convention();
        let stub = convention.stub_convention();

        let spread_bp_decimal = Decimal::try_from(spread_bp).map_err(|e| {
            finstack_core::Error::Validation(format!(
                "spread_bp {} cannot be represented as Decimal: {}",
                spread_bp, e
            ))
        })?;

        let cds = CreditDefaultSwapBuilder::new()
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
                spread_bp: spread_bp_decimal,
                discount_curve_id: discount_curve_id.into(),
            })
            .protection(ProtectionLegSpec {
                credit_curve_id: credit_id.into(),
                recovery_rate: crate::instruments::cds::parameters::RECOVERY_SENIOR_UNSECURED,
                settlement_delay: convention.settlement_delay(),
            })
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()?;

        // Validate all parameters before returning
        cds.validate()?;
        Ok(cds)
    }

    /// Create a new CDS with standard ISDA conventions using explicit inputs.
    ///
    /// This is an internal helper method used by synthetic CDS creation in
    /// cds_option and cds_index modules. For public API, use `buy_protection()`,
    /// `sell_protection()`, or `builder()`.
    ///
    /// # Arguments
    ///
    /// * `spread_bp` - Spread in basis points as Decimal
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new_isda(
        id: impl Into<InstrumentId>,
        notional: Money,
        side: PayReceive,
        convention: CDSConvention,
        spread_bp: Decimal,
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
            upfront: None,
            margin_spec: None,
            attributes: Attributes::new(),
        }
    }

    /// Validate recovery rate is within valid bounds [0, 1].
    ///
    /// Returns an error if recovery rate is outside the valid range.
    #[inline]
    pub fn validate_recovery_rate(recovery_rate: f64) -> finstack_core::Result<()> {
        ProtectionLegSpec::validate_recovery_rate(recovery_rate)
    }

    /// Validate all CDS parameters.
    ///
    /// Performs comprehensive validation of the CDS instrument:
    /// - Premium leg start date must be before end date
    /// - Recovery rate must be in [0, 1]
    ///
    /// Note: Zero notional and negative spreads are allowed as they represent
    /// valid edge cases (testing scenarios, unusual market conditions).
    ///
    /// # Errors
    ///
    /// Returns `Error::Validation` with a descriptive message if any validation fails.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let cds = CreditDefaultSwap::buy_protection(...)?;
    /// cds.validate()?; // Validates all parameters
    /// ```
    pub fn validate(&self) -> finstack_core::Result<()> {
        // Validate date ordering (start must not be after end)
        // Note: start == end is allowed for "expired" CDS (valuation handles this edge case)
        if self.premium.start > self.premium.end {
            return Err(finstack_core::Error::Validation(format!(
                "CDS premium start date ({}) must not be after end date ({})",
                self.premium.start, self.premium.end
            )));
        }

        // Validate recovery rate (must be in [0, 1])
        Self::validate_recovery_rate(self.protection.recovery_rate)?;

        // Note: Zero notional is allowed for testing scenarios
        // Note: Negative spreads are allowed (theoretically possible in unusual market conditions)

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

        // Convert spread_bp to f64 for calculation (bps to decimal: /10000)
        let spread_decimal = self.premium.spread_bp.to_f64().ok_or_else(|| {
            finstack_core::Error::Validation(format!(
                "spread_bp {} cannot be converted to f64",
                self.premium.spread_bp
            ))
        })? / 10000.0;

        let mut flows = Vec::with_capacity(dates.len() - 1);
        let mut prev = dates[0];
        for &d in &dates[1..] {
            let year_frac = self.premium.dc.year_fraction(
                prev,
                d,
                finstack_core::dates::DayCountCtx::default(),
            )?;
            let amount = self.notional.amount() * spread_decimal * year_frac;
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
    /// Calculate the net present value of this CDS
    pub fn npv(
        &self,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<Money> {
        let npv_amount = self.npv_raw(market, as_of)?;
        Ok(Money::new(npv_amount, self.notional.currency()))
    }

    /// Calculate the raw net present value of this CDS (f64)
    pub fn npv_raw(
        &self,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        let disc = market.get_discount_ref(&self.premium.discount_curve_id)?;
        let surv = market.get_hazard_ref(&self.protection.credit_curve_id)?;
        let pricer = CDSPricer::new();

        // Calculate NPV as protection leg PV - premium leg PV (from buyer's perspective)
        let protection_pv = pricer.pv_protection_leg_raw(self, disc, surv, as_of)?;
        let premium_pv = pricer.pv_premium_leg_raw(self, disc, surv, as_of)?;

        // Calculate Upfront PV
        let upfront_pv = if let Some((dt, amount)) = self.upfront {
            if dt >= as_of {
                let df = disc.df_between_dates(as_of, dt)?;
                amount.amount() * df
            } else {
                0.0 // Past cashflow
            }
        } else {
            0.0
        };

        // Apply sign convention based on side
        // Base NPV = Protection (received) - Premium (paid) [as Buyer]
        // Upfront: Positive amount is paid by Buyer. So it reduces Buyer NPV.
        let npv_amount = match self.side {
            PayReceive::PayFixed => {
                // Protection buyer: pays premium, receives protection, pays upfront (if positive)
                protection_pv - premium_pv - upfront_pv
            }
            PayReceive::ReceiveFixed => {
                // Protection seller: receives premium, pays protection, receives upfront (if positive)
                premium_pv - protection_pv + upfront_pv
            }
        };

        Ok(npv_amount)
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
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        self.npv(market, as_of)
    }

    fn value_raw(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        self.npv_raw(market, as_of)
    }

    fn price_with_metrics(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
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
            None,
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
    fn notional(&self) -> Option<finstack_core::money::Money> {
        Some(self.notional)
    }

    fn build_schedule(
        &self,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<DatedFlows> {
        // For theta calculation, we only care about premium cashflows
        // Protection leg is continuous and doesn't have discrete cashflows
        let mut flows = self.build_premium_schedule(curves, as_of)?;

        // Add upfront if present
        if let Some((dt, amount)) = self.upfront {
            flows.push((dt, amount));
            // Sort by date to maintain schedule order
            flows.sort_by_key(|(d, _)| *d);
        }

        Ok(flows)
    }
}
