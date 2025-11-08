//! Tests for TermLoan capital structure integration.
//!
//! This test suite validates that TermLoan instruments from the valuations crate
//! can be properly integrated into financial statement models.

use finstack_statements::types::DebtInstrumentSpec;
use finstack_statements::CapitalStructureSpec;

mod common;

// ============================================================================
// TermLoan Variant Tests
// ============================================================================

#[test]
fn test_term_loan_variant_serialization() {
    let spec = DebtInstrumentSpec::TermLoan {
        id: "TL-001".to_string(),
        spec: serde_json::json!({
            "id": "TL-001",
            "notional": {
                "amount": 5000000.0,
                "currency": "USD"
            },
            // Additional TermLoan fields would go here
        }),
    };

    // Test serialization roundtrip
    let json = serde_json::to_string(&spec).unwrap();
    let deserialized: DebtInstrumentSpec = serde_json::from_str(&json).unwrap();

    match deserialized {
        DebtInstrumentSpec::TermLoan { id, .. } => {
            assert_eq!(id, "TL-001");
        }
        _ => panic!("Expected TermLoan variant"),
    }
}

#[test]
fn test_debt_instrument_spec_all_variants() {
    // Verify all variants can be created and serialized
    let bond = DebtInstrumentSpec::Bond {
        id: "BOND-001".to_string(),
        spec: serde_json::json!({}),
    };

    let swap = DebtInstrumentSpec::Swap {
        id: "SWAP-001".to_string(),
        spec: serde_json::json!({}),
    };

    let term_loan = DebtInstrumentSpec::TermLoan {
        id: "TL-001".to_string(),
        spec: serde_json::json!({}),
    };

    let generic = DebtInstrumentSpec::Generic {
        id: "GEN-001".to_string(),
        spec: serde_json::json!({}),
    };

    // All should serialize without error
    assert!(serde_json::to_string(&bond).is_ok());
    assert!(serde_json::to_string(&swap).is_ok());
    assert!(serde_json::to_string(&term_loan).is_ok());
    assert!(serde_json::to_string(&generic).is_ok());
}

// ============================================================================
// Capital Structure with TermLoan (Placeholder)
// ============================================================================

#[test]
fn test_term_loan_in_capital_structure_placeholder() {
    // This is a placeholder test. Full integration testing requires:
    // 1. Creating a valid TermLoan spec matching valuations crate structure
    // 2. Adding it to a model's capital structure
    // 3. Providing market context with appropriate curves
    // 4. Evaluating and verifying cashflows

    // For now, we verify the type infrastructure is in place
    let capital_structure = CapitalStructureSpec {
        debt_instruments: vec![DebtInstrumentSpec::TermLoan {
            id: "TL-001".to_string(),
            spec: serde_json::json!({
                "id": "TL-001",
                "notional": {
                    "amount": 5000000.0,
                    "currency": "USD"
                }
            }),
        }],
        equity_instruments: vec![],
        meta: indexmap::IndexMap::new(),
    };

    // Verify it serializes correctly
    let json = serde_json::to_string(&capital_structure).unwrap();
    let deserialized: CapitalStructureSpec = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.debt_instruments.len(), 1);
    match &deserialized.debt_instruments[0] {
        DebtInstrumentSpec::TermLoan { id, .. } => {
            assert_eq!(id, "TL-001");
        }
        _ => panic!("Expected TermLoan variant"),
    }
}

// Note: Full end-to-end TermLoan integration tests require:
// - Proper TermLoan spec construction matching valuations crate requirements
// - Market context with discount and forward curves
// - These will be added as the integration matures
