//! Volatility surface arbitrage detection framework.
//!
//! Provides model-free arbitrage detection for any [`VolSurface`] regardless
//! of how it was constructed (market quotes, SABR, SVI, etc.), plus
//! SVI-specific checks that operate on raw calibrated parameters.
//!
//! # Architecture
//!
//! - **Types** ([`types`]): Violation taxonomy, severity model, and report
//! - **Checks** ([`checks`]): Composable [`ArbitrageCheck`] trait with
//!   implementations for butterfly, calendar spread, and local vol density
//! - **SVI checks** ([`checks::svi`]): Moment bounds, Gatheral-Jacquier
//!   density, and cross-slice calendar spread for SVI parameterizations
//! - **Orchestrator** ([`check_surface`]): Runs all enabled checks and
//!   aggregates results into an [`ArbitrageReport`]
//!
//! # Usage
//!
//! ```rust
//! use finstack_core::market_data::surfaces::VolSurface;
//! use finstack_core::market_data::arbitrage::{check_surface, ArbitrageCheckConfig};
//!
//! let surface = VolSurface::builder("TEST")
//!     .expiries(&[0.5, 1.0, 2.0])
//!     .strikes(&[80.0, 90.0, 100.0, 110.0, 120.0])
//!     .row(&[0.30, 0.25, 0.20, 0.22, 0.28])
//!     .row(&[0.28, 0.24, 0.19, 0.21, 0.26])
//!     .row(&[0.26, 0.22, 0.18, 0.20, 0.24])
//!     .build()
//!     .expect("surface should build");
//!
//! let report = check_surface(&surface, &ArbitrageCheckConfig {
//!     forward: Some(100.0),
//!     ..Default::default()
//! });
//!
//! if report.passed {
//!     println!("Surface is arbitrage-free");
//! } else {
//!     for v in &report.violations {
//!         println!("{}: {}", v.severity, v.description);
//!     }
//! }
//! ```

pub mod checks;
pub mod types;

pub use checks::{
    ArbitrageCheck, ButterflyCheck, CalendarSpreadCheck, LocalVolDensityCheck, SviArbitrageCheck,
};
pub use types::{
    ArbitrageReport, ArbitrageSeverity, ArbitrageType, ArbitrageViolation, ViolationLocation,
};

use crate::market_data::surfaces::VolSurface;
use std::collections::HashMap;

/// Configuration for the arbitrage detection suite.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ArbitrageCheckConfig {
    /// Run butterfly (strike convexity) check.
    pub check_butterfly: bool,
    /// Run calendar spread (expiry monotonicity) check.
    pub check_calendar_spread: bool,
    /// Run Dupire local vol density check.
    pub check_local_vol_density: bool,
    /// Forward price, required for local vol density check.
    /// If None, local vol density check is skipped.
    pub forward: Option<f64>,
    /// Tolerance for all checks (total-variance units).
    pub tolerance: f64,
    /// Minimum severity to include in the report.
    /// Violations below this severity are filtered out.
    pub min_severity: ArbitrageSeverity,
}

impl Default for ArbitrageCheckConfig {
    fn default() -> Self {
        Self {
            check_butterfly: true,
            check_calendar_spread: true,
            check_local_vol_density: true,
            forward: None,
            tolerance: 1e-10,
            min_severity: ArbitrageSeverity::Negligible,
        }
    }
}

/// Run the full arbitrage detection suite on a volatility surface.
///
/// This is the primary entry point for standalone arbitrage checking.
/// Returns a report aggregating all violations found.
///
/// # Arguments
///
/// * `surface` -- the volatility surface to check
/// * `config` -- which checks to run and their parameters
pub fn check_surface(surface: &VolSurface, config: &ArbitrageCheckConfig) -> ArbitrageReport {
    let start = std::time::Instant::now();
    let mut all_violations: Vec<ArbitrageViolation> = Vec::new();

    // Build per-expiry forward vector from the single forward value
    let forwards: Vec<f64> = config
        .forward
        .map(|f| vec![f; surface.expiries().len()])
        .unwrap_or_default();

    if config.check_butterfly && !forwards.is_empty() {
        let checker = ButterflyCheck {
            forwards: forwards.clone(),
            tolerance: config.tolerance,
        };
        all_violations.extend(checker.check(surface));
    }

    if config.check_calendar_spread && !forwards.is_empty() {
        let checker = CalendarSpreadCheck {
            forwards: forwards.clone(),
            tolerance: config.tolerance,
        };
        all_violations.extend(checker.check(surface));
    }

    if config.check_local_vol_density {
        if let Some(fwd) = config.forward {
            let checker = LocalVolDensityCheck {
                forward: fwd,
                tolerance: config.tolerance,
            };
            all_violations.extend(checker.check(surface));
        }
    }

    // Filter by minimum severity
    all_violations.retain(|v| v.severity >= config.min_severity);

    // Sort: critical first
    all_violations.sort_by(|a, b| b.severity.cmp(&a.severity));

    // Build aggregation maps
    let mut counts_by_type: HashMap<ArbitrageType, usize> = HashMap::new();
    let mut counts_by_severity: HashMap<ArbitrageSeverity, usize> = HashMap::new();
    for v in &all_violations {
        *counts_by_type.entry(v.violation_type).or_insert(0) += 1;
        *counts_by_severity.entry(v.severity).or_insert(0) += 1;
    }

    let passed = !all_violations
        .iter()
        .any(|v| v.severity > ArbitrageSeverity::Negligible);

    ArbitrageReport {
        surface_id: surface.id().to_string(),
        violations: all_violations,
        passed,
        counts_by_type,
        counts_by_severity,
        elapsed_us: start.elapsed().as_micros() as u64,
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use crate::math::volatility::svi::SviParams;

    // ---- Helper surface constructors ----

    /// Flat vol surface: constant 20% vol everywhere. Must pass all checks.
    fn flat_surface() -> VolSurface {
        VolSurface::builder("FLAT-20")
            .expiries(&[0.25, 0.5, 1.0, 2.0])
            .strikes(&[80.0, 90.0, 100.0, 110.0, 120.0])
            .row(&[0.20, 0.20, 0.20, 0.20, 0.20])
            .row(&[0.20, 0.20, 0.20, 0.20, 0.20])
            .row(&[0.20, 0.20, 0.20, 0.20, 0.20])
            .row(&[0.20, 0.20, 0.20, 0.20, 0.20])
            .build()
            .expect("flat surface should build")
    }

    /// Clean smile surface: strongly convex in strike (total variance is
    /// quadratic), term structure with total variance strictly increasing.
    /// Should pass butterfly and calendar spread checks.
    fn clean_smile_surface() -> VolSurface {
        // Build a smile where total variance w = vol^2 * T is strongly convex
        // in strike. Use a wider strike spacing and ensure d2w/dk2 > 0 by
        // making wing vols significantly higher.
        VolSurface::builder("CLEAN-SMILE")
            .expiries(&[0.5, 1.0, 2.0])
            .strikes(&[80.0, 90.0, 100.0, 110.0, 120.0])
            .row(&[0.35, 0.27, 0.20, 0.27, 0.35])
            .row(&[0.33, 0.26, 0.20, 0.26, 0.33])
            .row(&[0.31, 0.25, 0.20, 0.25, 0.31])
            .build()
            .expect("clean smile should build")
    }

    /// Surface with butterfly violation: non-convex strike profile at T=1.0.
    /// The middle strike has vol lower than what convexity allows.
    fn butterfly_violation_surface() -> VolSurface {
        // Create a concave kink at K=100 for T=1.0:
        // Total variance w = vol^2 * T should be concave at K=100.
        // Normal convex smile: 0.30, 0.25, 0.20, 0.25, 0.30
        // Violation smile:     0.22, 0.25, 0.30, 0.25, 0.22
        //   (this is concave: center is higher than neighbors)
        VolSurface::builder("BUTTERFLY-BAD")
            .expiries(&[0.5, 1.0])
            .strikes(&[80.0, 90.0, 100.0, 110.0, 120.0])
            .row(&[0.30, 0.25, 0.20, 0.25, 0.30]) // T=0.5 clean
            .row(&[0.22, 0.25, 0.30, 0.25, 0.22]) // T=1.0 concave in total var
            .build()
            .expect("butterfly violation surface should build")
    }

    /// Surface with calendar spread violation: T=2 has lower total variance
    /// than T=1 at certain strikes.
    fn calendar_spread_violation_surface() -> VolSurface {
        // T=1.0: vol = 0.25 -> total var = 0.0625
        // T=2.0: vol = 0.15 -> total var = 0.045 < 0.0625 => violation
        VolSurface::builder("CALENDAR-BAD")
            .expiries(&[0.5, 1.0, 2.0])
            .strikes(&[80.0, 90.0, 100.0, 110.0, 120.0])
            .row(&[0.30, 0.25, 0.20, 0.25, 0.30])
            .row(&[0.30, 0.25, 0.25, 0.25, 0.30]) // T=1.0 higher vol
            .row(&[0.20, 0.15, 0.15, 0.15, 0.20]) // T=2.0 much lower vol
            .build()
            .expect("calendar spread violation surface should build")
    }

    // ---- Tests ----

    #[test]
    fn flat_surface_passes_all_checks() {
        let surface = flat_surface();
        let config = ArbitrageCheckConfig {
            forward: Some(100.0),
            ..Default::default()
        };
        let report = check_surface(&surface, &config);

        assert!(
            report.passed,
            "Flat surface should pass all checks. Violations: {:?}",
            report
                .violations
                .iter()
                .map(|v| &v.description)
                .collect::<Vec<_>>()
        );
        assert!(
            report.violations.is_empty()
                || report
                    .violations
                    .iter()
                    .all(|v| v.severity == ArbitrageSeverity::Negligible),
            "Flat surface should have no violations above Negligible"
        );
    }

    #[test]
    fn clean_smile_passes_butterfly_and_calendar() {
        let surface = clean_smile_surface();
        let config = ArbitrageCheckConfig {
            forward: Some(100.0),
            min_severity: ArbitrageSeverity::Minor,
            ..Default::default()
        };
        let report = check_surface(&surface, &config);

        // Filter to non-density checks (butterfly and calendar only)
        let butterfly_violations: Vec<_> = report
            .violations
            .iter()
            .filter(|v| v.violation_type == ArbitrageType::Butterfly)
            .collect();
        let calendar_violations: Vec<_> = report
            .violations
            .iter()
            .filter(|v| v.violation_type == ArbitrageType::CalendarSpread)
            .collect();

        assert!(
            butterfly_violations.is_empty(),
            "Clean smile should pass butterfly check. Violations: {:?}",
            butterfly_violations
                .iter()
                .map(|v| &v.description)
                .collect::<Vec<_>>()
        );
        assert!(
            calendar_violations.is_empty(),
            "Clean smile should pass calendar spread check. Violations: {:?}",
            calendar_violations
                .iter()
                .map(|v| &v.description)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn butterfly_violation_detected() {
        let surface = butterfly_violation_surface();
        let config = ArbitrageCheckConfig {
            check_butterfly: true,
            check_calendar_spread: false,
            check_local_vol_density: false,
            forward: Some(100.0),
            min_severity: ArbitrageSeverity::Minor,
            ..Default::default()
        };
        let report = check_surface(&surface, &config);

        let butterfly_violations: Vec<_> = report
            .violations
            .iter()
            .filter(|v| v.violation_type == ArbitrageType::Butterfly)
            .collect();

        assert!(
            !butterfly_violations.is_empty(),
            "Should detect butterfly violation on concave smile"
        );

        // Violation should be at T=1.0 (the concave slice)
        assert!(
            butterfly_violations
                .iter()
                .any(|v| (v.location.expiry - 1.0).abs() < 0.01),
            "Butterfly violation should be at T=1.0"
        );
    }

    #[test]
    fn calendar_spread_violation_detected() {
        let surface = calendar_spread_violation_surface();
        let config = ArbitrageCheckConfig {
            check_butterfly: false,
            check_calendar_spread: true,
            check_local_vol_density: false,
            forward: Some(100.0),
            min_severity: ArbitrageSeverity::Negligible,
            ..Default::default()
        };
        let report = check_surface(&surface, &config);

        let calendar_violations: Vec<_> = report
            .violations
            .iter()
            .filter(|v| v.violation_type == ArbitrageType::CalendarSpread)
            .collect();

        assert!(
            !calendar_violations.is_empty(),
            "Should detect calendar spread violation"
        );

        // Violation should reference T=2.0
        assert!(
            calendar_violations
                .iter()
                .any(|v| (v.location.expiry - 2.0).abs() < 0.01),
            "Calendar violation should be at T=2.0"
        );

        // Should have adjacent_expiry pointing back to T=1.0
        assert!(
            calendar_violations.iter().any(|v| v
                .location
                .adjacent_expiry
                .is_some_and(|ae| (ae - 1.0).abs() < 0.01)),
            "Calendar violation should reference T=1.0 as adjacent expiry"
        );
    }

    #[test]
    fn single_expiry_surface_no_calendar_violations() {
        let surface = VolSurface::builder("SINGLE-EXP")
            .expiries(&[1.0])
            .strikes(&[80.0, 90.0, 100.0, 110.0, 120.0])
            .row(&[0.30, 0.25, 0.20, 0.25, 0.30])
            .build()
            .expect("single expiry surface should build");

        let config = ArbitrageCheckConfig {
            check_calendar_spread: true,
            check_butterfly: false,
            check_local_vol_density: false,
            forward: Some(100.0),
            ..Default::default()
        };
        let report = check_surface(&surface, &config);

        assert!(
            report.violations.is_empty(),
            "Single expiry surface should have no calendar spread violations"
        );
    }

    #[test]
    fn two_strikes_no_butterfly_violations() {
        let surface = VolSurface::builder("TWO-STRIKES")
            .expiries(&[1.0, 2.0])
            .strikes(&[90.0, 110.0])
            .row(&[0.25, 0.25])
            .row(&[0.24, 0.24])
            .build()
            .expect("two strikes surface should build");

        let config = ArbitrageCheckConfig {
            check_butterfly: true,
            check_calendar_spread: false,
            check_local_vol_density: false,
            forward: Some(100.0),
            ..Default::default()
        };
        let report = check_surface(&surface, &config);

        let butterfly_violations: Vec<_> = report
            .violations
            .iter()
            .filter(|v| v.violation_type == ArbitrageType::Butterfly)
            .collect();
        assert!(
            butterfly_violations.is_empty(),
            "Two-strike surface should skip butterfly check (needs 3+ strikes)"
        );
    }

    #[test]
    fn report_severity_filtering() {
        let surface = calendar_spread_violation_surface();
        let config_all = ArbitrageCheckConfig {
            min_severity: ArbitrageSeverity::Negligible,
            forward: Some(100.0),
            ..Default::default()
        };
        let report = check_surface(&surface, &config_all);

        // Should have some violations
        assert!(
            !report.violations.is_empty(),
            "Should have violations on dirty surface"
        );

        // Filtering at Critical should return fewer or equal
        let critical_only = report.above_severity(ArbitrageSeverity::Critical);
        assert!(
            critical_only.len() <= report.violations.len(),
            "Critical filter should return subset"
        );
    }

    #[test]
    fn report_has_correct_counts() {
        let surface = calendar_spread_violation_surface();
        let config = ArbitrageCheckConfig {
            check_butterfly: false,
            check_calendar_spread: true,
            check_local_vol_density: false,
            forward: Some(100.0),
            ..Default::default()
        };
        let report = check_surface(&surface, &config);

        let type_count = report.counts_by_type.get(&ArbitrageType::CalendarSpread);
        let total_calendar: usize = report
            .violations
            .iter()
            .filter(|v| v.violation_type == ArbitrageType::CalendarSpread)
            .count();

        assert_eq!(
            type_count.copied().unwrap_or(0),
            total_calendar,
            "counts_by_type should match actual violation count"
        );
    }

    #[test]
    fn config_skips_checks_when_no_forward() {
        let surface = flat_surface();
        let config = ArbitrageCheckConfig {
            check_butterfly: true,
            check_calendar_spread: true,
            check_local_vol_density: true,
            forward: None,
            ..Default::default()
        };
        let report = check_surface(&surface, &config);

        assert!(
            report.violations.is_empty(),
            "All checks requiring forward should be skipped when forward is None"
        );
    }

    // ---- SVI-specific tests ----

    #[test]
    fn svi_moment_bound_violation_detected() {
        // b*(1+rho) = 1.5 * (1 + 0.5) = 2.25 > 2 => violation
        let bad_params = SviParams {
            a: 0.04,
            b: 1.5,
            rho: 0.5,
            m: 0.0,
            sigma: 0.1,
        };

        let check = SviArbitrageCheck {
            expiries: vec![1.0],
            params: vec![bad_params],
            k_range: (-2.0, 2.0),
            n_samples: 100,
        };

        let violations = check.check_moment_bounds();
        assert!(
            !violations.is_empty(),
            "Should detect moment bound violation for b*(1+rho) > 2"
        );
        assert!(
            violations
                .iter()
                .any(|v| v.violation_type == ArbitrageType::SviMomentBound),
            "Violation should be SviMomentBound type"
        );
    }

    #[test]
    fn svi_clean_params_pass_all_checks() {
        let clean_params = SviParams {
            a: 0.04,
            b: 0.3,
            rho: -0.3,
            m: 0.0,
            sigma: 0.15,
        };
        clean_params.validate().expect("params should be valid");

        let check = SviArbitrageCheck {
            expiries: vec![1.0],
            params: vec![clean_params],
            k_range: (-1.0, 1.0),
            n_samples: 200,
        };

        let violations = check.check_all();
        assert!(
            violations.is_empty(),
            "Clean SVI params should pass all checks. Violations: {:?}",
            violations
                .iter()
                .map(|v| &v.description)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn svi_calendar_spread_violation_detected() {
        // First slice has higher total variance than second at some k
        let slice1 = SviParams {
            a: 0.10,
            b: 0.3,
            rho: -0.2,
            m: 0.0,
            sigma: 0.15,
        };
        let slice2 = SviParams {
            a: 0.02, // much lower a => lower total variance
            b: 0.2,
            rho: -0.2,
            m: 0.0,
            sigma: 0.15,
        };

        let check = SviArbitrageCheck {
            expiries: vec![1.0, 2.0],
            params: vec![slice1, slice2],
            k_range: (-1.0, 1.0),
            n_samples: 100,
        };

        let violations = check.check_calendar_spread();
        assert!(
            !violations.is_empty(),
            "Should detect SVI calendar spread violation when T2 slice has lower total variance"
        );
        assert!(
            violations
                .iter()
                .all(|v| v.violation_type == ArbitrageType::SviCalendarSpread),
            "All violations should be SviCalendarSpread type"
        );
    }

    #[test]
    fn report_serialization_round_trip() {
        let surface = calendar_spread_violation_surface();
        let config = ArbitrageCheckConfig {
            forward: Some(100.0),
            ..Default::default()
        };
        let report = check_surface(&surface, &config);

        // Serialize to JSON
        let json = serde_json::to_string(&report).expect("report should serialize");

        // Deserialize back
        let deserialized: ArbitrageReport =
            serde_json::from_str(&json).expect("report should deserialize");

        assert_eq!(report.surface_id, deserialized.surface_id);
        assert_eq!(report.passed, deserialized.passed);
        assert_eq!(report.violations.len(), deserialized.violations.len());
        assert_eq!(report.elapsed_us, deserialized.elapsed_us);
    }

    #[test]
    fn config_serialization_round_trip() {
        let config = ArbitrageCheckConfig {
            check_butterfly: true,
            check_calendar_spread: false,
            check_local_vol_density: true,
            forward: Some(100.0),
            tolerance: 1e-8,
            min_severity: ArbitrageSeverity::Minor,
        };

        let json = serde_json::to_string(&config).expect("config should serialize");
        let deserialized: ArbitrageCheckConfig =
            serde_json::from_str(&json).expect("config should deserialize");

        assert_eq!(config.check_butterfly, deserialized.check_butterfly);
        assert_eq!(
            config.check_calendar_spread,
            deserialized.check_calendar_spread
        );
        assert_eq!(config.forward, deserialized.forward);
        assert!((config.tolerance - deserialized.tolerance).abs() < 1e-14);
        assert_eq!(config.min_severity, deserialized.min_severity);
    }
}
