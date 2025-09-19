//! Equity option instrument implementation using Black-Scholes model.

use crate::instruments::models::{d1, d2};
use crate::instruments::traits::Attributes;
use crate::instruments::underlying::EquityUnderlyingParams;
use crate::instruments::PricingOverrides;
use crate::instruments::{ExerciseStyle, OptionType, SettlementType};
use finstack_core::dates::Date;
use finstack_core::math::{norm_cdf, norm_pdf};
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_core::F;

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
    pub contract_size: F,
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
        strike: F,
        expiry: Date,
        notional: Money,
        contract_size: F,
    ) -> Self {
        let underlying = EquityUnderlyingParams::new(ticker, "EQUITY-SPOT")
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
        strike: F,
        expiry: Date,
        notional: Money,
        contract_size: F,
    ) -> Self {
        let underlying = EquityUnderlyingParams::new(ticker, "EQUITY-SPOT")
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
        strike: F,
        expiry: Date,
        notional: Money,
        contract_size: F,
    ) -> Self {
        let underlying = EquityUnderlyingParams::new(ticker, "EQUITY-SPOT")
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

    pub fn black_scholes_price(
        &self,
        spot: F,
        r: F,
        sigma: F,
        t: F,
        q: F,
    ) -> finstack_core::Result<Money> {
        if t <= 0.0 {
            let intrinsic = match self.option_type {
                OptionType::Call => (spot - self.strike.amount()).max(0.0),
                OptionType::Put => (self.strike.amount() - spot).max(0.0),
            };
            return Ok(Money::new(
                intrinsic * self.contract_size,
                self.strike.currency(),
            ));
        }
        let k = self.strike.amount();
        let d1 = d1(spot, k, r, sigma, t, q);
        let d2 = d2(spot, k, r, sigma, t, q);
        let price = match self.option_type {
            OptionType::Call => {
                spot * (-q * t).exp() * norm_cdf(d1) - k * (-r * t).exp() * norm_cdf(d2)
            }
            OptionType::Put => {
                k * (-r * t).exp() * norm_cdf(-d2) - spot * (-q * t).exp() * norm_cdf(-d1)
            }
        };
        Ok(Money::new(
            price * self.contract_size,
            self.strike.currency(),
        ))
    }

    pub fn delta(&self, spot: F, r: F, sigma: F, t: F, q: F) -> F {
        if t <= 0.0 {
            return match self.option_type {
                OptionType::Call => {
                    if spot > self.strike.amount() {
                        1.0
                    } else {
                        0.0
                    }
                }
                OptionType::Put => {
                    if spot < self.strike.amount() {
                        -1.0
                    } else {
                        0.0
                    }
                }
            };
        }
        let d1 = d1(spot, self.strike.amount(), r, sigma, t, q);
        let exp_q_t = (-q * t).exp();
        match self.option_type {
            OptionType::Call => exp_q_t * norm_cdf(d1),
            OptionType::Put => -exp_q_t * norm_cdf(-d1),
        }
    }

    pub fn gamma(&self, spot: F, r: F, sigma: F, t: F, q: F) -> F {
        if t <= 0.0 || sigma <= 0.0 {
            return 0.0;
        }
        let d1 = d1(spot, self.strike.amount(), r, sigma, t, q);
        let exp_q_t = (-q * t).exp();
        exp_q_t * norm_pdf(d1) / (spot * sigma * t.sqrt())
    }

    pub fn vega(&self, spot: F, r: F, sigma: F, t: F, q: F) -> F {
        if t <= 0.0 {
            return 0.0;
        }
        let d1 = d1(spot, self.strike.amount(), r, sigma, t, q);
        let exp_q_t = (-q * t).exp();
        spot * exp_q_t * norm_pdf(d1) * t.sqrt() / 100.0
    }

    pub fn theta(&self, spot: F, r: F, sigma: F, t: F, q: F) -> F {
        if t <= 0.0 {
            return 0.0;
        }
        let k = self.strike.amount();
        let d1 = d1(spot, k, r, sigma, t, q);
        let d2 = d2(spot, k, r, sigma, t, q);
        let sqrt_t = t.sqrt();
        match self.option_type {
            OptionType::Call => {
                let term1 = -spot * norm_pdf(d1) * sigma * (-q * t).exp() / (2.0 * sqrt_t);
                let term2 = q * spot * norm_cdf(d1) * (-q * t).exp();
                let term3 = -r * k * (-r * t).exp() * norm_cdf(d2);
                (term1 + term2 + term3) / 365.0
            }
            OptionType::Put => {
                let term1 = -spot * norm_pdf(d1) * sigma * (-q * t).exp() / (2.0 * sqrt_t);
                let term2 = -q * spot * norm_cdf(-d1) * (-q * t).exp();
                let term3 = r * k * (-r * t).exp() * norm_cdf(-d2);
                (term1 + term2 + term3) / 365.0
            }
        }
    }

    pub fn rho(&self, spot: F, r: F, sigma: F, t: F, q: F) -> F {
        if t <= 0.0 {
            return 0.0;
        }
        let k = self.strike.amount();
        let d2 = d2(spot, k, r, sigma, t, q);
        match self.option_type {
            OptionType::Call => k * t * (-r * t).exp() * norm_cdf(d2) / 100.0,
            OptionType::Put => -k * t * (-r * t).exp() * norm_cdf(-d2) / 100.0,
        }
    }
}

impl_instrument!(
    EquityOption,
    "EquityOption",
    pv = |s, curves, as_of| {
        let time_to_expiry = s.day_count.year_fraction(
            as_of,
            s.expiry,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        if time_to_expiry <= 0.0 {
            let spot_scalar = curves.price(&s.spot_id)?;
            let spot = match spot_scalar {
                finstack_core::market_data::scalars::MarketScalar::Unitless(val) => *val,
                finstack_core::market_data::scalars::MarketScalar::Price(money) => money.amount(),
            };
            let intrinsic = match s.option_type {
                OptionType::Call => (spot - s.strike.amount()).max(0.0),
                OptionType::Put => (s.strike.amount() - spot).max(0.0),
            };
            return Ok(finstack_core::money::Money::new(
                intrinsic * s.contract_size,
                s.strike.currency(),
            ));
        }
        // Discounting trait not needed explicitly here
        let disc_curve = curves
            .get_ref::<finstack_core::market_data::term_structures::discount_curve::DiscountCurve>(
                s.disc_id.as_str(),
            )?;
        let r = disc_curve.zero(time_to_expiry);
        let spot_scalar = curves.price(&s.spot_id)?;
        let spot = match spot_scalar {
            finstack_core::market_data::scalars::MarketScalar::Unitless(val) => *val,
            finstack_core::market_data::scalars::MarketScalar::Price(money) => money.amount(),
        };
        let q = if let Some(div_id) = &s.div_yield_id {
            match curves.price(div_id.as_str()) {
                Ok(scalar) => match scalar {
                    finstack_core::market_data::scalars::MarketScalar::Unitless(val) => *val,
                    finstack_core::market_data::scalars::MarketScalar::Price(_) => 0.0,
                },
                Err(_) => 0.0,
            }
        } else {
            0.0
        };
        let sigma = if let Some(impl_vol) = s.pricing_overrides.implied_volatility {
            impl_vol
        } else {
            let vol_surface = curves.surface_ref(s.vol_id.as_str())?;
            vol_surface.value_clamped(time_to_expiry, s.strike.amount())
        };
        s.black_scholes_price(spot, r, sigma, time_to_expiry, q)
    }
);
