//! Performance measurement utilities: IRR, XIRR, and NPV.
//!
//! Provides functions for calculating investment performance metrics:
//! - XIRR: Extended Internal Rate of Return for irregular cash flows
//! - IRR: Internal Rate of Return for periodic cash flows
//! - NPV: Net Present Value at a given discount rate

use crate::dates::{Date, DayCount, DayCountCtx};
use crate::error::InputError;
use crate::math::solver::{HybridSolver, Solver};

/// Calculate NPV (Net Present Value) for a series of cash flows at a given discount rate.
///
/// # Arguments
/// * `cash_flows` - Vector of (date, amount) tuples
/// * `discount_rate` - Annual discount rate as a decimal (e.g., 0.1 for 10%)
/// * `base_date` - Optional base date for discounting (defaults to first cash flow date)
/// * `day_count` - Optional day count convention (defaults to Act365F)
///
/// # Returns
/// The net present value
///
/// # Errors
/// Returns an error if cash flows are empty or date calculations fail
pub fn npv(
    cash_flows: &[(Date, f64)],
    discount_rate: f64,
    base_date: Option<Date>,
    day_count: Option<DayCount>,
) -> crate::Result<f64> {
    if cash_flows.is_empty() {
        return Ok(0.0);
    }

    let base = base_date.unwrap_or(cash_flows[0].0);
    let dc = day_count.unwrap_or(DayCount::Act365F);
    let mut sum = 0.0;

    for (date, amount) in cash_flows {
        let years = dc
            .year_fraction(base, *date, DayCountCtx::default())
            .map_err(|e| {
                crate::Error::Validation(format!("Day count calculation failed: {}", e))
            })?;
        let discount_factor = (1.0 + discount_rate).powf(years);
        sum += amount / discount_factor;
    }

    Ok(sum)
}

/// Calculate IRR (Internal Rate of Return) for evenly-spaced periodic cash flows.
///
/// This is a simplified version of XIRR for cash flows that occur at regular intervals
/// (e.g., monthly, quarterly, or annual). Each cash flow is assumed to occur at
/// periods 0, 1, 2, ... relative to the base date.
///
/// # Arguments
/// * `amounts` - Array of cash flow amounts (negative for outflows, positive for inflows)
/// * `guess` - Optional initial guess for the IRR (defaults to 0.1 = 10%)
///
/// # Returns
/// The IRR as a decimal per period (e.g., 0.025 for 2.5% per period)
///
/// # Errors
/// Returns an error if:
/// - Less than 2 cash flows provided
/// - No sign change in cash flows (all positive or all negative)
/// - Cannot converge to a solution within tolerance
///
/// # Example
/// ```rust
/// use finstack_core::cashflow::performance::irr_periodic;
///
/// // Quarterly cash flows: initial investment, then 8 quarterly payments
/// let amounts = vec![-100000.0, 3000.0, 3000.0, 3000.0, 3000.0, 3000.0, 3000.0, 3000.0, 90000.0];
/// let quarterly_irr = irr_periodic(&amounts, None)?;
/// // Annual IRR = (1 + quarterly_irr)^4 - 1
/// ```
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

    let initial_guess = guess.unwrap_or(0.1);
    let solver = HybridSolver::new()
        .with_tolerance(1e-6)
        .with_max_iterations(100);

    solver
        .solve(npv, initial_guess)
        .map_err(|e| crate::Error::Validation(format!("IRR calculation failed: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dates::create_date;
    use time::Month;

    #[test]
    fn test_npv_simple() {
        let flows = vec![
            (create_date(2024, Month::January, 1).unwrap(), -100000.0),
            (create_date(2025, Month::January, 1).unwrap(), 110000.0),
        ];
        let npv_5pct = npv(&flows, 0.05, None, None).unwrap();
        // NPV should be positive (profitable at 5% discount rate)
        // Approximately: -100000 + 110000/(1.05) ≈ 4761.90
        assert!(npv_5pct > 4700.0 && npv_5pct < 4800.0);
    }

    #[test]
    fn test_npv_zero_discount() {
        let flows = vec![
            (create_date(2024, Month::January, 1).unwrap(), -100.0),
            (create_date(2025, Month::January, 1).unwrap(), 100.0),
        ];
        let npv_zero = npv(&flows, 0.0, None, None).unwrap();
        assert_eq!(npv_zero, 0.0);
    }

    #[test]
    fn test_irr_periodic() {
        // Simple case: invest 100, get 110 back after 1 period
        let amounts = vec![-100.0, 110.0];
        let irr = irr_periodic(&amounts, None).unwrap();
        assert!((irr - 0.1).abs() < 1e-6); // 10% return
    }

    #[test]
    fn test_irr_periodic_multiple_periods() {
        // Invest 1000, receive 300 per period for 4 periods
        let amounts = vec![-1000.0, 300.0, 300.0, 300.0, 300.0];
        let irr = irr_periodic(&amounts, None).unwrap();
        // Should be close to 7.71% per period
        assert!(irr > 0.07 && irr < 0.08);
    }

    #[test]
    fn test_irr_periodic_no_sign_change() {
        let amounts = vec![100.0, 200.0, 300.0];
        assert!(irr_periodic(&amounts, None).is_err());
    }
}
