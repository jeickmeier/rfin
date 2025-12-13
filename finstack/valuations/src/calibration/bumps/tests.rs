use crate::calibration::bumps::BumpRequest;

// Simple test to ensure BumpRequest types are usable and basic logic compiles/runs.
// Comprehensive verification is done in scenarios integration tests.

#[test]
fn test_bump_request_creation() {
    let parallel = BumpRequest::Parallel(10.0);
    if let BumpRequest::Parallel(bp) = parallel {
        assert_eq!(bp, 10.0);
    } else {
        panic!("Wrong variant");
    }

    let tenors = BumpRequest::Tenors(vec![(1.0, 10.0), (5.0, 20.0)]);
    if let BumpRequest::Tenors(targets) = tenors {
        assert_eq!(targets.len(), 2);
    } else {
        panic!("Wrong variant");
    }
}

// Additional unit tests for specific logic (e.g. synthetic swap construction)
// would go here. For now, we rely on the integration tests in scenarios
// which exercise the full calibration loop.
