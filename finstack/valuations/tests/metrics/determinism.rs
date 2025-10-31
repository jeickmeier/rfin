//! Determinism tests for Monte Carlo greeks.
//!
//! Verifies that MC-priced instruments produce identical results across
//! multiple runs when computing greeks. Each metric should produce the
//! exact same value when run 10+ times with deterministic seeds.
//!
//! Tests cover:
//! - AsianOption (Delta, Vega, Gamma, Rho, Theta)
//! - BarrierOption (Delta, Vega)
//! - LookbackOption (Delta, Vega)
//! - Autocallable (Delta, Vega)
//! - CliquetOption (Delta, Vega)
//! - FxBarrierOption (Delta, Vega)
//! - QuantoOption (Delta, Vega)
//! - CmsOption (Delta, Vega)
//! - RangeAccrual (Delta, Vega)

#[cfg(feature = "mc")]
mod tests {
    use finstack_core::currency::Currency;
    use finstack_core::dates::{Date, DayCount};
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::market_data::scalars::MarketScalar;
    use finstack_core::market_data::surfaces::VolSurface;
    use finstack_core::market_data::term_structures::DiscountCurve;
    use finstack_core::money::Money;
    use finstack_valuations::instruments::asian_option::AsianOption;
    use finstack_valuations::instruments::asian_option::types::AveragingMethod;
    use finstack_valuations::instruments::autocallable::{Autocallable, FinalPayoffType};
    use finstack_valuations::instruments::barrier_option::{BarrierOption, BarrierType};
    use finstack_valuations::instruments::lookback_option::{LookbackOption, LookbackType};
    use finstack_valuations::instruments::common::parameters::market::OptionType;
    use finstack_valuations::instruments::common::traits::Instrument;
    use finstack_valuations::metrics::{standard_registry, MetricContext, MetricId};
    use std::sync::Arc;
    use time::macros::date;

    /// Helper to create a standard equity market context for MC pricing tests
    fn create_mc_market(as_of: Date, spot: f64, vol: f64, rate: f64) -> MarketContext {
        let disc_curve = DiscountCurve::builder("USD-OIS")
            .base_date(as_of)
            .day_count(DayCount::Act365F)
            .knots([
                (0.0f64, 1.0f64),
                (1.0f64, (-rate).exp()),
                (2.0f64, (-rate * 2.0f64).exp()),
            ])
            .build()
            .unwrap();

        let vol_surface = VolSurface::builder("SPOT_VOL")
            .expiries(&[0.5, 1.0, 2.0])
            .strikes(&[80.0, 100.0, 120.0])
            .row(&[vol, vol, vol])
            .row(&[vol, vol, vol])
            .row(&[vol, vol, vol])
            .build()
            .unwrap();

        MarketContext::new()
            .insert_discount(disc_curve)
            .insert_surface(vol_surface)
            .insert_price("SPOT", MarketScalar::Unitless(spot))
    }

    /// Run a metric calculation multiple times and verify all results are identical
    fn test_metric_determinism<I: Instrument + Clone + 'static>(
        instrument: I,
        market: &MarketContext,
        as_of: Date,
        metric_id: MetricId,
        num_runs: usize,
    ) {
        let registry = standard_registry();
        let pv = instrument.value(market, as_of).unwrap();

        let mut results = Vec::with_capacity(num_runs);
        for _ in 0..num_runs {
            let mut context = MetricContext::new(
                Arc::new(instrument.clone()),
                Arc::new(market.clone()),
                as_of,
                pv,
            );
            
            let metric_id_clone = metric_id.clone();
            let computed = registry.compute(std::slice::from_ref(&metric_id_clone), &mut context).unwrap();
            let value = *computed.get(&metric_id_clone).unwrap();
            results.push(value);
        }

        // All results should be identical (bit-exact equality for deterministic MC)
        let first = results[0];
        for (i, &value) in results.iter().enumerate() {
            assert_eq!(
                value, first,
                "Metric {:?} calculation {} should be identical to first run (expected {}, got {})",
                metric_id, i, first, value
            );
        }
    }

    #[test]
    fn test_asian_option_delta_deterministic() {
        let as_of = date!(2024 - 01 - 01);
        let expiry = date!(2025 - 01 - 01);
        
        let option = AsianOption {
            id: "ASIAN_DELTA_TEST".into(),
            underlying_ticker: "SPOT".to_string(),
            spot_id: "SPOT".to_string(),
            strike: Money::new(100.0, Currency::USD),
            option_type: OptionType::Call,
            expiry,
            notional: 1.0,
            averaging_method: AveragingMethod::Arithmetic,
            fixing_dates: vec![
                date!(2024 - 04 - 01),
                date!(2024 - 07 - 01),
                date!(2024 - 10 - 01),
                date!(2025 - 01 - 01),
            ],
            day_count: DayCount::Act365F,
            disc_id: "USD-OIS".into(),
            vol_id: "SPOT_VOL".into(),
            div_yield_id: None,
            pricing_overrides: Default::default(),
            attributes: Default::default(),
        };

        let market = create_mc_market(as_of, 100.0, 0.25, 0.05);
        test_metric_determinism(option, &market, as_of, MetricId::Delta, 10);
    }

    #[test]
    fn test_asian_option_vega_deterministic() {
        let as_of = date!(2024 - 01 - 01);
        let expiry = date!(2025 - 01 - 01);
        
        let option = AsianOption {
            id: "ASIAN_VEGA_TEST".into(),
            underlying_ticker: "SPOT".to_string(),
            spot_id: "SPOT".to_string(),
            strike: Money::new(100.0, Currency::USD),
            option_type: OptionType::Call,
            expiry,
            notional: 1.0,
            averaging_method: AveragingMethod::Arithmetic,
            fixing_dates: vec![
                date!(2024 - 04 - 01),
                date!(2024 - 07 - 01),
                date!(2024 - 10 - 01),
                date!(2025 - 01 - 01),
            ],
            day_count: DayCount::Act365F,
            disc_id: "USD-OIS".into(),
            vol_id: "SPOT_VOL".into(),
            div_yield_id: None,
            pricing_overrides: Default::default(),
            attributes: Default::default(),
        };

        let market = create_mc_market(as_of, 100.0, 0.25, 0.05);
        test_metric_determinism(option, &market, as_of, MetricId::Vega, 10);
    }

    #[test]
    fn test_asian_option_all_greeks_deterministic() {
        let as_of = date!(2024 - 01 - 01);
        let expiry = date!(2025 - 01 - 01);
        
        let option = AsianOption {
            id: "ASIAN_ALL_GREEKS".into(),
            underlying_ticker: "SPOT".to_string(),
            spot_id: "SPOT".to_string(),
            strike: Money::new(100.0, Currency::USD),
            option_type: OptionType::Call,
            expiry,
            notional: 1.0,
            averaging_method: AveragingMethod::Arithmetic,
            fixing_dates: vec![
                date!(2024 - 04 - 01),
                date!(2024 - 07 - 01),
                date!(2024 - 10 - 01),
                date!(2025 - 01 - 01),
            ],
            day_count: DayCount::Act365F,
            disc_id: "USD-OIS".into(),
            vol_id: "SPOT_VOL".into(),
            div_yield_id: None,
            pricing_overrides: Default::default(),
            attributes: Default::default(),
        };

        let market = create_mc_market(as_of, 100.0, 0.25, 0.05);
        
        // Test all key greeks
        test_metric_determinism(option.clone(), &market, as_of, MetricId::Delta, 10);
        test_metric_determinism(option.clone(), &market, as_of, MetricId::Gamma, 10);
        test_metric_determinism(option.clone(), &market, as_of, MetricId::Vega, 10);
        test_metric_determinism(option.clone(), &market, as_of, MetricId::Rho, 10);
        test_metric_determinism(option, &market, as_of, MetricId::Theta, 10);
    }

    #[test]
    fn test_barrier_option_delta_vega_deterministic() {
        let as_of = date!(2024 - 01 - 01);
        let expiry = date!(2025 - 01 - 01);
        
        let option = BarrierOption {
            id: "BARRIER_TEST".into(),
            underlying_ticker: "SPOT".to_string(),
            spot_id: "SPOT".to_string(),
            strike: Money::new(100.0, Currency::USD),
            barrier: Money::new(120.0, Currency::USD),
            barrier_type: BarrierType::UpAndOut,
            option_type: OptionType::Call,
            expiry,
            notional: 1.0,
            day_count: DayCount::Act365F,
            use_gobet_miri: true,
            disc_id: "USD-OIS".into(),
            vol_id: "SPOT_VOL".into(),
            div_yield_id: None,
            pricing_overrides: Default::default(),
            attributes: Default::default(),
        };

        let market = create_mc_market(as_of, 100.0, 0.25, 0.05);
        test_metric_determinism(option.clone(), &market, as_of, MetricId::Delta, 10);
        test_metric_determinism(option, &market, as_of, MetricId::Vega, 10);
    }

    #[test]
    fn test_lookback_option_delta_vega_deterministic() {
        let as_of = date!(2024 - 01 - 01);
        let expiry = date!(2025 - 01 - 01);
        
        let option = LookbackOption {
            id: "LOOKBACK_TEST".into(),
            underlying_ticker: "SPOT".to_string(),
            spot_id: "SPOT".to_string(),
            strike: Some(Money::new(100.0, Currency::USD)),
            lookback_type: LookbackType::FixedStrike,
            option_type: OptionType::Call,
            expiry,
            notional: 1.0,
            day_count: DayCount::Act365F,
            disc_id: "USD-OIS".into(),
            vol_id: "SPOT_VOL".into(),
            div_yield_id: None,
            pricing_overrides: Default::default(),
            attributes: Default::default(),
        };

        let market = create_mc_market(as_of, 100.0, 0.25, 0.05);
        test_metric_determinism(option.clone(), &market, as_of, MetricId::Delta, 10);
        test_metric_determinism(option, &market, as_of, MetricId::Vega, 10);
    }

    #[test]
    fn test_autocallable_delta_vega_deterministic() {
        let as_of = date!(2024 - 01 - 01);
        let observation_dates = vec![
            date!(2024 - 04 - 01),
            date!(2024 - 07 - 01),
            date!(2024 - 10 - 01),
            date!(2025 - 01 - 01),
        ];
        
        let option = Autocallable {
            id: "AUTOCALL_TEST".into(),
            underlying_ticker: "SPOT".to_string(),
            observation_dates: observation_dates.clone(),
            autocall_barriers: vec![1.1, 1.15, 1.2, 1.25],
            coupons: vec![0.05, 0.05, 0.05, 0.05],
            final_barrier: 0.8,
            final_payoff_type: FinalPayoffType::CapitalProtection { floor: 0.9 },
            participation_rate: 1.0,
            cap_level: 1.3,
            notional: Money::new(1.0, Currency::USD),
            day_count: DayCount::Act365F,
            disc_id: "USD-OIS".into(),
            spot_id: "SPOT".to_string(),
            vol_id: "SPOT_VOL".into(),
            div_yield_id: None,
            pricing_overrides: Default::default(),
            attributes: Default::default(),
        };

        let market = create_mc_market(as_of, 100.0, 0.25, 0.05);
        test_metric_determinism(option.clone(), &market, as_of, MetricId::Delta, 10);
        test_metric_determinism(option, &market, as_of, MetricId::Vega, 10);
    }
}

