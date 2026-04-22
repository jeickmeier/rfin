//! FX touch option pricer implementation.

use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::fx::fx_touch_option::types::{
    BarrierDirection, FxTouchOption, PayoutTiming, TouchType,
};
use crate::pricer::{
    InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingErrorContext,
};
use crate::results::ValuationResult;
use finstack_core::dates::{Date, DayCountContext};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::fx::FxQuery;
use finstack_core::money::Money;
use finstack_core::Result;

#[derive(Debug, Clone, Default)]
pub(crate) struct FxTouchOptionCalculator;

pub(crate) fn compute_pv(
    inst: &FxTouchOption,
    curves: &MarketContext,
    as_of: Date,
) -> Result<Money> {
    FxTouchOptionCalculator.npv(inst, curves, as_of)
}

impl FxTouchOptionCalculator {
    pub(crate) fn npv(
        &self,
        inst: &FxTouchOption,
        curves: &MarketContext,
        as_of: Date,
    ) -> Result<Money> {
        let (spot, r_d, r_f, sigma, t) = self.collect_inputs(inst, curves, as_of)?;

        if t <= 0.0 {
            let observed_touch = inst.observed_touch.ok_or_else(|| {
                finstack_core::Error::Validation(
                    "Expired FX touch option requires explicit observed touch state".to_string(),
                )
            })?;
            let pv = match (inst.touch_type, observed_touch, inst.payout_timing) {
                (TouchType::OneTouch, true, PayoutTiming::AtExpiry) => inst.payout_amount.amount(),
                (TouchType::OneTouch, true, PayoutTiming::AtHit) => 0.0,
                (TouchType::OneTouch, false, _) => 0.0,
                (TouchType::NoTouch, true, _) => 0.0,
                (TouchType::NoTouch, false, _) => inst.payout_amount.amount(),
            };
            return Ok(Money::new(pv, inst.quote_currency));
        }

        let price = price_touch(
            spot,
            inst.barrier_level,
            r_d,
            r_f,
            sigma,
            t,
            inst.touch_type,
            inst.barrier_direction,
            inst.payout_timing,
            inst.payout_amount.amount(),
        );

        Ok(Money::new(price, inst.quote_currency))
    }

    pub(crate) fn collect_inputs(
        &self,
        inst: &FxTouchOption,
        curves: &MarketContext,
        as_of: Date,
    ) -> Result<(f64, f64, f64, f64, f64)> {
        if as_of >= inst.expiry {
            return self.collect_inputs_expired(inst, curves, as_of);
        }

        let domestic_disc = curves.get_discount(inst.domestic_discount_curve_id.as_str())?;
        let foreign_disc = curves.get_discount(inst.foreign_discount_curve_id.as_str())?;

        let t_vol = inst
            .day_count
            .year_fraction(as_of, inst.expiry, DayCountContext::default())?;

        let df_d = domestic_disc.df(domestic_disc.day_count().year_fraction(
            as_of,
            inst.expiry,
            DayCountContext::default(),
        )?);
        let df_f = foreign_disc.df(foreign_disc.day_count().year_fraction(
            as_of,
            inst.expiry,
            DayCountContext::default(),
        )?);

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
            vol_surface.value_clamped(t_vol, inst.barrier_level)
        };

        Ok((spot, r_d, r_f, sigma, t_vol))
    }

    fn collect_inputs_expired(
        &self,
        inst: &FxTouchOption,
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

#[allow(clippy::too_many_arguments)]
fn price_touch(
    spot: f64,
    barrier: f64,
    r_d: f64,
    r_f: f64,
    sigma: f64,
    t: f64,
    touch_type: TouchType,
    barrier_direction: BarrierDirection,
    payout_timing: PayoutTiming,
    payout: f64,
) -> f64 {
    let already_breached = match barrier_direction {
        BarrierDirection::Down => spot <= barrier,
        BarrierDirection::Up => spot >= barrier,
    };
    if already_breached {
        return match touch_type {
            TouchType::OneTouch => match payout_timing {
                PayoutTiming::AtHit => payout,
                PayoutTiming::AtExpiry => (-r_d * t).exp() * payout,
            },
            TouchType::NoTouch => 0.0,
        };
    }

    let sigma2 = sigma * sigma;
    let sqrt_t = t.sqrt();
    let sigma_sqrt_t = sigma * sqrt_t;
    if sigma_sqrt_t <= 0.0 || t <= 0.0 {
        return 0.0;
    }

    let mu = (r_d - r_f - sigma2 / 2.0) / sigma2;
    let lambda_r = match payout_timing {
        PayoutTiming::AtExpiry => 0.0,
        PayoutTiming::AtHit => r_d,
    };
    let lambda_sq = mu * mu + 2.0 * lambda_r / sigma2;
    if lambda_sq < 0.0 {
        return 0.0;
    }
    let lambda = lambda_sq.sqrt();

    let log_hs = (barrier / spot).ln();
    let z = log_hs / sigma_sqrt_t + lambda * sigma_sqrt_t;
    let z_prime = log_hs / sigma_sqrt_t - lambda * sigma_sqrt_t;
    let eta = match barrier_direction {
        BarrierDirection::Down => 1.0,
        BarrierDirection::Up => -1.0,
    };
    let s_over_h = spot / barrier;
    let power1 = s_over_h.powf(-(mu + lambda));
    let power2 = s_over_h.powf(-(mu - lambda));
    let n_eta_z = finstack_core::math::norm_cdf(eta * z);
    let n_eta_z_prime = finstack_core::math::norm_cdf(eta * z_prime);
    let one_touch_prob = power1 * n_eta_z + power2 * n_eta_z_prime;

    match payout_timing {
        PayoutTiming::AtExpiry => {
            let df = (-r_d * t).exp();
            let one_touch_pv = df * payout * one_touch_prob;
            match touch_type {
                TouchType::OneTouch => one_touch_pv,
                TouchType::NoTouch => df * payout - one_touch_pv,
            }
        }
        PayoutTiming::AtHit => {
            let one_touch_pv = payout * one_touch_prob;
            match touch_type {
                TouchType::OneTouch => one_touch_pv,
                TouchType::NoTouch => {
                    let df = (-r_d * t).exp();
                    df * payout - one_touch_pv
                }
            }
        }
    }
}

/// FX touch option pricer using Rubinstein-Reiner closed-form.
pub struct SimpleFxTouchOptionPricer {
    model: ModelKey,
}

impl SimpleFxTouchOptionPricer {
    /// Create a new FX touch option pricer with default model.
    pub fn new() -> Self {
        Self {
            model: ModelKey::Black76,
        }
    }

    /// Create an FX touch option pricer with specified model key.
    pub fn with_model(model: ModelKey) -> Self {
        Self { model }
    }
}

impl Default for SimpleFxTouchOptionPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for SimpleFxTouchOptionPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::FxTouchOption, self.model)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> std::result::Result<ValuationResult, PricingError> {
        let fx_touch = instrument
            .as_any()
            .downcast_ref::<FxTouchOption>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::FxTouchOption, instrument.key())
            })?;

        let pv = compute_pv(fx_touch, market, as_of).map_err(|e| {
            PricingError::model_failure_with_context(e.to_string(), PricingErrorContext::default())
        })?;

        Ok(ValuationResult::stamped(fx_touch.id(), as_of, pv))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::instruments::common_impl::traits::Attributes;
    use crate::instruments::PricingOverrides;
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
        provider
            .set_quote(Currency::EUR, Currency::USD, 1.20)
            .expect("valid rate");
        let fx_matrix = FxMatrix::new(Arc::new(provider));

        MarketContext::new()
            .insert(usd_curve)
            .insert(eur_curve)
            .insert_surface(vol_surface)
            .insert_fx(fx_matrix)
    }

    fn build_option(expiry: Date) -> FxTouchOption {
        FxTouchOption::builder()
            .id(InstrumentId::new("FX-TOUCH-TEST"))
            .base_currency(Currency::EUR)
            .quote_currency(Currency::USD)
            .barrier_level(1.10)
            .touch_type(crate::instruments::fx::fx_touch_option::TouchType::OneTouch)
            .barrier_direction(crate::instruments::fx::fx_touch_option::BarrierDirection::Down)
            .payout_amount(Money::new(100_000.0, Currency::USD))
            .payout_timing(crate::instruments::fx::fx_touch_option::PayoutTiming::AtExpiry)
            .expiry(expiry)
            .day_count(DayCount::Act365F)
            .domestic_discount_curve_id(CurveId::new("USD-OIS"))
            .foreign_discount_curve_id(CurveId::new("EUR-OIS"))
            .vol_surface_id(CurveId::new("EURUSD-VOL"))
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("fx touch option")
    }

    #[test]
    fn fx_touch_pricer_compute_pv_matches_instrument_value() {
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
