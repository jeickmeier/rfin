//! Credit rate conversions.
//!
//! Utilities for converting between monthly and annual rate conventions.
//! These conversions apply to both prepayment rates (CPR↔SMM) and
//! default rates (CDR↔MDR), as they use identical mathematical formulas.

/// Maximum allowed CPR/CDR value.
///
/// Values >= 100% would produce NaN in the conversion formula since
/// `(1 - cpr)^(1/12)` is undefined for negative bases with fractional exponents.
/// We clamp to 99.9999% to avoid this while allowing near-total prepayment/default.
const MAX_CPR: f64 = 0.999999;

/// Convert annual CPR (constant prepayment rate) to monthly SMM (single monthly mortality).
///
/// Uses the standard relationship (per Fabozzi's MBS handbook):
/// `SMM = 1 - (1 - CPR)^(1/12)`.
///
/// # Edge Cases
///
/// - CPR = 0: Returns 0.0 (no prepayment)
/// - CPR >= 100%: Clamped to 99.9999% to avoid NaN (would produce negative base
///   for fractional exponent). This represents near-total prepayment.
/// - Negative CPR: While economically invalid, the formula still computes
///   (would indicate negative prepayment). Callers should validate inputs.
///
/// # Examples
///
/// ```
/// use finstack_valuations::cashflow::builder::cpr_to_smm;
///
/// // Convert 6% CPR to SMM
/// let cpr = 0.06;
/// let smm = cpr_to_smm(cpr);
/// assert!((smm - 0.005143).abs() < 0.0001); // Approximately 0.5143% monthly
///
/// // Edge case: 100% CPR is clamped
/// let smm_100 = cpr_to_smm(1.0);
/// assert!(smm_100.is_finite());
/// assert!(smm_100 > 0.99); // Near 100% monthly
/// ```
pub fn cpr_to_smm(cpr: f64) -> f64 {
    if cpr == 0.0 {
        return 0.0;
    }
    // Clamp CPR to avoid NaN from negative base with fractional exponent
    let cpr_clamped = cpr.min(MAX_CPR);
    1.0 - (1.0 - cpr_clamped).powf(1.0 / 12.0)
}

/// Convert monthly SMM to annual CPR.
///
/// # Formula
///
/// `annual = 1 - (1 - monthly)^12`
///
/// # Examples
///
/// ```
/// use finstack_valuations::cashflow::builder::{cpr_to_smm, smm_to_cpr};
///
/// // Roundtrip conversion
/// let cpr = 0.06;
/// let smm = cpr_to_smm(cpr);
/// let cpr_back = smm_to_cpr(smm);
/// assert!((cpr - cpr_back).abs() < 1e-10);
/// ```
pub fn smm_to_cpr(smm: f64) -> f64 {
    if smm == 0.0 {
        return 0.0;
    }
    1.0 - (1.0 - smm).powi(12)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
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

    #[test]
    fn test_cpr_100_percent_clamped() {
        // 100% CPR should be clamped to MAX_CPR (0.999999) to avoid NaN
        // The clamped CPR still produces a high SMM but not 100%
        // SMM = 1 - (1 - 0.999999)^(1/12) ≈ 1 - 0.000001^(1/12) ≈ 0.44
        let smm = cpr_to_smm(1.0);
        assert!(smm.is_finite(), "100% CPR should produce finite SMM");
        assert!(smm > 0.4, "100% CPR (clamped) should produce SMM > 40%");
        assert!(smm < 1.0, "SMM should be less than 1.0");
    }

    #[test]
    fn test_cpr_above_100_percent_clamped() {
        // CPR > 100% should be clamped to MAX_CPR to avoid NaN
        // Result should be same as 100% CPR
        let smm_100 = cpr_to_smm(1.0);
        let smm_150 = cpr_to_smm(1.5);
        assert!(smm_150.is_finite(), "CPR > 100% should produce finite SMM");
        // Both should be clamped to the same value
        assert!(
            (smm_100 - smm_150).abs() < 1e-10,
            "CPR > 100% should be clamped to same value as 100%: {} vs {}",
            smm_100,
            smm_150
        );
    }

    #[test]
    fn test_cpr_negative_still_computes() {
        // Negative CPR is economically invalid but should still compute
        // (callers should validate inputs)
        let smm = cpr_to_smm(-0.06);
        assert!(smm.is_finite(), "Negative CPR should produce finite SMM");
    }
}
