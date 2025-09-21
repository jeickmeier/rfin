//! Integration tests for basket/ETF instruments.

use finstack_core::dates::Frequency;
use finstack_core::market_data::scalars::MarketScalar;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::MarketContext;
use finstack_core::prelude::*;
use finstack_valuations::instruments::basket::{
    AssetType, Basket, BasketConstituent, ConstituentReference, ReplicationMethod,
};
use finstack_valuations::instruments::bond::Bond;
use finstack_valuations::instruments::common::traits::Priceable;
use finstack_valuations::metrics::MetricId;
use time::Month;

fn test_date(year: i32, month: u8, day: u8) -> Date {
    Date::from_calendar_date(year, Month::try_from(month).unwrap(), day).unwrap()
}

fn create_test_market_context() -> MarketContext {
    let base_date = test_date(2025, 1, 1);

    // Create discount curve
    let discount_curve = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots([(0.0, 1.0), (1.0, 0.95), (5.0, 0.80)])
        .build()
        .unwrap();

    // Create market context with prices
    MarketContext::new()
        .insert_discount(discount_curve)
        .insert_price("AAPL", MarketScalar::Unitless(150.0))
        .insert_price("MSFT", MarketScalar::Unitless(300.0))
        .insert_price("GOOGL", MarketScalar::Unitless(2800.0))
        .insert_price("AMZN", MarketScalar::Unitless(3200.0))
        .insert_price("BOND_AAPL", MarketScalar::Unitless(98.5))
        .insert_price("BOND_MSFT", MarketScalar::Unitless(101.2))
        .insert_price("USD_CASH", MarketScalar::Unitless(1.0))
}

#[test]
fn test_equity_etf_creation_and_pricing() {
    let context = create_test_market_context();
    let base_date = test_date(2025, 1, 1);

    // Create equity ETF similar to SPY using market data references
    let spy_constituents = vec![
        BasketConstituent {
            id: "AAPL".to_string(),
            reference: ConstituentReference::MarketData {
                price_id: "AAPL".to_string(),
                asset_type: AssetType::Equity,
            },
            weight: 0.30,
            units: None,
            ticker: Some("AAPL".to_string()),
        },
        BasketConstituent {
            id: "MSFT".to_string(),
            reference: ConstituentReference::MarketData {
                price_id: "MSFT".to_string(),
                asset_type: AssetType::Equity,
            },
            weight: 0.25,
            units: None,
            ticker: Some("MSFT".to_string()),
        },
        BasketConstituent {
            id: "GOOGL".to_string(),
            reference: ConstituentReference::MarketData {
                price_id: "GOOGL".to_string(),
                asset_type: AssetType::Equity,
            },
            weight: 0.20,
            units: None,
            ticker: Some("GOOGL".to_string()),
        },
        BasketConstituent {
            id: "AMZN".to_string(),
            reference: ConstituentReference::MarketData {
                price_id: "AMZN".to_string(),
                asset_type: AssetType::Equity,
            },
            weight: 0.15,
            units: None,
            ticker: Some("AMZN".to_string()),
        },
        BasketConstituent {
            id: "USD_CASH".to_string(),
            reference: ConstituentReference::MarketData {
                price_id: "USD_CASH".to_string(),
                asset_type: AssetType::Cash,
            },
            weight: 0.10,
            units: None,
            ticker: Some("USD".to_string()),
        },
    ];

    let spy = Basket::builder()
        .id("SPY".into())
        .ticker("SPY".to_string())
        .name("SPDR S&P 500 ETF Trust".to_string())
        .constituents(spy_constituents)
        .expense_ratio(0.0009)
        .rebalance_freq(Frequency::quarterly())
        .creation_unit_size(50_000.0)
        .currency(Currency::USD)
        .shares_outstanding(900_000_000.0)
        .replication(ReplicationMethod::Physical)
        .build()
        .unwrap();

    // Verify basic properties
    assert_eq!(spy.id.as_str(), "SPY");
    assert_eq!(spy.ticker, Some("SPY".to_string()));
    assert_eq!(spy.constituents.len(), 5);
    assert_eq!(spy.expense_ratio, 0.0009); // 9 bps for equity ETF

    // Test pricing
    let nav = spy.value(&context, base_date).unwrap();
    assert!(nav.amount() > 0.0);
    assert_eq!(nav.currency(), Currency::USD);

    // Test validation
    assert!(spy.validate().is_ok());

    println!(
        "SPY ETF created successfully with NAV: ${:.2}",
        nav.amount()
    );
}

#[test]
fn test_bond_etf_creation_and_pricing() {
    let context = create_test_market_context();
    let base_date = test_date(2025, 1, 1);
    let maturity = test_date(2030, 1, 1);

    // Create sample bonds using the proper builder pattern
    let _aapl_bond = Bond::fixed_semiannual(
        "AAPL_4.65_2030",
        Money::new(1000.0, Currency::USD),
        0.0465,
        base_date,
        maturity,
        "USD-OIS",
    );

    let _msft_bond = Bond::fixed_semiannual(
        "MSFT_3.50_2030",
        Money::new(1000.0, Currency::USD),
        0.035,
        base_date,
        maturity,
        "USD-OIS",
    );

    // Create bond ETF similar to LQD
    let lqd_constituents = vec![
        BasketConstituent {
            id: "BOND_AAPL".to_string(),
            reference: ConstituentReference::MarketData {
                price_id: "BOND_AAPL".to_string(),
                asset_type: AssetType::Bond,
            },
            weight: 0.45,
            units: None,
            ticker: Some("AAPL_BOND".to_string()),
        },
        BasketConstituent {
            id: "BOND_MSFT".to_string(),
            reference: ConstituentReference::MarketData {
                price_id: "BOND_MSFT".to_string(),
                asset_type: AssetType::Bond,
            },
            weight: 0.45,
            units: None,
            ticker: Some("MSFT_BOND".to_string()),
        },
        BasketConstituent {
            id: "USD_CASH".to_string(),
            reference: ConstituentReference::MarketData {
                price_id: "USD_CASH".to_string(),
                asset_type: AssetType::Cash,
            },
            weight: 0.10,
            units: None,
            ticker: Some("USD".to_string()),
        },
    ];

    let lqd = Basket::builder()
        .id("LQD".into())
        .ticker("LQD".to_string())
        .name("iShares iBoxx $ IG Corporate Bond ETF".to_string())
        .constituents(lqd_constituents)
        .expense_ratio(0.0014)
        .rebalance_freq(Frequency::quarterly())
        .creation_unit_size(50_000.0)
        .currency(Currency::USD)
        .shares_outstanding(200_000_000.0)
        .replication(ReplicationMethod::Physical)
        .build()
        .unwrap();

    // Verify basic properties
    assert_eq!(lqd.id.as_str(), "LQD");
    assert_eq!(lqd.constituents.len(), 3);
    assert_eq!(lqd.expense_ratio, 0.0014); // 14 bps for bond ETF

    // Test pricing
    let nav = lqd.value(&context, base_date).unwrap();
    assert!(nav.amount() > 0.0);
    assert_eq!(nav.currency(), Currency::USD);

    // Test validation
    assert!(lqd.validate().is_ok());

    println!(
        "LQD Bond ETF created successfully with NAV: ${:.2}",
        nav.amount()
    );
}

#[test]
fn test_mixed_asset_basket() {
    let context = create_test_market_context();
    let base_date = test_date(2025, 1, 1);
    let maturity = test_date(2030, 1, 1);

    // Create a bond for the mixed basket using fixed_semiannual with available curve
    let _bond = Bond::fixed_semiannual(
        "TREASURY_2030",
        Money::new(1000.0, Currency::USD),
        0.025,
        base_date,
        maturity,
        "USD-OIS", // Use the available discount curve from our context
    );

    // Create mixed asset basket
    let balanced_constituents = vec![
        BasketConstituent {
            id: "AAPL".to_string(),
            reference: ConstituentReference::MarketData {
                price_id: "AAPL".to_string(),
                asset_type: AssetType::Equity,
            },
            weight: 0.40,
            units: None,
            ticker: Some("AAPL".to_string()),
        },
        BasketConstituent {
            id: "MSFT".to_string(),
            reference: ConstituentReference::MarketData {
                price_id: "MSFT".to_string(),
                asset_type: AssetType::Equity,
            },
            weight: 0.30,
            units: None,
            ticker: Some("MSFT".to_string()),
        },
        BasketConstituent {
            id: "BOND_AAPL".to_string(),
            reference: ConstituentReference::MarketData {
                price_id: "BOND_AAPL".to_string(),
                asset_type: AssetType::Bond,
            },
            weight: 0.20,
            units: None,
            ticker: Some("AAPL_BOND".to_string()),
        },
        BasketConstituent {
            id: "USD_CASH".to_string(),
            reference: ConstituentReference::MarketData {
                price_id: "USD_CASH".to_string(),
                asset_type: AssetType::Cash,
            },
            weight: 0.10,
            units: None,
            ticker: Some("USD".to_string()),
        },
    ];

    let balanced = Basket::builder()
        .id("BALANCED".into())
        .name("Balanced ETF".to_string())
        .constituents(balanced_constituents)
        .expense_ratio(0.0025)
        .rebalance_freq(Frequency::quarterly())
        .creation_unit_size(50_000.0)
        .currency(Currency::USD)
        .shares_outstanding(50_000_000.0)
        .replication(ReplicationMethod::Physical)
        .build()
        .unwrap();

    // Test mixed asset pricing
    let nav = balanced.value(&context, base_date).unwrap();
    assert!(nav.amount() > 0.0);

    // Verify constituent count and allocation
    assert_eq!(balanced.constituents.len(), 4);
    let total_weight: f64 = balanced.constituents.iter().map(|c| c.weight).sum();
    assert!((total_weight - 1.0).abs() < 0.001);

    println!(
        "Balanced ETF created successfully with NAV: ${:.2}",
        nav.amount()
    );
}

#[test]
fn test_basket_metrics_integration() {
    let context = create_test_market_context();
    let base_date = test_date(2025, 1, 1);

    // Create simple equity basket
    let basket_consts = vec![
        BasketConstituent {
            id: "AAPL".to_string(),
            reference: ConstituentReference::MarketData {
                price_id: "AAPL".to_string(),
                asset_type: AssetType::Equity,
            },
            weight: 0.6,
            units: None,
            ticker: Some("AAPL".to_string()),
        },
        BasketConstituent {
            id: "MSFT".to_string(),
            reference: ConstituentReference::MarketData {
                price_id: "MSFT".to_string(),
                asset_type: AssetType::Equity,
            },
            weight: 0.4,
            units: None,
            ticker: Some("MSFT".to_string()),
        },
    ];

    let basket = Basket::builder()
        .id("TEST_BASKET".into())
        .name("Test Basket".to_string())
        .constituents(basket_consts)
        .expense_ratio(0.001)
        .rebalance_freq(Frequency::quarterly())
        .creation_unit_size(50_000.0)
        .currency(Currency::USD)
        .shares_outstanding(1_000_000.0)
        .replication(ReplicationMethod::Physical)
        .build()
        .unwrap();

    // Test metrics calculation
    let metrics = vec![
        MetricId::Nav,
        MetricId::BasketValue,
        MetricId::ConstituentCount,
        MetricId::ExpenseRatio,
    ];

    let result = basket
        .price_with_metrics(&context, base_date, &metrics)
        .unwrap();

    // Verify metrics were calculated
    assert!(result.measures.contains_key("nav"));
    assert!(result.measures.contains_key("basket_value"));
    assert!(result.measures.contains_key("constituent_count"));
    assert!(result.measures.contains_key("expense_ratio"));

    // Verify constituent count
    assert_eq!(result.measures["constituent_count"], 2.0);

    println!("Basket metrics calculated successfully:");
    for (metric, value) in &result.measures {
        println!("  {}: {:.4}", metric, value);
    }
}

#[test]
fn test_basket_weight_validation() {
    // Invalid: weights don't sum to 1.0
    let invalid_consts = vec![
        BasketConstituent {
            id: "AAPL".to_string(),
            reference: ConstituentReference::MarketData {
                price_id: "AAPL".to_string(),
                asset_type: AssetType::Equity,
            },
            weight: 0.5,
            units: None,
            ticker: Some("AAPL".to_string()),
        },
        BasketConstituent {
            id: "MSFT".to_string(),
            reference: ConstituentReference::MarketData {
                price_id: "MSFT".to_string(),
                asset_type: AssetType::Equity,
            },
            weight: 0.4,
            units: None,
            ticker: Some("MSFT".to_string()),
        },
    ];
    let result = Basket::builder()
        .id("INVALID_BASKET".into())
        .name("Invalid".to_string())
        .constituents(invalid_consts)
        .expense_ratio(0.001)
        .rebalance_freq(Frequency::quarterly())
        .creation_unit_size(50_000.0)
        .currency(Currency::USD)
        .replication(ReplicationMethod::Physical)
        .build();
    // Build may succeed; validate should fail
    let basket = result.unwrap();
    assert!(basket.validate().is_err());

    // Valid: weights sum to 1.0
    let valid_consts = vec![
        BasketConstituent {
            id: "AAPL".to_string(),
            reference: ConstituentReference::MarketData {
                price_id: "AAPL".to_string(),
                asset_type: AssetType::Equity,
            },
            weight: 0.6,
            units: None,
            ticker: Some("AAPL".to_string()),
        },
        BasketConstituent {
            id: "MSFT".to_string(),
            reference: ConstituentReference::MarketData {
                price_id: "MSFT".to_string(),
                asset_type: AssetType::Equity,
            },
            weight: 0.4,
            units: None,
            ticker: Some("MSFT".to_string()),
        },
    ];
    let result = Basket::builder()
        .id("VALID_BASKET".into())
        .name("Valid".to_string())
        .constituents(valid_consts)
        .expense_ratio(0.001)
        .rebalance_freq(Frequency::quarterly())
        .creation_unit_size(50_000.0)
        .currency(Currency::USD)
        .replication(ReplicationMethod::Physical)
        .build();
    let basket = result.unwrap();
    assert!(basket.validate().is_ok());
}

#[test]
fn test_basket_currency_consistency() {
    let context = create_test_market_context();
    let base_date = test_date(2025, 1, 1);

    // Create basket with USD currency
    let basket = Basket::builder()
        .id("USD_BASKET".into())
        .name("USD Basket".to_string())
        .constituents(vec![
            BasketConstituent {
                id: "AAPL".to_string(),
                reference: ConstituentReference::MarketData {
                    price_id: "AAPL".to_string(),
                    asset_type: AssetType::Equity,
                },
                weight: 0.5,
                units: None,
                ticker: Some("AAPL".to_string()),
            },
            BasketConstituent {
                id: "MSFT".to_string(),
                reference: ConstituentReference::MarketData {
                    price_id: "MSFT".to_string(),
                    asset_type: AssetType::Equity,
                },
                weight: 0.5,
                units: None,
                ticker: Some("MSFT".to_string()),
            },
        ])
        .expense_ratio(0.001)
        .rebalance_freq(Frequency::quarterly())
        .creation_unit_size(50_000.0)
        .currency(Currency::USD)
        .replication(ReplicationMethod::Physical)
        .build()
        .unwrap();

    // Test that pricing works with correct currency
    let nav = basket.value(&context, base_date).unwrap();
    assert_eq!(nav.currency(), Currency::USD);
}

#[test]
fn test_creation_unit_mechanics() {
    let _context = create_test_market_context();
    let _base_date = test_date(2025, 1, 1);

    let spy = Basket::builder()
        .id("SPY".into())
        .ticker("SPY".to_string())
        .name("SPDR S&P 500 ETF".to_string())
        .creation_unit_size(50000.0)
        .constituents(vec![
            BasketConstituent {
                id: "AAPL".to_string(),
                reference: ConstituentReference::MarketData {
                    price_id: "AAPL".to_string(),
                    asset_type: AssetType::Equity,
                },
                weight: 0.6,
                units: None,
                ticker: Some("AAPL".to_string()),
            },
            BasketConstituent {
                id: "MSFT".to_string(),
                reference: ConstituentReference::MarketData {
                    price_id: "MSFT".to_string(),
                    asset_type: AssetType::Equity,
                },
                weight: 0.4,
                units: None,
                ticker: Some("MSFT".to_string()),
            },
        ])
        .expense_ratio(0.0009)
        .rebalance_freq(Frequency::quarterly())
        .currency(Currency::USD)
        .replication(ReplicationMethod::Physical)
        .build()
        .unwrap();

    // Test creation basket calculation
    let creation_basket = spy.creation_basket(1.0);
    assert_eq!(creation_basket.creation_basket.len(), 2);

    // Verify transaction costs are calculated
    assert!(creation_basket.transaction_cost.amount() > 0.0);
}

#[test]
fn test_nav_vs_basket_value() {
    let context = create_test_market_context();
    let base_date = test_date(2025, 1, 1);

    let basket = Basket::builder()
        .id("TEST".into())
        .name("Test Basket".to_string())
        .constituents(vec![
            BasketConstituent {
                id: "AAPL".to_string(),
                reference: ConstituentReference::MarketData {
                    price_id: "AAPL".to_string(),
                    asset_type: AssetType::Equity,
                },
                weight: 0.5,
                units: None,
                ticker: Some("AAPL".to_string()),
            },
            BasketConstituent {
                id: "MSFT".to_string(),
                reference: ConstituentReference::MarketData {
                    price_id: "MSFT".to_string(),
                    asset_type: AssetType::Equity,
                },
                weight: 0.5,
                units: None,
                ticker: Some("MSFT".to_string()),
            },
        ])
        .expense_ratio(0.001)
        .rebalance_freq(Frequency::quarterly())
        .creation_unit_size(50_000.0)
        .currency(Currency::USD)
        .shares_outstanding(1_000_000.0)
        .replication(ReplicationMethod::Physical)
        .build()
        .unwrap();

    let nav = basket.nav(&context, base_date).unwrap();
    let basket_value = basket.basket_value(&context, base_date).unwrap();

    // NAV should be basket_value / shares_outstanding
    let expected_nav = basket_value.amount() / 1_000_000.0;
    assert!((nav.amount() - expected_nav).abs() < 1e-3);
}
