//! Agency MBS passthrough types and implementations.
//!
//! Defines the `AgencyMbsPassthrough` instrument for agency mortgage-backed
//! securities (FNMA, FHLMC, GNMA) with prepayment modeling, servicing fees,
//! and payment delay conventions.

use crate::cashflow::builder::specs::PrepaymentModelSpec;
use crate::instruments::common::traits::Attributes;
use crate::instruments::PricingOverrides;
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount};
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_core::Result;

/// Agency program enumeration.
///
/// Identifies the government-sponsored enterprise (GSE) or government agency
/// that guarantees the mortgage-backed security.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "SCREAMING_SNAKE_CASE"))]
pub enum AgencyProgram {
    /// Fannie Mae (Federal National Mortgage Association)
    Fnma,
    /// Freddie Mac (Federal Home Loan Mortgage Corporation)
    Fhlmc,
    /// Ginnie Mae (Government National Mortgage Association) - government-backed
    Gnma,
}

impl AgencyProgram {
    /// Returns the standard payment delay in days for this agency program.
    ///
    /// # Payment Delay Conventions
    ///
    /// - FNMA: 25 days (Fannie Mae standard)
    /// - FHLMC: 45 days (Freddie Mac Gold)
    /// - GNMA: 45 days (Ginnie Mae II)
    pub fn payment_delay_days(&self) -> u32 {
        match self {
            AgencyProgram::Fnma => 25,
            AgencyProgram::Fhlmc => 45,
            AgencyProgram::Gnma => 45,
        }
    }

    /// Returns the canonical string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            AgencyProgram::Fnma => "FNMA",
            AgencyProgram::Fhlmc => "FHLMC",
            AgencyProgram::Gnma => "GNMA",
        }
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
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
/// - FNMA: 25 days
/// - FHLMC: 45 days
/// - GNMA: 45 days
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
///     .pool_id("MA1234".to_string())
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
///     .maturity_date(Date::from_calendar_date(2052, Month::January, 1).unwrap())
///     .prepayment_model(PrepaymentModelSpec::psa(1.0))
///     .discount_curve_id(CurveId::new("USD-OIS"))
///     .day_count(finstack_core::dates::DayCount::Thirty360)
///     .build()
///     .expect("Valid MBS");
/// ```
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct AgencyMbsPassthrough {
    /// Unique instrument identifier.
    pub id: InstrumentId,
    /// Pool identifier (CUSIP or internal pool ID).
    pub pool_id: String,
    /// Agency program (FNMA, FHLMC, GNMA).
    pub agency: AgencyProgram,
    /// Pool type (generic or specified).
    #[builder(default)]
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
    pub servicing_fee_rate: f64,
    /// Guarantee fee rate (annual, as decimal e.g., 0.0025 for 25 bps).
    pub guarantee_fee_rate: f64,
    /// Weighted average maturity in months.
    pub wam: u32,
    /// Issue date of the pool.
    pub issue_date: Date,
    /// Legal maturity date.
    pub maturity_date: Date,
    /// Optional custom payment delay (overrides agency default).
    #[builder(optional)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub payment_delay_days: Option<u32>,
    /// Prepayment model specification.
    pub prepayment_model: PrepaymentModelSpec,
    /// Discount curve identifier for pricing.
    pub discount_curve_id: CurveId,
    /// Day count convention for accrual.
    pub day_count: DayCount,
    /// Pricing overrides (including quoted price for OAS).
    #[builder(default)]
    #[cfg_attr(feature = "serde", serde(default))]
    pub pricing_overrides: PricingOverrides,
    /// Attributes for scenario selection and tagging.
    #[builder(default)]
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
            .pool_id("MA1234".to_string())
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
            .maturity_date(date!(2052 - 01 - 01))
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
        self.payment_delay_days
            .unwrap_or_else(|| self.agency.payment_delay_days())
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

impl crate::instruments::common::traits::CurveDependencies for AgencyMbsPassthrough {
    fn curve_dependencies(&self) -> crate::instruments::common::traits::InstrumentCurves {
        crate::instruments::common::traits::InstrumentCurves::builder()
            .discount(self.discount_curve_id.clone())
            .build()
    }
}

impl crate::instruments::common::traits::Instrument for AgencyMbsPassthrough {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::AgencyMbsPassthrough
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
        crate::instruments::agency_mbs_passthrough::pricer::price_mbs(self, market, as_of)
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
            None,
        )
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
        assert_eq!(AgencyProgram::Fnma.payment_delay_days(), 25);
        assert_eq!(AgencyProgram::Fhlmc.payment_delay_days(), 45);
        assert_eq!(AgencyProgram::Gnma.payment_delay_days(), 45);
    }

    #[test]
    fn test_agency_program_display() {
        assert_eq!(AgencyProgram::Fnma.as_str(), "FNMA");
        assert_eq!(AgencyProgram::Fhlmc.as_str(), "FHLMC");
        assert_eq!(AgencyProgram::Gnma.as_str(), "GNMA");
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
        assert_eq!(mbs.effective_payment_delay(), 25);

        // Custom delay should override
        let mut mbs_custom = mbs.clone();
        mbs_custom.payment_delay_days = Some(55);
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

    #[cfg(feature = "serde")]
    #[test]
    fn test_mbs_serde_roundtrip() {
        let mbs = AgencyMbsPassthrough::example();
        let json = serde_json::to_string(&mbs).expect("serialize");
        let deserialized: AgencyMbsPassthrough = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(mbs.id.as_str(), deserialized.id.as_str());
        assert_eq!(mbs.agency, deserialized.agency);
    }
}
