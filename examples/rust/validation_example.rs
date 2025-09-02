//! Example demonstrating the enhanced validation framework
//!
//! This example shows how to use the finstack validation framework for:
//! - Multi-error accumulation instead of failing fast
//! - Domain-specific financial validation
//! - Comprehensive feedback to API consumers

use finstack_core::prelude::*;

fn main() -> Result<()> {
    println!("=== Finstack Enhanced Validation Framework Example ===\n");

    // Example 1: Multi-error accumulation
    demonstrate_multi_error_accumulation();

    // Example 2: Domain-specific financial validation
    demonstrate_financial_validation();

    // Example 3: Complex validation with warnings
    demonstrate_validation_with_warnings();

    // Example 4: Multi-validator usage
    demonstrate_multi_validator();

    Ok(())
}

fn demonstrate_multi_error_accumulation() {
    println!("1. Multi-Error Accumulation:");
    println!("   Instead of failing on the first error, we collect all validation issues\n");

    let mut errors = ValidationErrors::new();

    // Simulate validating multiple fields with various issues
    FinancialValidators::validate_positive_amount(&mut errors, "principal", -1000.0); // Error
    FinancialValidators::validate_rate(&mut errors, "interest_rate", 55.0); // Error (too high)
    FinancialValidators::validate_probability(&mut errors, "default_prob", 1.5); // Error
    FinancialValidators::validate_rate(&mut errors, "spread", 0.02); // Valid
    FinancialValidators::validate_rate(&mut errors, "high_vol", 8.0); // Warning (very high)

    println!("   Validation Status: {:?}", errors.status());
    println!("   Has Errors: {}", errors.has_errors());
    println!("   Has Warnings: {}", errors.has_warnings());
    
    if errors.has_errors() {
        println!("   Errors:");
        for (field, message) in &errors.errors {
            println!("     - {}: {}", field, message);
        }
    }
    
    if errors.has_warnings() {
        println!("   Warnings:");
        for warning in &errors.warnings {
            println!("     - {}", warning);
        }
    }
    
    println!();
}

fn demonstrate_financial_validation() {
    println!("2. Domain-Specific Financial Validation:");
    println!("   Specialized validators for common financial patterns\n");

    let mut errors = ValidationErrors::new();

    // Validate various financial primitives
    let rates = vec![0.02, 0.025, 0.03]; // 2%, 2.5%, 3%
    let amounts = vec![
        Money::new(1000.0, Currency::USD),
        Money::new(2000.0, Currency::USD),
        Money::new(500.0, Currency::USD),
    ];

    FinancialValidators::validate_monotonic_sequence(&mut errors, "rate_curve", &rates);
    FinancialValidators::validate_currency_consistency(&mut errors, "amounts", &amounts);
    FinancialValidators::validate_min_points(&mut errors, "market_data", &rates, 3);

    if errors.has_errors() {
        println!("   ❌ Validation failed with {} errors", errors.errors.len());
    } else {
        println!("   ✅ All financial validations passed!");
        if errors.has_warnings() {
            println!("   ⚠️  {} warnings detected", errors.warnings.len());
        }
    }

    println!();
}

fn demonstrate_validation_with_warnings() {
    println!("3. Validation with Warnings:");
    println!("   Demonstrating how warnings provide additional context without failing\n");

    let mut errors = ValidationErrors::new();

    // This rate is valid but very high, should generate a warning
    FinancialValidators::validate_rate(&mut errors, "equity_premium", 12.0); // 1200% - very high

    // Add a range validation that's near the boundary
    FinancialValidators::validate_range_with_warnings(&mut errors, "correlation", 0.98, 0.0, 1.0, true);

    println!("   Status: {:?}", errors.status());
    if errors.has_warnings() {
        println!("   Warnings collected:");
        for warning in &errors.warnings {
            println!("     - {}", warning);
        }
    }

    // Show how this can be converted to a result
    let result: ValidationResult<String> = errors.into_result("validated_data".to_string());
    println!("   Final result: {}", result);
    println!("   Is success: {}", result.is_success());
    
    println!();
}

fn demonstrate_multi_validator() {
    println!("4. Multi-Validator Pattern:");
    println!("   Running multiple validators and collecting all issues\n");

    // Create a multi-validator for interest rates
    let rate_validator = MultiValidator::new()
        .add_validator("positive_check", PositiveValidator::new())
        .add_validator("range_check", RangeValidator::new().min(0.0).max(1.0)); // 0% to 100%

    // Test with valid input
    let valid_rate = 0.05; // 5%
    let errors = rate_validator.validate_all(&valid_rate);
    println!("   Valid rate (5%): {:?}", errors.status());

    // Test with invalid input
    let invalid_rate = -0.02; // -2%
    let errors = rate_validator.validate_all(&invalid_rate);
    println!("   Invalid rate (-2%): {:?}", errors.status());
    if errors.has_errors() {
        println!("   Errors:");
        for (field, message) in &errors.errors {
            println!("     - {}: {}", field, message);
        }
    }

    println!();
}

// Helper struct to demonstrate the validator trait implementation
struct PositiveValidator;

impl Validator for PositiveValidator {
    type Input = f64;
    type Output = f64;

    fn validate(&self, input: &Self::Input) -> ValidationResult<Self::Output> {
        if *input > 0.0 {
            ValidationResult::pass(*input)
        } else {
            ValidationResult::fail("Value must be positive")
        }
    }

    fn description(&self) -> Option<&'static str> {
        Some("Positive value validator")
    }
}

impl PositiveValidator {
    fn new() -> Self {
        Self
    }
}
