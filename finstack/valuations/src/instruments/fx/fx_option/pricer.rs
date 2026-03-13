//! FX option pricer implementation using the Garman-Kohlhagen model.

use crate::instruments::common_impl::helpers::year_fraction;
use crate::instruments::common_impl::models::{bs_greeks, bs_price};
use crate::instruments::common_impl::parameters::OptionType;
use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::fx::fx_option::FxOption;
use crate::pricer::{
    InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingErrorContext,
};
use crate::results::ValuationResult;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::fx::FxQuery;
use finstack_core::money::Money;
use finstack_core::Result;

const STRIKE_ZERO_TOL: f64 = 1e-12;

/// Configuration for the FX option pricing engine.
#[derive(Debug, Clone)]
pub struct FxOptionPricerConfig {
    pub theta_days_per_year: f64,
    pub iv_initial_guess: f64,
}

impl Default for FxOptionPricerConfig {
    fn default() -> Self {
        Self {
            theta_days_per_year: 365.0,
            iv_initial_guess: 0.20,
        }
    }
}

/// Shared Garman-Kohlhagen calculator used by the FX option pricer entrypoints.
#[derive(Debug, Clone, Default)]
pub struct FxOptionCalculator {
    pub config: FxOptionPricerConfig,
}

pub(crate) fn compute_pv(inst: &FxOption, curves: &MarketContext, as_of: Date) -> Result<Money> {
    FxOptionCalculator::default().npv(inst, curves, as_of)
}

pub(crate) fn compute_greeks(
    inst: &FxOption,
    curves: &MarketContext,
    as_of: Date,
) -> Result<FxOptionGreeks> {
    FxOptionCalculator::default().compute_greeks(inst, curves, as_of)
}

pub(crate) fn implied_vol(
    inst: &FxOption,
    curves: &MarketContext,
    as_of: Date,
    target_price: f64,
    initial_guess: Option<f64>,
) -> Result<f64> {
    FxOptionCalculator::default().implied_vol(inst, curves, as_of, target_price, initial_guess)
}

impl FxOptionCalculator {
    pub fn npv(&self, inst: &FxOption, curves: &MarketContext, as_of: Date) -> Result<Money> {
        self.validate_exercise_style(inst)?;
        self.validate_currency(inst)?;
        let (spot, r_d, r_f, sigma, t) = self.collect_inputs(inst, curves, as_of)?;
        if spot <= 0.0 || inst.strike < 0.0 || inst.notional.amount() <= 0.0 {
            return Err(finstack_core::Error::Validation(format!(
                "FxOption requires spot > 0, strike >= 0, and notional > 0; got spot={spot}, strike={}, notional={}",
                inst.strike,
                inst.notional.amount()
            )));
        }

        if t <= 0.0 {
            let intrinsic = match inst.option_type {
                OptionType::Call => (spot - inst.strike).max(0.0),
                OptionType::Put => (inst.strike - spot).max(0.0),
            };
            return Ok(Money::new(
                intrinsic * inst.notional.amount(),
                inst.quote_currency,
            ));
        }

        if !inst.strike.is_finite() {
            return Err(finstack_core::Error::Validation(
                "FX option strike must be finite".to_string(),
            ));
        }

        if inst.strike.abs() < STRIKE_ZERO_TOL {
            let unit_price = match inst.option_type {
                OptionType::Call => spot * (-r_f * t).exp(),
                OptionType::Put => 0.0,
            };
            return Ok(Money::new(
                unit_price * inst.notional.amount(),
                inst.quote_currency,
            ));
        }

        let price = price_gk_core(spot, inst.strike, r_d, r_f, sigma, t, inst.option_type);
        Ok(Money::new(
            price * inst.notional.amount(),
            inst.quote_currency,
        ))
    }

    pub fn collect_inputs(
        &self,
        inst: &FxOption,
        curves: &MarketContext,
        as_of: Date,
    ) -> Result<(f64, f64, f64, f64, f64)> {
        if as_of >= inst.expiry {
            return self.collect_inputs_expired(inst, curves, as_of);
        }

        let domestic_disc = curves.get_discount(inst.domestic_discount_curve_id.as_str())?;
        let foreign_disc = curves.get_discount(inst.foreign_discount_curve_id.as_str())?;
        let t_disc_dom = year_fraction(domestic_disc.day_count(), as_of, inst.expiry)?;
        let t_disc_for = year_fraction(foreign_disc.day_count(), as_of, inst.expiry)?;
        let df_d = domestic_disc.df(t_disc_dom);
        let df_f = foreign_disc.df(t_disc_for);
        let t_vol = year_fraction(inst.day_count, as_of, inst.expiry)?;
        let r_d = if t_vol > 0.0 { -df_d.ln() / t_vol } else { 0.0 };
        let r_f = if t_vol > 0.0 { -df_f.ln() / t_vol } else { 0.0 };

        let fx_matrix = curves.fx().ok_or(finstack_core::Error::from(
            finstack_core::InputError::NotFound {
                id: "fx_matrix".to_string(),
            },
        ))?;
        let spot = fx_matrix
            .rate(FxQuery::new(inst.base_currency, inst.quote_currency, as_of))?
            .rate;

        let sigma = if let Some(impl_vol) = inst.pricing_overrides.market_quotes.implied_volatility
        {
            impl_vol
        } else {
            let vol_surface = curves.get_surface(inst.vol_surface_id.as_str())?;
            vol_surface.value_clamped(t_vol, inst.strike)
        };

        Ok((spot, r_d, r_f, sigma, t_vol))
    }

    fn collect_inputs_expired(
        &self,
        inst: &FxOption,
        curves: &MarketContext,
        as_of: Date,
    ) -> Result<(f64, f64, f64, f64, f64)> {
        let fx_matrix = curves.fx().ok_or(finstack_core::Error::from(
            finstack_core::InputError::NotFound {
                id: "fx_matrix".to_string(),
            },
        ))?;
        let spot = fx_matrix
            .rate(FxQuery::new(inst.base_currency, inst.quote_currency, as_of))?
            .rate;
        Ok((spot, 0.0, 0.0, 0.0, 0.0))
    }

    pub fn collect_inputs_no_vol(
        &self,
        inst: &FxOption,
        curves: &MarketContext,
        as_of: Date,
    ) -> Result<(f64, f64, f64, f64)> {
        if as_of >= inst.expiry {
            let fx_matrix = curves.fx().ok_or(finstack_core::Error::from(
                finstack_core::InputError::NotFound {
                    id: "fx_matrix".to_string(),
                },
            ))?;
            let spot = fx_matrix
                .rate(FxQuery::new(inst.base_currency, inst.quote_currency, as_of))?
                .rate;
            return Ok((spot, 0.0, 0.0, 0.0));
        }

        let domestic_disc = curves.get_discount(inst.domestic_discount_curve_id.as_str())?;
        let foreign_disc = curves.get_discount(inst.foreign_discount_curve_id.as_str())?;
        let t_disc_dom = year_fraction(domestic_disc.day_count(), as_of, inst.expiry)?;
        let t_disc_for = year_fraction(foreign_disc.day_count(), as_of, inst.expiry)?;
        let df_d = domestic_disc.df(t_disc_dom);
        let df_f = foreign_disc.df(t_disc_for);
        let t_vol = year_fraction(inst.day_count, as_of, inst.expiry)?;
        let r_d = if t_vol > 0.0 { -df_d.ln() / t_vol } else { 0.0 };
        let r_f = if t_vol > 0.0 { -df_f.ln() / t_vol } else { 0.0 };

        let fx_matrix = curves.fx().ok_or(finstack_core::Error::from(
            finstack_core::InputError::NotFound {
                id: "fx_matrix".to_string(),
            },
        ))?;
        let spot = fx_matrix
            .rate(FxQuery::new(inst.base_currency, inst.quote_currency, as_of))?
            .rate;

        Ok((spot, r_d, r_f, t_vol))
    }

    pub fn implied_vol(
        &self,
        inst: &FxOption,
        curves: &MarketContext,
        as_of: Date,
        target_price: f64,
        initial_guess: Option<f64>,
    ) -> Result<f64> {
        self.validate_currency(inst)?;
        let (spot, r_d, r_f, t) = self.collect_inputs_no_vol(inst, curves, as_of)?;
        if t <= 0.0 {
            return Ok(0.0);
        }
        if spot <= 0.0 || inst.strike <= 0.0 || inst.notional.amount() <= 0.0 {
            return Err(finstack_core::Error::Validation(format!(
                "Implied vol requires spot > 0, strike > 0, and notional > 0; got spot={spot}, strike={}, notional={}",
                inst.strike,
                inst.notional.amount()
            )));
        }

        let initial_guess = initial_guess.or(Some(self.config.iv_initial_guess));
        let target_unit = target_price / inst.notional.amount();
        let _ = initial_guess;

        crate::instruments::common_impl::models::bs_implied_vol(
            spot,
            inst.strike,
            r_d,
            r_f,
            t,
            inst.option_type,
            target_unit,
        )
    }

    pub fn compute_greeks(
        &self,
        inst: &FxOption,
        curves: &MarketContext,
        as_of: Date,
    ) -> Result<FxOptionGreeks> {
        self.validate_currency(inst)?;
        let (spot, r_d, r_f, sigma, t) = self.collect_inputs(inst, curves, as_of)?;
        if spot <= 0.0 || inst.strike < 0.0 || inst.notional.amount() <= 0.0 {
            return Err(finstack_core::Error::Validation(format!(
                "FxOption greeks require spot > 0, strike >= 0, and notional > 0; got spot={spot}, strike={}, notional={}",
                inst.strike,
                inst.notional.amount()
            )));
        }

        if t <= 0.0 {
            let spot_gt_strike = spot > inst.strike;
            let delta_unit = match inst.option_type {
                OptionType::Call => {
                    if spot_gt_strike {
                        1.0
                    } else {
                        0.0
                    }
                }
                OptionType::Put => {
                    if !spot_gt_strike {
                        -1.0
                    } else {
                        0.0
                    }
                }
            };
            let scale = inst.notional.amount();
            return Ok(FxOptionGreeks {
                delta: delta_unit * scale,
                ..Default::default()
            });
        }

        if !inst.strike.is_finite() {
            return Err(finstack_core::Error::Validation(
                "FX option strike must be finite".to_string(),
            ));
        }

        if inst.strike.abs() < STRIKE_ZERO_TOL {
            let scale = inst.notional.amount();
            let exp_rf_t = (-r_f * t).exp();
            let delta_unit = match inst.option_type {
                OptionType::Call => exp_rf_t,
                OptionType::Put => 0.0,
            };
            return Ok(FxOptionGreeks {
                delta: delta_unit * scale,
                ..Default::default()
            });
        }

        let greeks_unit = bs_greeks(
            spot,
            inst.strike,
            r_d,
            r_f,
            sigma,
            t,
            inst.option_type,
            self.config.theta_days_per_year,
        );
        let d1 = crate::instruments::common_impl::models::d1(spot, inst.strike, r_d, sigma, t, r_f);
        let d2 = d1 - sigma * t.sqrt();
        let cdf_d1 = finstack_core::math::norm_cdf(d1);
        let cdf_d2 = finstack_core::math::norm_cdf(d2);
        let exp_rd_t = (-r_d * t).exp();
        let delta_forward_unit = match inst.option_type {
            OptionType::Call => cdf_d1,
            OptionType::Put => cdf_d1 - 1.0,
        };
        let delta_premium_adjusted_unit = match inst.option_type {
            OptionType::Call => (inst.strike / spot) * exp_rd_t * cdf_d2,
            OptionType::Put => (inst.strike / spot) * exp_rd_t * (cdf_d2 - 1.0),
        };

        let scale = inst.notional.amount();
        Ok(FxOptionGreeks {
            delta: greeks_unit.delta * scale,
            delta_forward: delta_forward_unit * scale,
            delta_premium_adjusted: delta_premium_adjusted_unit * scale,
            gamma: greeks_unit.gamma * scale,
            vega: greeks_unit.vega * scale,
            theta: greeks_unit.theta * scale,
            rho_domestic: greeks_unit.rho_r * scale,
            rho_foreign: greeks_unit.rho_q * scale,
        })
    }

    #[inline]
    fn validate_exercise_style(&self, inst: &FxOption) -> Result<()> {
        use crate::instruments::ExerciseStyle;
        if inst.exercise_style != ExerciseStyle::European {
            return Err(finstack_core::Error::Validation(format!(
                "FxOption only supports European exercise style. \
                 Got {:?}. American and Bermudan options require \
                 specialized pricers not yet implemented.",
                inst.exercise_style
            )));
        }
        Ok(())
    }

    #[inline]
    fn validate_currency(&self, inst: &FxOption) -> Result<()> {
        if inst.notional.currency() as i32 != inst.base_currency as i32 {
            return Err(finstack_core::Error::CurrencyMismatch {
                expected: inst.base_currency,
                actual: inst.notional.currency(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default)]
#[allow(dead_code)]
pub struct FxOptionGreeks {
    pub delta: f64,
    pub delta_forward: f64,
    pub delta_premium_adjusted: f64,
    pub gamma: f64,
    pub vega: f64,
    pub theta: f64,
    pub rho_domestic: f64,
    pub rho_foreign: f64,
}

#[inline]
fn price_gk_core(
    spot: f64,
    strike: f64,
    r_d: f64,
    r_f: f64,
    sigma: f64,
    t: f64,
    option_type: OptionType,
) -> f64 {
    bs_price(spot, strike, r_d, r_f, sigma, t, option_type)
}

/// Registry-facing pricer for vanilla FX options.
pub struct SimpleFxOptionBlackPricer {
    model: ModelKey,
}

impl SimpleFxOptionBlackPricer {
    /// Create a new FX option pricer using the default model key.
    pub fn new() -> Self {
        Self {
            model: ModelKey::Black76,
        }
    }

    /// Create an FX option pricer with a specific model key.
    pub fn with_model(model: ModelKey) -> Self {
        Self { model }
    }
}

impl Default for SimpleFxOptionBlackPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for SimpleFxOptionBlackPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::FxOption, self.model)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> std::result::Result<ValuationResult, PricingError> {
        let fx_option = instrument
            .as_any()
            .downcast_ref::<FxOption>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::FxOption, instrument.key())
            })?;

        let pv = compute_pv(fx_option, market, as_of).map_err(|e| {
            PricingError::model_failure_with_context(e.to_string(), PricingErrorContext::default())
        })?;

        Ok(ValuationResult::stamped(fx_option.id(), as_of, pv))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, deprecated)]
mod delegation_tests {
    use super::*;
    use crate::instruments::common_impl::traits::Attributes;
    use crate::instruments::{ExerciseStyle, PricingOverrides, SettlementType};
    use finstack_core::currency::Currency;
    use finstack_core::dates::{Date, DayCount};
    use finstack_core::market_data::surfaces::VolSurface;
    use finstack_core::market_data::term_structures::DiscountCurve;
    use finstack_core::money::fx::{FxMatrix, SimpleFxProvider};
    use finstack_core::types::{CurveId, InstrumentId};
    use std::sync::Arc;
    use time::macros::date;

    fn build_market(as_of: Date) -> MarketContext {
        let usd_curve = DiscountCurve::builder("USD-OIS")
            .base_date(as_of)
            .day_count(DayCount::Act365F)
            .knots([(0.0, 1.0), (1.0, (-0.03_f64).exp())])
            .build()
            .expect("usd curve");
        let eur_curve = DiscountCurve::builder("EUR-OIS")
            .base_date(as_of)
            .day_count(DayCount::Act365F)
            .knots([(0.0, 1.0), (1.0, (-0.01_f64).exp())])
            .build()
            .expect("eur curve");
        let vol_surface = VolSurface::builder("EURUSD-VOL")
            .expiries(&[0.25, 0.5, 1.0, 2.0])
            .strikes(&[0.9, 1.0, 1.1, 1.2, 1.3])
            .row(&[0.15; 5])
            .row(&[0.15; 5])
            .row(&[0.15; 5])
            .row(&[0.15; 5])
            .build()
            .expect("vol surface");
        let provider = SimpleFxProvider::new().with_quote(Currency::EUR, Currency::USD, 1.20);
        let fx_matrix = FxMatrix::new(Arc::new(provider));

        MarketContext::new()
            .insert(usd_curve)
            .insert(eur_curve)
            .insert_surface(vol_surface)
            .insert_fx(fx_matrix)
    }

    fn build_option(expiry: Date) -> FxOption {
        FxOption::builder()
            .id(InstrumentId::new("FX-OPTION-TEST"))
            .base_currency(Currency::EUR)
            .quote_currency(Currency::USD)
            .strike(1.20)
            .option_type(OptionType::Call)
            .exercise_style(ExerciseStyle::European)
            .expiry(expiry)
            .day_count(DayCount::Act365F)
            .notional(Money::new(1_000_000.0, Currency::EUR))
            .settlement(SettlementType::Cash)
            .domestic_discount_curve_id(CurveId::new("USD-OIS"))
            .foreign_discount_curve_id(CurveId::new("EUR-OIS"))
            .vol_surface_id(CurveId::new("EURUSD-VOL"))
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("fx option")
    }

    #[test]
    fn fx_option_pricer_compute_pv_matches_instrument_value() {
        let as_of = date!(2024 - 01 - 01);
        let expiry = date!(2025 - 01 - 01);
        let option = build_option(expiry);
        let market = build_market(as_of);

        let via_pricer = compute_pv(&option, &market, as_of).expect("pricer pv");
        let via_instrument = option.value(&market, as_of).expect("instrument pv");

        assert!((via_pricer.amount() - via_instrument.amount()).abs() < 1e-10);
        assert_eq!(via_pricer.currency(), via_instrument.currency());
    }
}
