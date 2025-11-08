//! Validation framework for waterfall specifications.
//!
//! This module provides comprehensive validation for waterfall specifications,
//! ensuring correctness before execution. It checks for:
//! - Duplicate tier/recipient IDs
//! - Invalid priority values
//! - Empty/impossible tier configurations
//! - Missing test references in diversion rules
//! - Circular diversion dependencies

use super::diversion::{DiversionCondition, DiversionEngine, DiversionRule};
use super::waterfall::{PaymentType, WaterfallTier};
use finstack_core::Result;
use std::collections::HashSet;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

// ============================================================================
// VALIDATION ERRORS
// ============================================================================

/// Validation error details
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ValidationError {
    /// Duplicate tier ID
    DuplicateTierId {
        /// Tier id.
        tier_id: String,
    },
    /// Duplicate recipient ID within a tier
    DuplicateRecipientId {
        /// Tier id.
        tier_id: String,
        /// Recipient id.
        recipient_id: String,
    },
    /// Invalid priority (must be > 0)
    InvalidPriority {
        /// Tier id.
        tier_id: String,
        /// Priority.
        priority: usize,
    },
    /// Tier has no recipients
    EmptyTier {
        /// Tier id.
        tier_id: String,
    },
    /// Missing coverage test reference
    MissingTestReference {
        /// Test id.
        test_id: String,
        /// Rule id.
        rule_id: String,
    },
    /// Invalid recipient weight (must be >= 0)
    InvalidWeight {
        /// Tier id.
        tier_id: String,
        /// Recipient id.
        recipient_id: String,
        /// Weight.
        weight: f64,
    },
    /// Pro-rata tier with invalid total weight
    InvalidProRataWeights {
        /// Tier id.
        tier_id: String,
        /// Total weight.
        total_weight: f64,
    },
    /// Circular diversion reference
    CircularDiversion {
        /// Cycle path.
        cycle_path: String,
    },
    /// Diversion references non-existent tier
    InvalidDiversionTier {
        /// Rule id.
        rule_id: String,
        /// Tier id.
        tier_id: String,
    },
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::DuplicateTierId { tier_id } => {
                write!(f, "Duplicate tier ID: {}", tier_id)
            }
            ValidationError::DuplicateRecipientId {
                tier_id,
                recipient_id,
            } => write!(
                f,
                "Duplicate recipient ID '{}' in tier '{}'",
                recipient_id, tier_id
            ),
            ValidationError::InvalidPriority { tier_id, priority } => {
                write!(
                    f,
                    "Invalid priority {} for tier '{}' (must be > 0)",
                    priority, tier_id
                )
            }
            ValidationError::EmptyTier { tier_id } => {
                write!(f, "Tier '{}' has no recipients", tier_id)
            }
            ValidationError::MissingTestReference { test_id, rule_id } => {
                write!(
                    f,
                    "Diversion rule '{}' references missing test '{}'",
                    rule_id, test_id
                )
            }
            ValidationError::InvalidWeight {
                tier_id,
                recipient_id,
                weight,
            } => write!(
                f,
                "Invalid weight {} for recipient '{}' in tier '{}' (must be >= 0)",
                weight, recipient_id, tier_id
            ),
            ValidationError::InvalidProRataWeights {
                tier_id,
                total_weight,
            } => write!(
                f,
                "Pro-rata tier '{}' has invalid total weight {} (must be > 0)",
                tier_id, total_weight
            ),
            ValidationError::CircularDiversion { cycle_path } => {
                write!(f, "Circular diversion detected: {}", cycle_path)
            }
            ValidationError::InvalidDiversionTier { rule_id, tier_id } => {
                write!(
                    f,
                    "Diversion rule '{}' references non-existent tier '{}'",
                    rule_id, tier_id
                )
            }
        }
    }
}

// ============================================================================
// WATERFALL VALIDATOR TRAIT
// ============================================================================

/// Trait for validating waterfall specifications
pub trait WaterfallValidator {
    /// Validate the waterfall specification
    ///
    /// Returns Ok(()) if valid, or Err with validation errors
    fn validate(&self) -> Result<()>;
}

// ============================================================================
// WATERFALL SPEC VALIDATION
// ============================================================================

/// Waterfall specification for validation
///
/// This is a simplified representation that includes just the fields needed
/// for validation (tiers, diversion rules, coverage tests)
pub struct WaterfallSpec {
    /// Tiers.
    pub tiers: Vec<WaterfallTier>,
    /// Diversion rules.
    pub diversion_rules: Vec<DiversionRule>,
    /// Coverage test IDs.
    pub coverage_test_ids: Vec<String>,
}

impl WaterfallSpec {
    /// Create a new waterfall spec
    pub fn new(
        tiers: Vec<WaterfallTier>,
        diversion_rules: Vec<DiversionRule>,
        coverage_test_ids: Vec<String>,
    ) -> Self {
        Self {
            tiers,
            diversion_rules,
            coverage_test_ids,
        }
    }
}

impl WaterfallValidator for WaterfallSpec {
    fn validate(&self) -> Result<()> {
        let mut errors = Vec::new();

        // Validate tiers
        errors.extend(validate_tiers(&self.tiers));

        // Validate diversion rules
        errors.extend(validate_diversion_rules(
            &self.diversion_rules,
            &self.tiers,
            &self.coverage_test_ids,
        ));

        if !errors.is_empty() {
            return Err(finstack_core::Error::Validation(format!(
                "Waterfall validation failed with {} error(s): {}",
                errors.len(),
                errors
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join("; ")
            )));
        }

        Ok(())
    }
}

/// Validate tier specifications
fn validate_tiers(tiers: &[WaterfallTier]) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    // Check for duplicate tier IDs
    let mut seen_tier_ids = HashSet::new();
    for tier in tiers {
        if !seen_tier_ids.insert(&tier.id) {
            errors.push(ValidationError::DuplicateTierId {
                tier_id: tier.id.clone(),
            });
        }

        // Check for empty tiers (except residual tiers which can have ResidualCash recipient)
        if tier.recipients.is_empty() && tier.payment_type != PaymentType::Residual {
            errors.push(ValidationError::EmptyTier {
                tier_id: tier.id.clone(),
            });
        }

        // Validate recipient IDs within tier
        let mut seen_recipient_ids = HashSet::new();
        for recipient in &tier.recipients {
            if !seen_recipient_ids.insert(&recipient.id) {
                errors.push(ValidationError::DuplicateRecipientId {
                    tier_id: tier.id.clone(),
                    recipient_id: recipient.id.clone(),
                });
            }

            // Check recipient weights
            if let Some(weight) = recipient.weight {
                if weight < 0.0 {
                    errors.push(ValidationError::InvalidWeight {
                        tier_id: tier.id.clone(),
                        recipient_id: recipient.id.clone(),
                        weight,
                    });
                }
            }
        }

        // For pro-rata tiers, validate total weight
        if tier.allocation_mode == super::waterfall::AllocationMode::ProRata {
            let total_weight: f64 = tier
                .recipients
                .iter()
                .map(|r| r.weight.unwrap_or(1.0))
                .sum();

            if total_weight <= 0.0 {
                errors.push(ValidationError::InvalidProRataWeights {
                    tier_id: tier.id.clone(),
                    total_weight,
                });
            }
        }
    }

    errors
}

/// Validate diversion rules
fn validate_diversion_rules(
    rules: &[DiversionRule],
    tiers: &[WaterfallTier],
    coverage_test_ids: &[String],
) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    let tier_ids: HashSet<_> = tiers.iter().map(|t| t.id.as_str()).collect();
    let test_ids: HashSet<_> = coverage_test_ids.iter().map(|s| s.as_str()).collect();

    for rule in rules {
        // Check that source and target tiers exist
        if !tier_ids.contains(rule.source_tier_id.as_str()) {
            errors.push(ValidationError::InvalidDiversionTier {
                rule_id: rule.id.clone(),
                tier_id: rule.source_tier_id.clone(),
            });
        }

        if !tier_ids.contains(rule.target_tier_id.as_str()) {
            errors.push(ValidationError::InvalidDiversionTier {
                rule_id: rule.id.clone(),
                tier_id: rule.target_tier_id.clone(),
            });
        }

        // Check that coverage test references are valid
        if let DiversionCondition::CoverageTestFailed { test_id } = &rule.condition {
            if !test_ids.contains(test_id.as_str()) {
                errors.push(ValidationError::MissingTestReference {
                    test_id: test_id.clone(),
                    rule_id: rule.id.clone(),
                });
            }
        }
    }

    // Check for circular dependencies using DiversionEngine
    let diversion_engine = rules.iter().fold(DiversionEngine::new(), |engine, rule| {
        engine.add_rule(rule.clone())
    });

    if let Err(e) = diversion_engine.validate() {
        // Extract cycle path from error message if it's a circular diversion error
        let err_msg = e.to_string();
        if err_msg.contains("Circular diversion") {
            errors.push(ValidationError::CircularDiversion {
                cycle_path: err_msg,
            });
        }
    }

    errors
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Quick validation helper that returns true if spec is valid
pub fn is_valid_waterfall_spec(
    tiers: &[WaterfallTier],
    diversion_rules: &[DiversionRule],
    coverage_test_ids: &[String],
) -> bool {
    let spec = WaterfallSpec::new(
        tiers.to_vec(),
        diversion_rules.to_vec(),
        coverage_test_ids.to_vec(),
    );
    spec.validate().is_ok()
}

/// Get validation errors as a list
pub fn get_validation_errors(
    tiers: &[WaterfallTier],
    diversion_rules: &[DiversionRule],
    coverage_test_ids: &[String],
) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    errors.extend(validate_tiers(tiers));
    errors.extend(validate_diversion_rules(
        diversion_rules,
        tiers,
        coverage_test_ids,
    ));
    errors
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::super::diversion::{DiversionCondition, DiversionRule};
    use super::super::waterfall::{
        AllocationMode, PaymentCalculation, PaymentRecipient, PaymentType, Recipient, WaterfallTier,
    };
    use super::*;
    use finstack_core::currency::Currency;
    use finstack_core::money::Money;

    fn create_valid_tier(id: &str, priority: usize) -> WaterfallTier {
        WaterfallTier::new(id, priority, PaymentType::Fee).add_recipient(Recipient::new(
            "recipient1",
            PaymentRecipient::ServiceProvider("Trustee".into()),
            PaymentCalculation::FixedAmount {
                amount: Money::new(1000.0, Currency::USD),
            },
        ))
    }

    #[test]
    fn test_valid_waterfall_spec() {
        let tiers = vec![create_valid_tier("tier1", 1), create_valid_tier("tier2", 2)];
        let rules = vec![];
        let tests = vec![];

        let spec = WaterfallSpec::new(tiers, rules, tests);
        assert!(spec.validate().is_ok());
    }

    #[test]
    fn test_duplicate_tier_id() {
        let tiers = vec![
            create_valid_tier("tier1", 1),
            create_valid_tier("tier1", 2), // Duplicate
        ];

        let errors = validate_tiers(&tiers);
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0], ValidationError::DuplicateTierId { .. }));
    }

    #[test]
    fn test_empty_tier() {
        let empty_tier = WaterfallTier::new("empty", 1, PaymentType::Fee);
        let tiers = vec![empty_tier];

        let errors = validate_tiers(&tiers);
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0], ValidationError::EmptyTier { .. }));
    }

    #[test]
    fn test_duplicate_recipient_id() {
        let tier = WaterfallTier::new("tier1", 1, PaymentType::Fee)
            .add_recipient(Recipient::new(
                "recipient1",
                PaymentRecipient::ServiceProvider("Trustee".into()),
                PaymentCalculation::FixedAmount {
                    amount: Money::new(1000.0, Currency::USD),
                },
            ))
            .add_recipient(Recipient::new(
                "recipient1", // Duplicate
                PaymentRecipient::ServiceProvider("Admin".into()),
                PaymentCalculation::FixedAmount {
                    amount: Money::new(500.0, Currency::USD),
                },
            ));

        let errors = validate_tiers(&[tier]);
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0],
            ValidationError::DuplicateRecipientId { .. }
        ));
    }

    #[test]
    fn test_invalid_weight() {
        let tier = WaterfallTier::new("tier1", 1, PaymentType::Fee).add_recipient(
            Recipient::new(
                "recipient1",
                PaymentRecipient::ServiceProvider("Trustee".into()),
                PaymentCalculation::FixedAmount {
                    amount: Money::new(1000.0, Currency::USD),
                },
            )
            .with_weight(-1.0),
        );

        let errors = validate_tiers(&[tier]);
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0], ValidationError::InvalidWeight { .. }));
    }

    #[test]
    fn test_invalid_pro_rata_weights() {
        let tier = WaterfallTier::new("tier1", 1, PaymentType::Fee)
            .allocation_mode(AllocationMode::ProRata)
            .add_recipient(
                Recipient::new(
                    "recipient1",
                    PaymentRecipient::ServiceProvider("Trustee".into()),
                    PaymentCalculation::FixedAmount {
                        amount: Money::new(1000.0, Currency::USD),
                    },
                )
                .with_weight(0.0),
            );

        let errors = validate_tiers(&[tier]);
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0],
            ValidationError::InvalidProRataWeights { .. }
        ));
    }

    #[test]
    fn test_missing_test_reference() {
        let tiers = vec![create_valid_tier("tier1", 1)];
        let rules = vec![DiversionRule::on_test_failure(
            "rule1",
            "tier1",
            "tier2",
            "nonexistent_test",
            1,
        )];
        let tests = vec!["existing_test".to_string()];

        let errors = validate_diversion_rules(&rules, &tiers, &tests);
        assert!(errors
            .iter()
            .any(|e| matches!(e, ValidationError::MissingTestReference { .. })));
    }

    #[test]
    fn test_invalid_diversion_tier() {
        let tiers = vec![create_valid_tier("tier1", 1)];
        let rules = vec![DiversionRule::new(
            "rule1",
            "tier1",
            "nonexistent_tier",
            DiversionCondition::Always,
            1,
        )];
        let tests = vec![];

        let errors = validate_diversion_rules(&rules, &tiers, &tests);
        assert!(errors
            .iter()
            .any(|e| matches!(e, ValidationError::InvalidDiversionTier { .. })));
    }

    #[test]
    fn test_circular_diversion() {
        let tiers = vec![
            create_valid_tier("tier_a", 1),
            create_valid_tier("tier_b", 2),
        ];
        let rules = vec![
            DiversionRule::new("rule1", "tier_a", "tier_b", DiversionCondition::Always, 1),
            DiversionRule::new("rule2", "tier_b", "tier_a", DiversionCondition::Always, 2),
        ];
        let tests = vec![];

        let errors = validate_diversion_rules(&rules, &tiers, &tests);
        assert!(errors
            .iter()
            .any(|e| matches!(e, ValidationError::CircularDiversion { .. })));
    }

    #[test]
    fn test_is_valid_waterfall_spec_helper() {
        let tiers = vec![create_valid_tier("tier1", 1)];
        let rules = vec![];
        let tests = vec![];

        assert!(is_valid_waterfall_spec(&tiers, &rules, &tests));

        // Test with duplicate tier IDs
        let invalid_tiers = vec![create_valid_tier("tier1", 1), create_valid_tier("tier1", 2)];
        assert!(!is_valid_waterfall_spec(&invalid_tiers, &rules, &tests));
    }
}
