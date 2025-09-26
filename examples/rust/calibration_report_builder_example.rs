//! Example demonstrating the CalibrationReport one-liner API

use finstack_valuations::calibration::CalibrationReport;
use std::collections::BTreeMap;

fn main() {
    println!("=== CalibrationReport One-Liner API Examples ===\n");

    // Example 1: Simple successful calibration (most common case)
    let mut residuals = BTreeMap::new();
    residuals.insert("USD_1Y".to_string(), 0.0001);
    residuals.insert("USD_2Y".to_string(), -0.0002);
    residuals.insert("USD_5Y".to_string(), 0.0001);

    let report1 = CalibrationReport::new(residuals.clone(), 15, true, "Yield curve calibration");
    println!("1. Simple Success Report:");
    println!("   Success: {}", report1.success);
    println!("   Iterations: {}", report1.iterations);
    println!("   Max Residual: {:.6}", report1.max_residual);
    println!("   RMSE: {:.6}", report1.rmse);
    println!("   Reason: {}\n", report1.convergence_reason);

    // Example 2: Type-specific calibration report
    let report2 = CalibrationReport::for_type("yield_curve", residuals.clone(), 20)
        .with_metadata("currency", "USD")
        .with_metadata("curve_id", "USD-OIS");

    println!("2. Typed Calibration Report:");
    println!("   Success: {}", report2.success);
    println!("   Iterations: {}", report2.iterations);
    println!("   Reason: {}", report2.convergence_reason);
    println!("   Metadata: {:?}\n", report2.metadata);

    // Example 3: Empty success (no quotes to calibrate)
    let report3 = CalibrationReport::success_empty("No inflation quotes provided");

    println!("3. Empty Success Report:");
    println!("   Success: {}", report3.success);
    println!("   Iterations: {}", report3.iterations);
    println!("   Reason: {}\n", report3.convergence_reason);

    // Example 4: Failure report
    let report4 = CalibrationReport::new(residuals.clone(), 100, false, "Maximum iterations reached")
        .with_metadata("solver", "Newton");

    println!("4. Failure Report:");
    println!("   Success: {}", report4.success);
    println!("   Iterations: {}", report4.iterations);
    println!("   Reason: {}\n", report4.convergence_reason);

    // Example 5: Complex report with metadata batch
    let report5 = CalibrationReport::for_type("hazard_curve", residuals.clone(), 25)
        .with_metadata("entity", "AAPL");

    println!("5. Complex Report with Metadata:");
    println!("   Success: {}", report5.success);
    println!("   Iterations: {}", report5.iterations);
    println!("   Objective: {:.2e}", report5.objective_value);
    println!("   Metadata: {:?}\n", report5.metadata);

    // Example 6: Report with dynamic residual building
    let mut report6 = CalibrationReport::new(BTreeMap::new(), 0, true, "Forward curves skipped");
    report6.metadata.insert("tenor".into(), "3M".into());

    println!("6. Dynamic Residual Building:");
    println!("   Success: {}", report6.success);
    println!("   Residuals Count: {}", report6.residuals.len());
    println!("   Max Residual: {:.6}", report6.max_residual);
    println!("   RMSE: {:.6}", report6.rmse);

    // Example 7: Complex report with dynamic metadata building
    let mut report7 = CalibrationReport::new(BTreeMap::new(), 5, true, "Credit spread calibration");
    for (key, value) in [
        ("entity", "APPLE_INC"),
        ("seniority", "Senior"),
        ("recovery_rate", "0.4"),
        ("discount_curve", "USD-OIS"),
    ] {
        report7.metadata.insert(key.into(), value.into());
    }

    println!("7. Complex Report with Dynamic Metadata:");
    println!("   Success: {}", report7.success);
    println!("   Iterations: {}", report7.iterations);
    println!("   Objective: {:.2e}", report7.objective_value);
    println!("   Metadata: {:?}\n", report7.metadata);
}
