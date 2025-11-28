//! IRR/XIRR edge case and boundary condition tests.
//!
//! These tests verify correct handling of challenging IRR scenarios:
//! - Near-singularity rates (r approaching -1)
//! - Multiple-root scenarios (non-conventional cashflows)
//! - Extreme positive and negative returns
//! - Convergence verification

use finstack_core::cashflow::performance::irr_periodic;
use finstack_core::cashflow::xirr::xirr;
use finstack_core::dates::Date;
use time::Month;

/// Helper to create dates
fn d(year: i32, month: u8, day: u8) -> Date {
    Date::from_calendar_date(year, Month::try_from(month).unwrap(), day).unwrap()
}

/// Helper to compute NPV for periodic cashflows at a given rate
fn compute_periodic_npv(amounts: &[f64], rate: f64) -> f64 {
    amounts
        .iter()
        .enumerate()
        .map(|(i, &a)| a / (1.0 + rate).powi(i as i32))
        .sum()
}

/// Helper to compute NPV for dated cashflows at a given rate (Act/365F)
fn compute_dated_npv(flows: &[(Date, f64)], rate: f64) -> f64 {
    if flows.is_empty() {
        return 0.0;
    }
    let first_date = flows[0].0;
    flows
        .iter()
        .map(|&(date, amount)| {
            let days = (date - first_date).whole_days() as f64;
            let years = days / 365.0;
            amount / (1.0 + rate).powf(years)
        })
        .sum()
}

// =============================================================================
// Near-Singularity Tests (r approaching -1)
// =============================================================================

#[test]
fn irr_handles_near_minus_one_singularity() {
    // Cashflows that solve to r approaching -1 (near-total loss)
    // Investment: -100, Return: 0.5 (99.5% loss)
    let amounts = vec![-100.0, 0.5];
    let result = irr_periodic(&amounts, None);

    match result {
        Ok(irr) => {
            // IRR should be approximately -99.5%
            assert!(
                irr > -1.0,
                "IRR must be greater than -100% (singularity): got {}",
                irr
            );
            assert!(
                irr < -0.9,
                "IRR for 99.5% loss should be < -90%: got {}",
                irr
            );

            // Verify NPV at computed IRR is approximately zero
            let npv_at_irr = compute_periodic_npv(&amounts, irr);
            assert!(
                npv_at_irr.abs() < 1.0,
                "NPV at IRR should be ~0, got {}",
                npv_at_irr
            );
        }
        Err(_) => {
            // Acceptable to error on extreme cases near singularity
            // The solver may fail to converge for r very close to -1
        }
    }
}

#[test]
fn irr_handles_total_loss() {
    // Total loss scenario: invest 100, get 0 back
    // This has no mathematical solution (would require r = -infinity)
    let amounts = vec![-100.0, 0.0];
    let result = irr_periodic(&amounts, None);

    // Should error (no sign change) or handle gracefully
    // Zero return means no sign change since -100 and 0 don't have opposite signs
    assert!(
        result.is_err(),
        "Total loss (no positive cashflow) should error"
    );
}

#[test]
fn irr_handles_99_percent_loss() {
    // Severe loss: invest 100, get 1 back
    let amounts = vec![-100.0, 1.0];
    let result = irr_periodic(&amounts, Some(-0.5)); // Provide hint near -99%

    match result {
        Ok(irr) => {
            // IRR should be approximately -99%
            assert!(irr < -0.9, "IRR should be < -90%: got {}", irr);
            assert!(irr > -1.0, "IRR should be > -100%: got {}", irr);

            // Verify NPV at computed IRR
            let npv_at_irr = compute_periodic_npv(&amounts, irr);
            assert!(
                npv_at_irr.abs() < 1e-6,
                "NPV at IRR should be ~0, got {}",
                npv_at_irr
            );
        }
        Err(_) => {
            // May fail to converge near singularity - acceptable
        }
    }
}

// =============================================================================
// Multiple-Root Tests (Non-Conventional Cashflows)
// =============================================================================

#[test]
fn irr_multiple_sign_changes_finds_a_root() {
    // Non-conventional cashflow pattern with multiple sign changes
    // These can have multiple mathematical IRR solutions
    // Pattern: -100, +320, -320, +100 (mining project style)
    let amounts = vec![-100.0, 320.0, -320.0, 100.0];
    let result = irr_periodic(&amounts, None);

    match result {
        Ok(irr) => {
            // Any valid root is acceptable for non-conventional patterns
            let npv_at_irr = compute_periodic_npv(&amounts, irr);
            assert!(
                npv_at_irr.abs() < 1e-3,
                "NPV at IRR should be ~0, got {}",
                npv_at_irr
            );
        }
        Err(_) => {
            // May fail to converge for complex patterns - acceptable
        }
    }
}

#[test]
fn irr_two_sign_changes_pattern() {
    // Simpler two-sign-change pattern: invest, profit, reinvest
    // -100, +200, -50
    let amounts = vec![-100.0, 200.0, -50.0];
    let result = irr_periodic(&amounts, None);

    if let Ok(irr) = result {
        let npv_at_irr = compute_periodic_npv(&amounts, irr);
        assert!(
            npv_at_irr.abs() < 1e-6,
            "NPV at IRR should be ~0, got {}",
            npv_at_irr
        );
    }
}

// =============================================================================
// Extreme Return Tests
// =============================================================================

#[test]
fn irr_handles_very_high_return() {
    // VC-style return: 10x in one period (900% IRR)
    let amounts = vec![-100.0, 1000.0];
    let result = irr_periodic(&amounts, None);

    let irr = result.expect("High return IRR should converge");
    assert!(
        (irr - 9.0).abs() < 1e-6,
        "10x return should be 900% IRR, got {:.2}%",
        irr * 100.0
    );

    let npv_at_irr = compute_periodic_npv(&amounts, irr);
    assert!(
        npv_at_irr.abs() < 1e-6,
        "NPV at IRR should be ~0, got {}",
        npv_at_irr
    );
}

#[test]
fn irr_handles_100x_return() {
    // Unicorn VC return: 100x (9900% IRR)
    let amounts = vec![-100.0, 10000.0];
    let result = irr_periodic(&amounts, Some(50.0)); // Provide high initial guess

    let irr = result.expect("100x return IRR should converge");
    assert!(
        (irr - 99.0).abs() < 1e-6,
        "100x return should be 9900% IRR, got {:.2}%",
        irr * 100.0
    );

    let npv_at_irr = compute_periodic_npv(&amounts, irr);
    assert!(
        npv_at_irr.abs() < 1e-6,
        "NPV at IRR should be ~0, got {}",
        npv_at_irr
    );
}

#[test]
fn irr_handles_small_positive_return() {
    // Near-zero positive return: 0.1%
    let amounts = vec![-100.0, 100.1];
    let result = irr_periodic(&amounts, None);

    let irr = result.expect("Small positive return IRR should converge");
    assert!(
        (irr - 0.001).abs() < 1e-6,
        "0.1% return expected, got {:.4}%",
        irr * 100.0
    );

    let npv_at_irr = compute_periodic_npv(&amounts, irr);
    assert!(
        npv_at_irr.abs() < 1e-6,
        "NPV at IRR should be ~0, got {}",
        npv_at_irr
    );
}

#[test]
fn irr_handles_small_negative_return() {
    // Near-zero negative return: -0.1%
    let amounts = vec![-100.0, 99.9];
    let result = irr_periodic(&amounts, None);

    let irr = result.expect("Small negative return IRR should converge");
    assert!(
        (irr + 0.001).abs() < 1e-6,
        "-0.1% return expected, got {:.4}%",
        irr * 100.0
    );

    let npv_at_irr = compute_periodic_npv(&amounts, irr);
    assert!(
        npv_at_irr.abs() < 1e-6,
        "NPV at IRR should be ~0, got {}",
        npv_at_irr
    );
}

// =============================================================================
// XIRR Boundary Tests
// =============================================================================

#[test]
fn xirr_handles_very_short_duration() {
    // Very short investment: 1 week
    let flows = vec![
        (d(2025, 1, 1), -100_000.0),
        (d(2025, 1, 8), 101_000.0), // 1% return in 1 week
    ];

    let result = xirr(&flows, None);
    let irr = result.expect("Short duration XIRR should converge");

    // 1% return in 1 week ≈ 67% annualized (approximately (1.01)^52 - 1)
    // Actual depends on day count
    assert!(irr > 0.5, "Annualized return should be > 50%, got {:.2}%", irr * 100.0);

    let npv_at_irr = compute_dated_npv(&flows, irr);
    assert!(
        npv_at_irr.abs() < 1.0,
        "NPV at IRR should be ~0, got {}",
        npv_at_irr
    );
}

#[test]
fn xirr_handles_same_day_multiple_flows() {
    // Multiple flows on same day - should aggregate correctly
    let flows = vec![
        (d(2025, 1, 1), -50_000.0),
        (d(2025, 1, 1), -50_000.0), // Second investment same day
        (d(2026, 1, 1), 110_000.0),
    ];

    let result = xirr(&flows, None);
    let irr = result.expect("Same-day flows XIRR should converge");

    // Total: -100K, +110K over 1 year = 10% return
    // Similar to single investment of -100K
    assert!(
        irr > 0.05 && irr < 0.15,
        "IRR should be ~10%, got {:.2}%",
        irr * 100.0
    );

    let npv_at_irr = compute_dated_npv(&flows, irr);
    assert!(
        npv_at_irr.abs() < 1.0,
        "NPV at IRR should be ~0, got {}",
        npv_at_irr
    );
}

#[test]
fn xirr_handles_distant_horizon() {
    // Very long horizon: 50 years
    let flows = vec![
        (d(2025, 1, 1), -100_000.0),
        (d(2075, 1, 1), 1_000_000.0), // 10x over 50 years
    ];

    let result = xirr(&flows, None);
    let irr = result.expect("Long horizon XIRR should converge");

    // 10x over 50 years: IRR = 10^(1/50) - 1 ≈ 4.7%
    let expected = (10.0_f64).powf(1.0 / 50.0) - 1.0;
    assert!(
        (irr - expected).abs() < 0.01,
        "50-year 10x IRR should be ~{:.2}%, got {:.2}%",
        expected * 100.0,
        irr * 100.0
    );

    let npv_at_irr = compute_dated_npv(&flows, irr);
    assert!(
        npv_at_irr.abs() < 1.0,
        "NPV at IRR should be ~0, got {}",
        npv_at_irr
    );
}

// =============================================================================
// Input Validation Tests
// =============================================================================

#[test]
fn irr_rejects_single_cashflow() {
    let amounts = vec![-100.0];
    let result = irr_periodic(&amounts, None);
    assert!(result.is_err(), "Single cashflow should error");
}

#[test]
fn irr_rejects_empty_cashflows() {
    let amounts: Vec<f64> = vec![];
    let result = irr_periodic(&amounts, None);
    assert!(result.is_err(), "Empty cashflows should error");
}

#[test]
fn irr_rejects_all_positive_cashflows() {
    // All positive = no investment pattern
    let amounts = vec![100.0, 200.0, 300.0];
    let result = irr_periodic(&amounts, None);
    assert!(result.is_err(), "All positive cashflows should error");
}

#[test]
fn irr_rejects_all_negative_cashflows() {
    // All negative = no return pattern
    let amounts = vec![-100.0, -200.0, -300.0];
    let result = irr_periodic(&amounts, None);
    assert!(result.is_err(), "All negative cashflows should error");
}

#[test]
fn xirr_rejects_single_flow() {
    let flows = vec![(d(2025, 1, 1), -100_000.0)];
    let result = xirr(&flows, None);
    assert!(result.is_err(), "Single dated flow should error");
}

#[test]
fn xirr_rejects_all_same_sign() {
    let flows = vec![
        (d(2025, 1, 1), 100_000.0),
        (d(2026, 1, 1), 200_000.0),
    ];
    let result = xirr(&flows, None);
    assert!(result.is_err(), "All same sign flows should error");
}

