//! Test that instruments can be serialized and deserialized with all user-facing fields.

use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_core::types::Date;
use finstack_core::types::{CurveId, InstrumentId};
use time::Month;

use finstack_valuations::instruments::PricingOverrides;
use finstack_valuations::instruments::*;

#[cfg(feature = "serde")]
#[test]
fn test_bond_with_pricing_overrides_and_attributes() {
    let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2027, Month::January, 15).unwrap();

    // Create a bond with pricing overrides and attributes
    let mut bond = Bond::fixed(
        InstrumentId::new("TEST_BOND"),
        Money::new(1_000_000.0, Currency::USD),
        0.05,
        issue,
        maturity,
        CurveId::new("USD-OIS"),
    );

    // Set pricing overrides
    bond.pricing_overrides = PricingOverrides::default()
        .with_clean_price(98.5)
        .with_spread_bp(150.0);

    // Set attributes
    bond.attributes.tags.insert("corporate".to_string());
    bond.attributes.tags.insert("investment-grade".to_string());
    bond.attributes
        .meta
        .insert("issuer".to_string(), "ACME Corp".to_string());
    bond.attributes
        .meta
        .insert("sector".to_string(), "Technology".to_string());

    // Serialize
    let json = serde_json::to_string(&bond).expect("Bond should serialize");
    println!("Serialized bond: {}", json);

    // Deserialize
    let deserialized: Bond = serde_json::from_str(&json).expect("Bond should deserialize");

    // Verify pricing overrides were preserved
    assert_eq!(
        deserialized.pricing_overrides.quoted_clean_price,
        Some(98.5)
    );
    assert_eq!(deserialized.pricing_overrides.quoted_spread_bp, Some(150.0));

    // Verify attributes were preserved
    assert!(deserialized.attributes.tags.contains("corporate"));
    assert!(deserialized.attributes.tags.contains("investment-grade"));
    assert_eq!(
        deserialized.attributes.meta.get("issuer"),
        Some(&"ACME Corp".to_string())
    );
    assert_eq!(
        deserialized.attributes.meta.get("sector"),
        Some(&"Technology".to_string())
    );

    println!("✓ Bond with pricing_overrides and attributes serialization works");
}

#[cfg(feature = "serde")]
#[test]
fn test_equity_with_attributes() {
    let mut equity = Equity::new("TEST_EQUITY", "AAPL", Currency::USD);

    // Set attributes
    equity.attributes.tags.insert("tech".to_string());
    equity.attributes.tags.insert("large-cap".to_string());
    equity
        .attributes
        .meta
        .insert("exchange".to_string(), "NASDAQ".to_string());

    // Serialize and deserialize
    let json = serde_json::to_string(&equity).expect("Equity should serialize");
    let deserialized: Equity = serde_json::from_str(&json).expect("Equity should deserialize");

    // Verify attributes were preserved
    assert!(deserialized.attributes.tags.contains("tech"));
    assert!(deserialized.attributes.tags.contains("large-cap"));
    assert_eq!(
        deserialized.attributes.meta.get("exchange"),
        Some(&"NASDAQ".to_string())
    );

    println!("✓ Equity with attributes serialization works");
}

#[cfg(feature = "serde")]
#[test]
fn test_equity_option_with_pricing_overrides() {
    let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();

    let mut option = EquityOption::european_call(
        "TEST_OPTION",
        "AAPL",
        150.0,
        issue,
        Money::new(1000.0, Currency::USD),
        100.0,
    );

    // Set pricing overrides with implied volatility
    option.pricing_overrides = PricingOverrides::default().with_implied_vol(0.25);

    // Serialize and deserialize
    let json = serde_json::to_string(&option).expect("EquityOption should serialize");
    let deserialized: EquityOption =
        serde_json::from_str(&json).expect("EquityOption should deserialize");

    // Verify pricing overrides were preserved
    assert_eq!(
        deserialized.pricing_overrides.implied_volatility,
        Some(0.25)
    );

    println!("✓ EquityOption with pricing_overrides serialization works");
}

#[cfg(feature = "serde")]
#[test]
fn test_cds_with_upfront_payment() {
    // Test that CDS upfront payment (in pricing_overrides) is serializable
    let pricing_overrides = PricingOverrides::default()
        .with_upfront(Money::new(50_000.0, Currency::USD))
        .with_spread_bp(300.0);

    let json =
        serde_json::to_string(&pricing_overrides).expect("PricingOverrides should serialize");
    let deserialized: PricingOverrides =
        serde_json::from_str(&json).expect("PricingOverrides should deserialize");

    assert_eq!(
        deserialized.upfront_payment,
        Some(Money::new(50_000.0, Currency::USD))
    );
    assert_eq!(deserialized.quoted_spread_bp, Some(300.0));

    println!("✓ PricingOverrides with upfront payment serialization works");
}

#[cfg(feature = "serde")]
#[test]
fn test_bond_with_calendar_id() {
    let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2027, Month::January, 15).unwrap();

    let mut bond = Bond::fixed(
        InstrumentId::new("TEST_BOND_CALENDAR"),
        Money::new(1_000_000.0, Currency::USD),
        0.05,
        issue,
        maturity,
        CurveId::new("USD-OIS"),
    );

    // Set calendar_id
    bond.calendar_id = Some("NYSE".to_string());

    // Serialize and deserialize
    let json = serde_json::to_string(&bond).expect("Bond should serialize");
    println!("Bond with calendar_id JSON: {}", json);

    let deserialized: Bond = serde_json::from_str(&json).expect("Bond should deserialize");

    // Verify calendar_id was preserved
    assert_eq!(deserialized.calendar_id, Some("NYSE".to_string()));

    println!("✓ Bond with calendar_id serialization works");
}

#[cfg(feature = "serde")]
#[test]
fn test_bond_with_linear_amortization() {
    use finstack_core::cashflow::primitives::AmortizationSpec;

    let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2027, Month::January, 15).unwrap();

    let mut bond = Bond::fixed(
        InstrumentId::new("TEST_AMORTIZING_BOND"),
        Money::new(1_000_000.0, Currency::USD),
        0.05,
        issue,
        maturity,
        CurveId::new("USD-OIS"),
    );

    // Set linear amortization
    bond.amortization = Some(AmortizationSpec::LinearTo {
        final_notional: Money::new(0.0, Currency::USD),
    });

    // Serialize and deserialize
    let json = serde_json::to_string(&bond).expect("Bond should serialize");
    println!("Bond with amortization JSON: {}", json);

    let deserialized: Bond = serde_json::from_str(&json).expect("Bond should deserialize");

    // Verify amortization was preserved
    assert!(deserialized.amortization.is_some());
    match deserialized.amortization.unwrap() {
        AmortizationSpec::LinearTo { final_notional } => {
            assert_eq!(final_notional, Money::new(0.0, Currency::USD));
        }
        _ => panic!("Expected LinearTo amortization"),
    }

    println!("✓ Bond with linear amortization serialization works");
}

#[cfg(feature = "serde")]
#[test]
fn test_bond_with_step_amortization() {
    use finstack_core::cashflow::primitives::AmortizationSpec;

    let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2027, Month::January, 15).unwrap();
    let step_date = Date::from_calendar_date(2026, Month::January, 15).unwrap();

    let mut bond = Bond::fixed(
        InstrumentId::new("TEST_STEP_AMORTIZING_BOND"),
        Money::new(1_000_000.0, Currency::USD),
        0.05,
        issue,
        maturity,
        CurveId::new("USD-OIS"),
    );

    // Set step remaining amortization
    bond.amortization = Some(AmortizationSpec::StepRemaining {
        schedule: vec![(step_date, Money::new(500_000.0, Currency::USD))],
    });

    // Serialize and deserialize
    let json = serde_json::to_string(&bond).expect("Bond should serialize");
    let deserialized: Bond = serde_json::from_str(&json).expect("Bond should deserialize");

    // Verify amortization was preserved
    assert!(deserialized.amortization.is_some());
    match deserialized.amortization.unwrap() {
        AmortizationSpec::StepRemaining { schedule } => {
            assert_eq!(schedule.len(), 1);
            assert_eq!(schedule[0].0, step_date);
            assert_eq!(schedule[0].1, Money::new(500_000.0, Currency::USD));
        }
        _ => panic!("Expected StepRemaining amortization"),
    }

    println!("✓ Bond with step amortization serialization works");
}

#[cfg(feature = "serde")]
#[test]
fn test_bond_with_percent_per_period_amortization() {
    use finstack_core::cashflow::primitives::AmortizationSpec;

    let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2027, Month::January, 15).unwrap();

    let mut bond = Bond::fixed(
        InstrumentId::new("TEST_PERCENT_AMORTIZING_BOND"),
        Money::new(1_000_000.0, Currency::USD),
        0.05,
        issue,
        maturity,
        CurveId::new("USD-OIS"),
    );

    // Set percent per period amortization (5% per period)
    bond.amortization = Some(AmortizationSpec::PercentPerPeriod { pct: 0.05 });

    // Serialize and deserialize
    let json = serde_json::to_string(&bond).expect("Bond should serialize");
    let deserialized: Bond = serde_json::from_str(&json).expect("Bond should deserialize");

    // Verify amortization was preserved
    assert!(deserialized.amortization.is_some());
    match deserialized.amortization.unwrap() {
        AmortizationSpec::PercentPerPeriod { pct } => {
            assert_eq!(pct, 0.05);
        }
        _ => panic!("Expected PercentPerPeriod amortization"),
    }

    println!("✓ Bond with percent per period amortization serialization works");
}

#[cfg(feature = "serde")]
#[test]
fn test_bond_with_calendar_and_amortization() {
    use finstack_core::cashflow::primitives::AmortizationSpec;

    let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2027, Month::January, 15).unwrap();

    let mut bond = Bond::fixed(
        InstrumentId::new("TEST_FULL_BOND"),
        Money::new(1_000_000.0, Currency::USD),
        0.05,
        issue,
        maturity,
        CurveId::new("USD-OIS"),
    );

    // Set both calendar_id and amortization
    bond.calendar_id = Some("TARGET".to_string());
    bond.amortization = Some(AmortizationSpec::LinearTo {
        final_notional: Money::new(100_000.0, Currency::USD),
    });

    // Set attributes and pricing overrides
    bond.attributes.tags.insert("european".to_string());
    bond.attributes
        .meta
        .insert("country".to_string(), "Germany".to_string());
    bond.pricing_overrides = PricingOverrides::default().with_clean_price(99.5);

    // Serialize and deserialize
    let json = serde_json::to_string(&bond).expect("Bond should serialize");
    println!("Full bond JSON: {}", json);

    let deserialized: Bond = serde_json::from_str(&json).expect("Bond should deserialize");

    // Verify all fields were preserved
    assert_eq!(deserialized.calendar_id, Some("TARGET".to_string()));
    assert!(deserialized.amortization.is_some());
    assert!(deserialized.attributes.tags.contains("european"));
    assert_eq!(
        deserialized.attributes.meta.get("country"),
        Some(&"Germany".to_string())
    );
    assert_eq!(
        deserialized.pricing_overrides.quoted_clean_price,
        Some(99.5)
    );

    println!("✓ Bond with calendar_id, amortization, attributes, and pricing_overrides serialization works");
}
