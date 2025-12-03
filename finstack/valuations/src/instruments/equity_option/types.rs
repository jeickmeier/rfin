//! Equity option instrument definition and Black–Scholes helpers.

// pricing formulas are implemented in the pricing engine; keep this module free of direct math imports
use crate::instruments::common::parameters::underlying::EquityUnderlyingParams;
use crate::instruments::common::traits::Attributes;
use crate::instruments::PricingOverrides;
use crate::instruments::{ExerciseStyle, OptionType, SettlementType};
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
//
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};

use super::parameters::EquityOptionParams;

/// Equity option instrument
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct EquityOption {
    /// Unique instrument identifier
    pub id: InstrumentId,
    /// Underlying equity ticker symbol
    pub underlying_ticker: String,
    /// Strike price
    pub strike: Money,
    /// Option type (call or put)
    pub option_type: OptionType,
    /// Exercise style (European or American)
    pub exercise_style: ExerciseStyle,
    /// Option expiry date
    pub expiry: Date,
    /// Contract size (number of shares per contract)
    pub contract_size: f64,
    /// Day count convention
    pub day_count: finstack_core::dates::DayCount,
    /// Settlement type (physical or cash)
    pub settlement: SettlementType,
    /// Discount curve ID for present value calculations
    pub discount_curve_id: CurveId,
    /// Equity spot price identifier
    pub spot_id: String,
    /// Equity volatility surface ID
    pub vol_surface_id: CurveId,
    /// Optional dividend yield curve ID
    pub div_yield_id: Option<String>,
    /// Pricing overrides (manual price, yield, spread)
    pub pricing_overrides: PricingOverrides,
    /// Attributes for scenario selection and grouping
    pub attributes: Attributes,
}

// Implement HasDiscountCurve for GenericParallelDv01
impl crate::metrics::HasDiscountCurve for EquityOption {
    fn discount_curve_id(&self) -> &finstack_core::types::CurveId {
        &self.discount_curve_id
    }
}

// Implement CurveDependencies for DV01 calculator
impl crate::instruments::common::traits::CurveDependencies for EquityOption {
    fn curve_dependencies(&self) -> crate::instruments::common::traits::InstrumentCurves {
        crate::instruments::common::traits::InstrumentCurves::builder()
            .discount(self.discount_curve_id.clone())
            .build()
    }
}

impl EquityOption {
    /// Create a canonical example equity option for testing and documentation.
    ///
    /// Returns an at-the-money SPX call option with 6 months to expiry.
    pub fn example() -> Self {
        Self::european_call(
            "SPX-CALL-4500",
            "SPX",
            4500.0,
            Date::from_calendar_date(2024, time::Month::June, 21).expect("Valid example date"),
            Money::new(100_000.0, Currency::USD),
            100.0,
        )
    }

    /// Create a European call option with standard conventions.
    ///
    /// This convenience constructor eliminates the builder for the most common case.
    pub fn european_call(
        id: impl Into<String>,
        ticker: impl Into<String>,
        strike: f64,
        expiry: Date,
        notional: Money,
        contract_size: f64,
    ) -> Self {
        let underlying = EquityUnderlyingParams::new(ticker, "EQUITY-SPOT", Currency::USD)
            .with_dividend_yield("EQUITY-DIVYIELD")
            .with_contract_size(contract_size);

        // Build directly using derive-generated builder setters
        Self::builder()
            .id(InstrumentId::new(id.into()))
            .underlying_ticker(underlying.ticker)
            .strike(Money::new(strike, notional.currency()))
            .option_type(OptionType::Call)
            .exercise_style(ExerciseStyle::European)
            .expiry(expiry)
            .contract_size(underlying.contract_size)
            .day_count(finstack_core::dates::DayCount::Act365F)
            .settlement(SettlementType::Cash)
            .discount_curve_id(CurveId::new("USD-OIS"))
            .spot_id(underlying.spot_id)
            .vol_surface_id(CurveId::new("EQUITY-VOL"))
            .div_yield_id_opt(underlying.div_yield_id)
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("European call construction should not fail")
    }

    /// Create a European put option with standard conventions.
    pub fn european_put(
        id: impl Into<String>,
        ticker: impl Into<String>,
        strike: f64,
        expiry: Date,
        notional: Money,
        contract_size: f64,
    ) -> Self {
        let underlying = EquityUnderlyingParams::new(ticker, "EQUITY-SPOT", Currency::USD)
            .with_dividend_yield("EQUITY-DIVYIELD")
            .with_contract_size(contract_size);

        Self::builder()
            .id(InstrumentId::new(id.into()))
            .underlying_ticker(underlying.ticker)
            .strike(Money::new(strike, notional.currency()))
            .option_type(OptionType::Put)
            .exercise_style(ExerciseStyle::European)
            .expiry(expiry)
            .contract_size(underlying.contract_size)
            .day_count(finstack_core::dates::DayCount::Act365F)
            .settlement(SettlementType::Cash)
            .discount_curve_id(CurveId::new("USD-OIS"))
            .spot_id(underlying.spot_id)
            .vol_surface_id(CurveId::new("EQUITY-VOL"))
            .div_yield_id_opt(underlying.div_yield_id)
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("European put construction should not fail")
    }

    /// Create an American call option with standard conventions.
    pub fn american_call(
        id: impl Into<String>,
        ticker: impl Into<String>,
        strike: f64,
        expiry: Date,
        notional: Money,
        contract_size: f64,
    ) -> Self {
        let underlying = EquityUnderlyingParams::new(ticker, "EQUITY-SPOT", Currency::USD)
            .with_dividend_yield("EQUITY-DIVYIELD")
            .with_contract_size(contract_size);

        Self::builder()
            .id(InstrumentId::new(id.into()))
            .underlying_ticker(underlying.ticker)
            .strike(Money::new(strike, notional.currency()))
            .option_type(OptionType::Call)
            .exercise_style(ExerciseStyle::American)
            .expiry(expiry)
            .contract_size(underlying.contract_size)
            .day_count(finstack_core::dates::DayCount::Act365F)
            .settlement(SettlementType::Cash)
            .discount_curve_id(CurveId::new("USD-OIS"))
            .spot_id(underlying.spot_id)
            .vol_surface_id(CurveId::new("EQUITY-VOL"))
            .div_yield_id_opt(underlying.div_yield_id)
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("American call construction should not fail")
    }

    /// Create a new equity option using parameter structs
    pub fn new(
        id: impl Into<String>,
        option_params: &EquityOptionParams,
        underlying_params: &EquityUnderlyingParams,
        discount_curve_id: CurveId,
        vol_surface_id: CurveId,
    ) -> Self {
        Self {
            id: InstrumentId::new(id.into()),
            underlying_ticker: underlying_params.ticker.clone(),
            strike: option_params.strike,
            option_type: option_params.option_type,
            exercise_style: option_params.exercise_style,
            expiry: option_params.expiry,
            contract_size: option_params.contract_size,
            day_count: finstack_core::dates::DayCount::Act365F,
            settlement: option_params.settlement,
            discount_curve_id,
            spot_id: underlying_params.spot_id.to_owned(),
            vol_surface_id,
            div_yield_id: underlying_params.div_yield_id.to_owned(),
            pricing_overrides: PricingOverrides::default(),
            attributes: Attributes::new(),
        }
    }

    /// Calculate the net present value of this equity option
    pub fn npv(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        use crate::instruments::equity_option::pricer;
        pricer::npv(self, curves, as_of)
    }

    /// Calculate Greeks for this equity option
    pub fn greeks(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<crate::instruments::equity_option::pricer::EquityOptionGreeks> {
        use crate::instruments::equity_option::pricer;
        pricer::compute_greeks(self, curves, as_of)
    }

    /// Calculate delta of this equity option
    pub fn delta(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        let greeks = self.greeks(curves, as_of)?;
        Ok(greeks.delta)
    }

    /// Calculate gamma of this equity option
    pub fn gamma(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        let greeks = self.greeks(curves, as_of)?;
        Ok(greeks.gamma)
    }

    /// Calculate vega of this equity option
    pub fn vega(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        let greeks = self.greeks(curves, as_of)?;
        Ok(greeks.vega)
    }

    /// Calculate theta of this equity option
    pub fn theta(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        let greeks = self.greeks(curves, as_of)?;
        Ok(greeks.theta)
    }

    /// Calculate rho of this equity option
    pub fn rho(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        let greeks = self.greeks(curves, as_of)?;
        Ok(greeks.rho)
    }

    /// Calculate implied volatility of this equity option
    pub fn implied_vol(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
        market_price: f64,
    ) -> finstack_core::Result<f64> {
        let t = self.day_count.year_fraction(
            as_of,
            self.expiry,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        if t <= 0.0 {
            return Ok(0.0);
        }

        // Collect inputs except vol
        let (spot, r, q, _sigma, _t) = {
            use crate::instruments::equity_option::pricer;
            let (spot, r, q, sigma, t) = pricer::collect_inputs(self, curves, as_of)?;
            (spot, r, q, sigma, t)
        };

        if market_price <= 0.0 {
            return Ok(0.0);
        }

        // Solve for sigma using bracketed bisection
        let k = self.strike.amount();
        let price_at = |sigma: f64| -> f64 {
            if sigma <= 0.0 {
                return 0.0;
            }
            use crate::instruments::equity_option::pricer;
            pricer::price_bs_unit(spot, k, r, q, sigma, t, self.option_type) * self.contract_size
        };

        const MIN_VOL: f64 = 1e-6;
        const MAX_VOL_BRACKET: f64 = 10.0;
        const SOLVER_TOL: f64 = 1e-8;
        const SOLVER_MAX_ITER: usize = 100;

        let mut lo = MIN_VOL;
        let mut hi = 3.0;
        let tol = SOLVER_TOL;
        let max_iter = SOLVER_MAX_ITER;

        let mut f_lo = price_at(lo) - market_price;
        let mut f_hi = price_at(hi) - market_price;
        if f_lo * f_hi > 0.0 {
            let mut tries = 0;
            while f_lo * f_hi > 0.0 && hi < MAX_VOL_BRACKET && tries < 10 {
                hi *= 1.5;
                f_hi = price_at(hi) - market_price;
                tries += 1;
            }
            if f_lo * f_hi > 0.0 {
                return Ok(0.0);
            }
        }

        let mut mid = 0.5 * (lo + hi);
        for _ in 0..max_iter {
            mid = 0.5 * (lo + hi);
            let f_mid = price_at(mid) - market_price;
            if f_mid.abs() < tol || (hi - lo) < tol {
                return Ok(mid);
            }

            // Guarded Newton step using closed-form vega
            let vega_per_1pct = {
                let d1 = crate::instruments::common::models::d1(spot, k, r, mid, t, q);
                let exp_q_t = (-q * t).exp();
                let sqrt_t = t.sqrt();
                spot * exp_q_t * finstack_core::math::norm_pdf(d1) * sqrt_t / 100.0
            } * self.contract_size;
            let vega_abs = vega_per_1pct * 100.0;
            if vega_abs.abs() > 1e-12 {
                let newton = mid - f_mid / vega_abs;
                if newton.is_finite() && newton > lo && newton < hi {
                    mid = newton;
                    let f_new = price_at(mid) - market_price;
                    if f_lo * f_new <= 0.0 {
                        hi = mid;
                    } else {
                        lo = mid;
                        f_lo = f_new;
                    }
                    continue;
                }
            }

            if f_lo * f_mid <= 0.0 {
                hi = mid;
            } else {
                lo = mid;
                f_lo = f_mid;
            }
        }

        Ok(mid)
    }
}

impl crate::instruments::common::traits::Instrument for EquityOption {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::EquityOption
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
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::equity_option::pricer;
    use crate::instruments::{
        common::traits::Attributes, ExerciseStyle, OptionType, PricingOverrides, SettlementType,
    };
    use finstack_core::{
        currency::Currency,
        dates::{Date, DayCount},
        market_data::{
            context::MarketContext, scalars::MarketScalar, surfaces::vol_surface::VolSurface,
            term_structures::discount_curve::DiscountCurve,
        },
        money::Money,
        types::{CurveId, InstrumentId},
    };
    use time::Month;

    const DISC_ID: &str = "USD-OIS";
    const SPOT_ID: &str = "SPX-SPOT";
    const VOL_ID: &str = "SPX-VOL";
    const DIV_ID: &str = "SPX-DIV";

    fn date(year: i32, month: u8, day: u8) -> Date {
        Date::from_calendar_date(
            year,
            Month::try_from(month).expect("Valid month (1-12)"),
            day,
        )
        .expect("Valid test date")
    }

    fn build_discount_curve(as_of: Date, flat_rate: f64) -> DiscountCurve {
        let df_5y = (-flat_rate * 5.0).exp();
        DiscountCurve::builder(DISC_ID)
            .base_date(as_of)
            .knots([(0.0, 1.0), (5.0, df_5y)])
            .build()
            .expect("DiscountCurve builder should succeed with valid test data")
    }

    fn build_surface(base_vol: f64) -> VolSurface {
        let expiries = [0.25, 0.5, 1.0, 2.0];
        let strikes = [80.0, 90.0, 100.0, 110.0, 120.0];
        let row = [base_vol; 5];
        let mut builder = VolSurface::builder(VOL_ID)
            .expiries(&expiries)
            .strikes(&strikes);
        for _ in expiries {
            builder = builder.row(&row);
        }
        builder
            .build()
            .expect("VolSurface builder should succeed with valid test data")
    }

    fn build_market_context(
        as_of: Date,
        spot: f64,
        vol: f64,
        rate: f64,
        div_yield: f64,
    ) -> MarketContext {
        MarketContext::new()
            .insert_discount(build_discount_curve(as_of, rate))
            .insert_surface(build_surface(vol))
            .insert_price(SPOT_ID, MarketScalar::Unitless(spot))
            .insert_price(DIV_ID, MarketScalar::Unitless(div_yield))
    }

    fn base_option(expiry: Date) -> EquityOption {
        EquityOption::builder()
            .id(InstrumentId::new("EQ-OPT"))
            .underlying_ticker("SPX".to_string())
            .strike(Money::new(100.0, Currency::USD))
            .option_type(OptionType::Call)
            .exercise_style(ExerciseStyle::European)
            .expiry(expiry)
            .contract_size(100.0)
            .day_count(DayCount::Act365F)
            .settlement(SettlementType::Cash)
            .discount_curve_id(CurveId::new(DISC_ID))
            .spot_id(SPOT_ID.to_string())
            .vol_surface_id(CurveId::new(VOL_ID))
            .div_yield_id_opt(Some(DIV_ID.to_string()))
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("should succeed")
    }

    fn approx_eq(actual: f64, expected: f64, tol: f64) {
        let diff = (actual - expected).abs();
        assert!(
            diff <= tol,
            "expected {expected}, got {actual} (diff {diff} > {tol})"
        );
    }

    #[test]
    fn convenience_constructors_apply_standard_conventions() {
        let expiry = date(2025, 12, 31);
        let notional = Money::new(1_000_000.0, Currency::USD);
        let call = EquityOption::european_call("SPX-CALL", "SPX", 100.0, expiry, notional, 100.0);
        assert_eq!(call.exercise_style, ExerciseStyle::European);
        assert_eq!(call.option_type, OptionType::Call);
        assert_eq!(call.discount_curve_id, CurveId::new(DISC_ID));
        assert_eq!(call.spot_id, "EQUITY-SPOT");
        assert_eq!(call.vol_surface_id, CurveId::new("EQUITY-VOL"));

        let put = EquityOption::european_put("SPX-PUT", "SPX", 90.0, expiry, notional, 50.0);
        assert_eq!(put.option_type, OptionType::Put);
        assert_eq!(put.contract_size, 50.0);

        let american =
            EquityOption::american_call("SPX-AMER", "SPX", 105.0, expiry, notional, 75.0);
        assert_eq!(american.exercise_style, ExerciseStyle::American);
        assert_eq!(american.contract_size, 75.0);
    }

    #[test]
    fn npv_and_greeks_match_pricer_outputs() {
        let as_of = date(2025, 1, 3);
        let expiry = date(2025, 7, 3);
        let option = base_option(expiry);
        let curves = build_market_context(as_of, 105.0, 0.22, 0.03, 0.01);

        let price = option
            .npv(&curves, as_of)
            .expect("NPV calculation should succeed in test");
        let (spot, r, q, sigma, t) = pricer::collect_inputs(&option, &curves, as_of)
            .expect("Input collection should succeed in test");
        let expected_unit = pricer::price_bs_unit(
            spot,
            option.strike.amount(),
            r,
            q,
            sigma,
            t,
            option.option_type,
        );
        // Slightly wider tolerance due to MonotoneConvex interpolation (vs Linear)
        approx_eq(price.amount(), expected_unit * option.contract_size, 5e-3);

        let greeks = option
            .greeks(&curves, as_of)
            .expect("Greeks calculation should succeed in test");
        let expected = pricer::compute_greeks(&option, &curves, as_of)
            .expect("Greeks computation should succeed in test");
        approx_eq(greeks.delta, expected.delta, 1e-6);
        approx_eq(greeks.gamma, expected.gamma, 1e-10);
        approx_eq(greeks.vega, expected.vega, 1e-6);
        approx_eq(greeks.theta, expected.theta, 1e-8);
        approx_eq(greeks.rho, expected.rho, 1e-6);

        approx_eq(
            option
                .delta(&curves, as_of)
                .expect("Delta calculation should succeed in test"),
            greeks.delta,
            1e-12,
        );
        approx_eq(
            option
                .gamma(&curves, as_of)
                .expect("Gamma calculation should succeed in test"),
            greeks.gamma,
            1e-12,
        );
        approx_eq(
            option
                .vega(&curves, as_of)
                .expect("Vega calculation should succeed in test"),
            greeks.vega,
            1e-12,
        );
        approx_eq(
            option
                .theta(&curves, as_of)
                .expect("Theta calculation should succeed in test"),
            greeks.theta,
            1e-12,
        );
        approx_eq(
            option
                .rho(&curves, as_of)
                .expect("Rho calculation should succeed in test"),
            greeks.rho,
            1e-12,
        );
    }

    #[test]
    fn implied_volatility_recovers_surface_value_and_respects_override() {
        let as_of = date(2025, 1, 3);
        let expiry = date(2025, 7, 3);
        let option = base_option(expiry);
        let curves = build_market_context(as_of, 100.0, 0.30, 0.02, 0.01);

        let npv = option.npv(&curves, as_of).expect("should succeed");
        let implied = option
            .implied_vol(&curves, as_of, npv.amount())
            .expect("should succeed");
        approx_eq(implied, 0.30, 1e-5);

        let mut override_option = base_option(expiry);
        let overrides = PricingOverrides {
            implied_volatility: Some(0.45),
            ..Default::default()
        };
        override_option.pricing_overrides = overrides;
        let override_price = override_option.npv(&curves, as_of).expect("should succeed");
        let (spot, r, q, _, t) =
            pricer::collect_inputs(&override_option, &curves, as_of).expect("should succeed");
        let expected = pricer::price_bs_unit(
            spot,
            override_option.strike.amount(),
            r,
            q,
            0.45,
            t,
            override_option.option_type,
        ) * override_option.contract_size;
        // Slightly wider tolerance due to MonotoneConvex interpolation (vs Linear)
        approx_eq(override_price.amount(), expected, 5e-3);
    }

    #[test]
    fn expired_options_return_intrinsic_value_and_static_greeks() {
        let expiry = date(2025, 1, 3);
        let as_of = expiry;
        let mut option = base_option(expiry);
        option.contract_size = 50.0;
        let curves = build_market_context(as_of, 120.0, 0.25, 0.01, 0.0);

        let pv = option.npv(&curves, as_of).expect("should succeed");
        assert_eq!(pv.amount(), (120.0 - 100.0) * 50.0);

        let greeks = option.greeks(&curves, as_of).expect("should succeed");
        assert_eq!(greeks.delta, 50.0);
        assert_eq!(greeks.gamma, 0.0);
        assert_eq!(greeks.vega, 0.0);
        assert_eq!(greeks.theta, 0.0);
        assert_eq!(greeks.rho, 0.0);

        let implied = option
            .implied_vol(&curves, as_of, pv.amount())
            .expect("should succeed");
        assert_eq!(implied, 0.0);
    }

    /// Tests that separate day count handling works correctly when the discount curve
    /// uses ACT/360 (typical OIS convention) and volatility uses ACT/365F (equity standard).
    ///
    /// Market standard: Equity options use ACT/365F for vol time, but may discount on OIS
    /// curves with ACT/360. Mixing bases without proper separation rescales vols/rates
    /// and misstates greeks/theta.
    #[test]
    fn mixed_day_count_act365_vol_act360_discount() {
        let as_of = date(2025, 1, 3);
        let expiry = date(2026, 1, 3); // 1 year expiry

        // Create an ACT/360 discount curve (typical OIS convention)
        let flat_rate: f64 = 0.05;
        let df_5y: f64 = (-flat_rate * 5.0).exp();
        let act360_curve = DiscountCurve::builder(DISC_ID)
            .base_date(as_of)
            .day_count(DayCount::Act360) // OIS convention
            .knots([(0.0, 1.0), (5.0, df_5y)])
            .build()
            .expect("DiscountCurve with ACT/360 should build successfully");

        let curves = MarketContext::new()
            .insert_discount(act360_curve)
            .insert_surface(build_surface(0.20)) // 20% vol using ACT/365F time
            .insert_price(SPOT_ID, MarketScalar::Unitless(100.0))
            .insert_price(DIV_ID, MarketScalar::Unitless(0.02));

        let option = base_option(expiry);

        // Verify inputs are correctly separated
        let inputs = pricer::collect_inputs_extended(&option, &curves, as_of)
            .expect("collect_inputs_extended should succeed");

        // ACT/360: 365 days / 360 = 1.01389 years
        // ACT/365F: 365 days / 365 = 1.0 years
        let expected_t_rate = 365.0 / 360.0; // ACT/360 for rate
        let expected_t_vol = 365.0 / 365.0; // ACT/365F for vol

        approx_eq(inputs.t_rate, expected_t_rate, 1e-4);
        approx_eq(inputs.t_vol, expected_t_vol, 1e-4);

        // The difference between t_rate and t_vol should be non-trivial
        let time_diff = (inputs.t_rate - inputs.t_vol).abs();
        assert!(
            time_diff > 0.01,
            "t_rate and t_vol should differ significantly with ACT/360 vs ACT/365F: got {time_diff}"
        );

        // Verify pricing works and produces reasonable values
        let pv = option
            .npv(&curves, as_of)
            .expect("NPV should succeed with mixed day counts");
        assert!(pv.amount() > 0.0, "Call option should have positive value");

        // Verify greeks are computed correctly
        let greeks = option
            .greeks(&curves, as_of)
            .expect("Greeks should succeed with mixed day counts");
        assert!(greeks.delta > 0.0 && greeks.delta < option.contract_size);
        assert!(greeks.gamma > 0.0);
        assert!(greeks.vega > 0.0);

        // Verify price is within Black-Scholes tolerance
        // Using the inputs directly in the BS formula
        let bs_price = pricer::price_bs_unit(
            inputs.spot,
            option.strike.amount(),
            inputs.r,
            inputs.q,
            inputs.sigma,
            inputs.t_vol,
            option.option_type,
        ) * option.contract_size;

        // Slightly wider tolerance due to MonotoneConvex interpolation (vs Linear)
        // Same tolerance as other tests in this file
        approx_eq(pv.amount(), bs_price, 5e-3);
    }
}
