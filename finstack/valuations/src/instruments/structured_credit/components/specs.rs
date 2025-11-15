//! Behavioral model specifications for structured credit instruments.
//!
//! This module re-exports unified behavioral model specs from the cashflow builder.
//! These specs are now shared across all credit instruments (CLO, ABS, RMBS, CMBS,
//! bonds, term loans, revolving credit, etc.), reducing duplication and ensuring
//! consistency across the library.

// Re-export builder specs as single source of truth
pub use crate::cashflow::builder::{
    DefaultCurve, DefaultModelSpec, PrepaymentCurve, PrepaymentModelSpec, RecoveryModelSpec,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prepayment_spec_calculation() {
        let spec = PrepaymentModelSpec::psa(1.5);

        // Calculate prepayment rate at month 30
        let smm = spec.smm(30);

        // 150% PSA at month 30 = 9% CPR ≈ 0.77% SMM
        assert!(smm > 0.0);
        assert!(smm < 0.01); // Less than 1% monthly
    }

    #[test]
    fn test_default_spec_calculation() {
        let spec = DefaultModelSpec::sda(2.0);

        // Calculate default rate at peak month
        let mdr = spec.mdr(30);

        // Should be positive
        assert!(mdr > 0.0);
    }

    #[test]
    fn test_recovery_spec() {
        let spec = RecoveryModelSpec::with_lag(0.6, 12);

        assert_eq!(spec.rate, 0.6);
        assert_eq!(spec.recovery_lag, 12);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_prepayment_spec_json() {
        let spec = PrepaymentModelSpec::psa(1.5);
        let json = serde_json::to_string(&spec).expect("should succeed");
        let recovered: PrepaymentModelSpec = serde_json::from_str(&json).expect("should succeed");

        match recovered.curve {
            Some(PrepaymentCurve::Psa { speed_multiplier }) => {
                assert!((speed_multiplier - 1.5).abs() < 1e-10);
            }
            _ => panic!("Expected PSA curve"),
        }
    }

    #[test]
    fn test_all_prepayment_variants() {
        let specs = vec![
            PrepaymentModelSpec::psa(1.0),
            PrepaymentModelSpec::constant_cpr(0.15),
        ];

        for spec in specs {
            let smm = spec.smm(12);
            assert!((0.0..=1.0).contains(&smm), "SMM should be valid: {}", smm);
        }
    }

    #[test]
    fn test_all_default_variants() {
        let specs = vec![
            DefaultModelSpec::sda(1.0),
            DefaultModelSpec::constant_cdr(0.02),
        ];

        for spec in specs {
            let mdr = spec.mdr(12);
            assert!((0.0..=1.0).contains(&mdr), "MDR should be valid: {}", mdr);
        }
    }
}
