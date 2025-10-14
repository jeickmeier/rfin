use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::scalars::MarketScalar;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::money::Money;
use finstack_valuations::instruments::basket::pricer::BasketCalculator;
use finstack_valuations::instruments::basket::types::{AssetType, Basket, BasketConstituent, BasketPricingConfig, ConstituentReference};
use time::Month;

fn d(y: i32, m: u8, day: u8) -> Date {
    Date::from_calendar_date(y, Month::try_from(m).unwrap(), day).unwrap()
}

fn ctx() -> MarketContext {
    let disc = DiscountCurve::builder("USD-OIS")
        .base_date(d(2025, 1, 2))
        .knots([(0.0, 1.0), (1.0, 0.95)])
        .build()
        .unwrap();
    MarketContext::new()
        .insert_discount(disc)
        .insert_price("AAPL-P", MarketScalar::Unitless(200.0))
        .insert_price("MSFT-P", MarketScalar::Unitless(300.0))
}

fn basket_units() -> Basket {
    Basket {
        id: "BASKET-UNITS".into(),
        constituents: vec![
            BasketConstituent {
                id: "AAPL".into(),
                reference: ConstituentReference::MarketData { price_id: "AAPL-P".into(), asset_type: AssetType::Equity },
                weight: 0.0,
                units: Some(10.0),
                ticker: None,
            },
            BasketConstituent {
                id: "MSFT".into(),
                reference: ConstituentReference::MarketData { price_id: "MSFT-P".into(), asset_type: AssetType::Equity },
                weight: 0.0,
                units: Some(5.0),
                ticker: None,
            },
        ],
        expense_ratio: 0.0,
        currency: Currency::USD,
        discount_curve_id: "USD-OIS".into(),
        attributes: finstack_valuations::instruments::common::traits::Attributes::new(),
        pricing_config: BasketPricingConfig::default(),
    }
}

fn basket_weights() -> Basket {
    Basket {
        id: "BASKET-WEIGHTS".into(),
        constituents: vec![
            BasketConstituent {
                id: "AAPL".into(),
                reference: ConstituentReference::MarketData { price_id: "AAPL-P".into(), asset_type: AssetType::Equity },
                weight: 0.6,
                units: None,
                ticker: None,
            },
            BasketConstituent {
                id: "MSFT".into(),
                reference: ConstituentReference::MarketData { price_id: "MSFT-P".into(), asset_type: AssetType::Equity },
                weight: 0.4,
                units: None,
                ticker: None,
            },
        ],
        expense_ratio: 0.36525, // ~0.1% daily with default days_in_year
        currency: Currency::USD,
        discount_curve_id: "USD-OIS".into(),
        attributes: finstack_valuations::instruments::common::traits::Attributes::new(),
        pricing_config: BasketPricingConfig::default(),
    }
}

#[test]
fn calculator_nav_and_basket_value_units_mode() {
    let b = basket_units();
    let c = BasketCalculator::with_defaults();
    let market = ctx();
    let as_of = d(2025, 1, 2);

    let nav = c.nav(&b, &market, as_of, 100.0).unwrap();
    let val = c.basket_value(&b, &market, as_of, Some(100.0)).unwrap();

    // 10*200 + 5*300 = 2000 + 1500 = 3500 total; per share = 35.0
    assert_eq!(val.amount(), 3500.0);
    assert_eq!(nav.amount(), 35.0);
}

#[test]
fn calculator_nav_with_aum_weight_mode() {
    let b = basket_weights();
    let c = BasketCalculator::with_defaults();
    let market = ctx();
    let as_of = d(2025, 1, 2);

    let aum = Money::new(1_000_000.0, Currency::USD);
    let nav = c.nav_with_aum(&b, &market, as_of, aum, 100_000.0).unwrap();
    let val = c.basket_value_with_aum(&b, &market, as_of, aum).unwrap();

    // All weight-based: total should equal AUM less one day of expense drag
    let daily_drag = 1_000_000.0 * (b.expense_ratio / b.pricing_config.days_in_year);
    assert!((val.amount() - (1_000_000.0 - daily_drag)).abs() < 1e-8);
    assert!((nav.amount() - ((1_000_000.0 - daily_drag) / 100_000.0)).abs() < 1e-8);
}


