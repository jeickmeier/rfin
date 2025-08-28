#![cfg(test)]

use finstack_valuations::metrics::{MetricRegistry, ParRateCalculator, MetricCalculator};
use std::sync::Arc;

#[test]
fn test_manual_registration() {
    let mut registry = MetricRegistry::new();
    
    // Register ParRateCalculator manually
    let calc = Arc::new(ParRateCalculator);
    println!("Before registration - calc.id(): {}", calc.id());
    println!("Before registration - calc.is_applicable('IRS'): {}", calc.is_applicable("IRS"));
    
    registry.register(calc.clone());
    
    // Check if it's registered
    assert!(registry.has_metric("par_rate"), "par_rate should be registered");
    
    // Check IRS metrics
    let irs_metrics = registry.metrics_for_instrument("IRS");
    println!("IRS metrics after manual registration: {:?}", irs_metrics);
    
    // This should work
    assert!(
        irs_metrics.contains(&"par_rate".to_string()), 
        "par_rate should be in IRS metrics! Got: {:?}", 
        irs_metrics
    );
}
