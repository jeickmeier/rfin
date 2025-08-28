#![cfg(test)]

use finstack_valuations::metrics::{ParRateCalculator, MetricCalculator};

#[test]  
fn test_par_rate_calculator_direct() {
    let calc = ParRateCalculator;
    
    // Check ID
    assert_eq!(calc.id(), "par_rate");
    
    // Check applicability
    assert!(calc.is_applicable("IRS"), "ParRateCalculator should be applicable to IRS");
    assert!(!calc.is_applicable("Bond"), "ParRateCalculator should not be applicable to Bond");
    
    // Check dependencies
    let deps = calc.dependencies();
    assert_eq!(deps, vec!["annuity"]);
    
    println!("ParRateCalculator direct test passed!");
}
