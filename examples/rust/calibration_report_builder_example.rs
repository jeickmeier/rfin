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
    
    let report1 = CalibrationReport::success_simple(residuals.clone(), 15);
    println!("1. Simple Success Report:");
    println!("   Success: {}", report1.success);
    println!("   Iterations: {}", report1.iterations);
    println!("   Max Residual: {:.6}", report1.max_residual);
    println!("   RMSE: {:.6}", report1.rmse);
    println!("   Reason: {}\n", report1.convergence_reason);

    // Example 2: Type-specific calibration report
    let report2 = CalibrationReport::for_type("yield_curve", residuals.clone(), 20)
        .with_metadata("currency", "USD")
        .with_metadata("interpolation", "LogLinear");
    
    println!("2. Typed Calibration Report:");
    println!("   Success: {}", report2.success);
    println!("   Iterations: {}", report2.iterations);
    println!("   Reason: {}", report2.convergence_reason);
    println!("   Metadata: {:?}\n", report2.metadata);

    // Example 3: Empty success (no quotes to calibrate)
    let report3 = CalibrationReport::empty_success("No inflation quotes provided");
    
    println!("3. Empty Success Report:");
    println!("   Success: {}", report3.success);
    println!("   Iterations: {}", report3.iterations);
    println!("   Reason: {}\n", report3.convergence_reason);

    // Example 4: Failure report
    let report4 = CalibrationReport::failure_simple("Maximum iterations reached", 100);
    
    println!("4. Failure Report:");
    println!("   Success: {}", report4.success);
    println!("   Iterations: {}", report4.iterations);
    println!("   Reason: {}\n", report4.convergence_reason);

    // Example 5: Complex report with metadata batch
    let metadata_entries = vec![
        ("entity", "APPLE_INC"),
        ("seniority", "Senior"),
        ("recovery_rate", "0.4"),
        ("discount_curve", "USD-OIS"),
    ];
    
    let report5 = CalibrationReport::for_type("hazard_curve", residuals.clone(), 25)
        .with_metadata_batch(metadata_entries)
        .with_objective_value(1.5e-8);
    
    println!("5. Complex Report with Metadata:");
    println!("   Success: {}", report5.success);
    println!("   Iterations: {}", report5.iterations);
    println!("   Objective: {:.2e}", report5.objective_value);
    println!("   Metadata: {:?}\n", report5.metadata);

    // Example 6: Report with dynamic residual building
    let mut report6 = CalibrationReport::success_simple(BTreeMap::new(), 0);
    
    // Dynamically add residuals
    report6.push_residual("VOL_1M_ATM", 0.001);
    report6.push_residual("VOL_3M_ATM", -0.0005);
    report6.push_residual("VOL_6M_ATM", 0.0008);
    
    println!("6. Dynamic Residual Building:");
    println!("   Success: {}", report6.success);
    println!("   Residuals Count: {}", report6.residuals.len());
    println!("   Max Residual: {:.6}", report6.max_residual);
    println!("   RMSE: {:.6}", report6.rmse);
}
