//! Comprehensive example demonstrating the enhanced validation framework
//!
//! This example showcases the composable validation traits and standard validators
//! for financial computation, including:
//! - Monotonic term structure validation
//! - Discount factor range validation  
//! - Interest rate bounds validation
//! - Date range validation
//! - Chaining and combining validators

use finstack_core::{
    dates::{Date, Month},
    validation::{
        DateRangeValidator, DiscountFactorCurveValidator, DiscountFactorValidator,
        MonotonicTermStructureValidator, RateBoundsValidator, TermStructureKnotsValidator,
        ValidationResult, Validator, ValidatorExt,
    },
    F,
};
use time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Finstack Validation Framework Demo\n");

    // Example 1: Monotonic Term Structure Validation
    println!("📈 Example 1: Monotonic Term Structure Validation");
    
    let decreasing_validator = MonotonicTermStructureValidator::decreasing("Discount factors");
    
    // Valid decreasing sequence (typical discount factor curve)
    let valid_dfs = vec![1.0, 0.98, 0.95, 0.90, 0.85, 0.80];
    let result = decreasing_validator.validate(&valid_dfs);
    print_validation_result("Valid DF sequence", &result);
    
    // Invalid increasing sequence (arbitrage opportunity)
    let invalid_dfs = vec![0.80, 0.85, 0.90, 0.95]; // Arbitrage!
    let result = decreasing_validator.validate(&invalid_dfs);
    print_validation_result("Invalid DF sequence", &result);
    
    println!();

    // Example 2: Discount Factor Range Validation
    println!("💰 Example 2: Discount Factor Range Validation");
    
    let df_validator = DiscountFactorValidator::new();
    
    // Valid discount factors
    let valid_dfs = vec![1.0, 0.95, 0.90, 0.001]; // Very low but valid
    let result = df_validator.validate(&valid_dfs);
    print_validation_result("Valid DF ranges", &result);
    
    // Invalid discount factors
    let invalid_dfs = vec![1.0, 1.05, 0.95]; // DF > 1.0
    let result = df_validator.validate(&invalid_dfs);
    print_validation_result("Invalid DF ranges", &result);
    
    // Negative discount factors
    let negative_dfs = vec![1.0, -0.95, 0.90];
    let result = df_validator.validate(&negative_dfs);
    print_validation_result("Negative DFs", &result);
    
    println!();

    // Example 3: Combined Discount Factor Curve Validation
    println!("🔗 Example 3: Combined DF Curve Validation");
    
    let curve_validator = DiscountFactorCurveValidator::new();
    
    // Perfect discount factor curve
    let perfect_curve = vec![1.0, 0.98, 0.95, 0.90, 0.85];
    let result = curve_validator.validate(&perfect_curve);
    print_validation_result("Perfect DF curve", &result);
    
    // Curve with arbitrage
    let arbitrage_curve = vec![1.0, 0.95, 0.97, 0.90]; // Uptick = arbitrage
    let result = curve_validator.validate(&arbitrage_curve);
    print_validation_result("Arbitrage DF curve", &result);
    
    println!();

    // Example 4: Interest Rate Bounds Validation
    println!("📊 Example 4: Interest Rate Bounds Validation");
    
    let interest_validator = RateBoundsValidator::interest_rate();
    let credit_validator = RateBoundsValidator::credit_spread();
    let vol_validator = RateBoundsValidator::volatility();
    
    // Test various rates
    let rates = [0.05, 0.30, -0.05, 1.5]; // 5%, 30%, -5%, 150%
    
    for &rate in &rates {
        println!("  Rate: {:.2}%", rate * 100.0);
        
        let ir_result = interest_validator.validate(&rate);
        print_validation_result("    Interest", &ir_result);
        
        let cs_result = credit_validator.validate(&rate);
        print_validation_result("    Credit", &cs_result);
        
        let vol_result = vol_validator.validate(&rate);
        print_validation_result("    Volatility", &vol_result);
        
        println!();
    }

    // Example 5: Date Range Validation
    println!("📅 Example 5: Date Range Validation");
    
    let date_validator = DateRangeValidator::financial_instrument();
    
    let start = Date::from_calendar_date(2024, Month::January, 15)?;
    let end = Date::from_calendar_date(2024, Month::December, 15)?;
    
    // Valid range
    let result = date_validator.validate(&(start, end));
    print_validation_result("Valid date range", &result);
    
    // Invalid inverted range
    let result = date_validator.validate(&(end, start));
    print_validation_result("Inverted date range", &result);
    
    // Weekend dates (should warn)
    let weekend_start = Date::from_calendar_date(2024, Month::January, 13)?; // Saturday
    let result = date_validator.validate(&(weekend_start, end));
    print_validation_result("Weekend start date", &result);
    
    println!();

    // Example 6: Term Structure Knots Validation
    println!("⏰ Example 6: Term Structure Knots Validation");
    
    let knots_validator = TermStructureKnotsValidator::standard_curve();
    
    // Valid time points
    let valid_knots = vec![0.0, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0];
    let result = knots_validator.validate(&valid_knots);
    print_validation_result("Valid time points", &result);
    
    // Invalid non-increasing
    let invalid_knots = vec![0.0, 1.0, 0.5, 2.0];
    let result = knots_validator.validate(&invalid_knots);
    print_validation_result("Non-increasing knots", &result);
    
    println!();

    // Example 7: Validator Chaining and Composition
    println!("🔗 Example 7: Validator Chaining");
    
    // Chain knots validation with monotonic DF validation
    let comprehensive_validator = TermStructureKnotsValidator::standard_curve()
        .and_then(|_knots| {
            // In a real scenario, you'd validate the corresponding DFs
            MonotonicTermStructureValidator::decreasing("Chained DFs")
        });
    
    // This is a simplified example - normally you'd have corresponding DF data
    let knots = vec![0.0, 1.0, 2.0, 5.0];
    let result = comprehensive_validator.validate(&knots);
    print_validation_result("Chained validation", &result);
    
    println!();

    // Example 8: Validation with Transformations
    println!("🔄 Example 8: Validation with Transformations");
    
    let transform_validator = RateBoundsValidator::interest_rate()
        .map(|rate| rate * 100.0) // Convert to percentage
        .with_condition(|&pct| pct >= 0.0, "Percentage must be non-negative");
    
    let rate = 0.05; // 5%
    let result = transform_validator.validate(&rate);
    print_validation_result("Rate to percentage", &result);
    println!("    Transformed value: {:.2}%", result.value().unwrap_or(0.0));
    
    println!();

    // Example 9: Real-world Curve Validation Pipeline
    println!("🏭 Example 9: Real-world Curve Validation");
    
    // Simulate building a discount curve with full validation
    let time_points = vec![0.0, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0];
    let discount_factors = vec![1.0, 0.9875, 0.975, 0.95, 0.90, 0.75, 0.60];
    
    // Validate time points
    let knots_result = TermStructureKnotsValidator::standard_curve()
        .validate(&time_points);
    
    if knots_result.is_success() {
        println!("✅ Time points validation passed");
        
        // Validate discount factors
        let dfs_result = DiscountFactorCurveValidator::new()
            .validate(&discount_factors);
        
        match dfs_result.status() {
            finstack_core::validation::ValidationStatus::Pass => {
                println!("✅ Discount factor curve validation passed");
                println!("🎉 Curve is ready for use in pricing models!");
            }
            finstack_core::validation::ValidationStatus::Warning => {
                println!("⚠️  Discount factor curve validation passed with warnings:");
                for warning in dfs_result.warnings() {
                    println!("    - {}", warning);
                }
                println!("📊 Curve can be used but consider reviewing warnings.");
            }
            finstack_core::validation::ValidationStatus::Fail => {
                println!("❌ Discount factor curve validation failed:");
                if let Some(error) = dfs_result.error_message() {
                    println!("    {}", error);
                }
            }
        }
    } else {
        println!("❌ Time points validation failed - cannot proceed with curve construction");
    }

    Ok(())
}

/// Helper function to format and print validation results
fn print_validation_result<T>(label: &str, result: &ValidationResult<T>) {
    match result.status() {
        finstack_core::validation::ValidationStatus::Pass => {
            print!("  ✅ {}: PASS", label);
            if result.has_warnings() {
                print!(" (with {} warnings)", result.warnings().len());
            }
            println!();
        }
        finstack_core::validation::ValidationStatus::Warning => {
            println!("  ⚠️  {}: PASS with warnings", label);
            for warning in result.warnings() {
                println!("      - {}", warning);
            }
        }
        finstack_core::validation::ValidationStatus::Fail => {
            println!("  ❌ {}: FAIL", label);
            if let Some(error) = result.error_message() {
                println!("      {}", error);
            }
        }
    }
}
