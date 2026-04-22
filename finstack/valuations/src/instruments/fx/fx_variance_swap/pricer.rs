use crate::instruments::common_impl::models::bs_price;
use crate::instruments::common_impl::parameters::OptionType;
use crate::instruments::common_impl::pricing::variance_replication::carr_madan_forward_variance;
use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::fx::fx_variance_swap::FxVarianceSwap;

type OhlcVecs = (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>);
use crate::pricer::{
    InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingErrorContext,
};
use crate::results::ValuationResult;
use finstack_core::{
    dates::{Date, DateExt, DayCountContext},
    market_data::context::MarketContext,
    math::stats::realized_variance,
    money::Money,
    Result,
};

/// Registry-facing pricer for FX variance swaps.
pub(crate) struct SimpleFxVarianceSwapDiscountingPricer;

pub(crate) fn compute_pv(
    inst: &FxVarianceSwap,
    context: &MarketContext,
    as_of: Date,
) -> Result<Money> {
    inst.validate_as_of(context, as_of)?;

    let dom = context.get_discount(inst.domestic_discount_curve_id.as_str())?;

    if as_of >= inst.maturity {
        let realized_var = if inst.realized_var_method.requires_ohlc() {
            let (open, high, low, close) = get_historical_ohlc(inst, context, as_of)?;
            if close.is_empty() {
                return Ok(Money::new(0.0, inst.notional.currency()));
            }
            finstack_core::math::stats::realized_variance_ohlc(
                &open,
                &high,
                &low,
                &close,
                inst.realized_var_method,
                annualization_factor(inst),
            )?
        } else {
            let prices = get_historical_prices(inst, context, as_of)?;
            if prices.is_empty() {
                return Ok(Money::new(0.0, inst.notional.currency()));
            }
            realized_variance(
                &prices,
                inst.realized_var_method,
                annualization_factor(inst),
            )?
        };
        return Ok(inst.payoff(realized_var));
    }

    if as_of < inst.start_date {
        let forward_var = remaining_forward_variance(inst, context, as_of)?;
        let undiscounted = inst.payoff(forward_var);
        let t = inst
            .day_count
            .year_fraction(as_of, inst.maturity, DayCountContext::default())?;
        return Ok(undiscounted * dom.df(t.max(0.0)));
    }

    let realized = partial_realized_variance(inst, context, as_of)?;
    let forward = remaining_forward_variance(inst, context, as_of)?;
    let w = realized_fraction_by_observations(inst, as_of);
    let expected_var = realized * w + forward * (1.0 - w);
    let undiscounted = inst.payoff(expected_var);
    let t = inst
        .day_count
        .year_fraction(as_of, inst.maturity, DayCountContext::default())?;
    Ok(undiscounted * dom.df(t.max(0.0)))
}

pub(crate) fn observation_dates(inst: &FxVarianceSwap) -> Vec<Date> {
    let mut dates = Vec::new();
    let mut current = inst.start_date;

    if let Some(months_step) = inst.observation_freq.months() {
        while current <= inst.maturity {
            dates.push(current);
            current = current.add_months(months_step as i32);
            if current > inst.maturity {
                break;
            }
        }
    } else if let Some(days_step) = inst.observation_freq.days() {
        if days_step == 1 {
            while current <= inst.maturity {
                if current.weekday() != time::Weekday::Saturday
                    && current.weekday() != time::Weekday::Sunday
                {
                    dates.push(current);
                }
                current += time::Duration::days(1);
            }
        } else {
            while current <= inst.maturity {
                dates.push(current);
                current += time::Duration::days(days_step as i64);
                if current > inst.maturity {
                    break;
                }
            }
        }
    } else {
        while current <= inst.maturity {
            if current.weekday() != time::Weekday::Saturday
                && current.weekday() != time::Weekday::Sunday
            {
                dates.push(current);
            }
            current += time::Duration::days(1);
        }
    }

    if (dates.is_empty() || dates.last() != Some(&inst.maturity)) && !dates.contains(&inst.maturity)
    {
        dates.push(inst.maturity);
    }

    dates
}

pub(crate) fn annualization_factor(inst: &FxVarianceSwap) -> f64 {
    if let Some(months) = inst.observation_freq.months() {
        return match months {
            1 => 12.0,
            3 => 4.0,
            6 => 2.0,
            12 => 1.0,
            _ => 252.0,
        };
    }
    if let Some(days) = inst.observation_freq.days() {
        return match days {
            1 => 252.0,
            7 => 52.0,
            14 => 26.0,
            _ => 252.0,
        };
    }
    252.0
}

pub(crate) fn realized_fraction_by_observations(inst: &FxVarianceSwap, as_of: Date) -> f64 {
    let all = observation_dates(inst);
    if all.is_empty() {
        return 0.0;
    }
    if as_of <= inst.start_date {
        return 0.0;
    }
    if as_of >= inst.maturity {
        return 1.0;
    }
    let total = all.len() as f64;
    let realized = all.iter().filter(|&&d| d <= as_of).count() as f64;
    (realized / total).clamp(0.0, 1.0)
}

pub(crate) fn get_historical_prices(
    inst: &FxVarianceSwap,
    context: &MarketContext,
    as_of: Date,
) -> Result<Vec<f64>> {
    let close_id_owned = inst
        .close_series_id
        .clone()
        .unwrap_or_else(|| inst.series_id());
    if let Ok(series) = context.get_series(&close_id_owned) {
        let dates: Vec<Date> = observation_dates(inst)
            .into_iter()
            .filter(|&d| d <= as_of)
            .collect();
        if dates.len() >= 2 {
            return series.values_on(&dates);
        }
    }

    let spot = inst.spot_rate(context, as_of)?;
    Ok(vec![spot])
}

/// Load aligned OHLC histories from the market context for OHLC-based estimators.
///
/// Returns `Err(Validation)` if any required series ID is missing.
pub(crate) fn get_historical_ohlc(
    inst: &FxVarianceSwap,
    context: &MarketContext,
    as_of: Date,
) -> Result<OhlcVecs> {
    let default_close = inst
        .close_series_id
        .clone()
        .unwrap_or_else(|| inst.series_id());

    let method_label = inst.realized_var_method.label();
    let inst_id = inst.id.as_str().to_owned();

    let open_id = inst.open_series_id.as_deref().ok_or_else(|| {
        finstack_core::Error::Validation(format!(
            "FxVarianceSwap '{inst_id}': 'open_series_id' is required for \
             realized_var_method={method_label}. Set the corresponding *_series_id field."
        ))
    })?;
    let high_id = inst.high_series_id.as_deref().ok_or_else(|| {
        finstack_core::Error::Validation(format!(
            "FxVarianceSwap '{inst_id}': 'high_series_id' is required for \
             realized_var_method={method_label}. Set the corresponding *_series_id field."
        ))
    })?;
    let low_id = inst.low_series_id.as_deref().ok_or_else(|| {
        finstack_core::Error::Validation(format!(
            "FxVarianceSwap '{inst_id}': 'low_series_id' is required for \
             realized_var_method={method_label}. Set the corresponding *_series_id field."
        ))
    })?;

    let dates: Vec<Date> = observation_dates(inst)
        .into_iter()
        .filter(|&d| d <= as_of)
        .collect();

    if dates.len() < 2 {
        return Ok((vec![], vec![], vec![], vec![]));
    }

    let open_vals = context.get_series(open_id)?.values_on(&dates)?;
    let high_vals = context.get_series(high_id)?.values_on(&dates)?;
    let low_vals = context.get_series(low_id)?.values_on(&dates)?;
    let close_vals = context.get_series(&default_close)?.values_on(&dates)?;

    Ok((open_vals, high_vals, low_vals, close_vals))
}

pub(crate) fn partial_realized_variance(
    inst: &FxVarianceSwap,
    context: &MarketContext,
    as_of: Date,
) -> Result<f64> {
    if inst.realized_var_method.requires_ohlc() {
        let (open, high, low, close) = get_historical_ohlc(inst, context, as_of)?;
        if close.len() < 2 {
            return Ok(0.0);
        }
        return finstack_core::math::stats::realized_variance_ohlc(
            &open,
            &high,
            &low,
            &close,
            inst.realized_var_method,
            annualization_factor(inst),
        );
    }
    let prices = get_historical_prices(inst, context, as_of)?;
    if prices.len() < 2 {
        return Ok(0.0);
    }
    realized_variance(
        &prices,
        inst.realized_var_method,
        annualization_factor(inst),
    )
}

pub(crate) fn remaining_forward_variance(
    inst: &FxVarianceSwap,
    context: &MarketContext,
    as_of: Date,
) -> Result<f64> {
    let t = inst
        .day_count
        .year_fraction(as_of, inst.maturity, DayCountContext::default())?;
    if t <= 0.0 {
        return Ok(0.0);
    }

    let spot = inst.spot_rate(context, as_of)?;
    let surface = context.get_surface(inst.vol_surface_id.as_str())?;
    let dom = context.get_discount(inst.domestic_discount_curve_id.as_str())?;
    let for_curve = context.get_discount(inst.foreign_discount_curve_id.as_str())?;
    let t_dom = dom
        .day_count()
        .year_fraction(as_of, inst.maturity, DayCountContext::default())?;
    let t_for =
        for_curve
            .day_count()
            .year_fraction(as_of, inst.maturity, DayCountContext::default())?;
    let df_dom = dom.df(t_dom.max(0.0));
    let df_for = for_curve.df(t_for.max(0.0));

    let r_d = -df_dom.ln() / t;
    let r_f = -df_for.ln() / t;
    let fwd = spot * ((r_d - r_f) * t).exp();
    let strikes = surface.strikes();
    {
        let vol_fn = |t_exp: f64, k: f64| surface.value_clamped(t_exp, k);
        let bs_fn =
            |k: f64, v: f64, opt: OptionType| -> f64 { bs_price(spot, k, r_d, r_f, v, t, opt) };
        if let Some(variance) = carr_madan_forward_variance(strikes, fwd, r_d, t, vol_fn, bs_fn) {
            return Ok(variance);
        }
    }

    let vol_atm = surface.value_clamped(t, fwd.max(1e-12));
    if vol_atm.is_finite() && vol_atm > 0.0 {
        return Ok(vol_atm * vol_atm);
    }

    Ok(inst.strike_variance)
}

impl Default for SimpleFxVarianceSwapDiscountingPricer {
    fn default() -> Self {
        Self
    }
}

impl Pricer for SimpleFxVarianceSwapDiscountingPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::FxVarianceSwap, ModelKey::Discounting)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: Date,
    ) -> std::result::Result<ValuationResult, PricingError> {
        let swap = instrument
            .as_any()
            .downcast_ref::<FxVarianceSwap>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::FxVarianceSwap, instrument.key())
            })?;

        let pv = compute_pv(swap, market, as_of).map_err(|e| {
            PricingError::model_failure_with_context(e.to_string(), PricingErrorContext::default())
        })?;

        Ok(ValuationResult::stamped(swap.id(), as_of, pv))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::instruments::common_impl::traits::Instrument;
    use finstack_core::currency::Currency;
    use finstack_core::dates::Date;
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::market_data::term_structures::DiscountCurve;
    use finstack_core::money::fx::{FxMatrix, SimpleFxProvider};
    use std::sync::Arc;
    use time::macros::date;

    fn build_market(as_of: Date) -> MarketContext {
        let usd_curve = DiscountCurve::builder("USD-OIS")
            .base_date(as_of)
            .knots([(0.0, 1.0), (1.0, (-0.03_f64).exp())])
            .build()
            .expect("usd curve");
        let eur_curve = DiscountCurve::builder("EUR-OIS")
            .base_date(as_of)
            .knots([(0.0, 1.0), (1.0, (-0.01_f64).exp())])
            .build()
            .expect("eur curve");
        let provider = SimpleFxProvider::new();
        provider
            .set_quote(Currency::EUR, Currency::USD, 1.10)
            .expect("valid rate");
        let fx = FxMatrix::new(Arc::new(provider));
        MarketContext::new()
            .insert(usd_curve)
            .insert(eur_curve)
            .insert_fx(fx)
    }

    #[test]
    fn fx_variance_swap_pricer_compute_pv_matches_instrument_value() {
        let swap = FxVarianceSwap::example();
        let as_of = date!(2025 - 01 - 02);
        let market = build_market(as_of);

        let via_pricer = compute_pv(&swap, &market, as_of).expect("pricer pv");
        let via_instrument = swap.value(&market, as_of).expect("instrument pv");

        assert_eq!(via_pricer, via_instrument);
    }
}
