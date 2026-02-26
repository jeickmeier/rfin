//! Agency MBS passthrough types and implementations.
//!
//! Defines the `AgencyMbsPassthrough` instrument for agency mortgage-backed
//! securities (FNMA, FHLMC, GNMA) with prepayment modeling, servicing fees,
//! and payment delay conventions.

use crate::cashflow::builder::specs::PrepaymentModelSpec;
use crate::impl_instrument_base;
use crate::instruments::common_impl::traits::Attributes;
use crate::instruments::PricingOverrides;
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount};
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId, PoolId};
use finstack_core::Result;

/// Agency program enumeration.
///
/// Identifies the government-sponsored enterprise (GSE) or government agency
/// that guarantees the mortgage-backed security.
///
/// # GNMA Programs
///
/// Ginnie Mae has two distinct programs with different payment delay conventions:
/// - **GNMA I**: Single-issuer pools with a 14-day stated delay. Payments on the 15th.
/// - **GNMA II**: Multi-issuer pools with a 45-day stated delay. Payments on the 20th.
///
/// Use `GnmaI` or `GnmaII` to select the appropriate convention. The legacy `Gnma`
/// variant maps to GNMA II (the larger and more actively traded program).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AgencyProgram {
    /// Fannie Mae (Federal National Mortgage Association)
    Fnma,
    /// Freddie Mac (Federal Home Loan Mortgage Corporation)
    Fhlmc,
    /// Ginnie Mae II (Government National Mortgage Association) - multi-issuer pools.
    ///
    /// This is the legacy variant; equivalent to `GnmaII`. Use `GnmaI` or `GnmaII`
    /// for explicit program selection.
    Gnma,
    /// Ginnie Mae I - single-issuer pools with 14-day stated delay.
    ///
    /// GNMA I securities pay on the 15th of the month following the accrual
    /// period, resulting in a 14-day stated delay from month-end.
    GnmaI,
    /// Ginnie Mae II - multi-issuer pools with 45-day stated delay.
    ///
    /// GNMA II securities pay on the 20th of the month following the accrual
    /// period, resulting in a 45-day stated delay from month-end. This is the
    /// larger and more actively traded GNMA program.
    GnmaII,
}

impl AgencyProgram {
    /// Returns the standard payment delay in days for this agency program.
    ///
    /// # Payment Delay Conventions
    ///
    /// | Program | Delay | Payment Day | Source |
    /// |---------|-------|-------------|--------|
    /// | FNMA | 55 days | 25th of month | Fannie Mae |
    /// | FHLMC | 75 days | ~15th of following month | Freddie Mac |
    /// | GNMA I | 14 days | 15th of month | Ginnie Mae |
    /// | GNMA II | 45 days | 20th of month | Ginnie Mae |
    pub fn payment_lag_days(&self) -> u32 {
        match self {
            AgencyProgram::Fnma => 55,
            AgencyProgram::Fhlmc => 75,
            AgencyProgram::GnmaI => 14,
            AgencyProgram::Gnma | AgencyProgram::GnmaII => 45,
        }
    }

    /// Returns the canonical string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            AgencyProgram::Fnma => "FNMA",
            AgencyProgram::Fhlmc => "FHLMC",
            AgencyProgram::Gnma | AgencyProgram::GnmaII => "GNMA_II",
            AgencyProgram::GnmaI => "GNMA_I",
        }
    }

    /// Returns `true` if this is a Ginnie Mae program (any variant).
    pub fn is_gnma(&self) -> bool {
        matches!(
            self,
            AgencyProgram::Gnma | AgencyProgram::GnmaI | AgencyProgram::GnmaII
        )
    }
}

impl std::fmt::Display for AgencyProgram {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Pool type classification.
///
/// Distinguishes between generic (TBA-eligible) pools and specified pools
/// with known characteristics.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum PoolType {
    /// Generic pool (TBA-eligible, standard assumptions)
    #[default]
    Generic,
    /// Specified pool with known loan-level characteristics
    Specified,
}

/// Agency MBS passthrough instrument (pool or specified pool).
///
/// Represents an agency mortgage-backed security where principal and interest
/// payments from the underlying mortgage pool are passed through to investors,
/// net of servicing and guarantee fees.
///
/// # Cashflow Sign Convention
///
/// All cashflows are from the holder's (investor's) perspective:
/// - Principal and interest received are positive
/// - The initial purchase price is handled at trade level
///
/// # Payment Delay
///
/// Agency MBS have standardized payment delays between the accrual period end
/// and the actual payment date:
/// - FNMA: 55 days
/// - FHLMC: 75 days
/// - GNMA: 45 days (GNMA II; GNMA I uses 14 days)
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::instruments::fixed_income::mbs_passthrough::{
///     AgencyMbsPassthrough, AgencyProgram, PoolType,
/// };
/// use finstack_valuations::cashflow::builder::specs::PrepaymentModelSpec;
/// use finstack_core::currency::Currency;
/// use finstack_core::money::Money;
/// use finstack_core::dates::Date;
/// use finstack_core::types::{CurveId, InstrumentId};
/// use time::Month;
///
/// let mbs = AgencyMbsPassthrough::builder()
///     .id(InstrumentId::new("FN-MA1234"))
///     .pool_id("MA1234".into())
///     .agency(AgencyProgram::Fnma)
///     .pool_type(PoolType::Generic)
///     .original_face(Money::new(1_000_000.0, Currency::USD))
///     .current_face(Money::new(950_000.0, Currency::USD))
///     .current_factor(0.95)
///     .wac(0.045)
///     .pass_through_rate(0.04)
///     .servicing_fee_rate(0.0025)
///     .guarantee_fee_rate(0.0025)
///     .wam(348)
///     .issue_date(Date::from_calendar_date(2022, Month::January, 1).unwrap())
///     .maturity(Date::from_calendar_date(2052, Month::January, 1).unwrap())
///     .prepayment_model(PrepaymentModelSpec::psa(1.0))
///     .discount_curve_id(CurveId::new("USD-OIS"))
///     .day_count(finstack_core::dates::DayCount::Thirty360)
///     .build()
///     .expect("Valid MBS");
/// ```
#[derive(
    Clone, Debug, finstack_valuations_macros::FinancialBuilder, serde::Serialize, serde::Deserialize,
)]
#[serde(deny_unknown_fields)]
pub struct AgencyMbsPassthrough {
    /// Unique instrument identifier.
    pub id: InstrumentId,
    /// Pool identifier (CUSIP or internal pool ID).
    pub pool_id: PoolId,
    /// Agency program (FNMA, FHLMC, GNMA).
    pub agency: AgencyProgram,
    /// Pool type (generic or specified).
    #[builder(default)]
    #[serde(default)]
    pub pool_type: PoolType,
    /// Original face amount (initial principal balance).
    pub original_face: Money,
    /// Current face amount (remaining principal balance).
    pub current_face: Money,
    /// Current pool factor (current_face / original_face).
    pub current_factor: f64,
    /// Weighted average coupon (gross rate on underlying mortgages).
    pub wac: f64,
    /// Pass-through rate (net coupon to investor).
    pub pass_through_rate: f64,
    /// Servicing fee rate (annual, as decimal e.g., 0.0025 for 25 bps).
    ///
    /// Defaults to `0.0` when omitted.
    #[builder(default)]
    #[serde(default)]
    pub servicing_fee_rate: f64,
    /// Guarantee fee rate (annual, as decimal e.g., 0.0025 for 25 bps).
    ///
    /// Defaults to `0.0` when omitted.
    #[builder(default)]
    #[serde(default)]
    pub guarantee_fee_rate: f64,
    /// Weighted average maturity in months.
    pub wam: u32,
    /// Issue date of the pool.
    pub issue_date: Date,
    /// Legal maturity date.
    #[serde(alias = "maturity")]
    pub maturity: Date,
    /// Optional custom payment delay (overrides agency default).
    #[builder(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment_lag_days: Option<u32>,
    /// Prepayment model specification.
    pub prepayment_model: PrepaymentModelSpec,
    /// Discount curve identifier for pricing.
    pub discount_curve_id: CurveId,
    /// Day count convention for accrual.
    pub day_count: DayCount,
    /// Pricing overrides (including quoted price for OAS).
    #[builder(default)]
    #[serde(default)]
    pub pricing_overrides: PricingOverrides,
    /// Attributes for scenario selection and tagging.
    #[builder(default)]
    #[serde(default)]
    /// Attributes for scenario selection and tagging
    pub attributes: Attributes,
}

impl AgencyMbsPassthrough {
    /// Create a canonical example MBS for testing and documentation.
    ///
    /// Returns a FNMA 30-year pool with realistic parameters.
    pub fn example() -> Self {
        use time::macros::date;
        Self::builder()
            .id(InstrumentId::new("FN-MA1234"))
            .pool_id("MA1234".into())
            .agency(AgencyProgram::Fnma)
            .pool_type(PoolType::Generic)
            .original_face(Money::new(1_000_000.0, Currency::USD))
            .current_face(Money::new(950_000.0, Currency::USD))
            .current_factor(0.95)
            .wac(0.045)
            .pass_through_rate(0.04)
            .servicing_fee_rate(0.0025)
            .guarantee_fee_rate(0.0025)
            .wam(348)
            .issue_date(date!(2022 - 01 - 01))
            .maturity(date!(2052 - 01 - 01))
            .prepayment_model(PrepaymentModelSpec::psa(1.0))
            .discount_curve_id(CurveId::new("USD-OIS"))
            .day_count(DayCount::Thirty360)
            .pricing_overrides(PricingOverrides::default())
            .attributes(
                Attributes::new()
                    .with_tag("mbs")
                    .with_tag("agency")
                    .with_meta("program", "fnma"),
            )
            .build()
            .unwrap_or_else(|_| unreachable!("Example MBS with valid constants should never fail"))
    }

    /// Get the effective payment delay in days.
    ///
    /// Uses custom delay if set, otherwise uses agency-standard delay.
    pub fn effective_payment_delay(&self) -> u32 {
        self.payment_lag_days
            .unwrap_or_else(|| self.agency.payment_lag_days())
    }

    /// Calculate seasoning in months from issue date to given date.
    pub fn seasoning_months(&self, as_of: Date) -> u32 {
        let days = (as_of - self.issue_date).whole_days();
        if days <= 0 {
            0
        } else {
            (days as f64 / 30.4375).floor() as u32
        }
    }

    /// Get SMM (single monthly mortality) for given date.
    pub fn smm(&self, as_of: Date) -> f64 {
        let seasoning = self.seasoning_months(as_of);
        self.prepayment_model.smm(seasoning)
    }

    /// Calculate net coupon (pass-through rate) from WAC and fees.
    ///
    /// Should equal: WAC - servicing_fee_rate - guarantee_fee_rate
    pub fn calculated_net_coupon(&self) -> f64 {
        self.wac - self.servicing_fee_rate - self.guarantee_fee_rate
    }

    /// Validate that pass-through rate is consistent with WAC and fees.
    pub fn validate_coupon_consistency(&self) -> Result<()> {
        let calculated = self.calculated_net_coupon();
        let diff = (self.pass_through_rate - calculated).abs();
        if diff > 1e-6 {
            return Err(finstack_core::Error::Validation(format!(
                "Pass-through rate {} does not match WAC {} - servicing {} - g-fee {} = {}",
                self.pass_through_rate,
                self.wac,
                self.servicing_fee_rate,
                self.guarantee_fee_rate,
                calculated
            )));
        }
        Ok(())
    }
}

impl crate::instruments::common_impl::traits::CurveDependencies for AgencyMbsPassthrough {
    fn curve_dependencies(
        &self,
    ) -> finstack_core::Result<crate::instruments::common_impl::traits::InstrumentCurves> {
        crate::instruments::common_impl::traits::InstrumentCurves::builder()
            .discount(self.discount_curve_id.clone())
            .build()
    }
}

impl crate::instruments::common_impl::traits::Instrument for AgencyMbsPassthrough {
    impl_instrument_base!(crate::pricer::InstrumentType::AgencyMbsPassthrough);

    fn value(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        crate::instruments::fixed_income::mbs_passthrough::pricer::price_mbs(self, market, as_of)
    }

    fn effective_start_date(&self) -> Option<Date> {
        Some(self.issue_date)
    }

    fn scenario_overrides_mut(
        &mut self,
    ) -> Option<&mut crate::instruments::pricing_overrides::PricingOverrides> {
        Some(&mut self.pricing_overrides)
    }

    fn scenario_overrides(
        &self,
    ) -> Option<&crate::instruments::pricing_overrides::PricingOverrides> {
        Some(&self.pricing_overrides)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use time::Month;

    #[test]
    fn test_agency_program_payment_delays() {
        assert_eq!(AgencyProgram::Fnma.payment_lag_days(), 55);
        assert_eq!(AgencyProgram::Fhlmc.payment_lag_days(), 75);
        assert_eq!(AgencyProgram::Gnma.payment_lag_days(), 45);
        assert_eq!(AgencyProgram::GnmaI.payment_lag_days(), 14);
        assert_eq!(AgencyProgram::GnmaII.payment_lag_days(), 45);
    }

    #[test]
    fn test_agency_program_display() {
        assert_eq!(AgencyProgram::Fnma.as_str(), "FNMA");
        assert_eq!(AgencyProgram::Fhlmc.as_str(), "FHLMC");
        assert_eq!(AgencyProgram::Gnma.as_str(), "GNMA_II");
        assert_eq!(AgencyProgram::GnmaI.as_str(), "GNMA_I");
        assert_eq!(AgencyProgram::GnmaII.as_str(), "GNMA_II");
    }

    #[test]
    fn test_agency_program_is_gnma() {
        assert!(!AgencyProgram::Fnma.is_gnma());
        assert!(!AgencyProgram::Fhlmc.is_gnma());
        assert!(AgencyProgram::Gnma.is_gnma());
        assert!(AgencyProgram::GnmaI.is_gnma());
        assert!(AgencyProgram::GnmaII.is_gnma());
    }

    #[test]
    fn test_mbs_example() {
        let mbs = AgencyMbsPassthrough::example();
        assert_eq!(mbs.id.as_str(), "FN-MA1234");
        assert_eq!(mbs.agency, AgencyProgram::Fnma);
        assert_eq!(mbs.pool_type, PoolType::Generic);
        assert!((mbs.current_factor - 0.95).abs() < 1e-10);
        assert!(mbs.attributes.has_tag("mbs"));
    }

    #[test]
    fn test_effective_payment_delay() {
        let mbs = AgencyMbsPassthrough::example();
        // Default should use agency standard
        assert_eq!(mbs.effective_payment_delay(), 55);

        // Custom delay should override
        let mut mbs_custom = mbs.clone();
        mbs_custom.payment_lag_days = Some(55);
        assert_eq!(mbs_custom.effective_payment_delay(), 55);
    }

    #[test]
    fn test_seasoning_months() {
        let mbs = AgencyMbsPassthrough::example();
        let as_of = Date::from_calendar_date(2024, Month::January, 1).expect("valid date");
        let seasoning = mbs.seasoning_months(as_of);
        // 2 years = ~24 months (allow for slight calculation differences)
        assert!((23..=24).contains(&seasoning));
    }

    #[test]
    fn test_calculated_net_coupon() {
        let mbs = AgencyMbsPassthrough::example();
        let calculated = mbs.calculated_net_coupon();
        // 4.5% - 0.25% - 0.25% = 4.0%
        assert!((calculated - 0.04).abs() < 1e-10);
    }

    #[test]
    fn test_coupon_consistency_validation() {
        let mbs = AgencyMbsPassthrough::example();
        assert!(mbs.validate_coupon_consistency().is_ok());

        // Create inconsistent MBS
        let mut bad_mbs = mbs.clone();
        bad_mbs.pass_through_rate = 0.05; // Wrong rate
        assert!(bad_mbs.validate_coupon_consistency().is_err());
    }

    #[test]
    fn test_smm_calculation() {
        let mbs = AgencyMbsPassthrough::example();
        let as_of = Date::from_calendar_date(2024, Month::January, 1).expect("valid date");
        let smm = mbs.smm(as_of);
        // PSA 100% at 24 months seasoning should give ~0.5% SMM
        assert!(smm > 0.0 && smm < 0.02);
    }

    #[test]
    fn test_mbs_serde_roundtrip() {
        let mbs = AgencyMbsPassthrough::example();
        let json = serde_json::to_string(&mbs).expect("serialize");
        let deserialized: AgencyMbsPassthrough = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(mbs.id.as_str(), deserialized.id.as_str());
        assert_eq!(mbs.agency, deserialized.agency);
    }

    #[test]
    fn test_serde_defaults_fee_rates_to_zero_when_omitted() {
        let mut value = serde_json::to_value(AgencyMbsPassthrough::example()).expect("serialize");
        let obj = value
            .as_object_mut()
            .expect("AgencyMbsPassthrough should serialize to an object");
        obj.remove("servicing_fee_rate");
        obj.remove("guarantee_fee_rate");

        let mbs: AgencyMbsPassthrough = serde_json::from_value(value).expect("deserialize");
        assert_eq!(mbs.servicing_fee_rate, 0.0);
        assert_eq!(mbs.guarantee_fee_rate, 0.0);
    }
}
