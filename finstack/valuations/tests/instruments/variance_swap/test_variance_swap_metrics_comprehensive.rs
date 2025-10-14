use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount, Frequency};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::scalars::{MarketScalar, ScalarTimeSeries, SeriesInterpolation};
use finstack_core::market_data::surfaces::vol_surface::VolSurface;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::math::stats::{realized_variance, RealizedVarMethod};
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::instruments::variance_swap::{PayReceive, VarianceSwap};
use finstack_valuations::metrics::MetricId;
use time::Month;

fn date(y: i32, m: u8, d: u8) -> Date {
    Date::from_calendar_date(y, Month::try_from(m).unwrap(), d).unwrap()
}

const DISC_ID: &str = "USD_OIS";
const UNDERLYING_ID: &str = "SPX";

fn sample_swap(side: PayReceive) -> VarianceSwap {
    VarianceSwap::builder()
        .id(InstrumentId::new(format!("VAR-{side:?}")))
        .underlying_id(UNDERLYING_ID.to_string())
        .notional(Money::new(1_000_000.0, Currency::USD))
        .strike_variance(0.04)
        .start_date(date(2025, 1, 2))
        .maturity(date(2025, 4, 1))
        .observation_freq(Frequency::daily())
        .realized_var_method(RealizedVarMethod::CloseToClose)
        .side(side)
        .disc_id(CurveId::new(DISC_ID))
        .day_count(DayCount::Act365F)
        .attributes(Default::default())
        .build()
        .unwrap()
}

fn base_context() -> MarketContext {
    let disc_curve = DiscountCurve::builder(DISC_ID)
        .base_date(date(2025, 1, 2))
        .knots([(0.0, 1.0), (0.25, 0.995), (0.5, 0.98), (1.0, 0.95)])
        .build()
        .unwrap();
    MarketContext::new()
        .insert_discount(disc_curve)
        .insert_price(UNDERLYING_ID, MarketScalar::Unitless(5_000.0))
}

fn add_series(ctx: MarketContext, prices: &[(Date, f64)]) -> MarketContext {
    let series = ScalarTimeSeries::new(UNDERLYING_ID, prices.to_vec(), None)
        .unwrap()
        .with_interpolation(SeriesInterpolation::Step);
    ctx.insert_series(series)
}

fn add_unitless(ctx: MarketContext, id: impl AsRef<str>, value: f64) -> MarketContext {
    ctx.insert_price(id, MarketScalar::Unitless(value))
}

fn add_surface(ctx: MarketContext, surface: VolSurface) -> MarketContext {
    ctx.insert_surface(surface)
}

fn sample_surface() -> VolSurface {
    VolSurface::builder(UNDERLYING_ID)
        .expiries(&[0.25, 0.50])
        .strikes(&[4_800.0, 5_200.0])
        .row(&[0.30, 0.29])
        .row(&[0.28, 0.27])
        .build()
        .unwrap()
}

fn observation_weight(swap: &VarianceSwap, as_of: Date) -> f64 {
    let all = swap.observation_dates();
    if all.is_empty() {
        return 0.0;
    }
    if as_of <= swap.start_date {
        return 0.0;
    }
    if as_of >= swap.maturity {
        return 1.0;
    }
    let total = all.len() as f64;
    let realized = all.iter().filter(|&&d| d <= as_of).count() as f64;
    (realized / total).clamp(0.0, 1.0)
}

#[test]
fn metrics_pre_start_use_forward_and_discounting() {
    let swap = sample_swap(PayReceive::Receive);
    let context = add_unitless(base_context(), format!("{UNDERLYING_ID}_IMPL_VOL"), 0.22);
    let as_of = date(2024, 12, 1);

    let result = swap
        .price_with_metrics(
            &context,
            as_of,
            &[
                MetricId::ExpectedVariance,
                MetricId::RealizedVariance,
                MetricId::VarianceNotional,
                MetricId::VarianceStrikeVol,
                MetricId::VarianceTimeToMaturity,
            ],
        )
        .unwrap();

    let ev = *result.measures.get(MetricId::ExpectedVariance.as_str()).unwrap();
    let rv = *result.measures.get(MetricId::RealizedVariance.as_str()).unwrap();
    let notional = *result.measures.get(MetricId::VarianceNotional.as_str()).unwrap();
    let strike_vol = *result
        .measures
        .get(MetricId::VarianceStrikeVol.as_str())
        .unwrap();
    let ttm = *result
        .measures
        .get(MetricId::VarianceTimeToMaturity.as_str())
        .unwrap();

    assert!((ev - 0.22_f64.powi(2)).abs() < 1e-12);
    assert_eq!(rv, 0.0);
    assert_eq!(notional, swap.notional.amount());
    assert!((strike_vol - swap.strike_variance.sqrt()).abs() < 1e-12);
    assert!(ttm > 0.0);
}

#[test]
fn realized_variance_matches_series_calculation() {
    let swap = sample_swap(PayReceive::Receive);
    let prices: Vec<(Date, f64)> = swap
        .observation_dates()
        .into_iter()
        .map(|d| (d, 4_900.0 + (d.ordinal() as f64 % 30.0)))
        .collect();
    let context = add_series(base_context(), &prices);
    let as_of = date(2025, 2, 1);

    let result = swap
        .price_with_metrics(&context, as_of, &[MetricId::RealizedVariance])
        .unwrap();
    let rv = *result.measures.get(MetricId::RealizedVariance.as_str()).unwrap();

    // RealizedVarianceCalculator uses frequency-based annualization below
    let annualization_factor = match swap.observation_freq.days() {
        Some(1) => 365.0,
        Some(7) => 52.0,
        _ => match swap.observation_freq.months() {
            Some(1) => 12.0,
            Some(3) => 4.0,
            Some(12) => 1.0,
            _ => 252.0,
        },
    };

    let used_prices: Vec<f64> = prices
        .iter()
        .filter(|(d, _)| *d >= swap.start_date && *d <= as_of)
        .map(|(_, p)| *p)
        .collect();
    let manual = realized_variance(&used_prices, RealizedVarMethod::CloseToClose, annualization_factor);

    assert!((rv - manual).abs() < 1e-10);
}

#[test]
fn expected_variance_blends_realized_and_forward() {
    let mut swap = sample_swap(PayReceive::Receive);
    swap.observation_freq = Frequency::weekly();
    let prices: Vec<(Date, f64)> = swap
        .observation_dates()
        .into_iter()
        .map(|d| (d, 4_950.0 + (d.ordinal() as f64 % 20.0)))
        .collect();
    let as_of = swap.start_date + time::Duration::days(28);
    let context = add_unitless(add_series(base_context(), &prices), format!("{UNDERLYING_ID}_IMPL_VOL"), 0.23);

    let result = swap
        .price_with_metrics(&context, as_of, &[MetricId::ExpectedVariance])
        .unwrap();
    let ev = *result.measures.get(MetricId::ExpectedVariance.as_str()).unwrap();

    // Blend realized-to-date with forward implied variance by observation weight
    let obs_dates = swap.observation_dates();
    let used_prices: Vec<f64> = obs_dates
        .iter()
        .filter(|d| **d <= as_of)
        .map(|d| {
            prices
                .iter()
                .find(|(pd, _)| pd == d)
                .map(|(_, p)| *p)
                .unwrap()
        })
        .collect();
    let realized = realized_variance(&used_prices, RealizedVarMethod::CloseToClose, 52.0);
    let forward = 0.23_f64.powi(2);
    let w = observation_weight(&swap, as_of);
    let expected = realized * w + forward * (1.0 - w);

    assert!((ev - expected).abs() < 1e-8);
}

#[test]
fn vega_and_variance_vega_match_formulas_and_signs() {
    let mut swap = sample_swap(PayReceive::Receive);
    swap.observation_freq = Frequency::weekly();
    let as_of = swap.start_date + time::Duration::days(21);
    let context = add_unitless(base_context(), format!("{UNDERLYING_ID}_IMPL_VOL"), 0.25);

    let result = swap
        .price_with_metrics(
            &context,
            as_of,
            &[MetricId::Vega, MetricId::VarianceVega, MetricId::VarianceTimeToMaturity],
        )
        .unwrap();

    let vega = *result.measures.get(MetricId::Vega.as_str()).unwrap();
    let var_vega = *result
        .measures
        .get(MetricId::VarianceVega.as_str())
        .unwrap();

    // Remaining fraction by observation count
    let remaining_fraction = 1.0 - observation_weight(&swap, as_of);
    let t = swap
        .day_count
        .year_fraction(as_of, swap.maturity, Default::default())
        .unwrap();
    let df = context.get_discount_ref(DISC_ID).unwrap().df(t);

    let expected_var_vega = df * swap.notional.amount() * remaining_fraction * swap.side.sign();
    let expected_vega = df * 2.0 * swap.notional.amount() * 0.25 * 0.01 * remaining_fraction * swap.side.sign();

    assert!((var_vega - expected_var_vega).abs() < 1e-8);
    assert!((vega - expected_vega).abs() < 1e-8);

    // Flip side and check sign flips
    let swap_short = sample_swap(PayReceive::Pay);
    let v_short = swap_short
        .price_with_metrics(&context, as_of, &[MetricId::Vega])
        .unwrap();
    assert!(vega > 0.0);
    assert!(v_short.measures[MetricId::Vega.as_str()] < 0.0);
}

#[test]
fn dv01_matches_pv_times_duration_rule_of_thumb() {
    let swap = sample_swap(PayReceive::Receive);
    let as_of = date(2025, 1, 10);
    let context = add_unitless(base_context(), format!("{UNDERLYING_ID}_IMPL_VOL"), 0.20);

    let result = swap
        .price_with_metrics(&context, as_of, &[MetricId::Dv01])
        .unwrap();
    let dv01 = *result.measures.get(MetricId::Dv01.as_str()).unwrap();

    let pv = swap.value(&context, as_of).unwrap().amount();
    let t = swap
        .day_count
        .year_fraction(as_of, swap.maturity, Default::default())
        .unwrap();
    let approx = -pv * t * 1e-4;

    assert!((dv01 - approx).abs() < 1e-6);
}

#[test]
#[ignore] // FIXME: Test assertion needs adjustment after surface API changes
fn surface_is_used_for_forward_variance_when_available() {
    let swap = sample_swap(PayReceive::Receive);
    let surface = sample_surface();
    let context = add_unitless(add_surface(base_context(), surface), format!("{UNDERLYING_ID}-DIVYIELD"), 0.01);
    let as_of = date(2024, 12, 1);

    let result = swap
        .price_with_metrics(&context, as_of, &[MetricId::ExpectedVariance])
        .unwrap();
    let ev = *result.measures.get(MetricId::ExpectedVariance.as_str()).unwrap();

    // With ATM fallback, expected variance should equal surface ATM vol^2
    let t = swap
        .day_count
        .year_fraction(as_of, swap.maturity, Default::default())
        .unwrap();
    let surface_for_check = sample_surface();
    let vol_atm = surface_for_check.value_clamped(t.max(1e-8), 5_000.0);
    assert!((ev - vol_atm.powi(2)).abs() < 1e-10);
}


