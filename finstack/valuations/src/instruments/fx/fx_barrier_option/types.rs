//! FX barrier option instrument definition.

use crate::instruments::barrier_option::types::BarrierType;
use crate::instruments::common::traits::Attributes;
use crate::instruments::OptionType;
use crate::instruments::PricingOverrides;
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};

/// FX barrier option instrument.
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct FxBarrierOption {
    /// Unique instrument identifier
    pub id: InstrumentId,
    /// Strike exchange rate
    pub strike: Money,
    /// Barrier level (exchange rate that triggers knock-in/out)
    pub barrier: Money,
    /// Optional rebate amount (paid at expiry if barrier condition met)
    pub rebate: Option<Money>,
    /// Option type (call or put on foreign currency)
    pub option_type: OptionType,
    /// Barrier type (up/down, in/out)
    pub barrier_type: BarrierType,
    /// Option expiry date
    pub expiry: Date,
    /// Notional amount in foreign currency
    pub notional: Money,
    /// Domestic currency (quote currency)
    pub domestic_currency: Currency,
    /// Foreign currency (base currency)
    pub foreign_currency: Currency,
    /// Day count convention
    pub day_count: finstack_core::dates::DayCount,
    /// Whether to use Gobet-Miri continuous barrier adjustment
    pub use_gobet_miri: bool,
    /// Domestic discount curve ID
    pub domestic_discount_curve_id: CurveId,
    /// Foreign discount curve ID
    pub foreign_discount_curve_id: CurveId,
    /// FX spot price identifier
    pub fx_spot_id: String,
    /// FX volatility surface ID
    pub fx_vol_id: CurveId,
    /// Pricing overrides (manual price, yield, spread)
    pub pricing_overrides: PricingOverrides,
    /// Attributes for scenario selection and grouping
    pub attributes: Attributes,
}

// Implement HasDiscountCurve for GenericParallelDv01
impl crate::instruments::common::pricing::HasDiscountCurve for FxBarrierOption {
    fn discount_curve_id(&self) -> &finstack_core::types::CurveId {
        &self.domestic_discount_curve_id
    }
}

// Implement CurveDependencies for DV01 calculator
// FxBarrierOption uses both domestic and foreign curves for FX carry calculation
impl crate::instruments::common::traits::CurveDependencies for FxBarrierOption {
    fn curve_dependencies(&self) -> crate::instruments::common::traits::InstrumentCurves {
        crate::instruments::common::traits::InstrumentCurves::builder()
            .discount(self.domestic_discount_curve_id.clone())
            .discount(self.foreign_discount_curve_id.clone())
            .build()
    }
}

impl FxBarrierOption {
    /// Create a canonical example FX barrier option (EURUSD up-and-out call).
    ///
    /// # Currency Conventions
    ///
    /// For EUR/USD (foreign=EUR, domestic=USD):
    /// - Strike and barrier are in USD (the domestic/quote currency)
    /// - Notional is in EUR (the foreign/base currency being bought)
    #[allow(clippy::expect_used)] // Example uses hardcoded valid values
    pub fn example() -> Self {
        use finstack_core::dates::DayCount;
        use time::Month;
        FxBarrierOptionBuilder::new()
            .id(InstrumentId::new("FXBAR-EURUSD-UO-CALL"))
            .strike(Money::new(1.10, Currency::USD)) // Strike rate in USD
            .barrier(Money::new(1.20, Currency::USD)) // Barrier rate in USD
            .option_type(crate::instruments::OptionType::Call)
            .barrier_type(BarrierType::UpAndOut)
            .expiry(
                Date::from_calendar_date(2024, Month::December, 20).expect("Valid example date"),
            )
            .notional(Money::new(1_000_000.0, Currency::EUR)) // Notional in foreign currency (EUR)
            .domestic_currency(Currency::USD)
            .foreign_currency(Currency::EUR)
            .day_count(DayCount::Act365F)
            .use_gobet_miri(false)
            .domestic_discount_curve_id(CurveId::new("USD-OIS"))
            .foreign_discount_curve_id(CurveId::new("EUR-OIS"))
            .fx_spot_id("EURUSD-SPOT".to_string())
            .fx_vol_id(CurveId::new("EURUSD-VOL"))
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("Example FxBarrierOption construction should not fail")
    }
    /// Calculate the net present value using Monte Carlo.
    #[cfg(feature = "mc")]
    pub fn npv_mc(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        use crate::instruments::fx_barrier_option::pricer;
        pricer::compute_pv(self, curves, as_of)
    }
}

// ================================================================================================
// Option risk metric providers (metrics adapters)
// ================================================================================================

impl crate::instruments::common::traits::OptionDeltaProvider for FxBarrierOption {
    fn option_delta(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        use crate::instruments::common::traits::Instrument;

        let t = self.day_count.year_fraction(
            as_of,
            self.expiry,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        if t <= 0.0 {
            return Ok(0.0);
        }

        let spot_scalar = market.price(&self.fx_spot_id)?;
        let current_spot = match spot_scalar {
            finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
            finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
        };
        let bump_size = current_spot * crate::metrics::bump_sizes::SPOT;
        if bump_size <= 0.0 {
            return Ok(0.0);
        }

        let up = crate::metrics::bump_scalar_price(
            market,
            &self.fx_spot_id,
            crate::metrics::bump_sizes::SPOT,
        )?;
        let pv_up = self.value(&up, as_of)?.amount();
        let down = crate::metrics::bump_scalar_price(
            market,
            &self.fx_spot_id,
            -crate::metrics::bump_sizes::SPOT,
        )?;
        let pv_down = self.value(&down, as_of)?.amount();

        Ok((pv_up - pv_down) / (2.0 * bump_size))
    }
}

impl crate::instruments::common::traits::OptionGammaProvider for FxBarrierOption {
    fn option_gamma(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        use crate::instruments::common::traits::Instrument;

        let t = self.day_count.year_fraction(
            as_of,
            self.expiry,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        if t <= 0.0 {
            return Ok(0.0);
        }

        let base_pv = self.value(market, as_of)?.amount();

        let spot_scalar = market.price(&self.fx_spot_id)?;
        let current_spot = match spot_scalar {
            finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
            finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
        };
        let bump_size = current_spot * crate::metrics::bump_sizes::SPOT;
        if bump_size <= 0.0 {
            return Ok(0.0);
        }

        let up = crate::metrics::bump_scalar_price(
            market,
            &self.fx_spot_id,
            crate::metrics::bump_sizes::SPOT,
        )?;
        let pv_up = self.value(&up, as_of)?.amount();
        let down = crate::metrics::bump_scalar_price(
            market,
            &self.fx_spot_id,
            -crate::metrics::bump_sizes::SPOT,
        )?;
        let pv_down = self.value(&down, as_of)?.amount();

        Ok((pv_up - 2.0 * base_pv + pv_down) / (bump_size * bump_size))
    }
}

impl crate::instruments::common::traits::OptionVegaProvider for FxBarrierOption {
    fn option_vega(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        use crate::instruments::common::traits::Instrument;

        let t = self.day_count.year_fraction(
            as_of,
            self.expiry,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        if t <= 0.0 {
            return Ok(0.0);
        }

        let base_pv = self.value(market, as_of)?.amount();
        let bumped = crate::metrics::bump_surface_vol_absolute(
            market,
            self.fx_vol_id.as_str(),
            crate::metrics::bump_sizes::VOLATILITY,
        )?;
        let pv_bumped = self.value(&bumped, as_of)?.amount();
        Ok((pv_bumped - base_pv) / crate::metrics::bump_sizes::VOLATILITY)
    }
}

impl crate::instruments::common::traits::OptionRhoProvider for FxBarrierOption {
    fn option_rho_bp(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        use crate::instruments::common::traits::Instrument;

        let t = self.day_count.year_fraction(
            as_of,
            self.expiry,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        if t <= 0.0 {
            return Ok(0.0);
        }

        let base_pv = self.value(market, as_of)?.amount();
        let bump_bp = self.pricing_overrides.rho_bump_bp();
        let bumped = crate::metrics::bump_discount_curve_parallel(
            market,
            &self.domestic_discount_curve_id,
            bump_bp,
        )?;
        let pv_bumped = self.value(&bumped, as_of)?.amount();
        Ok(pv_bumped - base_pv)
    }
}

impl crate::instruments::common::traits::OptionVannaProvider for FxBarrierOption {
    fn option_vanna(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        use crate::instruments::common::traits::Instrument;

        let t = self.day_count.year_fraction(
            as_of,
            self.expiry,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        if t <= 0.0 {
            return Ok(0.0);
        }

        let spot_scalar = market.price(&self.fx_spot_id)?;
        let current_spot = match spot_scalar {
            finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
            finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
        };

        let spot_bump = current_spot * crate::metrics::bump_sizes::SPOT;
        if spot_bump <= 0.0 {
            return Ok(0.0);
        }
        let vol_bump = crate::metrics::bump_sizes::VOLATILITY;

        // Delta at vol_up (central diff in spot)
        let curves_vol_up =
            crate::metrics::bump_surface_vol_absolute(market, self.fx_vol_id.as_str(), vol_bump)?;
        let curves_up = crate::metrics::bump_scalar_price(
            &curves_vol_up,
            &self.fx_spot_id,
            crate::metrics::bump_sizes::SPOT,
        )?;
        let curves_dn = crate::metrics::bump_scalar_price(
            &curves_vol_up,
            &self.fx_spot_id,
            -crate::metrics::bump_sizes::SPOT,
        )?;
        let pv_up = self.value(&curves_up, as_of)?.amount();
        let pv_dn = self.value(&curves_dn, as_of)?.amount();
        let delta_vol_up = (pv_up - pv_dn) / (2.0 * spot_bump);

        // Delta at vol_down
        let curves_vol_dn =
            crate::metrics::bump_surface_vol_absolute(market, self.fx_vol_id.as_str(), -vol_bump)?;
        let curves_up = crate::metrics::bump_scalar_price(
            &curves_vol_dn,
            &self.fx_spot_id,
            crate::metrics::bump_sizes::SPOT,
        )?;
        let curves_dn = crate::metrics::bump_scalar_price(
            &curves_vol_dn,
            &self.fx_spot_id,
            -crate::metrics::bump_sizes::SPOT,
        )?;
        let pv_up = self.value(&curves_up, as_of)?.amount();
        let pv_dn = self.value(&curves_dn, as_of)?.amount();
        let delta_vol_dn = (pv_up - pv_dn) / (2.0 * spot_bump);

        Ok((delta_vol_up - delta_vol_dn) / (2.0 * vol_bump))
    }
}

impl crate::instruments::common::traits::OptionVolgaProvider for FxBarrierOption {
    fn option_volga(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
        base_pv: f64,
    ) -> finstack_core::Result<f64> {
        use crate::instruments::common::traits::Instrument;

        let t = self.day_count.year_fraction(
            as_of,
            self.expiry,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        if t <= 0.0 {
            return Ok(0.0);
        }

        let vol_bump = crate::metrics::bump_sizes::VOLATILITY;
        let up =
            crate::metrics::bump_surface_vol_absolute(market, self.fx_vol_id.as_str(), vol_bump)?;
        let dn =
            crate::metrics::bump_surface_vol_absolute(market, self.fx_vol_id.as_str(), -vol_bump)?;
        let pv_up = self.value(&up, as_of)?.amount();
        let pv_dn = self.value(&dn, as_of)?.amount();
        Ok((pv_up - 2.0 * base_pv + pv_dn) / (vol_bump * vol_bump))
    }
}

impl crate::instruments::common::traits::Instrument for FxBarrierOption {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::FxBarrierOption
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
        use crate::instruments::fx_barrier_option::pricer::FxBarrierOptionAnalyticalPricer;
        use crate::pricer::Pricer;

        let pricer = FxBarrierOptionAnalyticalPricer::new();
        let result = pricer
            .price_dyn(self, market, as_of)
            .map_err(|e| finstack_core::Error::Validation(e.to_string()))?;
        Ok(result.value)
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
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::instruments::common::traits::CurveDependencies;

    #[test]
    fn test_fx_barrier_option_curve_dependencies_includes_both_curves() {
        let option = FxBarrierOption::example();
        let deps = option.curve_dependencies();

        // Should include both domestic and foreign discount curves
        assert_eq!(
            deps.discount_curves.len(),
            2,
            "FxBarrierOption should depend on both domestic and foreign curves"
        );
        assert!(
            deps.discount_curves.iter().any(|c| c.as_str() == "USD-OIS"),
            "Should include domestic curve"
        );
        assert!(
            deps.discount_curves.iter().any(|c| c.as_str() == "EUR-OIS"),
            "Should include foreign curve"
        );
    }

    #[test]
    fn test_fx_barrier_option_example_has_correct_currency_semantics() {
        let option = FxBarrierOption::example();

        // Strike and barrier should be in domestic currency (USD)
        assert_eq!(
            option.strike.currency(),
            option.domestic_currency,
            "Strike should be in domestic currency"
        );
        assert_eq!(
            option.barrier.currency(),
            option.domestic_currency,
            "Barrier should be in domestic currency"
        );

        // Notional should be in foreign currency (EUR)
        assert_eq!(
            option.notional.currency(),
            option.foreign_currency,
            "Notional should be in foreign currency"
        );
    }

    #[test]
    fn test_fx_barrier_option_creation_with_correct_currencies() {
        use finstack_core::dates::DayCount;
        use time::Month;

        // Valid: strike/barrier in USD, notional in EUR
        let option = FxBarrierOptionBuilder::new()
            .id(InstrumentId::new("TEST-FXBAR"))
            .strike(Money::new(1.10, Currency::USD))
            .barrier(Money::new(1.20, Currency::USD))
            .option_type(OptionType::Call)
            .barrier_type(BarrierType::UpAndOut)
            .expiry(Date::from_calendar_date(2025, Month::June, 15).expect("valid date"))
            .notional(Money::new(1_000_000.0, Currency::EUR))
            .domestic_currency(Currency::USD)
            .foreign_currency(Currency::EUR)
            .day_count(DayCount::Act365F)
            .use_gobet_miri(false)
            .domestic_discount_curve_id(CurveId::new("USD-OIS"))
            .foreign_discount_curve_id(CurveId::new("EUR-OIS"))
            .fx_spot_id("EURUSD-SPOT".to_string())
            .fx_vol_id(CurveId::new("EURUSD-VOL"))
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("should build");

        assert_eq!(option.strike.currency(), Currency::USD);
        assert_eq!(option.barrier.currency(), Currency::USD);
        assert_eq!(option.notional.currency(), Currency::EUR);
    }
}
