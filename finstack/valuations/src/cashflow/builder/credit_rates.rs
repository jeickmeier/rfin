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
#[deprecated(
    since = "0.1.0",
    note = "use cpr_to_smm (hazard-style CPR→SMM) instead; not for interest-rate compounding"
)]
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
#[deprecated(
    since = "0.1.0",
    note = "use smm_to_cpr (hazard-style SMM→CPR) instead; not for interest-rate compounding"
)]
pub fn monthly_to_annual(monthly_rate: f64) -> f64 {
    1.0 - (1.0 - monthly_rate).powi(12)
}

/// Convert annual CPR (constant prepayment rate) to monthly SMM (single monthly mortality).
///
/// Uses the standard relationship:
/// `SMM = 1 - (1 - CPR)^(1/12)`.
pub fn cpr_to_smm(cpr: f64) -> f64 {
    if cpr == 0.0 {
        return 0.0;
    }
    1.0 - (1.0 - cpr).powf(1.0 / 12.0)
}

/// Convert monthly SMM to annual CPR.
///
pub fn smm_to_cpr(smm: f64) -> f64 {
    if smm == 0.0 {
        return 0.0;
    }
    1.0 - (1.0 - smm).powi(12)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_annual_to_monthly_conversion() {
        let annual = 0.06; // 6% annual
        let monthly = cpr_to_smm(annual);

        // 6% annual should be approximately 0.5143% monthly
        assert!((monthly - 0.005143).abs() < 0.0001);
        assert!(monthly > 0.0);
        assert!(monthly < annual); // Monthly should be less than annual
    }

    #[test]
    fn test_monthly_to_annual_conversion() {
        let monthly = 0.01; // 1% monthly
        let annual = smm_to_cpr(monthly);

        // Should be positive and greater than monthly
        assert!(annual > monthly);
        assert!(annual < 1.0);
    }

    #[test]
    fn test_roundtrip_conversion() {
        let original = 0.06;
        let monthly = cpr_to_smm(original);
        let back = smm_to_cpr(monthly);

        // Should roundtrip with high precision
        assert!((original - back).abs() < 1e-10);
    }

    #[test]
    fn test_zero_rate() {
        assert_eq!(cpr_to_smm(0.0), 0.0);
        assert_eq!(smm_to_cpr(0.0), 0.0);
    }

    #[test]
    fn test_consistency_across_rates() {
        // Test that prepayment (CPR) and default (CDR) use the same formula
        let rate = 0.05;
        let monthly_prepay = cpr_to_smm(rate);
        let monthly_default = cpr_to_smm(rate);

        // Should be identical
        assert!((monthly_prepay - monthly_default).abs() < 1e-15);
    }

    #[test]
    fn test_cpr_smm_roundtrip_via_new_names() {
        let cpr = 0.06;
        let smm = cpr_to_smm(cpr);
        let cpr_back = smm_to_cpr(smm);
        assert!((cpr - cpr_back).abs() < 1e-10);
    }
}
