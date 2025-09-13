//! XIRR (Extended Internal Rate of Return) implementation.
//!
//! Calculates the internal rate of return for a series of cash flows that may occur
//! at irregular intervals. XIRR is widely used for evaluating investment returns
//! when cash flows are not evenly spaced in time.

use finstack_core::dates::{Date, DayCount};
use finstack_core::error::InputError;
use finstack_core::math::solver::{HybridSolver, Solver};
use finstack_core::F;

/// Calculates XIRR (Extended Internal Rate of Return) for a series of cash flows.
///
/// XIRR finds the discount rate that makes the net present value of all cash flows
/// equal to zero. It's particularly useful for investments with irregular timing.
///
/// # Arguments
/// * `cash_flows` - Vector of (date, amount) tuples. Negative amounts represent outflows,
///                  positive amounts represent inflows.
/// * `guess` - Optional initial guess for the IRR (defaults to 0.1 = 10%)
///
/// # Returns
/// The XIRR as a decimal (e.g., 0.15 for 15% annual return)
///
/// # Errors
/// Returns an error if:
/// - Less than 2 cash flows provided
/// - No sign change in cash flows (all positive or all negative)
/// - Cannot converge to a solution within tolerance
///
/// See unit tests and `examples/` for usage.
pub fn xirr(cash_flows: &[(Date, F)], guess: Option<F>) -> finstack_core::Result<F> {
    // Validate inputs
    if cash_flows.len() < 2 {
        return Err(InputError::TooFewPoints.into());
    }

    // Check for sign change
    if !has_sign_change(cash_flows) {
        return Err(InputError::Invalid.into());
    }

    let first_date = cash_flows[0].0;
    let dc = DayCount::Act365F; // Standard day count for XIRR

    // Precompute (year_fraction, amount) once for performance
    let years_and_amounts: Vec<(F, F)> = cash_flows
        .iter()
        .map(|&(date, amount)| {
            let years = dc
                .year_fraction(
                    first_date,
                    date,
                    finstack_core::dates::DayCountCtx::default(),
                )
                .unwrap_or(0.0);
            (years, amount)
        })
        .collect();

    // NPV function for root finding
    let npv = |rate: F| -> F {
        let mut sum = 0.0;
        for &(years, amount) in &years_and_amounts {
            let discount = (1.0 + rate).powf(years);
            sum += amount / discount;
        }
        sum
    };

    // Use HybridSolver for Newton-Raphson with automatic Brent fallback
    let initial_guess = guess.unwrap_or(0.1);
    let solver = HybridSolver::new()
        .with_tolerance(1e-6)
        .with_max_iterations(100);
    
    solver.solve(npv, initial_guess)
}

/// Checks if cash flows have at least one sign change.
fn has_sign_change(cash_flows: &[(Date, F)]) -> bool {
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

    fn compute_npv(flows: &[(Date, F)], rate: F) -> F {
        let first_date = flows[0].0;
        let dc = DayCount::Act365F;
        let mut sum = 0.0;

        for &(date, amount) in flows {
            let years = dc
                .year_fraction(
                    first_date,
                    date,
                    finstack_core::dates::DayCountCtx::default(),
                )
                .unwrap_or(0.0);
            let discount = (1.0 + rate).powf(years);
            sum += amount / discount;
        }
        sum
    }
}
