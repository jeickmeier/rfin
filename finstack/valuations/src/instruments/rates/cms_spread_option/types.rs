//! CMS Spread Option instrument definition.

use crate::impl_instrument_base;
use crate::instruments::common_impl::traits::Attributes;
use crate::instruments::common_impl::validation;
use crate::instruments::PricingOverrides;
use finstack_core::dates::{Date, DayCount, Tenor};
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};

/// Call or put on a CMS spread.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum CmsSpreadOptionType {
    /// max(CMS_long - CMS_short - K, 0)
    Call,
    /// max(K - (CMS_long - CMS_short), 0)
    Put,
}

impl std::fmt::Display for CmsSpreadOptionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CmsSpreadOptionType::Call => write!(f, "call"),
            CmsSpreadOptionType::Put => write!(f, "put"),
        }
    }
}

/// CMS Spread Option.
///
/// Option on the spread between two CMS rates of different tenors.
///
/// ```text
/// Payoff = max(CMS_long - CMS_short - strike, 0) * notional    [for a call]
/// Payoff = max(strike - (CMS_long - CMS_short), 0) * notional   [for a put]
/// ```
///
/// Typically: long tenor = 10Y or 30Y CMS, short tenor = 2Y CMS.
///
/// # Pricing Approach
///
/// 1. Each CMS rate has SABR marginal distribution (reuses CMS option SABR calibration)
/// 2. Joint distribution via Gaussian copula with rank correlation
/// 3. CMS convexity adjustment applied to each leg via static replication
///
/// # References
///
/// - Hagan, P. S. (2003). "Convexity Conundrums." *Wilmott Magazine*.
/// - Antonov, A., Konikov, M., & Spector, M. (2013). "SABR Spreads." *Risk*.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CmsSpreadOption {
    /// Unique instrument identifier.
    pub id: InstrumentId,
    /// Long CMS tenor (e.g., 10Y).
    pub long_cms_tenor: Tenor,
    /// Short CMS tenor (e.g., 2Y).
    pub short_cms_tenor: Tenor,
    /// Strike spread (in decimal, e.g., 0.005 = 50bp).
    pub strike: f64,
    /// Call or put on the spread.
    pub option_type: CmsSpreadOptionType,
    /// Notional amount.
    pub notional: Money,
    /// Option expiry date.
    #[schemars(with = "String")]
    pub expiry_date: Date,
    /// Payment date (may differ from expiry).
    #[schemars(with = "String")]
    pub payment_date: Date,
    /// Swaption volatility surface for long tenor.
    pub long_vol_surface_id: CurveId,
    /// Swaption volatility surface for short tenor.
    pub short_vol_surface_id: CurveId,
    /// Discount curve ID.
    pub discount_curve_id: CurveId,
    /// Forward curve ID (for swap rate projection).
    pub forward_curve_id: CurveId,
    /// Rank correlation between the two CMS rates.
    pub spread_correlation: f64,
    /// Day count convention.
    pub day_count: DayCount,
    /// Pricing overrides.
    pub pricing_overrides: PricingOverrides,
    /// Attributes.
    pub attributes: Attributes,
}

impl CmsSpreadOption {
    /// Validate the CMS spread option parameters.
    ///
    /// Checks:
    /// - Long tenor must be strictly longer than short tenor
    /// - Strike is finite
    /// - Expiry date is before or on payment date
    /// - Correlation is in [-1, 1]
    pub fn validate(&self) -> finstack_core::Result<()> {
        // Tenor comparison: long must be > short (compare months if both are month/year based)
        validation::require_with(
            self.long_cms_tenor.months() > self.short_cms_tenor.months(),
            || {
                format!(
                    "CmsSpreadOption long_cms_tenor ({}) must be longer than short_cms_tenor ({})",
                    self.long_cms_tenor, self.short_cms_tenor
                )
            },
        )?;

        validation::require_with(self.strike.is_finite(), || {
            format!(
                "CmsSpreadOption strike ({}) must be finite",
                self.strike
            )
        })?;

        validation::require_with(self.payment_date >= self.expiry_date, || {
            format!(
                "CmsSpreadOption payment_date ({}) must be on or after expiry_date ({})",
                self.payment_date, self.expiry_date
            )
        })?;

        validation::require_with(
            (-1.0..=1.0).contains(&self.spread_correlation),
            || {
                format!(
                    "CmsSpreadOption spread_correlation ({}) must be in [-1, 1]",
                    self.spread_correlation
                )
            },
        )?;

        Ok(())
    }

    /// Create a canonical example CMS spread option for testing.
    #[allow(clippy::expect_used)]
    pub fn example() -> Self {
        use finstack_core::currency::Currency;
        use time::Month;

        CmsSpreadOption {
            id: InstrumentId::new("CMS-SPREAD-10Y2Y"),
            long_cms_tenor: Tenor::new(10, finstack_core::dates::TenorUnit::Years),
            short_cms_tenor: Tenor::new(2, finstack_core::dates::TenorUnit::Years),
            strike: 0.005, // 50bp
            option_type: CmsSpreadOptionType::Call,
            notional: Money::new(10_000_000.0, Currency::USD),
            expiry_date: Date::from_calendar_date(2027, Month::March, 29).expect("valid"),
            payment_date: Date::from_calendar_date(2027, Month::March, 31).expect("valid"),
            long_vol_surface_id: CurveId::new("USD-SWAPTION-VOL-10Y"),
            short_vol_surface_id: CurveId::new("USD-SWAPTION-VOL-2Y"),
            discount_curve_id: CurveId::new("USD-OIS"),
            forward_curve_id: CurveId::new("USD-SOFR-3M"),
            spread_correlation: 0.85,
            day_count: DayCount::Act360,
            pricing_overrides: PricingOverrides::default(),
            attributes: Attributes::new(),
        }
    }
}

impl crate::instruments::common_impl::traits::Instrument for CmsSpreadOption {
    impl_instrument_base!(crate::pricer::InstrumentType::CmsSpreadOption);

    fn default_model(&self) -> crate::pricer::ModelKey {
        crate::pricer::ModelKey::StaticReplication
    }

    fn market_dependencies(
        &self,
    ) -> finstack_core::Result<crate::instruments::common_impl::dependencies::MarketDependencies>
    {
        crate::instruments::common_impl::dependencies::MarketDependencies::from_curve_dependencies(
            self,
        )
    }

    fn value(
        &self,
        _market: &finstack_core::market_data::context::MarketContext,
        _as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        self.validate()?;
        Err(finstack_core::Error::Validation(
            "CMS Spread Option pricing requires copula-based engine with SABR marginals. \
             Use price_with_metrics with the static replication pricer."
                .to_string(),
        ))
    }

    fn effective_start_date(&self) -> Option<Date> {
        Some(self.expiry_date)
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

impl crate::instruments::common_impl::traits::CurveDependencies for CmsSpreadOption {
    fn curve_dependencies(
        &self,
    ) -> finstack_core::Result<crate::instruments::common_impl::traits::InstrumentCurves> {
        crate::instruments::common_impl::traits::InstrumentCurves::builder()
            .discount(self.discount_curve_id.clone())
            .forward(self.forward_curve_id.clone())
            .build()
    }
}

crate::impl_empty_cashflow_provider!(
    CmsSpreadOption,
    crate::cashflow::builder::CashflowRepresentation::Placeholder
);

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn example_validates() {
        let opt = CmsSpreadOption::example();
        assert!(opt.validate().is_ok());
    }

    #[test]
    fn long_tenor_shorter_than_short_fails() {
        let mut opt = CmsSpreadOption::example();
        // Swap the tenors so long < short
        opt.long_cms_tenor = Tenor::new(2, finstack_core::dates::TenorUnit::Years);
        opt.short_cms_tenor = Tenor::new(10, finstack_core::dates::TenorUnit::Years);
        assert!(opt.validate().is_err());
    }

    #[test]
    fn correlation_out_of_range_fails() {
        let mut opt = CmsSpreadOption::example();
        opt.spread_correlation = 1.5;
        assert!(opt.validate().is_err());
    }

    #[test]
    fn payment_before_expiry_fails() {
        use time::Month;
        let mut opt = CmsSpreadOption::example();
        opt.payment_date = Date::from_calendar_date(2027, Month::March, 28).expect("valid");
        assert!(opt.validate().is_err());
    }

    #[test]
    fn instrument_trait() {
        use crate::instruments::common_impl::traits::Instrument;
        let opt = CmsSpreadOption::example();
        assert_eq!(opt.id(), "CMS-SPREAD-10Y2Y");
        assert_eq!(opt.key(), crate::pricer::InstrumentType::CmsSpreadOption);
    }

    #[test]
    fn serde_roundtrip() {
        let opt = CmsSpreadOption::example();
        let json = serde_json::to_string(&opt).expect("serialize");
        let deser: CmsSpreadOption = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deser.id, opt.id);
        assert!((deser.strike - opt.strike).abs() < 1e-12);
    }
}
