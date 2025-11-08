//! Internal Rate of Return (IRR) calculations for revolving credit facilities.
//!
//! Provides IRR calculation for Monte Carlo paths using cashflows from lender perspective.

use finstack_core::cashflow::xirr::xirr;
use finstack_core::dates::{Date, DayCount};

/// Calculate IRR from path cashflows.
///
/// Computes the annualized Internal Rate of Return using XIRR for irregular cashflows.
/// The cashflows are assumed to be from the lender's perspective:
/// - Negative: principal deployment (outflow from lender)
/// - Positive: receipts (interest, fees, principal repayment)
///
/// # Arguments
///
/// * `cashflows` - Vector of (time_in_years, amount) tuples
/// * `base_date` - Base date for time calculations
/// * `day_count` - Day count convention to use
///
/// # Returns
///
/// IRR as an annualized decimal (e.g., 0.08 for 8% annual return), or None if IRR doesn't exist.
///
/// # Example
///
/// ```rust,ignore
/// use finstack_valuations::instruments::revolving_credit::metrics::irr::calculate_path_irr;
/// use finstack_core::dates::{Date, DayCount};
/// use time::Month;
///
/// let base = Date::from_calendar_date(2025, Month::January, 1).unwrap();
/// let cashflows = vec![
///     (0.0, -1_000_000.0),    // Initial deployment
///     (0.25, 12_500.0),       // Interest + fees (Q1)
///     (0.5, 12_500.0),        // Interest + fees (Q2)
///     (0.75, 12_500.0),       // Interest + fees (Q3)
///     (1.0, 1_012_500.0),     // Final interest + principal return
/// ];
///
/// let irr = calculate_path_irr(&cashflows, base, DayCount::Act365F);
/// assert!(irr.is_some());
/// assert!(irr.unwrap() > 0.04); // Should be ~5% IRR
/// ```
pub fn calculate_path_irr(
    cashflows: &[(f64, f64)],
    base_date: Date,
    _day_count: DayCount,
) -> Option<f64> {
    // XIRR requires at least 2 cashflows with sign change
    if cashflows.len() < 2 {
        return None;
    }

    // Check for sign change (required for IRR to exist)
    let has_positive = cashflows.iter().any(|(_, amt)| *amt > 0.0);
    let has_negative = cashflows.iter().any(|(_, amt)| *amt < 0.0);
    if !has_positive || !has_negative {
        return None;
    }

    // Convert time_in_years to dates for XIRR
    let dated_cashflows: Vec<(Date, f64)> = cashflows
        .iter()
        .filter_map(|(time_years, amount)| {
            // Convert years to days (approximate, using 365.25 days/year)
            let days = (*time_years * 365.25).round() as i64;

            // Add days to base date
            base_date
                .checked_add(time::Duration::days(days))
                .map(|date| (date, *amount))
        })
        .collect();

    if dated_cashflows.len() < 2 {
        return None;
    }

    // Use XIRR from finstack_core
    xirr(&dated_cashflows, None).ok()
}

/// Calculate IRR from simple evenly-spaced cashflows.
///
/// Uses periodic IRR for evenly-spaced cashflows (simpler than XIRR).
/// Assumes cashflows occur at periods 0, 1, 2, ... with equal spacing.
///
/// # Arguments
///
/// * `amounts` - Cashflow amounts at each period
///
/// # Returns
///
/// IRR as a decimal per period (e.g., 0.02 for 2% per period), or None if IRR doesn't exist.
///
/// # Note
///
/// For quarterly periods, convert to annual IRR using: `(1 + quarterly_irr)^4 - 1`
pub fn calculate_periodic_irr(amounts: &[f64]) -> Option<f64> {
    use finstack_core::cashflow::performance::irr_periodic;

    if amounts.len() < 2 {
        return None;
    }

    // Check for sign change
    let has_positive = amounts.iter().any(|&amt| amt > 0.0);
    let has_negative = amounts.iter().any(|&amt| amt < 0.0);
    if !has_positive || !has_negative {
        return None;
    }

    irr_periodic(amounts, None).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Month;

    #[test]
    fn test_calculate_path_irr_simple() {
        let base = Date::from_calendar_date(2025, Month::January, 1).unwrap();

        // Simple case: deploy 1M, receive 1.05M in 1 year = 5% IRR
        let cashflows = vec![(0.0, -1_000_000.0), (1.0, 1_050_000.0)];

        let irr = calculate_path_irr(&cashflows, base, DayCount::Act365F);
        assert!(irr.is_some());
        let irr_val = irr.unwrap();
        assert!(
            (irr_val - 0.05).abs() < 0.001,
            "Expected ~5% IRR, got {}",
            irr_val
        );
    }

    #[test]
    fn test_calculate_path_irr_quarterly_payments() {
        let base = Date::from_calendar_date(2025, Month::January, 1).unwrap();

        // Quarterly payments: deploy 1M, receive 12.5k quarterly + 1M at end
        // ~5% annual IRR
        let cashflows = vec![
            (0.0, -1_000_000.0),
            (0.25, 12_500.0),
            (0.5, 12_500.0),
            (0.75, 12_500.0),
            (1.0, 1_012_500.0),
        ];

        let irr = calculate_path_irr(&cashflows, base, DayCount::Act365F);
        assert!(irr.is_some());
        let irr_val = irr.unwrap();
        // Should be around 5% annual
        assert!(
            irr_val > 0.04 && irr_val < 0.06,
            "Expected ~5% IRR, got {}",
            irr_val
        );
    }

    #[test]
    fn test_calculate_path_irr_no_sign_change() {
        let base = Date::from_calendar_date(2025, Month::January, 1).unwrap();

        // All positive - no IRR
        let cashflows = vec![(0.0, 1_000_000.0), (1.0, 1_050_000.0)];

        let irr = calculate_path_irr(&cashflows, base, DayCount::Act365F);
        assert!(irr.is_none());
    }

    #[test]
    fn test_calculate_periodic_irr() {
        // Simple: invest 100, get back 110 next period = 10% per period
        let amounts = vec![-100.0, 110.0];
        let irr = calculate_periodic_irr(&amounts);
        assert!(irr.is_some());
        let irr_val = irr.unwrap();
        assert!(
            (irr_val - 0.1).abs() < 0.001,
            "Expected 10% IRR, got {}",
            irr_val
        );
    }

    #[test]
    fn test_calculate_periodic_irr_multiple_periods() {
        // Quarterly: invest 1000, receive 25/quarter + 1000 at end
        // 25*4 = 100 annual on 1000 = 10% annual, ~2.41% quarterly
        let amounts = vec![-1000.0, 25.0, 25.0, 25.0, 1025.0];
        let irr = calculate_periodic_irr(&amounts);
        assert!(irr.is_some());
        let irr_val = irr.unwrap();
        // Should be around 2.4% per quarter
        assert!(
            irr_val > 0.02 && irr_val < 0.03,
            "Expected ~2.4% quarterly IRR, got {}",
            irr_val
        );
    }
}
