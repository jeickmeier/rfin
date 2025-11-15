//! Tests for sensitivity helper functions.

use crate::metrics::sensitivities::dv01::standard_ir_dv01_buckets;

/// Format bucket label from years (same logic used in multiple modules).
fn format_bucket_label(years: f64) -> String {
    if years < 1.0 {
        format!("{:.0}m", (years * 12.0).round())
    } else {
        format!("{:.0}y", years)
    }
}

#[test]
fn test_bucket_label_formatting() {
    assert_eq!(format_bucket_label(0.25), "3m");
    assert_eq!(format_bucket_label(0.5), "6m");
    assert_eq!(format_bucket_label(1.0), "1y");
    assert_eq!(format_bucket_label(5.0), "5y");
    assert_eq!(format_bucket_label(30.0), "30y");
}

#[test]
fn test_standard_ir_dv01_buckets() {
    let buckets = standard_ir_dv01_buckets();
    assert_eq!(buckets.len(), 11);
    assert_eq!(buckets[0], 0.25);
    assert_eq!(buckets[10], 30.0);
}

