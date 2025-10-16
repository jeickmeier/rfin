//! Unit tests for waterfall engine and payment distribution.
//!
//! Tests cover:
//! - Waterfall builder construction
//! - Payment rule ordering
//! - Sequential distribution logic
//! - OC/IC diversion triggers
//! - Payment calculation methods

use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_valuations::instruments::structured_credit::{
    ManagementFeeType, PaymentCalculation, PaymentRecipient, PaymentRule, WaterfallBuilder,
    WaterfallEngine,
};

// ============================================================================
// Waterfall Builder Tests
// ============================================================================

#[test]
fn test_waterfall_builder_creates_proper_priority_order() {
    // Arrange & Act
    let waterfall = WaterfallBuilder::new(Currency::USD)
        .add_senior_expenses(Money::new(25_000.0, Currency::USD), "Trustee")
        .add_management_fee(0.004, ManagementFeeType::Senior)
        .add_tranche_interest("CLASS_A", true)
        .add_tranche_principal("CLASS_A")
        .add_equity_distribution()
        .build();

    // Assert
    assert_eq!(waterfall.payment_rules.len(), 5);
    
    // Verify priority order (1, 2, 3, 4, 5)
    for (i, rule) in waterfall.payment_rules.iter().enumerate() {
        assert_eq!(rule.priority, (i + 1) as u32);
    }
}

#[test]
fn test_waterfall_builder_fee_types() {
    // Arrange & Act
    let waterfall = WaterfallBuilder::new(Currency::USD)
        .add_management_fee(0.004, ManagementFeeType::Senior)
        .add_management_fee(0.002, ManagementFeeType::Subordinated)
        .build();

    // Assert
    assert_eq!(waterfall.payment_rules.len(), 2);
    
    // Verify fee recipients
    match &waterfall.payment_rules[0].recipient {
        PaymentRecipient::ManagerFee(ManagementFeeType::Senior) => {},
        _ => panic!("Expected senior management fee"),
    }
    
    match &waterfall.payment_rules[1].recipient {
        PaymentRecipient::ManagerFee(ManagementFeeType::Subordinated) => {},
        _ => panic!("Expected subordinated management fee"),
    }
}

#[test]
fn test_waterfall_builder_tranche_payments() {
    // Arrange & Act
    let waterfall = WaterfallBuilder::new(Currency::USD)
        .add_tranche_interest("A", false)
        .add_tranche_interest("B", true) // Divertible
        .add_tranche_principal("A")
        .build();

    // Assert
    assert_eq!(waterfall.payment_rules.len(), 3);
    
    // Check divertible flag
    assert!(!waterfall.payment_rules[0].divertible); // A interest not divertible
    assert!(waterfall.payment_rules[1].divertible); // B interest divertible
}

#[test]
fn test_waterfall_builder_coverage_triggers() {
    // Arrange & Act
    let waterfall = WaterfallBuilder::new(Currency::USD)
        .add_tranche_interest("A", true)
        .add_oc_ic_trigger("A", Some(1.25), Some(1.20))
        .build();

    // Assert
    assert_eq!(waterfall.coverage_triggers.len(), 1);
    assert_eq!(waterfall.coverage_triggers[0].tranche_id, "A");
    assert_eq!(waterfall.coverage_triggers[0].oc_trigger, Some(1.25));
    assert_eq!(waterfall.coverage_triggers[0].ic_trigger, Some(1.20));
}

// ============================================================================
// Payment Rule Tests
// ============================================================================

#[test]
fn test_payment_rule_creation() {
    // Arrange & Act
    let rule = PaymentRule::new(
        "test_fee",
        1,
        PaymentRecipient::ServiceProvider("Trustee".to_string()),
        PaymentCalculation::FixedAmount {
            amount: Money::new(50_000.0, Currency::USD),
        },
    );

    // Assert
    assert_eq!(rule.id, "test_fee");
    assert_eq!(rule.priority, 1);
    assert!(!rule.divertible);
}

#[test]
fn test_payment_rule_divertible_flag() {
    // Arrange & Act
    let rule = PaymentRule::new(
        "equity_payment",
        5,
        PaymentRecipient::Equity,
        PaymentCalculation::ResidualCash,
    )
    .divertible();

    // Assert
    assert!(rule.divertible);
}

// ============================================================================
// Waterfall Engine Tests
// ============================================================================

#[test]
fn test_waterfall_engine_sorts_by_priority() {
    // Arrange
    let mut waterfall = WaterfallEngine::new(Currency::USD);

    // Add rules out of order
    waterfall = waterfall.add_rule(PaymentRule::new(
        "third",
        3,
        PaymentRecipient::Equity,
        PaymentCalculation::ResidualCash,
    ));
    waterfall = waterfall.add_rule(PaymentRule::new(
        "first",
        1,
        PaymentRecipient::ServiceProvider("Trustee".to_string()),
        PaymentCalculation::FixedAmount {
            amount: Money::new(10_000.0, Currency::USD),
        },
    ));
    waterfall = waterfall.add_rule(PaymentRule::new(
        "second",
        2,
        PaymentRecipient::ManagerFee(ManagementFeeType::Senior),
        PaymentCalculation::PercentageOfCollateral {
            rate: 0.004,
            annualized: true,
        },
    ));

    // Assert: Should be automatically sorted
    assert_eq!(waterfall.payment_rules[0].id, "first");
    assert_eq!(waterfall.payment_rules[1].id, "second");
    assert_eq!(waterfall.payment_rules[2].id, "third");
}

// ============================================================================
// Payment Recipient Tests
// ============================================================================

#[test]
fn test_payment_recipient_variants() {
    // Arrange & Act
    let service = PaymentRecipient::ServiceProvider("Trustee".to_string());
    let manager = PaymentRecipient::ManagerFee(ManagementFeeType::Senior);
    let tranche = PaymentRecipient::Tranche("A".to_string());
    let equity = PaymentRecipient::Equity;

    // Assert: All variants are distinct
    assert_ne!(service, manager);
    assert_ne!(service, tranche);
    assert_ne!(service, equity);
}

// ============================================================================
// Payment Calculation Tests
// ============================================================================

#[test]
fn test_payment_calculation_fixed_amount() {
    // Arrange & Act
    let calc = PaymentCalculation::FixedAmount {
        amount: Money::new(100_000.0, Currency::USD),
    };

    // Assert
    match calc {
        PaymentCalculation::FixedAmount { amount } => {
            assert_eq!(amount.amount(), 100_000.0);
        }
        _ => panic!("Expected FixedAmount"),
    }
}

#[test]
fn test_payment_calculation_percentage_of_collateral() {
    // Arrange & Act
    let calc = PaymentCalculation::PercentageOfCollateral {
        rate: 0.0040,
        annualized: true,
    };

    // Assert
    match calc {
        PaymentCalculation::PercentageOfCollateral { rate, annualized } => {
            assert_eq!(rate, 0.0040);
            assert!(annualized);
        }
        _ => panic!("Expected PercentageOfCollateral"),
    }
}

#[test]
fn test_payment_calculation_residual_cash() {
    // Arrange & Act
    let calc = PaymentCalculation::ResidualCash;

    // Assert
    assert!(matches!(calc, PaymentCalculation::ResidualCash));
}

