//! FX option instrument implementation using Garman-Kohlhagen model.

use crate::instruments::PricingOverrides;
use crate::instruments::fx::FxUnderlyingParams;
use crate::instruments::options::models::{d1, d2};
use crate::instruments::options::{ExerciseStyle, OptionType, SettlementType};
use crate::instruments::traits::Attributes;
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::math::{norm_cdf, norm_pdf};
use finstack_core::money::Money;
use finstack_core::F;
use num_traits::ToPrimitive;

use super::parameters::FxOptionParams;

/// FX option instrument (Garman-Kohlhagen model)
#[derive(Clone, Debug, finstack_macros::FinancialBuilder)]
pub struct FxOption {
    pub id: String,
    pub base_currency: Currency,
    pub quote_currency: Currency,
    pub strike: F,
    pub option_type: OptionType,
    pub exercise_style: ExerciseStyle,
    pub expiry: Date,
    pub day_count: finstack_core::dates::DayCount,
    pub notional: Money,
    pub settlement: SettlementType,
    pub domestic_disc_id: &'static str,
    pub foreign_disc_id: &'static str,
    pub vol_id: &'static str,
    pub pricing_overrides: PricingOverrides,
    pub attributes: Attributes,
}

impl FxOption {
    /// Create a European call option on an FX pair with standard conventions.
    pub fn european_call(
        id: impl Into<String>,
        base_currency: Currency,
        quote_currency: Currency,
        strike: F,
        expiry: Date,
        notional: Money,
    ) -> Self {

        let fx_underlying = if quote_currency == Currency::USD && base_currency == Currency::EUR {
            FxUnderlyingParams::usd_eur()
        } else if quote_currency == Currency::USD && base_currency == Currency::GBP {
            FxUnderlyingParams::gbp_usd()
        } else {
            // Fallback for other pairs - use USD for both curves
            FxUnderlyingParams::new(base_currency, quote_currency, "USD-OIS", "USD-OIS")
        };
        Self::builder()
            .id(id.into())
            .base_currency(fx_underlying.base_currency)
            .quote_currency(fx_underlying.quote_currency)
            .strike(strike)
            .option_type(OptionType::Call)
            .exercise_style(ExerciseStyle::European)
            .expiry(expiry)
            .day_count(finstack_core::dates::DayCount::Act365F)
            .notional(notional)
            .settlement(SettlementType::Cash)
            .domestic_disc_id(fx_underlying.domestic_disc_id)
            .foreign_disc_id(fx_underlying.foreign_disc_id)
            .vol_id("FX-VOL")
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("FX European call construction should not fail")
    }

    /// Create a European put option on an FX pair with standard conventions.
    pub fn european_put(
        id: impl Into<String>,
        base_currency: Currency,
        quote_currency: Currency,
        strike: F,
        expiry: Date,
        notional: Money,
    ) -> Self {

        let fx_underlying = if quote_currency == Currency::USD && base_currency == Currency::EUR {
            FxUnderlyingParams::usd_eur()
        } else if quote_currency == Currency::USD && base_currency == Currency::GBP {
            FxUnderlyingParams::gbp_usd()
        } else {
            // Fallback for other pairs - use USD for both curves
            FxUnderlyingParams::new(base_currency, quote_currency, "USD-OIS", "USD-OIS")
        };
        Self::builder()
            .id(id.into())
            .base_currency(fx_underlying.base_currency)
            .quote_currency(fx_underlying.quote_currency)
            .strike(strike)
            .option_type(OptionType::Put)
            .exercise_style(ExerciseStyle::European)
            .expiry(expiry)
            .day_count(finstack_core::dates::DayCount::Act365F)
            .notional(notional)
            .settlement(SettlementType::Cash)
            .domestic_disc_id(fx_underlying.domestic_disc_id)
            .foreign_disc_id(fx_underlying.foreign_disc_id)
            .vol_id("FX-VOL")
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("FX European put construction should not fail")
    }

    /// Create a new FX option using parameter structs
    pub fn new(
        id: impl Into<String>,
        option_params: &FxOptionParams,
        underlying_params: &FxUnderlyingParams,
        vol_id: &'static str,
    ) -> Self {
        Self {
            id: id.into(),
            base_currency: underlying_params.base_currency,
            quote_currency: underlying_params.quote_currency,
            strike: option_params.strike,
            option_type: option_params.option_type,
            exercise_style: option_params.exercise_style,
            expiry: option_params.expiry,
            day_count: finstack_core::dates::DayCount::Act365F,
            notional: option_params.notional,
            settlement: option_params.settlement,
            domestic_disc_id: underlying_params.domestic_disc_id,
            foreign_disc_id: underlying_params.foreign_disc_id,
            vol_id,
            pricing_overrides: PricingOverrides::default(),
            attributes: Attributes::new(),
        }
    }

    pub fn garman_kohlhagen_price(
        &self,
        spot: F,
        r_d: F,
        r_f: F,
        sigma: F,
        t: F,
    ) -> finstack_core::Result<Money> {
        if t <= 0.0 {
            let intrinsic = match self.option_type {
                OptionType::Call => (spot - self.strike).max(0.0),
                OptionType::Put => (self.strike - spot).max(0.0),
            };
            return Ok(Money::new(
                intrinsic * self.notional.amount(),
                self.quote_currency,
            ));
        }
        let d1 = d1(spot, self.strike, r_d, sigma, t, r_f);
        let d2 = d2(spot, self.strike, r_d, sigma, t, r_f);
        let price = match self.option_type {
            OptionType::Call => {
                spot * (-r_f * t).exp() * norm_cdf(d1)
                    - self.strike * (-r_d * t).exp() * norm_cdf(d2)
            }
            OptionType::Put => {
                self.strike * (-r_d * t).exp() * norm_cdf(-d2)
                    - spot * (-r_f * t).exp() * norm_cdf(-d1)
            }
        };
        Ok(Money::new(
            price * self.notional.amount(),
            self.quote_currency,
        ))
    }

    pub fn delta(&self, spot: F, r_d: F, r_f: F, sigma: F, t: F) -> F {
        if t <= 0.0 {
            return match self.option_type {
                OptionType::Call => {
                    if spot > self.strike {
                        1.0
                    } else {
                        0.0
                    }
                }
                OptionType::Put => {
                    if spot < self.strike {
                        -1.0
                    } else {
                        0.0
                    }
                }
            };
        }
        let d1 = d1(spot, self.strike, r_d, sigma, t, r_f);
        let exp_rf_t = (-r_f * t).exp();
        match self.option_type {
            OptionType::Call => exp_rf_t * norm_cdf(d1),
            OptionType::Put => -exp_rf_t * norm_cdf(-d1),
        }
    }

    pub fn gamma(&self, spot: F, r_d: F, r_f: F, sigma: F, t: F) -> F {
        if t <= 0.0 || sigma <= 0.0 {
            return 0.0;
        }
        let d1 = d1(spot, self.strike, r_d, sigma, t, r_f);
        let exp_rf_t = (-r_f * t).exp();
        exp_rf_t * norm_pdf(d1) / (spot * sigma * t.sqrt())
    }

    pub fn vega(&self, spot: F, r_d: F, r_f: F, sigma: F, t: F) -> F {
        if t <= 0.0 {
            return 0.0;
        }
        let d1 = d1(spot, self.strike, r_d, sigma, t, r_f);
        let exp_rf_t = (-r_f * t).exp();
        spot * exp_rf_t * norm_pdf(d1) * t.sqrt() / 100.0
    }

    pub fn theta(&self, spot: F, r_d: F, r_f: F, sigma: F, t: F) -> F {
        if t <= 0.0 {
            return 0.0;
        }
        let d1 = d1(spot, self.strike, r_d, sigma, t, r_f);
        let d2 = d2(spot, self.strike, r_d, sigma, t, r_f);
        let sqrt_t = t.sqrt();
        match self.option_type {
            OptionType::Call => {
                let term1 = -spot * norm_pdf(d1) * sigma * (-r_f * t).exp() / (2.0 * sqrt_t);
                let term2 = r_f * spot * norm_cdf(d1) * (-r_f * t).exp();
                let term3 = -r_d * self.strike * (-r_d * t).exp() * norm_cdf(d2);
                (term1 + term2 + term3) / 365.0
            }
            OptionType::Put => {
                let term1 = -spot * norm_pdf(d1) * sigma * (-r_f * t).exp() / (2.0 * sqrt_t);
                let term2 = -r_f * spot * norm_cdf(-d1) * (-r_f * t).exp();
                let term3 = r_d * self.strike * (-r_d * t).exp() * norm_cdf(-d2);
                (term1 + term2 + term3) / 365.0
            }
        }
    }

    pub fn rho_domestic(&self, spot: F, r_d: F, r_f: F, sigma: F, t: F) -> F {
        if t <= 0.0 {
            return 0.0;
        }
        let d2 = d2(spot, self.strike, r_d, sigma, t, r_f);
        match self.option_type {
            OptionType::Call => self.strike * t * (-r_d * t).exp() * norm_cdf(d2) / 100.0,
            OptionType::Put => -self.strike * t * (-r_d * t).exp() * norm_cdf(-d2) / 100.0,
        }
    }

    pub fn rho_foreign(&self, spot: F, r_d: F, r_f: F, sigma: F, t: F) -> F {
        if t <= 0.0 {
            return 0.0;
        }
        let d1 = d1(spot, self.strike, r_d, sigma, t, r_f);
        match self.option_type {
            OptionType::Call => -spot * t * (-r_f * t).exp() * norm_cdf(d1) / 100.0,
            OptionType::Put => spot * t * (-r_f * t).exp() * norm_cdf(-d1) / 100.0,
        }
    }
}

impl_instrument!(
    FxOption,
    "FxOption",
    pv = |s, curves, as_of| {
        let time_to_expiry = s.day_count.year_fraction(
            as_of,
            s.expiry,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        if time_to_expiry <= 0.0 {
            let fx_matrix =
                curves
                    .fx
                    .as_ref()
                    .ok_or(finstack_core::error::InputError::NotFound {
                        id: "fx_matrix".to_string(),
                    })?;
            let spot = fx_matrix.rate(finstack_core::money::fx::FxQuery {
                from: s.base_currency,
                to: s.quote_currency,
                on: as_of,
                policy: finstack_core::money::fx::FxConversionPolicy::CashflowDate,
                closure_check: None,
                want_meta: false,
            })?;
            let intrinsic = match s.option_type {
                OptionType::Call => (spot.rate.to_f64().unwrap_or(0.0) - s.strike).max(0.0),
                OptionType::Put => (s.strike - spot.rate.to_f64().unwrap_or(0.0)).max(0.0),
            };
            return Ok(finstack_core::money::Money::new(
                intrinsic * s.notional.amount(),
                s.quote_currency,
            ));
        }
        // Discounting trait not needed explicitly here
        let domestic_disc = curves
            .get_ref::<finstack_core::market_data::term_structures::discount_curve::DiscountCurve>(
                s.domestic_disc_id,
            )?;
        let foreign_disc = curves
            .get_ref::<finstack_core::market_data::term_structures::discount_curve::DiscountCurve>(
                s.foreign_disc_id,
            )?;
        let r_d = domestic_disc.zero(time_to_expiry);
        let r_f = foreign_disc.zero(time_to_expiry);
        let fx_matrix = curves
            .fx
            .as_ref()
            .ok_or(finstack_core::error::InputError::NotFound {
                id: "fx_matrix".to_string(),
            })?;
        let spot = fx_matrix.rate(finstack_core::money::fx::FxQuery {
            from: s.base_currency,
            to: s.quote_currency,
            on: as_of,
            policy: finstack_core::money::fx::FxConversionPolicy::CashflowDate,
            closure_check: None,
            want_meta: false,
        })?;
        let sigma = if let Some(impl_vol) = s.pricing_overrides.implied_volatility {
            impl_vol
        } else {
            let vol_surface = curves.surface_ref(s.vol_id)?;
            vol_surface.value_clamped(time_to_expiry, s.strike)
        };
        s.garman_kohlhagen_price(
            spot.rate.to_f64().unwrap_or(0.0),
            r_d,
            r_f,
            sigma,
            time_to_expiry,
        )
    }
);
