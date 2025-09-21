use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::scalars::MarketScalar;
use finstack_core::money::fx::{FxConversionPolicy, FxMatrix, FxProvider};
use finstack_core::money::Money;
use finstack_core::F;
use finstack_valuations::instruments::basket::metrics::register_basket_metrics;
use finstack_valuations::instruments::basket::types::{AssetType, Basket, BasketConstituent, ConstituentReference, ReplicationMethod};
use finstack_valuations::metrics::{traits::MetricCalculator, MetricContext, MetricRegistry};
use std::sync::Arc;
use time::Month;

fn d() -> Date { Date::from_calendar_date(2025, Month::January, 1).unwrap() }

fn usd(v: F) -> Money { Money::new(v, Currency::USD) }
fn eur(v: F) -> Money { Money::new(v, Currency::EUR) }

fn empty_context() -> MarketContext { MarketContext::new() }

fn simple_basket_base() -> Basket {
    Basket {
        id: finstack_core::types::InstrumentId::new("TEST_BASKET"),
        ticker: Some("TEST".to_string()),
        name: "Test Basket".to_string(),
        constituents: vec![],
        expense_ratio: 0.0,
        rebalance_freq: finstack_core::dates::Frequency::quarterly(),
        tracking_index: None,
        creation_unit_size: 50000.0,
        currency: Currency::USD,
        shares_outstanding: Some(100.0),
        replication: ReplicationMethod::Physical,
        attributes: finstack_valuations::instruments::common::traits::Attributes::new(),
    }
}

#[test]
fn basket_units_nav_and_value() {
    let mut basket = simple_basket_base();
    basket.expense_ratio = 0.0;
    basket.constituents = vec![
        BasketConstituent {
            id: "C1".into(),
            reference: ConstituentReference::MarketData { price_id: "C1P".into(), asset_type: AssetType::Equity },
            weight: 0.0,
            units: Some(10.0),
            ticker: None,
        },
        BasketConstituent {
            id: "C2".into(),
            reference: ConstituentReference::MarketData { price_id: "C2P".into(), asset_type: AssetType::Equity },
            weight: 0.0,
            units: Some(20.0),
            ticker: None,
        },
    ];

    let ctx = empty_context()
        .insert_price("C1P", MarketScalar::Unitless(100.0))
        .insert_price("C2P", MarketScalar::Unitless(50.0));

    let nav = basket.nav(&ctx, d()).unwrap();
    let val = basket.basket_value(&ctx, d()).unwrap();
    assert_eq!(val.amount(), 2000.0);
    assert_eq!(val.currency(), Currency::USD);
    assert_eq!(nav.amount(), 20.0); // 2000 / 100 shares
}

#[test]
fn basket_expense_ratio_reduces_value() {
    let mut basket = simple_basket_base();
    basket.expense_ratio = 0.36525; // ~0.1% daily with 365.25 default
    basket.constituents = vec![
        BasketConstituent {
            id: "C1".into(),
            reference: ConstituentReference::MarketData { price_id: "C1P".into(), asset_type: AssetType::Equity },
            weight: 0.0,
            units: Some(10.0),
            ticker: None,
        },
        BasketConstituent {
            id: "C2".into(),
            reference: ConstituentReference::MarketData { price_id: "C2P".into(), asset_type: AssetType::Equity },
            weight: 0.0,
            units: Some(20.0),
            ticker: None,
        },
    ];

    let ctx = empty_context()
        .insert_price("C1P", MarketScalar::Unitless(100.0))
        .insert_price("C2P", MarketScalar::Unitless(50.0));

    let val = basket.basket_value(&ctx, d()).unwrap();
    assert!((val.amount() - 1998.0).abs() < 1e-9); // 2000 - 0.1% of 2000 = 1998
    let nav = basket.nav(&ctx, d()).unwrap();
    assert!((nav.amount() - 19.98).abs() < 1e-9);
}

#[test]
fn basket_weight_requires_aum_or_units() {
    let mut basket = simple_basket_base();
    basket.expense_ratio = 0.0;
    basket.constituents = vec![
        BasketConstituent {
            id: "EQ1".into(),
            reference: ConstituentReference::MarketData { price_id: "EQ1P".into(), asset_type: AssetType::Equity },
            weight: 0.6,
            units: None,
            ticker: None,
        },
        BasketConstituent {
            id: "EQ2".into(),
            reference: ConstituentReference::MarketData { price_id: "EQ2P".into(), asset_type: AssetType::Equity },
            weight: 0.4,
            units: None,
            ticker: None,
        },
    ];

    let ctx = empty_context()
        .insert_price("EQ1P", MarketScalar::Unitless(100.0))
        .insert_price("EQ2P", MarketScalar::Unitless(200.0));

    // Remove shares to force error in weight-only without AUM/units
    basket.shares_outstanding = None;
    assert!(basket.basket_value(&ctx, d()).is_err());
    // Per-share NAV from weights is computable without shares
    let nav_ps = basket.nav(&ctx, d()).unwrap();
    assert!((nav_ps.amount() - (0.6 * 100.0 + 0.4 * 200.0)).abs() < 1e-12);

    // With AUM provided
    let aum = usd(1_000_000.0);
    let val = basket.basket_value_with_aum(&ctx, d(), aum).unwrap();
    assert_eq!(val.amount(), 1_000_000.0);

    // With shares, NAV per share equals AUM / shares
    basket.shares_outstanding = Some(50_000.0);
    let nav = basket.nav_with_aum(&ctx, d(), aum).unwrap();
    assert!((nav.amount() - 20.0).abs() < 1e-12);
}

// Simple static FX provider (EUR→USD = 1.2, USD→EUR = 0.8333...)
struct StaticFx;
impl FxProvider for StaticFx {
    fn rate(
        &self,
        from: Currency,
        to: Currency,
        _on: Date,
        _policy: FxConversionPolicy,
    ) -> finstack_core::Result<F> {
        match (from, to) {
            (Currency::EUR, Currency::USD) => Ok(1.2),
            (Currency::USD, Currency::EUR) => Ok(1.0 / 1.2),
            _ => Ok(1.0),
        }
    }
}

#[test]
fn basket_fx_conversion_applied_for_units() {
    // One EUR-priced constituent with units, one USD-priced
    let mut basket = simple_basket_base();
    basket.expense_ratio = 0.0;
    basket.constituents = vec![
        BasketConstituent {
            id: "EURC".into(),
            reference: ConstituentReference::MarketData { price_id: "EURP".into(), asset_type: AssetType::Equity },
            weight: 0.0,
            units: Some(10.0),
            ticker: None,
        },
        BasketConstituent {
            id: "USDC".into(),
            reference: ConstituentReference::MarketData { price_id: "USDP".into(), asset_type: AssetType::Equity },
            weight: 0.0,
            units: Some(1.0),
            ticker: None,
        },
    ];

    let fx = FxMatrix::new(Arc::new(StaticFx));
    let ctx = empty_context()
        .insert_fx(fx)
        .insert_price("EURP", MarketScalar::Price(eur(100.0)))
        .insert_price("USDP", MarketScalar::Price(usd(100.0)));

    // EUR leg: 100 EUR × 10 × 1.2 = 1200 USD; USD leg: 100 USD
    let val = basket.basket_value(&ctx, d()).unwrap();
    assert!((val.amount() - 1300.0).abs() < 1e-9);
}

#[test]
fn basket_metrics_register_and_compute() {
    // Units-only basket for deterministic NAV
    let mut basket = simple_basket_base();
    basket.expense_ratio = 0.0;
    basket.constituents = vec![
        BasketConstituent {
            id: "A".into(),
            reference: ConstituentReference::MarketData { price_id: "PA".into(), asset_type: AssetType::Equity },
            weight: 0.3,
            units: Some(5.0),
            ticker: None,
        },
        BasketConstituent {
            id: "B".into(),
            reference: ConstituentReference::MarketData { price_id: "PB".into(), asset_type: AssetType::Bond },
            weight: 0.7,
            units: Some(10.0),
            ticker: None,
        },
    ];

    let ctx = empty_context()
        .insert_price("PA", MarketScalar::Unitless(10.0))
        .insert_price("PB", MarketScalar::Unitless(20.0))
        .insert_price("TEST", MarketScalar::Unitless(22.0)); // market price for premium/discount

    let mut registry = MetricRegistry::new();
    register_basket_metrics(&mut registry);

    let inst: Arc<dyn finstack_valuations::instruments::common::traits::Instrument> = Arc::new(basket.clone());
    let mut mctx = MetricContext::new(inst, Arc::new(ctx), d(), usd(0.0));

    // NAV: (5*10 + 10*20) / 100 shares = (50 + 200)/100 = 2.5
    let nav_calc = finstack_valuations::instruments::basket::metrics::NavCalculator;
    let nav = nav_calc.calculate(&mut mctx).unwrap();
    assert!((nav - 2.5).abs() < 1e-12);

    // Basket value: 250
    let val_calc = finstack_valuations::instruments::basket::metrics::BasketValueCalculator;
    let val = val_calc.calculate(&mut mctx).unwrap();
    assert!((val - 250.0).abs() < 1e-12);

    // Count
    let cnt_calc = finstack_valuations::instruments::basket::metrics::ConstituentCountCalculator;
    assert_eq!(cnt_calc.calculate(&mut mctx).unwrap(), 2.0);

    // Expense ratio metric: 0.0 -> 0.0
    let er_calc = finstack_valuations::instruments::basket::metrics::ExpenseRatioCalculator;
    assert_eq!(er_calc.calculate(&mut mctx).unwrap(), 0.0);

    // Premium/discount: market 22 vs NAV 2.5 -> (22/2.5 - 1)*100 = 780%
    let pd_calc = finstack_valuations::instruments::basket::metrics::PremiumDiscountCalculator;
    let pd = pd_calc.calculate(&mut mctx).unwrap();
    assert!((pd - 780.0).abs() < 1e-9);

    // Tracking error placeholder returns 0.0
    let te_calc = finstack_valuations::instruments::basket::metrics::TrackingErrorCalculator;
    assert_eq!(te_calc.calculate(&mut mctx).unwrap(), 0.0);
}


