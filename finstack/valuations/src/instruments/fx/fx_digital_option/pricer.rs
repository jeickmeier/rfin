//! FX digital option pricer implementation.

use crate::instruments::common_impl::models::volatility::black::d1_d2;
use crate::instruments::common_impl::parameters::OptionType;
use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::fx::fx_digital_option::types::{DigitalPayoutType, FxDigitalOption};
use crate::pricer::{
    InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingErrorContext,
};
use crate::results::ValuationResult;
use finstack_core::dates::{Date, DayCountCtx};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::fx::FxQuery;
use finstack_core::money::Money;
use finstack_core::Result;

/// FX digital option calculator.
#[derive(Debug, Clone)]
pub struct FxDigitalOptionCalculator {
    /// Days per year for theta scaling.
    pub theta_days_per_year: f64,
}

impl Default for FxDigitalOptionCalculator {
    fn default() -> Self {
        Self {
            theta_days_per_year: 365.0,
        }
    }
}

pub(crate) fn compute_pv(
    inst: &FxDigitalOption,
    curves: &MarketContext,
    as_of: Date,
) -> Result<Money> {
    FxDigitalOptionCalculator::default().npv(inst, curves, as_of)
}

pub(crate) fn compute_greeks(
    inst: &FxDigitalOption,
    curves: &MarketContext,
    as_of: Date,
) -> Result<FxDigitalOptionGreeks> {
    FxDigitalOptionCalculator::default().compute_greeks(inst, curves, as_of)
}

impl FxDigitalOptionCalculator {
    pub fn npv(
        &self,
        inst: &FxDigitalOption,
        curves: &MarketContext,
        as_of: Date,
    ) -> Result<Money> {
        let (spot, r_d, r_f, sigma, t) = self.collect_inputs(inst, curves, as_of)?;

        if t <= 0.0 {
            let itm = match inst.option_type {
                OptionType::Call => spot > inst.strike,
                OptionType::Put => spot < inst.strike,
            };
            return if itm {
                match inst.payout_type {
                    DigitalPayoutType::CashOrNothing => Ok(inst.payout_amount),
                    DigitalPayoutType::AssetOrNothing => Ok(Money::new(
                        spot * inst.notional.amount(),
                        inst.quote_currency,
                    )),
                }
            } else {
                Ok(Money::new(0.0, inst.quote_currency))
            };
        }

        let price = price_digital(
            spot,
            inst.strike,
            r_d,
            r_f,
            sigma,
            t,
            inst.option_type,
            inst.payout_type,
            inst.payout_amount.amount(),
            inst.notional.amount(),
        );

        Ok(Money::new(price, inst.quote_currency))
    }

    pub fn compute_greeks(
        &self,
        inst: &FxDigitalOption,
        curves: &MarketContext,
        as_of: Date,
    ) -> Result<FxDigitalOptionGreeks> {
        let (spot, r_d, r_f, sigma, t) = self.collect_inputs(inst, curves, as_of)?;

        if t <= 0.0 {
            return Ok(FxDigitalOptionGreeks::default());
        }

        Ok(greeks_digital(
            spot,
            inst.strike,
            r_d,
            r_f,
            sigma,
            t,
            inst.option_type,
            inst.payout_type,
            inst.payout_amount.amount(),
            inst.notional.amount(),
            self.theta_days_per_year,
        ))
    }

    pub fn collect_inputs(
        &self,
        inst: &FxDigitalOption,
        curves: &MarketContext,
        as_of: Date,
    ) -> Result<(f64, f64, f64, f64, f64)> {
        if as_of >= inst.expiry {
            return self.collect_inputs_expired(inst, curves, as_of);
        }

        let domestic_disc = curves.get_discount(inst.domestic_discount_curve_id.as_str())?;
        let foreign_disc = curves.get_discount(inst.foreign_discount_curve_id.as_str())?;

        let t_disc_for =
            foreign_disc
                .day_count()
                .year_fraction(as_of, inst.expiry, DayCountCtx::default())?;
        let t_vol = inst
            .day_count
            .year_fraction(as_of, inst.expiry, DayCountCtx::default())?;
        let t_disc_dom =
            domestic_disc
                .day_count()
                .year_fraction(as_of, inst.expiry, DayCountCtx::default())?;

        let df_d = domestic_disc.df(t_disc_dom);
        let df_f = foreign_disc.df(t_disc_for);
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
        inst: &FxDigitalOption,
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
}

/// Greeks for an FX digital option.
#[derive(Debug, Clone, Copy, Default)]
pub struct FxDigitalOptionGreeks {
    pub delta: f64,
    pub gamma: f64,
    pub vega: f64,
    pub theta: f64,
    pub rho_domestic: f64,
}

#[allow(clippy::too_many_arguments)]
fn price_digital(
    spot: f64,
    strike: f64,
    r_d: f64,
    r_f: f64,
    sigma: f64,
    t: f64,
    option_type: OptionType,
    payout_type: DigitalPayoutType,
    payout_amount: f64,
    notional: f64,
) -> f64 {
    let (d1, d2) = d1_d2(spot, strike, r_d, sigma, t, r_f);
    let exp_rd_t = (-r_d * t).exp();
    let exp_rf_t = (-r_f * t).exp();

    match payout_type {
        DigitalPayoutType::CashOrNothing => match option_type {
            OptionType::Call => exp_rd_t * finstack_core::math::norm_cdf(d2) * payout_amount,
            OptionType::Put => exp_rd_t * finstack_core::math::norm_cdf(-d2) * payout_amount,
        },
        DigitalPayoutType::AssetOrNothing => match option_type {
            OptionType::Call => spot * exp_rf_t * finstack_core::math::norm_cdf(d1) * notional,
            OptionType::Put => spot * exp_rf_t * finstack_core::math::norm_cdf(-d1) * notional,
        },
    }
}

#[allow(clippy::too_many_arguments)]
fn greeks_digital(
    spot: f64,
    strike: f64,
    r_d: f64,
    r_f: f64,
    sigma: f64,
    t: f64,
    option_type: OptionType,
    payout_type: DigitalPayoutType,
    payout_amount: f64,
    notional: f64,
    theta_days_per_year: f64,
) -> FxDigitalOptionGreeks {
    let (d1, d2) = d1_d2(spot, strike, r_d, sigma, t, r_f);
    let exp_rd_t = (-r_d * t).exp();
    let exp_rf_t = (-r_f * t).exp();
    let sqrt_t = t.sqrt();
    let pdf_d1 = finstack_core::math::norm_pdf(d1);
    let pdf_d2 = finstack_core::math::norm_pdf(d2);
    let cdf_d1 = finstack_core::math::norm_cdf(d1);
    let cdf_d2 = finstack_core::math::norm_cdf(d2);
    let sigma_sqrt_t = sigma * sqrt_t;

    if sigma_sqrt_t <= 0.0 {
        return FxDigitalOptionGreeks::default();
    }

    match payout_type {
        DigitalPayoutType::CashOrNothing => {
            let delta_sign = match option_type {
                OptionType::Call => 1.0,
                OptionType::Put => -1.0,
            };
            let delta = delta_sign * exp_rd_t * pdf_d2 * payout_amount / (spot * sigma_sqrt_t);
            let gamma = -delta_sign * exp_rd_t * pdf_d2 * d1 * payout_amount
                / (spot * spot * sigma * sigma * t);
            let vega = -delta_sign * exp_rd_t * pdf_d2 * (d1 / sigma) * payout_amount / 100.0;

            let base_pv = match option_type {
                OptionType::Call => exp_rd_t * cdf_d2 * payout_amount,
                OptionType::Put => exp_rd_t * (1.0 - cdf_d2) * payout_amount,
            };
            let dt = 1.0 / theta_days_per_year;
            let t_minus = (t - dt).max(0.0);
            let pv_t_minus = if t_minus > 0.0 {
                price_digital(
                    spot,
                    strike,
                    r_d,
                    r_f,
                    sigma,
                    t_minus,
                    option_type,
                    payout_type,
                    payout_amount,
                    notional,
                )
            } else {
                let itm = match option_type {
                    OptionType::Call => spot > strike,
                    OptionType::Put => spot < strike,
                };
                if itm {
                    payout_amount
                } else {
                    0.0
                }
            };
            let theta = pv_t_minus - base_pv;

            let rho_sign = match option_type {
                OptionType::Call => 1.0,
                OptionType::Put => -1.0,
            };
            let rho_domestic = (-t * base_pv
                + rho_sign * exp_rd_t * pdf_d2 * (t / sigma_sqrt_t) * payout_amount)
                / 100.0;

            FxDigitalOptionGreeks {
                delta,
                gamma,
                vega,
                theta,
                rho_domestic,
            }
        }
        DigitalPayoutType::AssetOrNothing => {
            let delta = match option_type {
                OptionType::Call => exp_rf_t * (cdf_d1 + pdf_d1 / sigma_sqrt_t) * notional,
                OptionType::Put => exp_rf_t * ((1.0 - cdf_d1) - pdf_d1 / sigma_sqrt_t) * notional,
            };

            let bump = spot * 0.001;
            let pv_up = price_digital(
                spot + bump,
                strike,
                r_d,
                r_f,
                sigma,
                t,
                option_type,
                payout_type,
                payout_amount,
                notional,
            );
            let pv_dn = price_digital(
                spot - bump,
                strike,
                r_d,
                r_f,
                sigma,
                t,
                option_type,
                payout_type,
                payout_amount,
                notional,
            );
            let pv_base = price_digital(
                spot,
                strike,
                r_d,
                r_f,
                sigma,
                t,
                option_type,
                payout_type,
                payout_amount,
                notional,
            );
            let gamma = (pv_up - 2.0 * pv_base + pv_dn) / (bump * bump);

            let vol_bump = 0.01;
            let pv_vol_up = price_digital(
                spot,
                strike,
                r_d,
                r_f,
                sigma + vol_bump,
                t,
                option_type,
                payout_type,
                payout_amount,
                notional,
            );
            let vega = (pv_vol_up - pv_base) / (vol_bump * 100.0);

            let dt = 1.0 / theta_days_per_year;
            let t_minus = (t - dt).max(0.0);
            let pv_t_minus = if t_minus > 0.0 {
                price_digital(
                    spot,
                    strike,
                    r_d,
                    r_f,
                    sigma,
                    t_minus,
                    option_type,
                    payout_type,
                    payout_amount,
                    notional,
                )
            } else {
                let itm = match option_type {
                    OptionType::Call => spot > strike,
                    OptionType::Put => spot < strike,
                };
                if itm {
                    spot * notional
                } else {
                    0.0
                }
            };
            let theta = pv_t_minus - pv_base;

            let rate_bump = 0.0001;
            let pv_rate_up = price_digital(
                spot,
                strike,
                r_d + rate_bump,
                r_f,
                sigma,
                t,
                option_type,
                payout_type,
                payout_amount,
                notional,
            );
            let rho_domestic = (pv_rate_up - pv_base) / rate_bump / 100.0;

            FxDigitalOptionGreeks {
                delta,
                gamma,
                vega,
                theta,
                rho_domestic,
            }
        }
    }
}

/// FX digital option pricer using Garman-Kohlhagen adapted closed-form.
pub struct SimpleFxDigitalOptionPricer {
    model: ModelKey,
}

impl SimpleFxDigitalOptionPricer {
    /// Create a new FX digital option pricer with default model.
    pub fn new() -> Self {
        Self {
            model: ModelKey::Black76,
        }
    }

    /// Create an FX digital option pricer with specified model key.
    pub fn with_model(model: ModelKey) -> Self {
        Self { model }
    }
}

impl Default for SimpleFxDigitalOptionPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for SimpleFxDigitalOptionPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::FxDigitalOption, self.model)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> std::result::Result<ValuationResult, PricingError> {
        let fx_digital = instrument
            .as_any()
            .downcast_ref::<FxDigitalOption>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::FxDigitalOption, instrument.key())
            })?;

        let pv = compute_pv(fx_digital, market, as_of).map_err(|e| {
            PricingError::model_failure_with_context(e.to_string(), PricingErrorContext::default())
        })?;

        Ok(ValuationResult::stamped(fx_digital.id(), as_of, pv))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, deprecated)]
mod tests {
    use super::*;
    use crate::instruments::common_impl::traits::Attributes;
    use crate::instruments::{OptionType, PricingOverrides};
    use finstack_core::currency::Currency;
    use finstack_core::dates::{Date, DayCount};
    use finstack_core::market_data::surfaces::VolSurface;
    use finstack_core::market_data::term_structures::DiscountCurve;
    use finstack_core::money::fx::{FxMatrix, SimpleFxProvider};
    use finstack_core::money::Money;
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
        let provider = SimpleFxProvider::new();
        provider.set_quote(Currency::EUR, Currency::USD, 1.20).expect("valid rate");
        let fx_matrix = FxMatrix::new(Arc::new(provider));

        MarketContext::new()
            .insert(usd_curve)
            .insert(eur_curve)
            .insert_surface(vol_surface)
            .insert_fx(fx_matrix)
    }

    fn build_option(expiry: Date) -> FxDigitalOption {
        FxDigitalOption::builder()
            .id(InstrumentId::new("FX-DIGITAL-TEST"))
            .base_currency(Currency::EUR)
            .quote_currency(Currency::USD)
            .strike(1.20)
            .option_type(OptionType::Call)
            .payout_type(
                crate::instruments::fx::fx_digital_option::DigitalPayoutType::CashOrNothing,
            )
            .payout_amount(Money::new(100_000.0, Currency::USD))
            .expiry(expiry)
            .day_count(DayCount::Act365F)
            .notional(Money::new(1_000_000.0, Currency::EUR))
            .domestic_discount_curve_id(CurveId::new("USD-OIS"))
            .foreign_discount_curve_id(CurveId::new("EUR-OIS"))
            .vol_surface_id(CurveId::new("EURUSD-VOL"))
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("fx digital option")
    }

    #[test]
    fn fx_digital_pricer_compute_pv_matches_instrument_value() {
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
