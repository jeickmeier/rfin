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
#[derive(Clone, Debug, finstack_macros::FinancialBuilder)]
pub struct EquityOption {
    pub id: InstrumentId,
    pub underlying_ticker: String,
    pub strike: Money,
    pub option_type: OptionType,
    pub exercise_style: ExerciseStyle,
    pub expiry: Date,
    pub contract_size: f64,
    pub day_count: finstack_core::dates::DayCount,
    pub settlement: SettlementType,
    pub disc_id: CurveId,
    pub spot_id: String,
    pub vol_id: CurveId,
    pub div_yield_id: Option<String>,
    pub pricing_overrides: PricingOverrides,
    pub attributes: Attributes,
}

impl EquityOption {
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
            .disc_id(CurveId::new("USD-OIS"))
            .spot_id(underlying.spot_id)
            .vol_id(CurveId::new("EQUITY-VOL"))
            .div_yield_id_opt(underlying.dividend_yield_id)
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
            .disc_id(CurveId::new("USD-OIS"))
            .spot_id(underlying.spot_id)
            .vol_id(CurveId::new("EQUITY-VOL"))
            .div_yield_id_opt(underlying.dividend_yield_id)
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
            .disc_id(CurveId::new("USD-OIS"))
            .spot_id(underlying.spot_id)
            .vol_id(CurveId::new("EQUITY-VOL"))
            .div_yield_id_opt(underlying.dividend_yield_id)
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
        disc_id: CurveId,
        vol_id: CurveId,
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
            disc_id,
            spot_id: underlying_params.spot_id.clone(),
            vol_id,
            div_yield_id: underlying_params.dividend_yield_id.clone(),
            pricing_overrides: PricingOverrides::default(),
            attributes: Attributes::new(),
        }
    }

    /// Calculate the net present value of this equity option
    pub fn npv(
        &self,
        curves: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        use crate::instruments::equity_option::pricer;
        pricer::npv(self, curves, as_of)
    }

    /// Calculate Greeks for this equity option
    pub fn greeks(
        &self,
        curves: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<crate::instruments::equity_option::pricer::EquityOptionGreeks> {
        use crate::instruments::equity_option::pricer;
        pricer::compute_greeks(self, curves, as_of)
    }

    /// Calculate delta of this equity option
    pub fn delta(
        &self,
        curves: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        let greeks = self.greeks(curves, as_of)?;
        Ok(greeks.delta)
    }

    /// Calculate gamma of this equity option
    pub fn gamma(
        &self,
        curves: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        let greeks = self.greeks(curves, as_of)?;
        Ok(greeks.gamma)
    }

    /// Calculate vega of this equity option
    pub fn vega(
        &self,
        curves: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        let greeks = self.greeks(curves, as_of)?;
        Ok(greeks.vega)
    }

    /// Calculate theta of this equity option
    pub fn theta(
        &self,
        curves: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        let greeks = self.greeks(curves, as_of)?;
        Ok(greeks.theta)
    }

    /// Calculate rho of this equity option
    pub fn rho(
        &self,
        curves: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        let greeks = self.greeks(curves, as_of)?;
        Ok(greeks.rho)
    }

    /// Calculate implied volatility of this equity option
    pub fn implied_vol(
        &self,
        curves: &finstack_core::market_data::MarketContext,
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

impl_instrument!(
    EquityOption,
    crate::pricer::InstrumentType::EquityOption,
    "EquityOption",
    pv = |s, curves, as_of| {
        // Call the instrument's own NPV method
        s.npv(curves, as_of)
    }
);
