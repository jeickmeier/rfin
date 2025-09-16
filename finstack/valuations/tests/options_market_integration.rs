//! Integration tests for option pricing with live market data
//!
//! This test demonstrates that all option instruments can be priced and their
//! Greeks calculated using live market data from MarketContext.

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount, Frequency};
use finstack_core::market_data::context::MarketContext;
use finstack_core::math::interp::InterpStyle;
use finstack_core::market_data::scalars::MarketScalar;
use finstack_core::market_data::surfaces::vol_surface::VolSurface;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::term_structures::forward_curve::ForwardCurve;
use finstack_core::market_data::term_structures::hazard_curve::HazardCurve;
use finstack_core::money::{
    fx::{FxConversionPolicy, FxMatrix, FxProvider, FxRate},
    Money,
};
// use finstack_valuations::instruments::options::swaption::Swaption;
use finstack_valuations::instruments::options::swaption::builder::SwaptionBuilder;
use finstack_valuations::instruments::options::cap_floor::builder::IrOptionBuilder;
use finstack_valuations::instruments::common::MarketRefs;
use finstack_valuations::instruments::options::{
    CreditOption, EquityOption, FxOption,
};
use finstack_valuations::instruments::traits::{Instrument, Priceable};
use finstack_valuations::metrics::{MetricContext, MetricId, MetricRegistry};
use std::sync::Arc;
use time::Month;

fn create_test_market_context() -> MarketContext {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    // Create discount curves with extended maturity for swaption testing
    let usd_ois = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots(vec![
            (0.0, 1.0),
            (0.25, 0.9875),
            (0.5, 0.975),
            (1.0, 0.95),
            (2.0, 0.90),
            (5.0, 0.75),
            (10.0, 0.60),
        ])
        .set_interp(InterpStyle::MonotoneConvex)
        .build()
        .unwrap();

    let eur_ois = DiscountCurve::builder("EUR-OIS")
        .base_date(base_date)
        .knots(vec![
            (0.0, 1.0),
            (0.25, 0.99),
            (0.5, 0.98),
            (1.0, 0.96),
            (2.0, 0.92),
            (5.0, 0.80),
            (10.0, 0.65),
        ])
        .set_interp(InterpStyle::MonotoneConvex)
        .build()
        .unwrap();

    // Create forward curve with extended maturity
    let usd_sofr3m = ForwardCurve::builder("USD-SOFR-3M", 0.25)
        .base_date(base_date)
        .knots(vec![
            (0.0, 0.045),
            (1.0, 0.04),
            (2.0, 0.035),
            (5.0, 0.03),
            (10.0, 0.028),
        ])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();

    // Create hazard curve with extended maturity
    let abc_credit = HazardCurve::builder("ABC-SENIOR")
        .base_date(base_date)
        .recovery_rate(0.4)
        .knots(vec![
            (0.5, 0.03),
            (1.0, 0.035),
            (2.0, 0.04),
            (5.0, 0.05),
            (10.0, 0.055),
        ])
        .par_spreads(vec![
            (0.5, 150.0),
            (1.0, 180.0),
            (2.0, 200.0),
            (5.0, 250.0),
            (10.0, 280.0),
        ])
        .build()
        .unwrap();

    // Create volatility surfaces
    let aapl_vol = VolSurface::builder("AAPL-VOL")
        .expiries(&[0.25, 0.5, 1.0, 2.0])
        .strikes(&[80.0, 100.0, 120.0])
        .row(&[0.25, 0.20, 0.18]) // 3M
        .row(&[0.23, 0.18, 0.16]) // 6M
        .row(&[0.22, 0.17, 0.15]) // 1Y
        .row(&[0.21, 0.16, 0.14]) // 2Y
        .build()
        .unwrap();

    let eurusd_vol = VolSurface::builder("EURUSD-VOL")
        .expiries(&[0.25, 0.5, 1.0, 2.0])
        .strikes(&[1.0, 1.20, 1.40])
        .row(&[0.12, 0.10, 0.11]) // 3M
        .row(&[0.11, 0.09, 0.10]) // 6M
        .row(&[0.10, 0.08, 0.09]) // 1Y
        .row(&[0.09, 0.07, 0.08]) // 2Y
        .build()
        .unwrap();

    let cap_vol = VolSurface::builder("USD-CAP-VOL")
        .expiries(&[0.25, 0.5, 1.0, 2.0, 5.0])
        .strikes(&[0.02, 0.03, 0.04, 0.05])
        .row(&[0.30, 0.25, 0.22, 0.20]) // 3M
        .row(&[0.28, 0.23, 0.20, 0.18]) // 6M
        .row(&[0.25, 0.20, 0.18, 0.16]) // 1Y
        .row(&[0.22, 0.18, 0.16, 0.14]) // 2Y
        .row(&[0.20, 0.16, 0.14, 0.12]) // 5Y
        .build()
        .unwrap();

    let cds_vol = VolSurface::builder("ABC-CDS-VOL")
        .expiries(&[0.25, 0.5, 1.0, 2.0, 5.0, 10.0])
        .strikes(&[100.0, 200.0, 300.0])
        .row(&[0.40, 0.35, 0.32]) // 3M
        .row(&[0.38, 0.33, 0.30]) // 6M
        .row(&[0.35, 0.30, 0.28]) // 1Y
        .row(&[0.32, 0.28, 0.25]) // 2Y
        .row(&[0.30, 0.26, 0.23]) // 5Y
        .row(&[0.28, 0.24, 0.21]) // 10Y
        .build()
        .unwrap();

    // Create swaption volatility surface (reuse cap vol structure but with swaption strikes)
    let swaption_vol = VolSurface::builder("USD-SWAPTION-VOL")
        .expiries(&[0.25, 0.5, 1.0, 2.0, 5.0, 10.0])
        .strikes(&[0.02, 0.03, 0.035, 0.04, 0.05])
        .row(&[0.25, 0.20, 0.18, 0.17, 0.16]) // 3M
        .row(&[0.23, 0.18, 0.16, 0.15, 0.14]) // 6M
        .row(&[0.22, 0.17, 0.15, 0.14, 0.13]) // 1Y
        .row(&[0.21, 0.16, 0.14, 0.13, 0.12]) // 2Y
        .row(&[0.20, 0.15, 0.13, 0.12, 0.11]) // 5Y
        .row(&[0.19, 0.14, 0.12, 0.11, 0.10]) // 10Y
        .build()
        .unwrap();

    // Create simple FX provider for testing
    struct SimpleFxProvider;
    impl FxProvider for SimpleFxProvider {
        fn rate(
            &self,
            from: Currency,
            to: Currency,
            _on: Date,
            _policy: FxConversionPolicy,
        ) -> finstack_core::Result<FxRate> {
            if from == Currency::EUR && to == Currency::USD {
                return Ok(1.25);
            }
            if from == Currency::USD && to == Currency::EUR {
                return Ok(0.80);
            }
            Err(finstack_core::error::InputError::NotFound {
                id: "test_vol_surface".to_string(),
            }
            .into())
        }
    }

    let fx_matrix = FxMatrix::new(Arc::new(SimpleFxProvider));

    // Build market context
    MarketContext::new()
        .insert_discount(usd_ois)
        .insert_discount(eur_ois)
        .insert_forward(usd_sofr3m)
        .insert_hazard(abc_credit)
        .insert_surface(aapl_vol)
        .insert_surface(eurusd_vol)
        .insert_surface(cap_vol)
        .insert_surface(cds_vol)
        .insert_surface(swaption_vol)
        .insert_fx(fx_matrix)
        .insert_price("AAPL-SPOT", MarketScalar::Unitless(110.0))
        .insert_price("AAPL-DIV-YIELD", MarketScalar::Unitless(0.02))
}

#[test]
fn test_equity_option_full_integration() {
    let market = create_test_market_context();
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    // Create AAPL call option
    let option_params = finstack_valuations::instruments::common::EquityOptionParams::european_call(
        Money::new(100.0, Currency::USD),
        Date::from_calendar_date(2025, Month::December, 31).unwrap(),
        100.0, // 100 shares
    );
    let underlying_params = finstack_valuations::instruments::common::EquityUnderlyingParams::new(
        "AAPL",
        "AAPL-SPOT",
    );
    let market_refs = finstack_valuations::instruments::common::MarketRefs::option(
        finstack_core::types::CurveId::new("USD-OIS"),
        finstack_core::types::CurveId::new("AAPL-VOL"),
    );

    let option = EquityOption::new(
        "AAPL_CALL_100",
        &option_params,
        &underlying_params,
        &market_refs,
    );

    // Test pricing with market data
    let price = option.value(&market, as_of).unwrap();
    assert!(price.amount() > 0.0);
    assert_eq!(price.currency(), Currency::USD);

    // Test metrics calculation
    let mut registry = MetricRegistry::new();
    finstack_valuations::instruments::options::equity_option::metrics::register_equity_option_metrics(&mut registry);

    let instrument: Arc<dyn Instrument> = Arc::new(option);
    let mut context = MetricContext::new(instrument, Arc::new(market), as_of, price);

    let metrics = registry
        .compute(
            &[MetricId::Delta, MetricId::Gamma, MetricId::Vega],
            &mut context,
        )
        .unwrap();

    // Greeks should be non-zero for ITM call
    assert!(metrics.get(&MetricId::Delta).unwrap().abs() > 0.0);
    assert!(metrics.get(&MetricId::Gamma).unwrap().abs() > 0.0);
    assert!(metrics.get(&MetricId::Vega).unwrap().abs() > 0.0);
}

#[test]
fn test_fx_option_full_integration() {
    let market = create_test_market_context();
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    // Create EUR/USD call option
    let option_params = finstack_valuations::instruments::common::FxOptionParams::european_call(
        1.20,
        Date::from_calendar_date(2025, Month::June, 30).unwrap(),
        Money::new(1_000_000.0, Currency::EUR),
    );
    let underlying_params = finstack_valuations::instruments::common::FxUnderlyingParams::new(
        Currency::EUR,
        Currency::USD,
        "USD-OIS",
        "EUR-OIS",
    );

    let option = FxOption::new(
        "EURUSD_CALL_1.20",
        &option_params,
        &underlying_params,
        "EURUSD-VOL",
    );

    // Test pricing with market data
    let price = option.value(&market, as_of).unwrap();
    assert!(price.amount() > 0.0); // ITM call (spot 1.25 > strike 1.20)
    assert_eq!(price.currency(), Currency::USD);

    // Test metrics calculation
    let mut registry = MetricRegistry::new();
    finstack_valuations::instruments::options::fx_option::metrics::register_fx_option_metrics(
        &mut registry,
    );

    let instrument: Arc<dyn Instrument> = Arc::new(option);
    let mut context = MetricContext::new(instrument, Arc::new(market), as_of, price);

    let metrics = registry
        .compute(
            &[MetricId::Delta, MetricId::Gamma, MetricId::Vega],
            &mut context,
        )
        .unwrap();

    // Greeks should be non-zero for ITM call
    assert!(metrics.get(&MetricId::Delta).unwrap().abs() > 0.0);
    assert!(metrics.get(&MetricId::Gamma).unwrap().abs() > 0.0);
    assert!(metrics.get(&MetricId::Vega).unwrap().abs() > 0.0);
}

#[test]
fn test_interest_rate_option_full_integration() {
    let market = create_test_market_context();
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    // Create USD cap using MarketRefs-based builder
    let cap_mr = MarketRefs::rates("USD-OIS", "USD-SOFR-3M").with_volatility("USD-CAP-VOL");
    let cap = IrOptionBuilder::new()
        .id("USD_CAP_3%")
        .notional(Money::new(10_000_000.0, Currency::USD))
        .rate_option_type(finstack_valuations::instruments::options::cap_floor::RateOptionType::Cap)
        .strike_rate(0.03)
        .start_date(Date::from_calendar_date(2025, Month::March, 1).unwrap())
        .end_date(Date::from_calendar_date(2027, Month::March, 1).unwrap())
        .frequency(Frequency::quarterly())
        .day_count(DayCount::Act360)
        .market_refs(cap_mr)
        .build()
        .unwrap();

    // Test pricing with market data
    let price = cap.value(&market, as_of).unwrap();
    assert!(price.amount() > 0.0); // Cap should have positive value
    assert_eq!(price.currency(), Currency::USD);

    // Test metrics calculation
    let mut registry = MetricRegistry::new();
    finstack_valuations::instruments::options::cap_floor::metrics::register_interest_rate_option_metrics(&mut registry);

    let instrument: Arc<dyn Instrument> = Arc::new(cap);
    let mut context = MetricContext::new(instrument, Arc::new(market), as_of, price);

    let metrics = registry
        .compute(&[MetricId::Delta, MetricId::Vega], &mut context)
        .unwrap();

    // Greeks should be computed
    assert!(metrics.contains_key(&MetricId::Delta));
    assert!(metrics.contains_key(&MetricId::Vega));
}

#[test]
fn test_credit_option_full_integration() {
    let market = create_test_market_context();
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    // Create credit option
    let option_params = finstack_valuations::instruments::common::CreditOptionParams::call(
        200.0, // 200bp strike
        Date::from_calendar_date(2025, Month::June, 30).unwrap(),
        Date::from_calendar_date(2030, Month::June, 30).unwrap(),
        Money::new(10_000_000.0, Currency::USD),
    );
    let credit_params = finstack_valuations::instruments::common::CreditParams::new(
        "ABC Corp",
        0.4, // 40% recovery
        "ABC-SENIOR",
    );
    let market_refs = finstack_valuations::instruments::common::MarketRefs::discount_only(
        finstack_core::types::CurveId::new("USD-OIS"),
    ).with_credit(finstack_core::types::CurveId::new("ABC-SENIOR"))
        .with_volatility(finstack_core::types::CurveId::new("ABC-CDS-VOL"));

    let option = CreditOption::new(
        "ABC_CDS_CALL_200",
        &option_params,
        &credit_params,
        &market_refs,
    );

    // Test pricing with market data
    let price = option.value(&market, as_of).unwrap();
    assert!(price.amount() >= 0.0); // Should be non-negative
    assert_eq!(price.currency(), Currency::USD);

    // Test metrics calculation
    let mut registry = MetricRegistry::new();
    finstack_valuations::instruments::options::credit_option::metrics::register_credit_option_metrics(&mut registry);

    let instrument: Arc<dyn Instrument> = Arc::new(option);
    let mut context = MetricContext::new(instrument, Arc::new(market), as_of, price);

    let metrics = registry
        .compute(&[MetricId::Delta, MetricId::Gamma], &mut context)
        .unwrap();

    // Greeks should be computed
    assert!(metrics.contains_key(&MetricId::Delta));
    assert!(metrics.contains_key(&MetricId::Gamma));
}

#[test]
fn test_swaption_full_integration() {
    let market = create_test_market_context();
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    // Create 1Y5Y payer swaption
    let expiry = Date::from_calendar_date(2026, Month::January, 1).unwrap();
    let swap_start = expiry;
    let swap_end = Date::from_calendar_date(2031, Month::January, 1).unwrap();

    let sw_mr = MarketRefs::rates("USD-OIS", "USD-SOFR-3M").with_volatility("USD-SWAPTION-VOL");
    let swaption = SwaptionBuilder::new()
        .id("1Y5Y_PAYER")
        .payer()
        .notional(Money::new(10_000_000.0, Currency::USD))
        .strike_rate(0.035)
        .expiry(expiry)
        .swap_dates(swap_start, swap_end)
        .market_refs(sw_mr)
        .build()
        .unwrap();

    // Test pricing with market data
    let price = swaption.value(&market, as_of).unwrap();
    assert!(price.amount() > 0.0); // Should have positive value
    assert_eq!(price.currency(), Currency::USD);

    // Test metrics calculation
    let mut registry = MetricRegistry::new();
    finstack_valuations::instruments::options::swaption::metrics::register_swaption_metrics(
        &mut registry,
    );

    let instrument: Arc<dyn Instrument> = Arc::new(swaption);
    let mut context = MetricContext::new(instrument, Arc::new(market), as_of, price);

    let metrics = registry
        .compute(&[MetricId::Delta, MetricId::Vega], &mut context)
        .unwrap();

    // Greeks should be computed
    assert!(metrics.contains_key(&MetricId::Delta));
    assert!(metrics.contains_key(&MetricId::Vega));
}

#[test]
fn test_options_pricing_consistency() {
    let market = create_test_market_context();
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    // Test put-call parity for equity options
    let strike = Money::new(100.0, Currency::USD);
    let expiry = Date::from_calendar_date(2025, Month::June, 30).unwrap();

    let call_params = finstack_valuations::instruments::common::EquityOptionParams::european_call(
        strike,
        expiry,
        1.0, // 1 share
    );
    let put_params = finstack_valuations::instruments::common::EquityOptionParams::european_put(
        strike,
        expiry,
        1.0, // 1 share
    );
    let underlying_params = finstack_valuations::instruments::common::EquityUnderlyingParams::new(
        "AAPL",
        "AAPL-SPOT",
    );
    let market_refs = finstack_valuations::instruments::common::MarketRefs::option(
        finstack_core::types::CurveId::new("USD-OIS"),
        finstack_core::types::CurveId::new("AAPL-VOL"),
    );

    let call = EquityOption::new(
        "AAPL_CALL_100",
        &call_params,
        &underlying_params,
        &market_refs,
    );

    let put = EquityOption::new(
        "AAPL_PUT_100",
        &put_params,
        &underlying_params,
        &market_refs,
    );

    let call_price = call.value(&market, as_of).unwrap();
    let put_price = put.value(&market, as_of).unwrap();

    // Both should have positive prices
    assert!(call_price.amount() > 0.0);
    assert!(put_price.amount() > 0.0);

    // For ITM call (spot 110 > strike 100), call should be worth more than put
    assert!(call_price.amount() > put_price.amount());
}

#[test]
fn test_market_data_override_behavior() {
    let market = create_test_market_context();
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    // Create option with implied vol override
    let option_params = finstack_valuations::instruments::common::EquityOptionParams::european_call(
        Money::new(100.0, Currency::USD),
        Date::from_calendar_date(2025, Month::June, 30).unwrap(),
        1.0,
    );
    let underlying_params = finstack_valuations::instruments::common::EquityUnderlyingParams::new(
        "AAPL",
        "AAPL-SPOT",
    );
    let market_refs = finstack_valuations::instruments::common::MarketRefs::option(
        finstack_core::types::CurveId::new("USD-OIS"),
        finstack_core::types::CurveId::new("AAPL-VOL"),
    );
    
    let mut option = EquityOption::new(
        "AAPL_CALL_100_IMPLIED",
        &option_params,
        &underlying_params,
        &market_refs,
    );

    // Set implied vol to override surface
    option.pricing_overrides.implied_volatility = Some(0.30); // 30% vs surface ~20%

    let price_with_implied = option.value(&market, as_of).unwrap();

    // Remove implied vol to use surface
    option.pricing_overrides.implied_volatility = None;
    let price_with_surface = option.value(&market, as_of).unwrap();

    // Prices should be different due to different volatilities
    assert!((price_with_implied.amount() - price_with_surface.amount()).abs() > 0.01);
}
