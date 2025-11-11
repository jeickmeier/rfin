//! Credit rate conversions.
//!
//! Utilities for converting between monthly and annual rate conventions.
//! These conversions apply to both prepayment rates (CPR↔SMM) and
//! default rates (CDR↔MDR), as they use identical mathematical formulas.

/// Convert annual rate to monthly rate.
///
/// Works for both CPR→SMM and CDR→MDR conversions.
///
/// # Formula
///
/// `monthly = 1 - (1 - annual)^(1/12)`
///
/// # Examples
///
/// ```
/// use finstack_valuations::cashflow::builder::credit_rates::annual_to_monthly;
///
/// // Convert 6% CPR to SMM
/// let cpr = 0.06;
/// let smm = annual_to_monthly(cpr);
/// assert!((smm - 0.005143).abs() < 0.0001); // Approximately 0.5143% monthly
/// ```
pub fn annual_to_monthly(annual_rate: f64) -> f64 {
    1.0 - (1.0 - annual_rate).powf(1.0 / 12.0)
}

/// Convert monthly rate to annual rate.
///
/// Works for both SMM→CPR and MDR→CDR conversions.
///
/// # Formula
///
/// `annual = 1 - (1 - monthly)^12`
///
/// # Examples
///
/// ```
/// use finstack_valuations::cashflow::builder::credit_rates::{annual_to_monthly, monthly_to_annual};
///
/// // Roundtrip conversion
/// let cpr = 0.06;
/// let smm = annual_to_monthly(cpr);
/// let cpr_back = monthly_to_annual(smm);
/// assert!((cpr - cpr_back).abs() < 1e-10);
/// ```
pub fn monthly_to_annual(monthly_rate: f64) -> f64 {
    1.0 - (1.0 - monthly_rate).powi(12)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_annual_to_monthly_conversion() {
        let annual = 0.06; // 6% annual
        let monthly = annual_to_monthly(annual);

        // 6% annual should be approximately 0.5143% monthly
        assert!((monthly - 0.005143).abs() < 0.0001);
        assert!(monthly > 0.0);
        assert!(monthly < annual); // Monthly should be less than annual
    }

    #[test]
    fn test_monthly_to_annual_conversion() {
        let monthly = 0.01; // 1% monthly
        let annual = monthly_to_annual(monthly);

        // Should be positive and greater than monthly
        assert!(annual > monthly);
        assert!(annual < 1.0);
    }

    #[test]
    fn test_roundtrip_conversion() {
        let original = 0.06;
        let monthly = annual_to_monthly(original);
        let back = monthly_to_annual(monthly);

        // Should roundtrip with high precision
        assert!((original - back).abs() < 1e-10);
    }

    #[test]
    fn test_zero_rate() {
        assert_eq!(annual_to_monthly(0.0), 0.0);
        assert_eq!(monthly_to_annual(0.0), 0.0);
    }

    #[test]
    fn test_consistency_across_rates() {
        // Test that prepayment (CPR) and default (CDR) use the same formula
        let rate = 0.05;
        let monthly_prepay = annual_to_monthly(rate);
        let monthly_default = annual_to_monthly(rate);

        // Should be identical
        assert!((monthly_prepay - monthly_default).abs() < 1e-15);
    }
}

