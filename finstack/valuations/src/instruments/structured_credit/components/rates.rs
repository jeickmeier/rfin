//! Interest rate and prepayment/default rate conversions.
//!
//! This module provides utility functions for converting between different
//! rate conventions used in structured credit modeling.

/// Convert CPR (Constant Prepayment Rate) to SMM (Single Monthly Mortality).
///
/// CPR is an annualized prepayment rate, while SMM is the monthly equivalent.
///
/// # Formula
///
/// SMM = 1 - (1 - CPR)^(1/12)
///
/// # Examples
///
/// ```
/// use finstack_valuations::instruments::structured_credit::components::rates::cpr_to_smm;
///
/// let cpr = 0.06; // 6% CPR
/// let smm = cpr_to_smm(cpr);
/// assert!((smm - 0.005143).abs() < 0.0001); // Approximately 0.5143% monthly
/// ```
pub fn cpr_to_smm(cpr: f64) -> f64 {
    1.0 - (1.0 - cpr).powf(1.0 / 12.0)
}

/// Convert SMM (Single Monthly Mortality) to CPR (Constant Prepayment Rate).
///
/// SMM is a monthly prepayment rate, while CPR is the annualized equivalent.
///
/// # Formula
///
/// CPR = 1 - (1 - SMM)^12
///
/// # Examples
///
/// ```
/// use finstack_valuations::instruments::structured_credit::components::rates::{cpr_to_smm, smm_to_cpr};
///
/// let cpr = 0.06;
/// let smm = cpr_to_smm(cpr);
/// let cpr_back = smm_to_cpr(smm);
/// assert!((cpr - cpr_back).abs() < 1e-10);
/// ```
pub fn smm_to_cpr(smm: f64) -> f64 {
    1.0 - (1.0 - smm).powi(12)
}

/// Convert CDR (Constant Default Rate) to MDR (Monthly Default Rate).
///
/// CDR is an annualized default rate, while MDR is the monthly equivalent.
///
/// # Formula
///
/// MDR = 1 - (1 - CDR)^(1/12)
///
/// # Examples
///
/// ```
/// use finstack_valuations::instruments::structured_credit::components::rates::cdr_to_mdr;
///
/// let cdr = 0.02; // 2% CDR
/// let mdr = cdr_to_mdr(cdr);
/// assert!(mdr < cdr); // Monthly rate is lower than annual
/// ```
pub fn cdr_to_mdr(cdr: f64) -> f64 {
    1.0 - (1.0 - cdr).powf(1.0 / 12.0)
}

/// Convert MDR (Monthly Default Rate) to CDR (Constant Default Rate).
///
/// MDR is a monthly default rate, while CDR is the annualized equivalent.
///
/// # Formula
///
/// CDR = 1 - (1 - MDR)^12
///
/// # Examples
///
/// ```
/// use finstack_valuations::instruments::structured_credit::components::rates::{cdr_to_mdr, mdr_to_cdr};
///
/// let cdr = 0.02;
/// let mdr = cdr_to_mdr(cdr);
/// let cdr_back = mdr_to_cdr(mdr);
/// assert!((cdr - cdr_back).abs() < 1e-10);
/// ```
pub fn mdr_to_cdr(mdr: f64) -> f64 {
    1.0 - (1.0 - mdr).powi(12)
}

/// Convert PSA speed to CPR at a given month.
///
/// The PSA (Public Securities Association) standard assumes prepayments
/// ramp up linearly over 30 months to a terminal rate of 6% CPR.
///
/// # Arguments
///
/// * `psa_speed` - PSA multiplier (1.0 = 100% PSA, 1.5 = 150% PSA)
/// * `month` - Seasoning month (months since origination)
///
/// # Formula
///
/// - Months 1-30: CPR = (month / 30) × 6% × PSA_speed
/// - Months 31+: CPR = 6% × PSA_speed
///
/// # Examples
///
/// ```
/// use finstack_valuations::instruments::structured_credit::components::rates::psa_to_cpr;
///
/// // 150% PSA at month 30
/// let cpr = psa_to_cpr(1.5, 30);
/// assert!((cpr - 0.09).abs() < 0.0001); // 9% CPR (6% × 1.5)
///
/// // 100% PSA at month 15
/// let cpr = psa_to_cpr(1.0, 15);
/// assert!((cpr - 0.03).abs() < 0.0001); // 3% CPR (halfway to 6%)
/// ```
pub fn psa_to_cpr(psa_speed: f64, month: u32) -> f64 {
    const PSA_RAMP_MONTHS: u32 = 30;
    const PSA_TERMINAL_CPR: f64 = 0.06;

    let base_cpr = if month <= PSA_RAMP_MONTHS {
        (month as f64 / PSA_RAMP_MONTHS as f64) * PSA_TERMINAL_CPR
    } else {
        PSA_TERMINAL_CPR
    };

    base_cpr * psa_speed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpr_smm_roundtrip() {
        let cpr = 0.06;
        let smm = cpr_to_smm(cpr);

        // 6% CPR should be approximately 0.5143% SMM
        assert!((smm - 0.005143).abs() < 0.0001);

        // Test roundtrip
        let cpr_back = smm_to_cpr(smm);
        assert!((cpr - cpr_back).abs() < 1e-10);
    }

    #[test]
    fn test_cdr_mdr_roundtrip() {
        let cdr = 0.02;
        let mdr = cdr_to_mdr(cdr);

        // MDR should be positive and less than CDR
        assert!(mdr > 0.0);
        assert!(mdr < cdr);

        // Test roundtrip
        let cdr_back = mdr_to_cdr(mdr);
        assert!((cdr - cdr_back).abs() < 1e-10);
    }

    #[test]
    fn test_psa_to_cpr() {
        // 100% PSA at month 30 should be 6% CPR
        let cpr = psa_to_cpr(1.0, 30);
        assert!((cpr - 0.06).abs() < 0.0001);

        // 150% PSA at month 30 should be 9% CPR
        let cpr = psa_to_cpr(1.5, 30);
        assert!((cpr - 0.09).abs() < 0.0001);

        // 100% PSA at month 15 should be 3% CPR (halfway)
        let cpr = psa_to_cpr(1.0, 15);
        assert!((cpr - 0.03).abs() < 0.0001);

        // 100% PSA after month 30 should stay at 6% CPR
        let cpr = psa_to_cpr(1.0, 60);
        assert!((cpr - 0.06).abs() < 0.0001);
    }

    #[test]
    fn test_conversion_consistency() {
        // Verify that CPR/SMM and CDR/MDR use the same formula
        let rate = 0.05;
        let monthly_prepay = cpr_to_smm(rate);
        let monthly_default = cdr_to_mdr(rate);

        // Should be identical formulas
        assert!((monthly_prepay - monthly_default).abs() < 1e-15);
    }
}
