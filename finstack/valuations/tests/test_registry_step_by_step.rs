#![cfg(test)]

use finstack_valuations::metrics::{MetricRegistry, bond_metrics, irs_metrics, deposit_metrics};

#[test]
fn test_registry_registration_steps() {
    let mut registry = MetricRegistry::new();
    
    println!("Initial registry - all metrics: {:?}", registry.available_metrics());
    println!("Initial registry - IRS metrics: {:?}", registry.metrics_for_instrument("IRS"));
    
    // Register bond metrics
    bond_metrics::register_bond_metrics(&mut registry);
    println!("\nAfter bond registration - all metrics: {:?}", registry.available_metrics());
    println!("After bond registration - IRS metrics: {:?}", registry.metrics_for_instrument("IRS"));
    
    // Register IRS metrics
    irs_metrics::register_irs_metrics(&mut registry);
    println!("\nAfter IRS registration - all metrics: {:?}", registry.available_metrics());
    println!("After IRS registration - IRS metrics: {:?}", registry.metrics_for_instrument("IRS"));
    assert!(registry.has_metric("par_rate"), "par_rate should be registered after IRS registration");
    
    // Check if par_rate is in IRS metrics
    let irs_metrics = registry.metrics_for_instrument("IRS");
    assert!(
        irs_metrics.contains(&"par_rate".to_string()),
        "par_rate should be in IRS metrics after IRS registration! Got: {:?}",
        irs_metrics
    );
    
    // Register deposit metrics
    deposit_metrics::register_deposit_metrics(&mut registry);
    println!("\nAfter deposit registration - all metrics: {:?}", registry.available_metrics());
    println!("After deposit registration - IRS metrics: {:?}", registry.metrics_for_instrument("IRS"));
    
    // Final check
    let final_irs_metrics = registry.metrics_for_instrument("IRS");
    assert!(
        final_irs_metrics.contains(&"par_rate".to_string()),
        "par_rate should still be in IRS metrics at the end! Got: {:?}",
        final_irs_metrics
    );
}
