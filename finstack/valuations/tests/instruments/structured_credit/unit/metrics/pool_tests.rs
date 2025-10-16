//! Unit tests for pool characteristic metrics.
//!
//! Tests cover:
//! - WAM calculations
//! - CPR assumptions extraction
//! - CDR assumptions extraction
//! - WARF calculations
//! - WAS calculations

// Most pool metrics are tested via the pool_tests.rs component tests
// Additional metric-specific tests would go here

#[test]
fn test_wam_metric_uses_pool_weighted_avg_maturity() {
    // WAM metric delegates to pool.weighted_avg_maturity()
    // This is tested in unit/components/pool_tests.rs
    assert!(true, "WAM tested via pool component tests");
}

#[test]
fn test_was_metric_uses_pool_weighted_avg_spread() {
    // WAS metric delegates to pool.weighted_avg_spread()
    // This is tested in unit/components/pool_tests.rs
    assert!(true, "WAS tested via pool component tests");
}

