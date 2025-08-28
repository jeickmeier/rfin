#![cfg(test)]

use finstack_valuations::metrics::standard_registry;

#[test]
fn debug_registry_contents() {
    let registry = standard_registry();
    
    // Get all available metrics
    let all_metrics = registry.available_metrics();
    println!("All available metrics in registry: {:?}", all_metrics);
    println!("Total count: {}", all_metrics.len());
    
    // Check specifically for par_rate
    assert!(registry.has_metric("par_rate"), "par_rate metric not registered!");
    
    // Get IRS-specific metrics
    let irs_metrics = registry.metrics_for_instrument("IRS");
    println!("\nMetrics for IRS: {:?}", irs_metrics);
    println!("IRS metric count: {}", irs_metrics.len());
    assert!(irs_metrics.contains(&"par_rate".to_string()), "par_rate not marked as applicable to IRS!");
}
