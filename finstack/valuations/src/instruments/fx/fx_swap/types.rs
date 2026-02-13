//! FX Swap types and instrument integration.
//!
//! This file defines the `FxSwap` instrument shape and provides the
//! integration with the shared instrument trait via the `impl_instrument!`
//! macro. Core PV logic is delegated to `pricing::engine` to follow the
//! repository standards. Metrics live under `metrics/` and are registered
//! via the instrument metrics module.

use crate::instruments::common_impl::parameters::FxUnderlyingParams;
use crate::instruments::common_impl::traits::Attributes;
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};

use super::parameters::FxSwapParams;
use crate::impl_instrument_base;

/// FX Swap instrument definition
#[derive(
    Clone, Debug, finstack_valuations_macros::FinancialBuilder, serde::Serialize, serde::Deserialize,
)]
#[serde(deny_unknown_fields)]
pub struct FxSwap {
    /// Unique instrument identifier
    pub id: InstrumentId,
    /// Base currency (foreign)
    pub base_currency: Currency,
    /// Quote currency (domestic)
    pub quote_currency: Currency,
    /// Near leg settlement date (spot leg)
    pub near_date: Date,
    /// Far leg settlement date (forward leg)
    pub far_date: Date,
    /// Notional amount in base currency (exchanged on near, reversed on far)
    pub base_notional: Money,
    /// Domestic discount curve id (quote currency)
    pub domestic_discount_curve_id: CurveId,
    /// Foreign discount curve id (base currency)
    pub foreign_discount_curve_id: CurveId,
    /// Optional near leg FX rate (quote per base). If None, source from market.
    #[builder(optional)]
    pub near_rate: Option<f64>,
    /// Optional far leg FX rate (quote per base). If None, source from forwards.
    #[builder(optional)]
    pub far_rate: Option<f64>,
    /// Optional base currency calendar for spot/settlement adjustment metadata.
    #[builder(default)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_calendar_id: Option<String>,
    /// Optional quote currency calendar for spot/settlement adjustment metadata.
    #[builder(default)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quote_calendar_id: Option<String>,
    /// Attributes for tagging and selection
    pub attributes: Attributes,
}

impl FxSwap {
    /// Create a canonical example FX swap for testing and documentation.
    ///
    /// Returns a 6-month EUR/USD swap with realistic forward points.
    #[allow(clippy::expect_used)] // Example uses hardcoded valid values
    pub fn example() -> Self {
        Self::builder()
            .id(InstrumentId::new("FXSWAP-EURUSD-6M"))
            .base_currency(Currency::EUR)
            .quote_currency(Currency::USD)
            .near_date(
                Date::from_calendar_date(2024, time::Month::January, 5)
                    .expect("Valid example date"),
            )
            .far_date(
                Date::from_calendar_date(2024, time::Month::July, 5).expect("Valid example date"),
            )
            .base_notional(Money::new(1_000_000.0, Currency::EUR))
            .domestic_discount_curve_id(CurveId::new("USD-OIS"))
            .foreign_discount_curve_id(CurveId::new("EUR-OIS"))
            .near_rate_opt(Some(1.10))
            .far_rate_opt(Some(1.12))
            .attributes(Attributes::new())
            .build()
            .expect("Example FX swap construction should not fail")
    }

    /// Construct an FX swap from trade date and tenor using joint calendar spot roll.
    ///
    /// `spot_lag_days` defaults to 2 in most markets; supply calendar IDs to enforce
    /// base/quote business-day adjustment.
    #[allow(clippy::too_many_arguments)]
    pub fn from_trade_date(
        id: impl Into<InstrumentId>,
        base_currency: Currency,
        quote_currency: Currency,
        trade_date: Date,
        far_tenor_days: i64,
        base_notional: Money,
        domestic_discount_curve_id: impl Into<CurveId>,
        foreign_discount_curve_id: impl Into<CurveId>,
        base_calendar_id: Option<String>,
        quote_calendar_id: Option<String>,
        spot_lag_days: u32,
        bdc: finstack_core::dates::BusinessDayConvention,
    ) -> finstack_core::Result<Self> {
        use crate::instruments::common_impl::fx_dates::{adjust_joint_calendar, roll_spot_date};
        let near_date = roll_spot_date(
            trade_date,
            spot_lag_days,
            bdc,
            base_calendar_id.as_deref(),
            quote_calendar_id.as_deref(),
        )?;
        let far_unadjusted = near_date + time::Duration::days(far_tenor_days);
        let far_date = adjust_joint_calendar(
            far_unadjusted,
            bdc,
            base_calendar_id.as_deref(),
            quote_calendar_id.as_deref(),
        )?;

        Self::builder()
            .id(id.into())
            .base_currency(base_currency)
            .quote_currency(quote_currency)
            .near_date(near_date)
            .far_date(far_date)
            .base_notional(base_notional)
            .domestic_discount_curve_id(domestic_discount_curve_id.into())
            .foreign_discount_curve_id(foreign_discount_curve_id.into())
            .base_calendar_id_opt(base_calendar_id)
            .quote_calendar_id_opt(quote_calendar_id)
            .attributes(Attributes::new())
            .build()
    }

    /// Create a new FX swap using parameter structs
    pub fn new(
        id: InstrumentId,
        swap_params: &FxSwapParams,
        underlying_params: &FxUnderlyingParams,
    ) -> Self {
        Self {
            id,
            base_currency: underlying_params.base_currency,
            quote_currency: underlying_params.quote_currency,
            near_date: swap_params.near_date,
            far_date: swap_params.far_date,
            base_notional: swap_params.base_notional,
            domestic_discount_curve_id: underlying_params.domestic_discount_curve_id.to_owned(),
            foreign_discount_curve_id: underlying_params.foreign_discount_curve_id.to_owned(),
            near_rate: swap_params.near_rate,
            far_rate: swap_params.far_rate,
            base_calendar_id: None,
            quote_calendar_id: None,
            attributes: Attributes::new(),
        }
    }

    // Builder entrypoint is provided via derive
}

impl crate::instruments::common_impl::traits::Instrument for FxSwap {
    impl_instrument_base!(crate::pricer::InstrumentType::FxSwap);

    fn market_dependencies(
        &self,
    ) -> finstack_core::Result<crate::instruments::common_impl::dependencies::MarketDependencies>
    {
        let mut deps =
            crate::instruments::common_impl::dependencies::MarketDependencies::from_curve_dependencies(
                self,
            )?;
        deps.add_fx_pair(self.base_currency, self.quote_currency);
        Ok(deps)
    }

    fn value(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        use super::pricing_helper::FxSwapPricingContext;

        // Validate date ordering
        if self.near_date > self.far_date {
            return Err(finstack_core::Error::Validation(format!(
                "FxSwap near_date ({}) must be <= far_date ({})",
                self.near_date, self.far_date
            )));
        }

        // If fully settled (on or after far date), return zero.
        // Uses >= for consistency with FxForward (settled when as_of >= maturity_date).
        if as_of >= self.far_date {
            return Ok(finstack_core::money::Money::new(0.0, self.quote_currency));
        }

        // Currency safety check before expensive calculations
        if self.base_notional.currency() != self.base_currency {
            return Err(finstack_core::Error::CurrencyMismatch {
                expected: self.base_currency,
                actual: self.base_notional.currency(),
            });
        }

        // Build pricing context (handles rate validation and CIP forward calculation)
        let ctx = FxSwapPricingContext::build(self, curves, as_of)?;

        // Calculate total PV using the helper
        let total_pv = ctx.total_pv();
        Ok(finstack_core::money::Money::new(
            total_pv,
            self.quote_currency,
        ))
    }

    fn expiry(&self) -> Option<finstack_core::dates::Date> {
        Some(self.far_date)
    }

    fn effective_start_date(&self) -> Option<finstack_core::dates::Date> {
        Some(self.near_date)
    }
}

impl crate::instruments::common_impl::traits::CurveDependencies for FxSwap {
    fn curve_dependencies(
        &self,
    ) -> finstack_core::Result<crate::instruments::common_impl::traits::InstrumentCurves> {
        crate::instruments::common_impl::traits::InstrumentCurves::builder()
            .discount(self.domestic_discount_curve_id.clone())
            .discount(self.foreign_discount_curve_id.clone())
            .build()
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::instruments::common_impl::traits::{CurveDependencies, Instrument};
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::market_data::term_structures::DiscountCurve;
    use time::Month;

    fn date(year: i32, month: Month, day: u8) -> Date {
        Date::from_calendar_date(year, month, day).expect("valid test date")
    }

    fn base_market(as_of: Date) -> MarketContext {
        let usd_curve = DiscountCurve::builder("USD-OIS")
            .base_date(as_of)
            .knots(vec![(0.0, 1.0), (1.0, 0.95)])
            .build()
            .expect("should build");
        let eur_curve = DiscountCurve::builder("EUR-OIS")
            .base_date(as_of)
            .knots(vec![(0.0, 1.0), (1.0, 0.97)])
            .build()
            .expect("should build");

        MarketContext::new()
            .insert_discount(usd_curve)
            .insert_discount(eur_curve)
    }

    #[test]
    fn test_fx_swap_example_creation() {
        let swap = FxSwap::example();
        assert_eq!(swap.id.as_str(), "FXSWAP-EURUSD-6M");
        assert_eq!(swap.base_currency, Currency::EUR);
        assert_eq!(swap.quote_currency, Currency::USD);
    }

    #[test]
    fn test_fx_swap_curve_dependencies() {
        let swap = FxSwap::example();
        let deps = swap.curve_dependencies().expect("curve_dependencies");

        assert_eq!(deps.discount_curves.len(), 2);
        assert!(deps.discount_curves.iter().any(|c| c.as_str() == "USD-OIS"));
        assert!(deps.discount_curves.iter().any(|c| c.as_str() == "EUR-OIS"));
    }

    #[test]
    fn test_fx_swap_rejects_invalid_date_ordering() {
        let as_of = date(2024, Month::January, 3);
        let market = base_market(as_of);

        // Create swap with near_date > far_date (invalid)
        let swap = FxSwap::builder()
            .id(InstrumentId::new("INVALID-SWAP"))
            .base_currency(Currency::EUR)
            .quote_currency(Currency::USD)
            .near_date(date(2024, Month::July, 5)) // Far date is actually earlier
            .far_date(date(2024, Month::January, 5))
            .base_notional(Money::new(1_000_000.0, Currency::EUR))
            .domestic_discount_curve_id(CurveId::new("USD-OIS"))
            .foreign_discount_curve_id(CurveId::new("EUR-OIS"))
            .near_rate_opt(Some(1.10))
            .far_rate_opt(Some(1.12))
            .attributes(Attributes::new())
            .build()
            .expect("should build");

        let result = swap.value(&market, as_of);
        assert!(result.is_err(), "Should reject invalid date ordering");
        let err_msg = result.expect_err("expected an error").to_string();
        assert!(
            err_msg.contains("near_date") && err_msg.contains("far_date"),
            "Error should mention date ordering: {}",
            err_msg
        );
    }

    #[test]
    fn test_fx_swap_returns_zero_when_fully_settled() {
        let as_of = date(2024, Month::August, 1); // After far_date
        let market = base_market(as_of);
        let swap = FxSwap::example(); // far_date is 2024-07-05

        let pv = swap.value(&market, as_of).expect("should price");
        assert_eq!(pv.amount(), 0.0, "Fully settled swap should have zero PV");
    }

    #[test]
    fn test_fx_swap_near_leg_settled_far_leg_active() {
        // as_of is between near_date and far_date
        let as_of = date(2024, Month::March, 1);
        let market = base_market(as_of);
        let swap = FxSwap::example();
        // Example has near_date=2024-01-05, far_date=2024-07-05

        // Should only include far leg since near has settled
        let result = swap.value(&market, as_of);
        assert!(
            result.is_ok(),
            "Should price when near settled but far active: {:?}",
            result.as_ref().err()
        );
        let pv = result.expect("should price");
        // PV should not be zero (far leg has value)
        // The sign depends on rates; just verify it's non-zero
        assert!(
            pv.amount().abs() > 1e-6,
            "PV should be non-zero when far leg is active"
        );
    }
}
