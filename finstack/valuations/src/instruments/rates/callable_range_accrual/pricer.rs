//! Hull-White 1F Monte Carlo pricer for callable range accruals.

use crate::calibration::hull_white::HullWhiteParams;
use crate::instruments::common_impl::pricing::time::relative_df_discount_curve;
use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::rates::callable_range_accrual::CallableRangeAccrual;
use crate::instruments::rates::exotics_shared::{
    standard_basis, ExerciseBoundaryPayoff, RateExoticHw1fLsmcPricer, RateExoticHw1fMcPricer,
    RateExoticMcConfig,
};
use crate::instruments::rates::range_accrual::BoundsType;
use crate::metrics::MetricId;
use crate::pricer::{
    InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingErrorContext, PricingResult,
};
use crate::results::ValuationResult;
use finstack_core::dates::{Date, DayCountContext};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;
use finstack_monte_carlo::results::MoneyEstimate;
use finstack_monte_carlo::seed;
use finstack_monte_carlo::traits::{PathState, Payoff, StateKey};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Default)]
struct CallableRangeAccrualEvent {
    is_observation: bool,
}

#[derive(Debug, Clone)]
struct CallableRangeAccrualSchedule {
    events: Vec<CallableRangeAccrualEvent>,
    event_times: Vec<f64>,
    exercise_times: Vec<f64>,
    exercise_discount_factors: Vec<f64>,
    call_prices: Vec<f64>,
    final_payment_discount_factor: f64,
    future_observations: usize,
}

#[derive(Debug, Clone)]
struct CallableRangeAccrualPayoff {
    lower_bound: f64,
    upper_bound: f64,
    coupon_rate: f64,
    notional: f64,
    events: Vec<CallableRangeAccrualEvent>,
    exercise_discount_factors: Vec<f64>,
    call_prices: Vec<f64>,
    final_payment_discount_factor: f64,
    past_in_range: usize,
    total_past_observations: usize,
    future_observations: usize,
    days_in_range: usize,
    observations_seen: usize,
    next_event: usize,
}

impl CallableRangeAccrualPayoff {
    #[allow(clippy::too_many_arguments)]
    fn new(
        lower_bound: f64,
        upper_bound: f64,
        coupon_rate: f64,
        notional: f64,
        events: Vec<CallableRangeAccrualEvent>,
        exercise_discount_factors: Vec<f64>,
        call_prices: Vec<f64>,
        final_payment_discount_factor: f64,
        past_in_range: usize,
        total_past_observations: usize,
        future_observations: usize,
    ) -> Self {
        Self {
            lower_bound,
            upper_bound,
            coupon_rate,
            notional,
            events,
            exercise_discount_factors,
            call_prices,
            final_payment_discount_factor,
            past_in_range,
            total_past_observations,
            future_observations,
            days_in_range: 0,
            observations_seen: 0,
            next_event: 0,
        }
    }

    fn is_in_range(&self, rate: f64) -> bool {
        rate >= self.lower_bound && rate <= self.upper_bound
    }

    fn coupon_value(&self) -> f64 {
        let total_observations = self.total_past_observations + self.future_observations;
        if total_observations == 0 {
            return 0.0;
        }
        let total_in_range = self.past_in_range + self.days_in_range;
        let accrual_fraction = total_in_range as f64 / total_observations as f64;
        self.coupon_rate * accrual_fraction * self.notional * self.final_payment_discount_factor
    }
}

impl Payoff for CallableRangeAccrualPayoff {
    fn on_event(&mut self, state: &mut PathState) {
        if self.next_event >= self.events.len() {
            return;
        }

        let event = self.events[self.next_event];
        if event.is_observation {
            let short_rate = state.get_key(StateKey::ShortRate).unwrap_or(0.0);
            if self.is_in_range(short_rate) {
                self.days_in_range += 1;
            }
            self.observations_seen += 1;
        }
        self.next_event += 1;
    }

    fn value(&self, currency: finstack_core::currency::Currency) -> Money {
        Money::new(self.coupon_value(), currency)
    }

    fn reset(&mut self) {
        self.days_in_range = 0;
        self.observations_seen = 0;
        self.next_event = 0;
    }
}

impl ExerciseBoundaryPayoff for CallableRangeAccrualPayoff {
    fn intrinsic_at(
        &self,
        exercise_idx: usize,
        _short_rate: f64,
        currency: finstack_core::currency::Currency,
    ) -> Money {
        let df = self
            .exercise_discount_factors
            .get(exercise_idx)
            .copied()
            .unwrap_or(0.0);
        let call_price = self.call_prices.get(exercise_idx).copied().unwrap_or(0.0);
        Money::new(call_price * self.notional * df, currency)
    }

    fn continuation_basis(&self, _exercise_idx: usize, t_years: f64, short_rate: f64) -> Vec<f64> {
        standard_basis(t_years, short_rate)
    }
}

/// Callable range accrual pricer using shared HW1F MC/LSMC infrastructure.
#[derive(Debug, Clone)]
pub struct CallableRangeAccrualPricer {
    hw_params: HullWhiteParams,
    config: RateExoticMcConfig,
}

impl CallableRangeAccrualPricer {
    /// Create a callable range accrual pricer with default HW1F and MC settings.
    pub fn new() -> Self {
        Self {
            hw_params: HullWhiteParams::default(),
            config: RateExoticMcConfig::default(),
        }
    }

    /// Create a callable range accrual pricer with explicit HW1F parameters.
    pub fn with_hw_params(hw_params: HullWhiteParams) -> Self {
        Self {
            hw_params,
            config: RateExoticMcConfig::default(),
        }
    }

    /// Create a callable range accrual pricer with explicit MC configuration.
    pub fn with_config(mut self, config: RateExoticMcConfig) -> Self {
        self.config = config;
        self
    }

    fn effective_hw_params(&self, inst: &CallableRangeAccrual) -> Result<HullWhiteParams> {
        let kappa = inst
            .pricing_overrides
            .model_config
            .mean_reversion
            .unwrap_or(self.hw_params.kappa);
        let sigma = inst
            .pricing_overrides
            .market_quotes
            .implied_volatility
            .unwrap_or(self.hw_params.sigma);
        HullWhiteParams::new(kappa, sigma)
    }

    fn effective_config(&self, inst: &CallableRangeAccrual) -> RateExoticMcConfig {
        let mut cfg = self.config;
        if let Some(paths) = inst.pricing_overrides.model_config.mc_paths {
            cfg.num_paths = paths.max(if cfg.antithetic { 2 } else { 1 });
        }
        cfg.seed = inst
            .pricing_overrides
            .metrics
            .mc_seed_scenario
            .as_deref()
            .map_or_else(
                || seed::derive_seed(&inst.id, "base"),
                |scenario| seed::derive_seed(&inst.id, scenario),
            );
        cfg
    }

    fn price_estimate(
        &self,
        inst: &CallableRangeAccrual,
        market: &MarketContext,
        as_of: Date,
    ) -> Result<MoneyEstimate> {
        inst.validate()?;
        let schedule = build_schedule(inst, market, as_of)?;
        if schedule.event_times.is_empty() {
            return Ok(zero_estimate(inst.range_accrual.notional.currency()));
        }

        let initial_rate = initial_short_rate(inst, market)?;
        let lower_bound = effective_lower_bound(inst, initial_rate);
        let upper_bound = effective_upper_bound(inst, initial_rate);
        let payoff = CallableRangeAccrualPayoff::new(
            lower_bound,
            upper_bound,
            inst.range_accrual.coupon_rate,
            inst.range_accrual.notional.amount(),
            schedule.events.clone(),
            schedule.exercise_discount_factors.clone(),
            schedule.call_prices.clone(),
            schedule.final_payment_discount_factor,
            inst.range_accrual.past_fixings_in_range.unwrap_or(0),
            inst.range_accrual.total_past_observations.unwrap_or(0),
            schedule.future_observations,
        );

        let hw_params = self.effective_hw_params(inst)?;
        let config = self.effective_config(inst);
        if schedule.exercise_times.is_empty() {
            let mc = RateExoticHw1fMcPricer {
                hw_params,
                r0: initial_rate,
                theta: initial_rate,
                event_times: schedule.event_times,
                config,
                currency: inst.range_accrual.notional.currency(),
            };
            return mc.price(|| payoff.clone());
        }

        let lsmc = RateExoticHw1fLsmcPricer {
            hw_params,
            r0: initial_rate,
            theta: initial_rate,
            event_times: schedule.event_times,
            exercise_times: schedule.exercise_times,
            call_prices: schedule.call_prices,
            notional: inst.range_accrual.notional.amount(),
            config,
            currency: inst.range_accrual.notional.currency(),
        };
        lsmc.price(|| payoff.clone())
    }

    fn price_internal(
        &self,
        inst: &CallableRangeAccrual,
        market: &MarketContext,
        as_of: Date,
    ) -> Result<Money> {
        Ok(self.price_estimate(inst, market, as_of)?.mean)
    }
}

impl Default for CallableRangeAccrualPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for CallableRangeAccrualPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(
            InstrumentType::CallableRangeAccrual,
            ModelKey::MonteCarloHullWhite1F,
        )
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: Date,
    ) -> PricingResult<ValuationResult> {
        let callable = instrument
            .as_any()
            .downcast_ref::<CallableRangeAccrual>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::CallableRangeAccrual, instrument.key())
            })?;
        let estimate = self.price_estimate(callable, market, as_of).map_err(|e| {
            PricingError::model_failure_with_context(
                e.to_string(),
                PricingErrorContext::from_instrument(instrument)
                    .model(ModelKey::MonteCarloHullWhite1F)
                    .curve_ids([callable
                        .range_accrual
                        .discount_curve_id
                        .as_str()
                        .to_string()]),
            )
        })?;

        let mut result = ValuationResult::stamped(callable.id.as_str(), as_of, estimate.mean);
        result.measures.insert(
            MetricId::custom("mc_num_paths"),
            estimate.num_simulated_paths as f64,
        );
        result
            .measures
            .insert(MetricId::custom("mc_stderr"), estimate.stderr);
        let (ci_low, ci_high) = estimate.ci_95;
        result
            .measures
            .insert(MetricId::custom("mc_ci95_low"), ci_low.amount());
        result
            .measures
            .insert(MetricId::custom("mc_ci95_high"), ci_high.amount());
        Ok(result)
    }

    fn price_raw_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: Date,
    ) -> PricingResult<f64> {
        let callable = instrument
            .as_any()
            .downcast_ref::<CallableRangeAccrual>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::CallableRangeAccrual, instrument.key())
            })?;
        self.price_internal(callable, market, as_of)
            .map(|m| m.amount())
            .map_err(|e| {
                PricingError::model_failure_with_context(
                    e.to_string(),
                    PricingErrorContext::from_instrument(instrument)
                        .model(ModelKey::MonteCarloHullWhite1F),
                )
            })
    }
}

fn build_schedule(
    inst: &CallableRangeAccrual,
    market: &MarketContext,
    as_of: Date,
) -> Result<CallableRangeAccrualSchedule> {
    let range = &inst.range_accrual;
    let discount_curve = market.get_discount(range.discount_curve_id.as_ref())?;
    let final_payment_date = range
        .payment_date
        .or_else(|| range.observation_dates.last().copied())
        .ok_or_else(|| {
            finstack_core::Error::Validation(format!(
                "CallableRangeAccrual {} requires at least one observation date",
                inst.id.as_str()
            ))
        })?;
    let final_payment_discount_factor =
        relative_df_discount_curve(discount_curve.as_ref(), as_of, final_payment_date)?;

    let mut event_dates: BTreeMap<Date, CallableRangeAccrualEvent> = BTreeMap::new();
    for &date in &range.observation_dates {
        if date > as_of {
            event_dates.entry(date).or_default().is_observation = true;
        }
    }

    let eligible_call_dates = inst
        .call_provision
        .eligible_call_dates(&range.observation_dates);
    for &date in &eligible_call_dates {
        if date > as_of {
            event_dates.entry(date).or_default();
        }
    }

    let mut events = Vec::with_capacity(event_dates.len());
    let mut event_times = Vec::with_capacity(event_dates.len());
    for (date, event) in event_dates {
        let t = range
            .day_count
            .year_fraction(as_of, date, DayCountContext::default())?;
        if t > 0.0 {
            events.push(event);
            event_times.push(t);
        }
    }

    let mut exercise_times = Vec::new();
    let mut exercise_discount_factors = Vec::new();
    let mut call_prices = Vec::new();
    for date in eligible_call_dates.into_iter().filter(|d| *d > as_of) {
        let t = range
            .day_count
            .year_fraction(as_of, date, DayCountContext::default())?;
        if t <= 0.0 {
            continue;
        }
        exercise_times.push(t);
        exercise_discount_factors.push(relative_df_discount_curve(
            discount_curve.as_ref(),
            as_of,
            date,
        )?);
        call_prices.push(inst.call_provision.call_price);
    }

    let future_observations = events.iter().filter(|event| event.is_observation).count();
    Ok(CallableRangeAccrualSchedule {
        events,
        event_times,
        exercise_times,
        exercise_discount_factors,
        call_prices,
        final_payment_discount_factor,
        future_observations,
    })
}

fn initial_short_rate(inst: &CallableRangeAccrual, market: &MarketContext) -> Result<f64> {
    let scalar = market.get_price(inst.range_accrual.spot_id.as_ref())?;
    let rate = match scalar {
        finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
        finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
    };
    if !rate.is_finite() {
        return Err(finstack_core::Error::Validation(format!(
            "CallableRangeAccrual {} initial rate is not finite",
            inst.id.as_str()
        )));
    }
    Ok(rate)
}

fn effective_lower_bound(inst: &CallableRangeAccrual, initial_rate: f64) -> f64 {
    match inst.range_accrual.bounds_type {
        BoundsType::Absolute => inst.range_accrual.lower_bound,
        BoundsType::RelativeToInitialSpot => inst.range_accrual.lower_bound * initial_rate,
    }
}

fn effective_upper_bound(inst: &CallableRangeAccrual, initial_rate: f64) -> f64 {
    match inst.range_accrual.bounds_type {
        BoundsType::Absolute => inst.range_accrual.upper_bound,
        BoundsType::RelativeToInitialSpot => inst.range_accrual.upper_bound * initial_rate,
    }
}

fn zero_estimate(currency: finstack_core::currency::Currency) -> MoneyEstimate {
    let zero = Money::new(0.0, currency);
    MoneyEstimate {
        mean: zero,
        stderr: 0.0,
        ci_95: (zero, zero),
        num_paths: 0,
        num_simulated_paths: 0,
        std_dev: Some(0.0),
        median: None,
        percentile_25: None,
        percentile_75: None,
        min: Some(0.0),
        max: Some(0.0),
        num_skipped: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::rates::exotics_shared::bermudan_call::BermudanCallProvision;
    use crate::instruments::rates::range_accrual::RangeAccrual;
    use crate::instruments::PricingOverrides;
    use finstack_core::currency::Currency;
    use finstack_core::dates::DayCount;
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::market_data::scalars::MarketScalar;
    use finstack_core::market_data::term_structures::DiscountCurve;
    use finstack_core::money::Money;
    use finstack_core::types::{CurveId, InstrumentId};
    use time::Month;

    fn date(year: i32, month: Month, day: u8) -> Date {
        Date::from_calendar_date(year, month, day).expect("valid date")
    }

    fn test_callable(
        call_dates: Vec<Date>,
        lockout_periods: usize,
        coupon_rate: f64,
    ) -> CallableRangeAccrual {
        let observation_dates = vec![
            date(2025, Month::July, 1),
            date(2026, Month::January, 1),
            date(2026, Month::July, 1),
        ];
        CallableRangeAccrual {
            id: InstrumentId::new("CALLABLE-RA-TEST"),
            range_accrual: RangeAccrual::builder()
                .id(InstrumentId::new("RA-TEST"))
                .underlying_ticker("SOFR".to_string())
                .observation_dates(observation_dates)
                .lower_bound(0.02)
                .upper_bound(0.04)
                .bounds_type(BoundsType::Absolute)
                .coupon_rate(coupon_rate)
                .notional(Money::new(1_000_000.0, Currency::USD))
                .day_count(DayCount::Act365F)
                .discount_curve_id(CurveId::new("USD-OIS"))
                .spot_id("SOFR-RATE".into())
                .vol_surface_id(CurveId::new("SOFR-VOL"))
                .div_yield_id_opt(None)
                .pricing_overrides(PricingOverrides::default())
                .attributes(Default::default())
                .payment_date_opt(None)
                .past_fixings_in_range_opt(None)
                .total_past_observations_opt(None)
                .build()
                .expect("range accrual"),
            call_provision: BermudanCallProvision::new(call_dates, 1.0, lockout_periods),
            pricing_overrides: PricingOverrides::default(),
            attributes: Default::default(),
        }
    }

    fn market(as_of: Date, discount_rate: f64, short_rate: f64) -> MarketContext {
        let discount = DiscountCurve::builder("USD-OIS")
            .base_date(as_of)
            .day_count(DayCount::Act365F)
            .knots([
                (0.0, 1.0),
                (0.5, (-discount_rate * 0.5).exp()),
                (1.5, (-discount_rate * 1.5).exp()),
            ])
            .build()
            .expect("discount curve");
        MarketContext::new()
            .insert(discount)
            .insert_price("SOFR-RATE", MarketScalar::Unitless(short_rate))
    }

    fn deterministic_pricer(paths: usize) -> CallableRangeAccrualPricer {
        CallableRangeAccrualPricer::with_hw_params(
            HullWhiteParams::new(0.05, 1e-12).expect("hw params"),
        )
        .with_config(RateExoticMcConfig {
            num_paths: paths,
            antithetic: false,
            min_steps_between_events: 1,
            ..Default::default()
        })
    }

    #[test]
    fn no_eligible_call_dates_prices_like_noncallable_coupon() {
        let as_of = date(2025, Month::January, 1);
        let curves = market(as_of, 0.02, 0.03);
        let inst = test_callable(vec![date(2025, Month::July, 1)], 10, 0.06);
        let maturity = *inst
            .range_accrual
            .observation_dates
            .last()
            .expect("maturity");
        let df = curves
            .get_discount("USD-OIS")
            .expect("discount")
            .df_between_dates(as_of, maturity)
            .expect("df");
        let expected = inst.range_accrual.notional.amount() * inst.range_accrual.coupon_rate * df;

        let estimate = deterministic_pricer(8)
            .price_estimate(&inst, &curves, as_of)
            .expect("price");

        assert!((estimate.mean.amount() - expected).abs() < 1.0);
    }

    #[test]
    fn deep_itm_issuer_call_caps_coupon_value_at_call_price() {
        let as_of = date(2025, Month::January, 1);
        let curves = market(as_of, 0.02, 0.03);
        let call_date = date(2025, Month::July, 1);
        let inst = test_callable(vec![call_date], 0, 2.0);
        let call_df = curves
            .get_discount("USD-OIS")
            .expect("discount")
            .df_between_dates(as_of, call_date)
            .expect("df");
        let expected_call_value = inst.range_accrual.notional.amount() * call_df;

        let estimate = deterministic_pricer(4)
            .price_estimate(&inst, &curves, as_of)
            .expect("price");

        assert!((estimate.mean.amount() - expected_call_value).abs() < 1.0);
    }

    #[test]
    fn price_dyn_returns_mc_measures() {
        let as_of = date(2025, Month::January, 1);
        let curves = market(as_of, 0.02, 0.03);
        let inst = test_callable(vec![date(2025, Month::July, 1)], 10, 0.06);
        let result = deterministic_pricer(8)
            .price_dyn(&inst, &curves, as_of)
            .expect("price");

        assert!(result.value.amount() > 0.0);
        assert!(result
            .measures
            .contains_key(&MetricId::custom("mc_num_paths")));
    }
}
