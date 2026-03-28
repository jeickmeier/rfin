//! Credit Default Swap (CDS) types and implementations.
//!
//! # Convention Defaults
//!
//! This module uses **ISDA North American (IsdaNa)** as the default convention
//! when no explicit convention is specified. This choice aligns with:
//!
//! - The ISDA CDS Standard Model (2014) which was developed primarily for
//!   US/Canadian CDS markets
//! - Bloomberg and Markit pricing tools which default to NA conventions
//! - The dominance of US credit markets in global CDS trading volume
//!
//! For European or Asian CDS, explicitly specify `CDSConvention::IsdaEu` or
//! `CDSConvention::IsdaAs` respectively. Use [`CDSConvention::detect_from_currency`]
//! for automatic detection based on currency.
//!
//! ## Regional Convention Summary
//!
//! | Region | Convention | Day Count | Payment Frequency | Settlement | Calendar |
//! |--------|-----------|-----------|-------------------|------------|----------|
//! | North America | `IsdaNa` | ACT/360 | Quarterly | T+3 | NYSE |
//! | Europe | `IsdaEu` | ACT/360 | Quarterly | T+1 | TARGET2 |
//! | Asia | `IsdaAs` | ACT/365F | Quarterly | T+3 | Tokyo |
//!
//! **Note**: European CDS settlement changed from T+3 to T+1 on June 20, 2009 as part of the
//! ISDA "Big Bang" protocol. This implementation uses the post-2009 T+1 standard.
//!
//! ## Example: Explicit Convention Selection
//!
//! ```ignore
//! // Detect from currency (recommended for cross-regional portfolios)
//! let convention = CDSConvention::detect_from_currency(Currency::EUR);
//! assert_eq!(convention, CDSConvention::IsdaEu);
//!
//! // Or specify explicitly
//! let european_cds = CreditDefaultSwap::builder()
//!     .convention(CDSConvention::IsdaEu)
//!     // ...
//!     .build()?;
//! ```

use crate::cashflow::traits::DatedFlows;
use crate::instruments::common_impl::traits::Attributes;
use crate::instruments::common_impl::validation;
use crate::instruments::PricingOverrides;
use crate::margin::types::OtcMarginSpec;
use crate::market::conventions::ids::CdsDocClause;
use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, StubKind, Tenor};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::InstrumentId;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use time::macros::date;

use crate::impl_instrument_base;
use crate::instruments::credit_derivatives::cds::pricer::CDSPricer;
use std::sync::OnceLock;

// Re-export PayReceive from common parameters (works for both IRS and CDS)
pub use crate::instruments::common_impl::parameters::legs::PayReceive;

/// ISDA CDS conventions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
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
            .map(|r| r.bdc)
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

    /// Detect the appropriate CDS convention based on currency.
    ///
    /// This helper automatically selects the regional convention based on the
    /// currency of the CDS notional. Useful for cross-regional portfolios
    /// where convention should match the underlying credit market.
    ///
    /// # Currency Mapping
    ///
    /// - **USD, CAD**: North American (`IsdaNa`) - T+3, ACT/360, NYSE calendar
    /// - **EUR, GBP, CHF**: European (`IsdaEu`) - T+1, ACT/360, TARGET2 calendar (post-2009 Big Bang)
    /// - **JPY, AUD, HKD, SGD**: Asian (`IsdaAs`) - T+3, ACT/365F, Tokyo calendar
    /// - **Other**: North American (default)
    ///
    /// # Example
    ///
    /// ```rust
    /// use finstack_core::currency::Currency;
    /// use finstack_valuations::instruments::credit_derivatives::cds::CDSConvention;
    ///
    /// let na = CDSConvention::detect_from_currency(Currency::USD);
    /// assert_eq!(na, CDSConvention::IsdaNa);
    ///
    /// let eu = CDSConvention::detect_from_currency(Currency::EUR);
    /// assert_eq!(eu, CDSConvention::IsdaEu);
    ///
    /// let asia = CDSConvention::detect_from_currency(Currency::JPY);
    /// assert_eq!(asia, CDSConvention::IsdaAs);
    /// ```
    #[must_use]
    pub fn detect_from_currency(currency: Currency) -> Self {
        match currency {
            // North American currencies
            Currency::USD | Currency::CAD => Self::IsdaNa,
            // European currencies
            Currency::EUR | Currency::GBP | Currency::CHF => Self::IsdaEu,
            // Asian/Pacific currencies
            Currency::JPY | Currency::AUD | Currency::HKD | Currency::SGD => Self::IsdaAs,
            // Default to North American for others (most liquid CDS market)
            _ => Self::IsdaNa,
        }
    }
}

impl Default for CDSConvention {
    /// Returns the default CDS convention.
    ///
    /// # Warning
    ///
    /// The default convention is **ISDA North American (IsdaNa)**. For non-US
    /// credits, consider using [`CDSConvention::detect_from_currency`] or
    /// explicitly specifying the convention.
    ///
    /// See module-level documentation for convention selection guidance.
    fn default() -> Self {
        Self::IsdaNa
    }
}

impl std::fmt::Display for CDSConvention {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IsdaNa => write!(f, "isda_na"),
            Self::IsdaEu => write!(f, "isda_eu"),
            Self::IsdaAs => write!(f, "isda_as"),
            Self::Custom => write!(f, "custom"),
        }
    }
}

impl std::str::FromStr for CDSConvention {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let normalized = s.trim().to_ascii_lowercase().replace('-', "_");
        match normalized.as_str() {
            "isda_na" | "isdana" | "na" => Ok(Self::IsdaNa),
            "isda_eu" | "isdaeu" | "eu" => Ok(Self::IsdaEu),
            "isda_as" | "isdaas" | "as" | "asia" => Ok(Self::IsdaAs),
            "custom" => Ok(Self::Custom),
            _ => Err(format!(
                "Unknown CDS convention: '{}'. Expected one of: isda_na, isda_eu, isda_as, custom",
                s
            )),
        }
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

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CdsConventionResolved {
    pub doc_clause: CDSConvention,
    pub day_count: DayCount,
    pub frequency: Tenor,
    pub bdc: BusinessDayConvention,
    pub stub_convention: StubKind,
    pub settlement_delay_days: u16,
    pub default_calendar_id: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CdsConventionRecord {
    doc_clause: CDSConvention,
    day_count: DayCount,
    payment_frequency: String,
    #[serde(alias = "business_day_convention")]
    bdc: BusinessDayConvention,
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
            bdc: self.bdc,
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
fn cds_conventions_registry() -> &'static finstack_core::HashMap<String, CdsConventionResolved> {
    static REGISTRY: OnceLock<finstack_core::HashMap<String, CdsConventionResolved>> =
        OnceLock::new();
    REGISTRY.get_or_init(|| {
        let json = include_str!("../../../../data/conventions/cds_conventions.json");
        let file: crate::market::conventions::loaders::json::RegistryFile<CdsConventionRecord> =
            serde_json::from_str(json)
                .expect("Failed to parse embedded CDS conventions registry JSON");

        // Build the registry, converting each record to resolved form
        let mut map = finstack_core::HashMap::default();
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
                    tracing::warn!(ids = ?entry.ids, error = %e, "Failed to load CDS convention");
                }
            }
        }
        map
    })
}

// Re-export from common parameters
pub use crate::instruments::common_impl::parameters::legs::{PremiumLegSpec, ProtectionLegSpec};

/// Resolve a meta documentation clause (e.g., `IsdaNa`, `IsdaEu`) to its
/// concrete restructuring variant. Concrete clauses pass through unchanged.
fn resolve_doc_clause(clause: CdsDocClause) -> CdsDocClause {
    match clause {
        CdsDocClause::IsdaNa => CdsDocClause::Xr14,
        CdsDocClause::IsdaEu => CdsDocClause::Mm14,
        CdsDocClause::IsdaAs => CdsDocClause::Xr14,
        CdsDocClause::IsdaAu => CdsDocClause::Xr14,
        CdsDocClause::IsdaNz => CdsDocClause::Xr14,
        // Concrete clauses pass through
        other => other,
    }
}

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
#[derive(
    Clone, Debug, finstack_valuations_macros::FinancialBuilder, serde::Serialize, serde::Deserialize,
)]
#[serde(deny_unknown_fields)]
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
    #[serde(default)]
    #[builder(default)]
    pub pricing_overrides: PricingOverrides,
    /// Upfront payment (Date, Money).
    ///
    /// The amount is defined as a payment from Protection Buyer to Protection Seller.
    /// - If positive: Buyer pays Seller.
    /// - If negative: Seller pays Buyer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upfront: Option<(Date, Money)>,
    /// ISDA documentation clause for restructuring credit events.
    ///
    /// Controls which restructuring events trigger protection payments and
    /// the maximum deliverable obligation maturity upon restructuring:
    ///
    /// - **Cr14** (Full Restructuring): All restructuring events trigger; no maturity cap.
    /// - **Mr14** (Modified Restructuring): Restructuring triggers with 30-month maturity cap.
    /// - **Mm14** (Modified-Modified Restructuring): Restructuring triggers with 60-month cap.
    /// - **Xr14** (No Restructuring): Restructuring does not trigger protection.
    ///
    /// If `None`, the effective clause is derived from the CDS convention:
    /// - `IsdaNa` / `IsdaAs` -> `Xr14` (no restructuring, North American / Asian standard)
    /// - `IsdaEu` -> `Mm14` (modified-modified restructuring, European standard)
    ///
    /// See [`doc_clause_effective`](Self::doc_clause_effective) for resolution logic.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc_clause: Option<CdsDocClause>,
    /// Optional protection effective date for forward-starting CDS.
    ///
    /// When `Some(date)`, protection begins on the specified date rather than
    /// the premium leg start date. This allows a CDS where premium accrues
    /// from the original start date but credit protection only kicks in later.
    ///
    /// Must satisfy: `premium.start <= protection_effective_date <= premium.end`.
    ///
    /// When `None`, protection starts on the premium leg start date (standard CDS).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protection_effective_date: Option<Date>,
    /// Optional OTC margin specification for VM/IM.
    ///
    /// For cleared CDS (e.g., via ICE Clear Credit), use
    /// `OtcMarginSpec::ice_clear_credit()`. For bilateral CDS,
    /// use `OtcMarginSpec::bilateral_simm()`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_spec: Option<OtcMarginSpec>,
    /// Additional attributes
    #[serde(default)]
    #[builder(default)]
    pub attributes: Attributes,
}

/// Parameters for building a CDS with standard ISDA conventions.
#[derive(Debug, Clone)]
pub struct IsdaCdsParams<'a> {
    /// Unique CDS identifier.
    pub id: InstrumentId,
    /// CDS notional.
    pub notional: Money,
    /// Direction of the premium leg.
    pub side: PayReceive,
    /// ISDA market convention family to apply.
    pub convention: CDSConvention,
    /// Running spread in basis points.
    pub spread_bp: f64,
    /// Premium-leg start date.
    pub start: Date,
    /// Premium-leg end date.
    pub end: Date,
    /// Assumed recovery rate in decimal form.
    pub recovery_rate: f64,
    /// Discount curve identifier for premium-leg discounting.
    pub discount_curve_id: &'a str,
    /// Credit curve identifier for hazard-rate / survival lookup.
    pub credit_curve_id: &'a str,
}

impl CreditDefaultSwap {
    /// Create a canonical example CDS for testing and documentation.
    ///
    /// Returns a 5-year investment-grade CDS with standard ISDA conventions.
    #[allow(clippy::expect_used)] // Example uses hardcoded valid values
    pub fn example() -> Self {
        let convention = CDSConvention::IsdaNa;
        let dc = convention.day_count();
        let freq = convention.frequency();
        let bdc = convention.business_day_convention();
        let stub = convention.stub_convention();

        let spread_bp_decimal = Decimal::try_from(100.0)
            .expect("Example CDS spread 100bp should always be representable as Decimal");

        let cds = CreditDefaultSwap::builder()
            .id(InstrumentId::new("CDS-CORP-5Y"))
            .notional(Money::new(10_000_000.0, Currency::USD))
            .side(PayReceive::PayFixed)
            .convention(convention)
            .premium(PremiumLegSpec {
                start: date!(2024 - 03 - 20),
                end: date!(2029 - 03 - 20),
                frequency: freq,
                stub,
                bdc,
                calendar_id: Some(convention.default_calendar().to_string()),
                day_count: dc,
                spread_bp: spread_bp_decimal,
                discount_curve_id: finstack_core::types::CurveId::new("USD-OIS"),
            })
            .protection(ProtectionLegSpec {
                credit_curve_id: finstack_core::types::CurveId::new("CORP-HAZARD"),
                recovery_rate: crate::instruments::credit_derivatives::cds::parameters::RECOVERY_SENIOR_UNSECURED,
                settlement_delay: convention.settlement_delay(),
            })
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("Example CDS construction should not fail");

        cds.validate()
            .expect("Example CDS validation should not fail");

        cds
    }

    /// Create a new CDS with standard ISDA conventions using explicit inputs.
    ///
    /// This is an internal helper method used by synthetic CDS creation in
    /// cds_option and cds_index modules. For public API, use `builder()`.
    ///
    /// # Arguments
    ///
    /// * `spread_bp` - Spread in basis points as Decimal
    ///
    /// # Errors
    ///
    /// Returns an error if validation fails (e.g., recovery rate out of bounds).
    pub fn from_isda(params: IsdaCdsParams<'_>) -> finstack_core::Result<Self> {
        let IsdaCdsParams {
            id,
            notional,
            side,
            convention,
            spread_bp,
            start,
            end,
            recovery_rate,
            discount_curve_id,
            credit_curve_id,
        } = params;

        Self::new_isda(
            id,
            notional,
            side,
            convention,
            crate::utils::decimal::f64_to_decimal(spread_bp, "spread_bp")?,
            start,
            end,
            recovery_rate,
            discount_curve_id,
            credit_curve_id,
        )
    }

    /// Create a new CDS with standard ISDA conventions using explicit inputs.
    ///
    /// Prefer [`Self::from_isda`] in public code. This helper remains available
    /// for internal call sites that already hold a decimal spread.
    ///
    /// # Errors
    ///
    /// Returns an error if validation fails (e.g., recovery rate out of bounds).
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
    ) -> finstack_core::Result<Self> {
        let dc = convention.day_count();
        let freq = convention.frequency();
        let bdc = convention.business_day_convention();
        let stub = convention.stub_convention();

        let cds = Self {
            id: id.into(),
            notional,
            side,
            convention,
            premium: PremiumLegSpec {
                start,
                end,
                frequency: freq,
                stub,
                bdc,
                calendar_id: Some(convention.default_calendar().to_string()),
                day_count: dc,
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
            doc_clause: None,
            protection_effective_date: None,
            margin_spec: None,
            attributes: Attributes::new(),
        };

        // Validate all parameters including recovery rate
        cds.validate()?;
        Ok(cds)
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
    /// let cds = CreditDefaultSwap::builder()
    ///     .id("CDS-EXAMPLE".into())
    ///     .notional(Money::new(10_000_000.0, Currency::USD))
    ///     .side(PayReceive::PayFixed)
    ///     .convention(CDSConvention::IsdaNa)
    ///     .premium(PremiumLegSpec {
    ///         start: date!(2024 - 03 - 20),
    ///         end: date!(2029 - 03 - 20),
    ///         freq: CDSConvention::IsdaNa.frequency(),
    ///         stub: CDSConvention::IsdaNa.stub_convention(),
    ///         bdc: CDSConvention::IsdaNa.business_day_convention(),
    ///         calendar_id: Some(CDSConvention::IsdaNa.default_calendar().to_string()),
    ///         dc: CDSConvention::IsdaNa.day_count(),
    ///         spread_bp: Decimal::try_from(100.0).expect("valid bps"),
    ///         discount_curve_id: CurveId::new("USD-OIS"),
    ///     })
    ///     .protection(ProtectionLegSpec {
    ///         credit_curve_id: CurveId::new("CORP-HAZARD"),
    ///         recovery_rate: RECOVERY_SENIOR_UNSECURED,
    ///         settlement_delay: CDSConvention::IsdaNa.settlement_delay(),
    ///     })
    ///     .pricing_overrides(PricingOverrides::default())
    ///     .attributes(Attributes::new())
    ///     .build()?;
    /// cds.validate()?; // Validates all parameters
    /// ```
    pub fn validate(&self) -> finstack_core::Result<()> {
        // Validate date ordering (start must not be after end)
        // Note: start == end is allowed for "expired" CDS (valuation handles this edge case)
        validation::validate_date_range_non_strict(
            self.premium.start,
            self.premium.end,
            "CDS premium",
        )?;

        // Validate recovery rate (must be in [0, 1])
        validation::validate_recovery_rate(self.protection.recovery_rate)?;

        // Validate protection_effective_date bounds if set
        if let Some(ped) = self.protection_effective_date {
            if ped < self.premium.start {
                return Err(finstack_core::Error::Validation(format!(
                    "CDS protection_effective_date ({}) must be >= premium start date ({})",
                    ped, self.premium.start
                )));
            }
            if ped > self.premium.end {
                return Err(finstack_core::Error::Validation(format!(
                    "CDS protection_effective_date ({}) must be <= premium end date ({})",
                    ped, self.premium.end
                )));
            }
        }

        // Note: Zero notional is allowed for testing scenarios
        // Note: Negative spreads are allowed (theoretically possible in unusual market conditions)

        Ok(())
    }

    /// Resolve the effective documentation clause.
    ///
    /// If an explicit `doc_clause` is set on the instrument, returns it directly.
    /// Otherwise, derives the standard clause from the CDS convention:
    ///
    /// | Convention | Default Clause | Rationale |
    /// |-----------|---------------|-----------|
    /// | `IsdaNa` | `Xr14` | NA standard: no restructuring (post-Big Bang) |
    /// | `IsdaEu` | `Mm14` | European standard: modified-modified restructuring |
    /// | `IsdaAs` | `Xr14` | Asian standard: follows NA convention |
    /// | `Custom` | `Xr14` | Conservative default |
    ///
    /// For meta-clauses (`IsdaNa`, `IsdaEu`, `IsdaAs` on `CdsDocClause`), the
    /// method further resolves them to their concrete restructuring variant.
    #[must_use]
    pub fn doc_clause_effective(&self) -> CdsDocClause {
        match self.doc_clause {
            Some(clause) => resolve_doc_clause(clause),
            None => match self.convention {
                CDSConvention::IsdaNa => CdsDocClause::Xr14,
                CDSConvention::IsdaEu => CdsDocClause::Mm14,
                CDSConvention::IsdaAs => CdsDocClause::Xr14,
                CDSConvention::Custom => CdsDocClause::Xr14,
            },
        }
    }

    /// Override the protection leg recovery rate.
    #[must_use]
    pub fn with_recovery_rate(mut self, recovery_rate: f64) -> Self {
        self.protection.recovery_rate = recovery_rate;
        self
    }

    /// Override the protection leg settlement delay (in business days).
    #[must_use]
    pub fn with_settlement_delay(mut self, settlement_delay: u16) -> Self {
        self.protection.settlement_delay = settlement_delay;
        self
    }

    /// Returns the effective protection start date.
    ///
    /// For a forward-starting CDS, this returns the `protection_effective_date`.
    /// For a standard (spot) CDS, this returns `premium.start`.
    #[must_use]
    pub fn protection_start(&self) -> Date {
        self.protection_effective_date.unwrap_or(self.premium.start)
    }

    fn build_premium_leg_schedule(
        &self,
    ) -> finstack_core::Result<crate::cashflow::builder::CashFlowSchedule> {
        let spread = self.premium.spread_bp.to_f64().ok_or_else(|| {
            finstack_core::Error::Validation(
                "premium spread_bp cannot be represented as f64".to_string(),
            )
        })? / 10_000.0;
        let schedule_dates = CDSPricer::new().generate_schedule(self, self.premium.start)?;
        let flows = schedule_dates
            .windows(2)
            .map(|window| {
                let start = window[0];
                let end = window[1];
                let accrual = self.premium.day_count.year_fraction(
                    start,
                    end,
                    finstack_core::dates::DayCountCtx::default(),
                )?;
                Ok(finstack_core::cashflow::CashFlow {
                    date: end,
                    reset_date: None,
                    amount: Money::new(
                        self.notional.amount() * spread * accrual,
                        self.notional.currency(),
                    ),
                    kind: finstack_core::cashflow::CFKind::Fixed,
                    accrual_factor: accrual,
                    rate: Some(spread),
                })
            })
            .collect::<finstack_core::Result<Vec<_>>>()?;

        Ok(crate::cashflow::builder::CashFlowSchedule::from_parts(
            flows,
            crate::cashflow::builder::Notional::par(
                self.notional.amount(),
                self.notional.currency(),
            ),
            self.premium.day_count,
            Default::default(),
        ))
    }

    /// Build premium leg cashflows
    pub fn build_premium_schedule(
        &self,
        _curves: &MarketContext,
        _as_of: Date,
    ) -> finstack_core::Result<DatedFlows> {
        Ok(self
            .build_premium_leg_schedule()?
            .flows
            .iter()
            .map(|cf| (cf.date, cf.amount))
            .collect())
    }

    /// ISDA-standard coupon date schedule (IMM 20th dates).
    ///
    /// This is **not** a pricing entry point; it is a schedule helper that
    /// exposes the convention-driven coupon dates used by the CDS pricer.
    ///
    /// - Dates are based on IMM 20th of Mar/Jun/Sep/Dec, with business day
    ///   adjustment per the instrument's calendar and BDC.
    /// - The returned schedule includes the start date and the (possibly adjusted)
    ///   maturity date.
    pub fn isda_coupon_schedule(&self) -> finstack_core::Result<Vec<Date>> {
        self.validate()?;
        let pricer = CDSPricer::new();
        pricer.generate_isda_schedule(self)
    }

    fn npv_raw_internal(
        &self,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        self.validate()?;
        let disc = market.get_discount(&self.premium.discount_curve_id)?;
        let surv = market.get_hazard(&self.protection.credit_curve_id)?;
        let pricer = CDSPricer::new();

        // Calculate NPV as protection leg PV - premium leg PV (from buyer's perspective)
        let protection_pv =
            pricer.pv_protection_leg_raw(self, disc.as_ref(), surv.as_ref(), as_of)?;
        let premium_pv = pricer.pv_premium_leg_raw(self, disc.as_ref(), surv.as_ref(), as_of)?;

        // Calculate dated upfront PV
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

        // PV adjustment upfront: an override that represents an additional model-level
        // upfront amount. Positive = paid by protection buyer (reduces buyer NPV,
        // increases seller NPV), matching the economic convention of the dated upfront.
        let upfront_adjustment = self
            .pricing_overrides
            .market_quotes
            .upfront_payment
            .map(|m| m.amount())
            .unwrap_or(0.0);

        // Apply sign convention based on side
        // Base NPV = Protection (received) - Premium (paid) [as Buyer]
        // Upfront: Positive amount is paid by Buyer. So it reduces Buyer NPV.
        let npv_amount = match self.side {
            PayReceive::PayFixed => {
                // Protection buyer: pays premium, receives protection, pays upfront (if positive)
                protection_pv - premium_pv - upfront_pv - upfront_adjustment
            }
            PayReceive::ReceiveFixed => {
                // Protection seller: receives premium, pays protection, receives upfront (if positive)
                premium_pv - protection_pv + upfront_pv + upfront_adjustment
            }
        };

        Ok(npv_amount)
    }

    // (no public/raw-NPV helper; use `Instrument::value_raw()` instead)
}

impl crate::instruments::common_impl::traits::Instrument for CreditDefaultSwap {
    impl_instrument_base!(crate::pricer::InstrumentType::CDS);

    fn market_dependencies(
        &self,
    ) -> finstack_core::Result<crate::instruments::common_impl::dependencies::MarketDependencies>
    {
        crate::instruments::common_impl::dependencies::MarketDependencies::from_curve_dependencies(
            self,
        )
    }
    fn as_marginable(&self) -> Option<&dyn finstack_margin::Marginable> {
        Some(self)
    }
    fn value(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        let npv_amount = self.npv_raw_internal(market, as_of)?;
        Ok(finstack_core::money::Money::new(
            npv_amount,
            self.notional.currency(),
        ))
    }

    fn value_raw(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        self.npv_raw_internal(market, as_of)
    }

    fn expiry(&self) -> Option<finstack_core::dates::Date> {
        Some(self.premium.end)
    }

    fn effective_start_date(&self) -> Option<finstack_core::dates::Date> {
        Some(self.premium.start)
    }

    fn pricing_overrides_mut(
        &mut self,
    ) -> Option<&mut crate::instruments::pricing_overrides::PricingOverrides> {
        Some(&mut self.pricing_overrides)
    }

    fn pricing_overrides(
        &self,
    ) -> Option<&crate::instruments::pricing_overrides::PricingOverrides> {
        Some(&self.pricing_overrides)
    }
}

impl crate::instruments::common_impl::traits::CurveDependencies for CreditDefaultSwap {
    fn curve_dependencies(
        &self,
    ) -> finstack_core::Result<crate::instruments::common_impl::traits::InstrumentCurves> {
        crate::instruments::common_impl::traits::InstrumentCurves::builder()
            .discount(self.premium.discount_curve_id.clone())
            .credit(self.protection.credit_curve_id.clone())
            .build()
    }
}

impl crate::cashflow::traits::CashflowProvider for CreditDefaultSwap {
    fn notional(&self) -> Option<finstack_core::money::Money> {
        Some(self.notional)
    }

    fn cashflow_schedule(
        &self,
        _curves: &MarketContext,
        _as_of: Date,
    ) -> finstack_core::Result<crate::cashflow::builder::CashFlowSchedule> {
        let mut schedule = self.build_premium_leg_schedule()?;

        if let Some((dt, amount)) = self.upfront {
            schedule.flows.push(finstack_core::cashflow::CashFlow {
                date: dt,
                reset_date: None,
                amount,
                kind: finstack_core::cashflow::CFKind::Fee,
                accrual_factor: 0.0,
                rate: None,
            });
            schedule.flows.sort_by_key(|cf| cf.date);
        }

        // Apply holder-view sign: protection buyer (PayFixed) pays premium,
        // protection seller (ReceiveFixed) receives premium.
        let sign = match self.side {
            PayReceive::PayFixed => -1.0,
            PayReceive::ReceiveFixed => 1.0,
        };
        for cf in &mut schedule.flows {
            cf.amount = Money::new(cf.amount.amount() * sign, cf.amount.currency());
        }

        schedule.meta.representation = crate::cashflow::builder::CashflowRepresentation::Projected;
        Ok(schedule)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use finstack_core::types::CurveId;
    use rust_decimal::prelude::ToPrimitive;
    use time::macros::date;

    #[test]
    fn from_isda_applies_standard_convention_fields() {
        let cds = CreditDefaultSwap::from_isda(IsdaCdsParams {
            id: InstrumentId::new("CDS-CORP-5Y"),
            notional: Money::new(10_000_000.0, Currency::USD),
            side: PayReceive::PayFixed,
            convention: CDSConvention::IsdaNa,
            spread_bp: 100.0,
            start: date!(2025 - 03 - 20),
            end: date!(2030 - 03 - 20),
            recovery_rate: 0.40,
            discount_curve_id: "USD-OIS",
            credit_curve_id: "CORP-HAZARD",
        })
        .expect("ISDA CDS constructor should succeed");

        assert_eq!(cds.id, InstrumentId::new("CDS-CORP-5Y"));
        assert_eq!(cds.notional, Money::new(10_000_000.0, Currency::USD));
        assert_eq!(cds.side, PayReceive::PayFixed);
        assert_eq!(cds.convention, CDSConvention::IsdaNa);
        assert_eq!(cds.premium.start, date!(2025 - 03 - 20));
        assert_eq!(cds.premium.end, date!(2030 - 03 - 20));
        assert_eq!(cds.premium.day_count, CDSConvention::IsdaNa.day_count());
        assert_eq!(cds.premium.frequency, CDSConvention::IsdaNa.frequency());
        assert_eq!(
            cds.premium.bdc,
            CDSConvention::IsdaNa.business_day_convention()
        );
        assert_eq!(
            cds.premium.calendar_id.as_deref(),
            Some(CDSConvention::IsdaNa.default_calendar())
        );
        assert_eq!(cds.premium.spread_bp.to_f64(), Some(100.0));
        assert_eq!(cds.premium.discount_curve_id, CurveId::new("USD-OIS"));
        assert_eq!(cds.protection.credit_curve_id, CurveId::new("CORP-HAZARD"));
        assert_eq!(cds.protection.recovery_rate, 0.40);
        assert_eq!(
            cds.protection.settlement_delay,
            CDSConvention::IsdaNa.settlement_delay()
        );
    }
}
