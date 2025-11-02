//! Extended Internal Rate of Return (XIRR) for irregular cashflows.
//!
//! XIRR generalizes the Internal Rate of Return (IRR) to handle cashflows that
//! occur at irregular dates. This is the industry standard for measuring investment
//! performance when contributions and withdrawals happen at arbitrary times.
//!
//! # Financial Context
//!
//! While traditional IRR assumes evenly-spaced periodic cashflows, XIRR uses
//! actual dates and day count conventions to compute returns for:
//! - Private equity investments with capital calls and distributions
//! - Mutual fund portfolios with irregular contributions/withdrawals
//! - Real estate projects with variable payment schedules
//! - Venture capital fund performance measurement
//!
//! # Mathematical Definition
//!
//! XIRR is the annual rate r that solves:
//! ```text
//! Σ CF_i / (1 + r)^t_i = 0
//!
//! where:
//!   CF_i = cashflow i
//!   t_i = years from first cashflow to cashflow i (Act/365F convention)
//! ```
//!
//! # Industry Standard
//!
//! XIRR is the standard metric defined by:
//! - **CFA Institute**: Global Investment Performance Standards (GIPS®)
//! - **Microsoft Excel**: XIRR function (de facto industry standard)
//! - **Bloomberg**: IRR calculation for irregular cashflows
//!
//! # Implementation
//!
//! Uses Act/365F day count convention (matching Excel XIRR) and hybrid
//! Newton-Brent solver for robust convergence. Inputs are internally
//! sorted by date ascending and the earliest date is used as the base,
//! making results invariant to input order.
//!
//! # Examples
//!
//! ```rust
//! use finstack_core::cashflow::xirr;
//! use finstack_core::dates::Date;
//! use time::Month;
//!
//! // Private equity investment example
//! let cashflows = vec![
//!     (Date::from_calendar_date(2023, Month::January, 15).unwrap(), -100_000.0), // Initial
//!     (Date::from_calendar_date(2023, Month::June, 30).unwrap(), -50_000.0),     // Follow-on
//!     (Date::from_calendar_date(2024, Month::March, 15).unwrap(), 75_000.0),     // Partial exit
//!     (Date::from_calendar_date(2024, Month::December, 31).unwrap(), 95_000.0),  // Final exit
//! ];
//!
//! let return_rate = xirr(&cashflows, None)?;
//! assert!(return_rate > 0.0); // Positive return
//! # Ok::<(), finstack_core::Error>(())
//! ```
//!
//! # References
//!
//! - **XIRR Standard**:
//!   - Microsoft Excel XIRR function specification (industry de facto standard)
//!   - CFA Institute (2020). *Global Investment Performance Standards (GIPS®)*.
//!
//! - **Time-Weighted vs Money-Weighted Returns**:
//!   - Dietz, P. O. (1966). "Pension Funds: Measuring Investment Performance."
//!     *Free Press*.
//!   - CFA Institute (2019). "Calculating and Using Time-Weighted and Money-Weighted
//!     Rates of Return." CFA Program Curriculum, Level I.

use crate::dates::{Date, DayCount, DayCountCtx};
use crate::error::InputError;
use crate::math::solver::{HybridSolver, Solver};

/// Calculates XIRR (Extended Internal Rate of Return) for irregular cashflows.
///
/// XIRR finds the annualized discount rate that makes the net present value of
/// all cashflows equal to zero, accounting for the actual dates of each cashflow.
/// This is the standard metric for investment performance with irregular timing.
///
/// # Mathematical Definition
///
/// XIRR is the annual rate r that solves:
/// ```text
/// Σ CF_i / (1 + r)^t_i = 0
///
/// where:
///   CF_i = cashflow i (negative for investments, positive for returns)
///   t_i = year fraction from first cashflow to cashflow i (Act/365F)
/// ```
///
/// # Arguments
///
/// * `cash_flows` - Vector of (date, amount) tuples in any order (internally sorted; earliest date used as base)
/// * `guess` - Optional initial guess for IRR (defaults to 0.1 = 10% annual)
///
/// # Returns
///
/// Annual return as decimal (e.g., 0.15 for 15% per year)
///
/// # Day Count Convention
///
/// Uses Actual/365 Fixed to match Excel XIRR and industry practice.
///
/// # Examples
///
/// ## Mutual fund with irregular contributions
///
/// ```rust
/// use finstack_core::cashflow::xirr;
/// use finstack_core::dates::Date;
/// use time::Month;
///
/// let cashflows = vec![
///     (Date::from_calendar_date(2024, Month::January, 1).unwrap(), -10_000.0),
///     (Date::from_calendar_date(2024, Month::April, 15).unwrap(), -5_000.0),
///     (Date::from_calendar_date(2024, Month::October, 1).unwrap(), -3_000.0),
///     (Date::from_calendar_date(2025, Month::January, 1).unwrap(), 19_500.0),
/// ];
///
/// let annual_return = xirr(&cashflows, None)?;
/// assert!(annual_return > 0.0);
/// # Ok::<(), finstack_core::Error>(())
/// ```
///
/// ## Private equity fund
///
/// ```rust
/// use finstack_core::cashflow::xirr;
/// use finstack_core::dates::Date;
/// use time::Month;
///
/// // Capital calls and distributions
/// let pe_cashflows = vec![
///     (Date::from_calendar_date(2020, Month::March, 1).unwrap(), -1_000_000.0),  // Call 1
///     (Date::from_calendar_date(2020, Month::September, 1).unwrap(), -500_000.0), // Call 2
///     (Date::from_calendar_date(2022, Month::June, 15).unwrap(), 750_000.0),      // Dist 1
///     (Date::from_calendar_date(2024, Month::December, 31).unwrap(), 1_200_000.0), // Exit
/// ];
///
/// let fund_irr = xirr(&pe_cashflows, None)?;
/// # Ok::<(), finstack_core::Error>(())
/// ```
///
/// # Errors
///
/// Returns error if:
/// - Less than 2 cashflows provided
/// - All cashflows have the same sign (no investment/return pattern)
/// - Solver fails to converge (rare; try adjusting initial guess)
///
/// # Limitations
///
/// - **Reinvestment assumption**: Assumes intermediate cashflows reinvested at XIRR
/// - **Multiple solutions**: Non-conventional cashflows may have multiple IRRs
/// - **No solution**: Some cashflow patterns have no real IRR
///
/// # References
///
/// - **XIRR Standard**:
///   - Microsoft Excel XIRR function (industry de facto standard)
///   - CFA Institute (2020). *Global Investment Performance Standards (GIPS®)*.
///
/// - **Performance Measurement**:
///   - Dietz, P. O. (1966). *Pension Funds: Measuring Investment Performance*. Free Press.
///   - Bacon, C. R. (2008). *Practical Portfolio Performance Measurement and Attribution*
///     (2nd ed.). Wiley. Chapter 2.
pub fn xirr(cash_flows: &[(Date, f64)], guess: Option<f64>) -> crate::Result<f64> {
    // Validate inputs
    if cash_flows.len() < 2 {
        return Err(InputError::TooFewPoints.into());
    }

    // Check for sign change
    if !has_sign_change(cash_flows) {
        return Err(InputError::Invalid.into());
    }

    // Sort flows by date and use earliest as the base date (Excel/GIPS semantics).
    let mut flows = cash_flows.to_vec();
    flows.sort_by_key(|(d, _)| *d);

    let first_date = flows[0].0;
    let dc = DayCount::Act365F; // Standard day count for XIRR

    // Precompute (year_fraction, amount) once for performance and
    // propagate any day-count errors rather than masking/panicking.
    let mut years_and_amounts: Vec<(f64, f64)> = Vec::with_capacity(flows.len());
    for (date, amount) in flows.iter().copied() {
        let years = dc.year_fraction(first_date, date, DayCountCtx::default())?;
        years_and_amounts.push((years, amount));
    }

    // NPV function for root finding
    let npv = |rate: f64| -> f64 {
        let mut sum = 0.0;
        for &(years, amount) in &years_and_amounts {
            let discount = (1.0 + rate).powf(years);
            sum += amount / discount;
        }
        sum
    };

    // Use HybridSolver for Newton-Raphson with automatic Brent fallback
    // Choose an initial guess by evaluating a small grid if none provided.
    let initial_guess = match guess {
        Some(g) => g,
        None => {
            let candidates: &[f64] = &[-0.5, 0.01, 0.05, 0.1, 0.2, 0.5, 1.0];
            let mut best = 0.1;
            let mut best_abs = f64::INFINITY;
            for &g in candidates {
                let val = npv(g);
                if val.is_finite() {
                    let a = val.abs();
                    if a < best_abs {
                        best_abs = a;
                        best = g;
                    }
                }
            }
            best
        }
    };
    let solver = HybridSolver::new()
        .with_tolerance(1e-6)
        .with_max_iterations(100);

    solver.solve(npv, initial_guess)
}

/// Checks if cash flows have at least one sign change.
fn has_sign_change(cash_flows: &[(Date, f64)]) -> bool {
    if cash_flows.len() < 2 {
        return false;
    }

    let mut has_positive = false;
    let mut has_negative = false;

    for &(_, amount) in cash_flows {
        if amount > 0.0 {
            has_positive = true;
        } else if amount < 0.0 {
            has_negative = true;
        }
        if has_positive && has_negative {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Month;

    #[test]
    fn test_xirr_basic() {
        let flows = vec![
            (
                Date::from_calendar_date(2024, Month::January, 1).unwrap(),
                -100_000.0,
            ),
            (
                Date::from_calendar_date(2025, Month::January, 1).unwrap(),
                110_000.0,
            ),
        ];

        let result = xirr(&flows, None).unwrap();
        assert!((result - 0.1).abs() < 0.001); // Should be approximately 10%
    }

    #[test]
    fn test_xirr_multiple_flows() {
        let flows = vec![
            (
                Date::from_calendar_date(2024, Month::January, 1).unwrap(),
                -100_000.0,
            ),
            (
                Date::from_calendar_date(2024, Month::July, 1).unwrap(),
                5_000.0,
            ),
            (
                Date::from_calendar_date(2025, Month::January, 1).unwrap(),
                110_000.0,
            ),
        ];

        let result = xirr(&flows, None).unwrap();
        assert!(result > 0.1 && result < 0.2); // Should be between 10% and 20%
    }

    #[test]
    fn test_xirr_unsorted_inputs_equivalence() {
        // Same cashflows, different order; result should be equivalent
        let sorted = vec![
            (
                Date::from_calendar_date(2024, Month::January, 1).unwrap(),
                -100_000.0,
            ),
            (
                Date::from_calendar_date(2025, Month::January, 1).unwrap(),
                110_000.0,
            ),
        ];
        let mut unsorted = sorted.clone();
        unsorted.reverse();

        let r1 = xirr(&sorted, None).unwrap();
        let r2 = xirr(&unsorted, None).unwrap();
        assert!((r1 - r2).abs() < 1e-8);
    }

    #[test]
    fn test_xirr_negative_return() {
        let flows = vec![
            (
                Date::from_calendar_date(2024, Month::January, 1).unwrap(),
                -100_000.0,
            ),
            (
                Date::from_calendar_date(2025, Month::January, 1).unwrap(),
                90_000.0,
            ),
        ];

        let result = xirr(&flows, None).unwrap();
        assert!((result + 0.1).abs() < 0.001); // Should be approximately -10%
    }

    #[test]
    fn test_xirr_no_sign_change() {
        let flows = vec![
            (
                Date::from_calendar_date(2024, Month::January, 1).unwrap(),
                100_000.0,
            ),
            (
                Date::from_calendar_date(2025, Month::January, 1).unwrap(),
                110_000.0,
            ),
        ];

        let result = xirr(&flows, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_xirr_too_few_flows() {
        let flows = vec![(
            Date::from_calendar_date(2024, Month::January, 1).unwrap(),
            -100_000.0,
        )];

        let result = xirr(&flows, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_xirr_complex_schedule() {
        // More realistic example with irregular payments
        let flows = vec![
            (
                Date::from_calendar_date(2023, Month::January, 15).unwrap(),
                -50_000.0,
            ),
            (
                Date::from_calendar_date(2023, Month::March, 31).unwrap(),
                -30_000.0,
            ),
            (
                Date::from_calendar_date(2023, Month::June, 15).unwrap(),
                10_000.0,
            ),
            (
                Date::from_calendar_date(2023, Month::September, 30).unwrap(),
                15_000.0,
            ),
            (
                Date::from_calendar_date(2023, Month::December, 31).unwrap(),
                20_000.0,
            ),
            (
                Date::from_calendar_date(2024, Month::June, 15).unwrap(),
                45_000.0,
            ),
        ];

        let result = xirr(&flows, None);
        assert!(result.is_ok());
        let irr = result.unwrap();

        // Verify NPV is approximately zero at the calculated rate
        let npv = compute_npv(&flows, irr);
        assert!(npv.abs() < 1.0); // NPV should be very close to zero
    }

    fn compute_npv(flows: &[(Date, f64)], rate: f64) -> f64 {
        let first_date = flows[0].0;
        let dc = DayCount::Act365F;
        let mut sum = 0.0;

        for &(date, amount) in flows {
            let years = dc
                .year_fraction(first_date, date, DayCountCtx::default())
                .unwrap_or(0.0);
            let discount = (1.0 + rate).powf(years);
            sum += amount / discount;
        }
        sum
    }
}
