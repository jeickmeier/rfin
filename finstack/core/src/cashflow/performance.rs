//! Performance measurement utilities for investment analysis.
//!
//! This module provides industry-standard metrics for evaluating investment
//! returns and comparing alternative cash flow streams. All implementations
//! use numerically robust solvers to handle edge cases.
//!
//! # Metrics
//!
//! ## Net Present Value (NPV)
//!
//! Discounts future cashflows to present value using a specified rate.
//! Fundamental to capital budgeting and project evaluation.
//!
//! ## Internal Rate of Return (IRR)
//!
//! The discount rate that makes NPV = 0. Represents the break-even yield
//! of an investment. For periodic cashflows only.
//!
//! ## Extended IRR (XIRR)
//!
//! Generalization of IRR for irregular cashflow dates. Uses actual date
//! arithmetic and day count conventions.
//!
//! # Mathematical Foundation
//!
//! NPV for irregular cashflows:
//! ```text
//! NPV = Σ CF_i * DF(t_i)
//! where DF(t) = (1 + r)^t
//! ```
//!
//! IRR is the rate r such that NPV(r) = 0:
//! ```text
//! Σ CF_i / (1 + r)^i = 0  (periodic)
//! Σ CF_i / (1 + r)^t_i = 0  (irregular, XIRR)
//! ```
//!
//! # Numerical Considerations
//!
//! - IRR may not exist if cashflows don't change sign
//! - Multiple IRRs possible for non-conventional cashflow patterns
//! - Uses hybrid Newton-Brent solver for robust convergence
//! - Initial guess impacts convergence speed
//!
//! # Examples
//!
//! ```rust
//! use finstack_core::cashflow::performance::{irr_periodic, npv};
//! use finstack_core::dates::{Date, DayCount};
//! use time::Month;
//!
//! // IRR for periodic cashflows
//! let amounts = vec![-1000.0, 300.0, 400.0, 500.0, 600.0];
//! let irr = irr_periodic(&amounts, None)?;
//!
//! // NPV at 5% discount rate
//! let base = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
//! let cf1 = (Date::from_calendar_date(2026, Month::January, 1).expect("Valid date"), 1050.0);
//! let pv = npv(&[cf1], 0.05, Some(base), Some(DayCount::Act365F))?;
//! assert!((pv - 1000.0).abs() < 1.0); // ~= 1050 / 1.05
//! # Ok::<(), finstack_core::Error>(())
//! ```
//!
//! # References
//!
//! - **NPV and IRR Theory**:
//!   - Brealey, R. A., Myers, S. C., & Allen, F. (2020). *Principles of Corporate Finance*
//!     (13th ed.). McGraw-Hill. Chapters 5-6.
//!   - Ross, S. A., Westerfield, R. W., & Jaffe, J. (2019). *Corporate Finance*
//!     (12th ed.). McGraw-Hill. Chapter 5.
//!
//! - **IRR Calculation**:
//!   - Lin, S. A. (1976). "The Modified Internal Rate of Return and Investment Criterion."
//!     *The Engineering Economist*, 21(4), 237-247.
//!   - Shull, D. M. (1992). "Overall Rates of Return: Investment Bases, Reinvestment Rates
//!     and Time Horizons." *The Engineering Economist*, 38(1), 1-21.
//!
//! - **XIRR**:
//!   - Microsoft Excel XIRR function documentation (industry standard implementation)

use crate::dates::{Date, DayCount, DayCountCtx};
use crate::error::InputError;
use crate::math::solver::NewtonSolver;

/// Calculate Net Present Value (NPV) for irregular cashflows.
///
/// Computes the present value of a series of dated cashflows using a constant
/// discount rate and specified day count convention. This is the fundamental
/// valuation metric in capital budgeting and project evaluation.
///
/// # Mathematical Definition
///
/// ```text
/// NPV = Σ CF_i / (1 + r)^t_i
///
/// where:
///   CF_i = cashflow i
///   r = discount rate (annual)
///   t_i = time to cashflow i in years (using day count convention)
/// ```
///
/// # Arguments
///
/// * `cash_flows` - Vector of (date, amount) tuples; negative = outflow, positive = inflow
/// * `discount_rate` - Annual discount rate as decimal (0.05 = 5%)
/// * `base_date` - Optional base date for discounting (defaults to first cashflow date)
/// * `day_count` - Optional day count convention (defaults to Act/365F)
///
/// # Returns
///
/// The net present value as of the base date
///
/// # Decision Rule
///
/// - NPV > 0: Project adds value, accept
/// - NPV = 0: Project breaks even, indifferent
/// - NPV < 0: Project destroys value, reject
///
/// # Examples
///
/// ```rust
/// use finstack_core::cashflow::performance::npv;
/// use finstack_core::dates::{Date, DayCount};
/// use time::Month;
///
/// let base = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
///
/// // Project: -$100k upfront, +$110k in 1 year
/// let cashflows = vec![
///     (base, -100_000.0),
///     (Date::from_calendar_date(2026, Month::January, 1).expect("Valid date"), 110_000.0),
/// ];
///
/// // NPV at 5% discount rate
/// let pv = npv(&cashflows, 0.05, Some(base), Some(DayCount::Act365F))?;
/// assert!(pv > 4_000.0); // NPV ≈ -100k + 110k/1.05 ≈ 4.76k
/// # Ok::<(), finstack_core::Error>(())
/// ```
///
/// # Errors
///
/// Returns error if:
/// - Cash flows vector is empty
/// - Day count calculation fails (invalid dates)
///
/// # Notes
/// - Dates prior to `base_date` are allowed; corresponding year-fractions are treated as negative.
///
/// # References
///
/// - Brealey, R. A., Myers, S. C., & Allen, F. (2020). *Principles of Corporate Finance*
///   (13th ed.). Chapter 5.
/// - Ross, S. A., Westerfield, R. W., & Jaffe, J. (2019). *Corporate Finance*
///   (12th ed.). Chapter 5.
pub fn npv(
    cash_flows: &[(Date, f64)],
    discount_rate: f64,
    base_date: Option<Date>,
    day_count: Option<DayCount>,
) -> crate::Result<f64> {
    if cash_flows.is_empty() {
        return Err(InputError::TooFewPoints.into());
    }

    let base = base_date.unwrap_or(cash_flows[0].0);
    let dc = day_count.unwrap_or(DayCount::Act365F);
    let mut sum = 0.0;

    for (date, amount) in cash_flows {
        let years = if *date == base {
            0.0
        } else if *date > base {
            dc.year_fraction(base, *date, DayCountCtx::default())
                .map_err(|e| {
                    crate::Error::Validation(format!("Day count calculation failed: {}", e))
                })?
        } else {
            -dc.year_fraction(*date, base, DayCountCtx::default())
                .map_err(|e| {
                    crate::Error::Validation(format!("Day count calculation failed: {}", e))
                })?
        };
        let discount_factor = (1.0 + discount_rate).powf(years);
        sum += amount / discount_factor;
    }

    Ok(sum)
}

/// Calculate Internal Rate of Return (IRR) for evenly-spaced periodic cashflows.
///
/// Computes the discount rate that makes the NPV of periodic cashflows equal to zero.
/// This represents the effective compound return of an investment assuming reinvestment
/// at the IRR. Cashflows are assumed to occur at periods 0, 1, 2, ... with equal spacing.
///
/// # Mathematical Definition
///
/// IRR is the rate r that solves:
/// ```text
/// Σ CF_i / (1 + r)^i = 0
///
/// where:
///   CF_i = cashflow at period i
///   i = period index (0, 1, 2, ...)
/// ```
///
/// # Arguments
///
/// * `amounts` - Array of cashflow amounts; negative = outflow, positive = inflow
/// * `guess` - Optional initial guess for IRR (defaults to 0.1 = 10% per period)
///
/// # Returns
///
/// The IRR as a decimal per period (e.g., 0.025 = 2.5% per period).
/// For annual IRR from quarterly returns: `(1 + quarterly_irr)^4 - 1`
///
/// # Decision Rule
///
/// Compare IRR to required return (hurdle rate):
/// - IRR > hurdle: Project adds value, accept
/// - IRR = hurdle: Project breaks even, indifferent
/// - IRR < hurdle: Project destroys value, reject
///
/// # Limitations
///
/// - **Reinvestment assumption**: Implicitly assumes cashflows are reinvested at IRR
/// - **Multiple IRRs**: Non-conventional cashflows (multiple sign changes) may have
///   multiple solutions or no solution
/// - **Scale blindness**: Cannot compare projects of different sizes using IRR alone
///
/// # Examples
///
/// ## Annual project evaluation
///
/// ```rust
/// use finstack_core::cashflow::performance::irr_periodic;
///
/// // Project: -$100k initial, then $30k/year for 5 years
/// let amounts = vec![-100_000.0, 30_000.0, 30_000.0, 30_000.0, 30_000.0, 30_000.0];
/// let irr = irr_periodic(&amounts, None)?;
///
/// // IRR should be around 15% annually
/// assert!(irr > 0.10 && irr < 0.20);
/// # Ok::<(), finstack_core::Error>(())
/// ```
///
/// ## Converting quarterly to annual IRR
///
/// ```rust
/// use finstack_core::cashflow::performance::irr_periodic;
///
/// // Quarterly cashflows
/// let amounts = vec![-10_000.0, 2_500.0, 2_500.0, 2_500.0, 2_500.0, 2_500.0];
/// let quarterly_irr = irr_periodic(&amounts, None)?;
///
/// // Convert to annual equivalent
/// let annual_irr = (1.0 + quarterly_irr).powi(4) - 1.0;
/// # Ok::<(), finstack_core::Error>(())
/// ```
///
/// # Errors
///
/// Returns error if:
/// - Less than 2 cashflows provided
/// - All cashflows have the same sign (no investment/return pattern)
/// - Solver fails to converge within tolerance
///
/// # References
///
/// - **IRR Theory**:
///   - Brealey, R. A., Myers, S. C., & Allen, F. (2020). *Principles of Corporate Finance*
///     (13th ed.). McGraw-Hill. Chapter 5.
///   - Ross, S. A., Westerfield, R. W., & Jaffe, J. (2019). *Corporate Finance*
///     (12th ed.). Chapter 5.
///
/// - **IRR Pitfalls**:
///   - Lin, S. A. (1976). "The Modified Internal Rate of Return and Investment Criterion."
///     *The Engineering Economist*, 21(4), 237-247.
///   - Hazen, G. B. (2003). "A New Perspective on Multiple Internal Rates of Return."
///     *The Engineering Economist*, 48(1), 31-51.
pub fn irr_periodic(amounts: &[f64], guess: Option<f64>) -> crate::Result<f64> {
    // Validate inputs
    if amounts.len() < 2 {
        return Err(InputError::TooFewPoints.into());
    }

    // Check for sign change
    let has_positive = amounts.iter().any(|&x| x > 0.0);
    let has_negative = amounts.iter().any(|&x| x < 0.0);
    if !has_positive || !has_negative {
        return Err(InputError::Invalid.into());
    }

    // NPV function for periodic cash flows
    let npv = |rate: f64| -> f64 {
        amounts
            .iter()
            .enumerate()
            .map(|(i, &amount)| amount / (1.0 + rate).powi(i as i32))
            .sum()
    };

    // Analytic derivative of NPV with respect to rate
    // d/dr [ Σ CF_i / (1 + r)^i ] = Σ -i * CF_i / (1 + r)^(i + 1)
    let npv_derivative = |rate: f64| -> f64 {
        amounts
            .iter()
            .enumerate()
            .map(|(i, &amount)| {
                let i_f64 = i as f64;
                -i_f64 * amount / (1.0 + rate).powf(i_f64 + 1.0)
            })
            .sum()
    };

    let initial_guess = guess.unwrap_or(0.1);
    let solver = NewtonSolver::new()
        .with_tolerance(1e-6)
        .with_max_iterations(100);

    // Try the user-provided guess first, then fall back to a small set of seeds to
    // improve robustness near challenging regions (e.g., r ≈ -1.0 or large r).
    let seeds: &[f64] = &[
        initial_guess,
        -0.99,
        -0.9,
        -0.75,
        -0.5,
        -0.25,
        -0.05,
        0.01,
        0.05,
        0.1,
        0.2,
        0.5,
        1.0,
        2.0,
    ];
    for &g in seeds {
        if let Ok(root) = solver.solve_with_derivative(npv, npv_derivative, g) {
            return Ok(root);
        }
    }
    Err(crate::Error::Validation(
        "IRR calculation failed: no convergence".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dates::create_date;
    use time::Month;

    #[test]
    fn test_npv_simple() {
        let flows = vec![
            (
                create_date(2024, Month::January, 1).expect("Valid test date"),
                -100000.0,
            ),
            (
                create_date(2025, Month::January, 1).expect("Valid test date"),
                110000.0,
            ),
        ];
        let npv_5pct =
            npv(&flows, 0.05, None, None).expect("NPV calculation should succeed in test");
        // NPV should be positive (profitable at 5% discount rate)
        // Approximately: -100000 + 110000/(1.05) ≈ 4761.90
        assert!(npv_5pct > 4700.0 && npv_5pct < 4800.0);
    }

    #[test]
    fn test_npv_zero_discount() {
        let flows = vec![
            (
                create_date(2024, Month::January, 1).expect("Valid test date"),
                -100.0,
            ),
            (
                create_date(2025, Month::January, 1).expect("Valid test date"),
                100.0,
            ),
        ];
        let npv_zero =
            npv(&flows, 0.0, None, None).expect("NPV calculation should succeed in test");
        assert_eq!(npv_zero, 0.0);
    }

    #[test]
    fn test_npv_allows_past_and_future_dates() {
        let base = create_date(2025, Month::January, 1).expect("Valid test date");
        let flows = vec![
            (
                create_date(2024, Month::July, 1).expect("Valid test date"),
                -50.0,
            ), // past relative to base
            (
                create_date(2025, Month::July, 1).expect("Valid test date"),
                55.0,
            ), // future relative to base
        ];
        // Should not error; just compute signed year fractions
        let pv = npv(&flows, 0.05, Some(base), Some(DayCount::Act365F))
            .expect("NPV calculation should succeed in test");
        // With positive rate and inflow slightly bigger than outflow, PV should be > 0
        assert!(pv > 0.0);
    }

    #[test]
    fn test_npv_errors_on_empty_flows_now() {
        let flows: Vec<(Date, f64)> = vec![];
        let err = npv(&flows, 0.05, None, None).expect_err("Should fail with empty flows");
        let _ = format!("{}", err);
    }

    #[test]
    fn test_irr_periodic() {
        // Simple case: invest 100, get 110 back after 1 period
        let amounts = vec![-100.0, 110.0];
        let irr = irr_periodic(&amounts, None).expect("IRR calculation should succeed in test");
        assert!((irr - 0.1).abs() < 1e-6); // 10% return
    }

    #[test]
    fn test_irr_periodic_multiple_periods() {
        // Invest 1000, receive 300 per period for 4 periods
        let amounts = vec![-1000.0, 300.0, 300.0, 300.0, 300.0];
        let irr = irr_periodic(&amounts, None).expect("IRR calculation should succeed in test");
        // Should be close to 7.71% per period
        assert!(irr > 0.07 && irr < 0.08);
    }

    #[test]
    fn test_irr_periodic_near_minus_100() {
        // Invest 100, get 1 back after one period → IRR close to -99%
        let amounts = vec![-100.0, 1.0];
        let irr =
            irr_periodic(&amounts, Some(-0.5)).expect("IRR calculation should succeed in test");
        assert!(irr < -0.9);
        // NPV at computed IRR should be ~0
        let f = |r: f64| {
            amounts
                .iter()
                .enumerate()
                .map(|(i, &a)| a / (1.0 + r).powi(i as i32))
                .sum::<f64>()
        };
        assert!(f(irr).abs() < 1e-6);
    }

    #[test]
    fn test_irr_periodic_high_positive() {
        // Invest 100, get 300 next period → ~200% IRR
        let amounts = vec![-100.0, 300.0];
        let irr =
            irr_periodic(&amounts, Some(0.5)).expect("IRR calculation should succeed in test");
        assert!(irr > 1.0);
        let f = |r: f64| {
            amounts
                .iter()
                .enumerate()
                .map(|(i, &a)| a / (1.0 + r).powi(i as i32))
                .sum::<f64>()
        };
        assert!(f(irr).abs() < 1e-6);
    }

    #[test]
    fn test_irr_periodic_no_sign_change() {
        let amounts = vec![100.0, 200.0, 300.0];
        assert!(irr_periodic(&amounts, None).is_err());
    }
}
