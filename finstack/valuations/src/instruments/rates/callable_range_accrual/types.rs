//! Callable Range Accrual instrument definition.

use crate::impl_instrument_base;
use crate::instruments::common_impl::traits::Attributes;
use crate::instruments::common_impl::validation;
use crate::instruments::rates::range_accrual::BoundsType;
use crate::instruments::rates::shared::bermudan_call::BermudanCallProvision;
use crate::instruments::PricingOverrides;
use finstack_core::dates::{Date, DayCount};
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId, PriceId};

/// Core range accrual parameters extracted for reuse in callable wrapper.
///
/// Mirrors the essential fields from [`crate::instruments::rates::range_accrual::RangeAccrual`]
/// without the instrument-level metadata (id, attributes, overrides).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RangeAccrualSpec {
    /// Underlying ticker.
    pub underlying_ticker: String,
    /// Observation dates (must be sorted ascending).
    #[schemars(with = "Vec<String>")]
    pub observation_dates: Vec<Date>,
    /// Lower bound of accrual range.
    pub lower_bound: f64,
    /// Upper bound of accrual range.
    pub upper_bound: f64,
    /// Bounds interpretation.
    #[serde(default)]
    pub bounds_type: BoundsType,
    /// Coupon rate when in range.
    pub coupon_rate: f64,
    /// Notional.
    pub notional: Money,
    /// Day count convention.
    pub day_count: DayCount,
    /// Spot price identifier.
    pub spot_id: PriceId,
}

impl RangeAccrualSpec {
    /// Validate the range accrual spec parameters.
    pub fn validate(&self) -> finstack_core::Result<()> {
        validation::require_with(!self.observation_dates.is_empty(), || {
            "RangeAccrualSpec requires at least one observation date".to_string()
        })?;

        validation::validate_sorted_strict(
            &self.observation_dates,
            "RangeAccrualSpec observation_dates",
        )?;

        validation::require_with(self.lower_bound < self.upper_bound, || {
            format!(
                "RangeAccrualSpec lower_bound ({}) must be strictly less than upper_bound ({})",
                self.lower_bound, self.upper_bound
            )
        })?;

        validation::require_with(self.coupon_rate >= 0.0, || {
            format!(
                "RangeAccrualSpec coupon_rate ({}) must be non-negative",
                self.coupon_rate
            )
        })?;

        Ok(())
    }
}

/// Callable Range Accrual.
///
/// Extends the existing range accrual concept with a Bermudan call provision
/// allowing the issuer to terminate early on specified call dates.
///
/// The call decision interacts with the range accrual feature: the issuer
/// will call when the expected future value of remaining range accrual
/// coupons exceeds the call price (par). Pricing requires backward
/// induction (LSMC or HW tree) combined with forward range accrual
/// coupon simulation.
///
/// # Pricing
///
/// - **LSMC**: Simulate paths with HW1F short rate model. At each call
///   date, compute continuation value via regression. Exercise if
///   call_price < continuation_value.
/// - **HW Tree**: Build trinomial tree, attach range accrual cashflows
///   at each node, apply backward induction with call decision.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CallableRangeAccrual {
    /// Unique instrument identifier.
    pub id: InstrumentId,
    /// Underlying range accrual specification (observation dates, bounds, coupon).
    pub range_accrual: RangeAccrualSpec,
    /// Bermudan call provision.
    pub call_provision: BermudanCallProvision,
    /// Discount curve ID.
    pub discount_curve_id: CurveId,
    /// Volatility surface ID for the reference rate.
    pub vol_surface_id: CurveId,
    /// Pricing overrides.
    pub pricing_overrides: PricingOverrides,
    /// Attributes.
    pub attributes: Attributes,
}

impl CallableRangeAccrual {
    /// Validate the callable range accrual parameters.
    ///
    /// Validates both the underlying range accrual spec and the call provision.
    pub fn validate(&self) -> finstack_core::Result<()> {
        self.range_accrual.validate()?;
        self.call_provision.validate()?;
        Ok(())
    }

    /// Create a canonical example callable range accrual for testing.
    #[allow(clippy::expect_used)]
    pub fn example() -> Self {
        use finstack_core::currency::Currency;
        use time::Month;

        let observation_dates = vec![
            Date::from_calendar_date(2026, Month::January, 31).expect("valid"),
            Date::from_calendar_date(2026, Month::February, 28).expect("valid"),
            Date::from_calendar_date(2026, Month::March, 31).expect("valid"),
            Date::from_calendar_date(2026, Month::April, 30).expect("valid"),
            Date::from_calendar_date(2026, Month::May, 31).expect("valid"),
            Date::from_calendar_date(2026, Month::June, 30).expect("valid"),
            Date::from_calendar_date(2026, Month::July, 31).expect("valid"),
            Date::from_calendar_date(2026, Month::August, 31).expect("valid"),
            Date::from_calendar_date(2026, Month::September, 30).expect("valid"),
            Date::from_calendar_date(2026, Month::October, 31).expect("valid"),
            Date::from_calendar_date(2026, Month::November, 30).expect("valid"),
            Date::from_calendar_date(2026, Month::December, 31).expect("valid"),
        ];

        let call_dates = vec![
            Date::from_calendar_date(2026, Month::June, 30).expect("valid"),
            Date::from_calendar_date(2026, Month::September, 30).expect("valid"),
            Date::from_calendar_date(2026, Month::December, 31).expect("valid"),
        ];

        CallableRangeAccrual {
            id: InstrumentId::new("CALLABLE-RA-SOFR-1Y"),
            range_accrual: RangeAccrualSpec {
                underlying_ticker: "SOFR".to_string(),
                observation_dates,
                lower_bound: 0.04,
                upper_bound: 0.06,
                bounds_type: BoundsType::Absolute,
                coupon_rate: 0.065,
                notional: Money::new(1_000_000.0, Currency::USD),
                day_count: DayCount::Act360,
                spot_id: "SOFR-RATE".into(),
            },
            call_provision: BermudanCallProvision::new(call_dates, 1.0, 1),
            discount_curve_id: CurveId::new("USD-OIS"),
            vol_surface_id: CurveId::new("SOFR-VOL"),
            pricing_overrides: PricingOverrides::default(),
            attributes: Attributes::new(),
        }
    }
}

impl crate::instruments::common_impl::traits::Instrument for CallableRangeAccrual {
    impl_instrument_base!(crate::pricer::InstrumentType::CallableRangeAccrual);

    fn default_model(&self) -> crate::pricer::ModelKey {
        crate::pricer::ModelKey::MonteCarloHullWhite1F
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
            "Callable Range Accrual pricing requires LSMC or HW tree (mc feature). \
             Use price_with_metrics with a MC pricer."
                .to_string(),
        ))
    }

    fn effective_start_date(&self) -> Option<Date> {
        self.range_accrual.observation_dates.first().copied()
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

impl crate::instruments::common_impl::traits::CurveDependencies for CallableRangeAccrual {
    fn curve_dependencies(
        &self,
    ) -> finstack_core::Result<crate::instruments::common_impl::traits::InstrumentCurves> {
        crate::instruments::common_impl::traits::InstrumentCurves::builder()
            .discount(self.discount_curve_id.clone())
            .build()
    }
}

crate::impl_empty_cashflow_provider!(
    CallableRangeAccrual,
    crate::cashflow::builder::CashflowRepresentation::Placeholder
);

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn example_validates() {
        let cra = CallableRangeAccrual::example();
        assert!(cra.validate().is_ok());
    }

    #[test]
    fn invalid_range_fails() {
        let mut cra = CallableRangeAccrual::example();
        cra.range_accrual.lower_bound = 0.06;
        cra.range_accrual.upper_bound = 0.04;
        assert!(cra.validate().is_err());
    }

    #[test]
    fn invalid_call_provision_fails() {
        let mut cra = CallableRangeAccrual::example();
        cra.call_provision.call_dates = vec![];
        assert!(cra.validate().is_err());
    }

    #[test]
    fn instrument_trait() {
        use crate::instruments::common_impl::traits::Instrument;
        let cra = CallableRangeAccrual::example();
        assert_eq!(cra.id(), "CALLABLE-RA-SOFR-1Y");
        assert_eq!(
            cra.key(),
            crate::pricer::InstrumentType::CallableRangeAccrual
        );
    }

    #[test]
    fn serde_roundtrip() {
        let cra = CallableRangeAccrual::example();
        let json = serde_json::to_string(&cra).expect("serialize");
        let deser: CallableRangeAccrual = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deser.id, cra.id);
        assert!(
            (deser.range_accrual.coupon_rate - cra.range_accrual.coupon_rate).abs() < 1e-12
        );
    }
}
