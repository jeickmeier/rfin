//! Credit rate conversions.
//!
//! Utilities for converting between monthly and annual rate conventions.
//! These conversions apply to both prepayment rates (CPR/SMM) and
//! default rates (CDR/MDR), as they use identical mathematical formulas.

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
///
/// # Errors
///
/// Returns `InputError::NegativeValue` if CPR is negative. Negative prepayment
/// rates are economically invalid.
///
/// # Examples
///
/// ```
/// use finstack_cashflows::builder::cpr_to_smm;
///
/// // Convert 6% CPR to SMM
/// let cpr = 0.06;
/// let smm = cpr_to_smm(cpr).unwrap();
/// assert!((smm - 0.005143).abs() < 0.0001); // Approximately 0.5143% monthly
///
/// // Edge case: 100% CPR is clamped
/// let smm_100 = cpr_to_smm(1.0).unwrap();
/// assert!(smm_100.is_finite());
/// // With MAX_CPR=0.999999, the implied SMM is about 0.684 (68.4% monthly).
/// let expected = 1.0 - (1.0 - 0.999999_f64).powf(1.0 / 12.0);
/// assert!((smm_100 - expected).abs() < 1e-12);
///
/// // Negative CPR is rejected
/// assert!(cpr_to_smm(-0.05).is_err());
/// ```
pub fn cpr_to_smm(cpr: f64) -> finstack_core::Result<f64> {
    if cpr < 0.0 {
        return Err(finstack_core::Error::Input(
            finstack_core::InputError::NegativeValue,
        ));
    }
    if cpr == 0.0 {
        return Ok(0.0);
    }
    // Clamp CPR to avoid NaN from negative base with fractional exponent.
    // Log a warning when clamping occurs, as this typically indicates a
    // speed_multiplier that produces unreasonably high CPR/CDR values.
    let cpr_clamped = if cpr > MAX_CPR {
        tracing::warn!(
            cpr = cpr,
            clamped_to = MAX_CPR,
            "CPR/CDR exceeds maximum allowed value and was clamped; \
             check speed_multiplier or input rate"
        );
        MAX_CPR
    } else {
        cpr
    };
    Ok(1.0 - (1.0 - cpr_clamped).powf(1.0 / 12.0))
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
/// use finstack_cashflows::builder::{cpr_to_smm, smm_to_cpr};
///
/// // Roundtrip conversion
/// let cpr = 0.06;
/// let smm = cpr_to_smm(cpr).unwrap();
/// let cpr_back = smm_to_cpr(smm);
/// assert!((cpr - cpr_back).abs() < 1e-10);
/// ```
pub fn smm_to_cpr(smm: f64) -> f64 {
    // SMM is a monthly mortality rate and must be non-negative.
    // cpr_to_smm already rejects negatives; this assertion ensures the
    // inverse direction has symmetric protection in debug/test builds.
    debug_assert!(
        smm >= 0.0,
        "smm_to_cpr: SMM must be non-negative, got {smm}. \
         Use cpr_to_smm(cpr)? → smm_to_cpr(smm) for roundtrip conversions."
    );
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
        let monthly = cpr_to_smm(annual).expect("valid CPR");

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
        let monthly = cpr_to_smm(original).expect("valid CPR");
        let back = smm_to_cpr(monthly);

        // Should roundtrip with high precision
        assert!((original - back).abs() < 1e-10);
    }

    #[test]
    fn test_zero_rate() {
        assert_eq!(cpr_to_smm(0.0).expect("zero CPR should succeed"), 0.0);
        assert_eq!(smm_to_cpr(0.0), 0.0);
    }

    #[test]
    fn test_consistency_across_rates() {
        // Test that prepayment (CPR) and default (CDR) use the same formula
        let rate = 0.05;
        let monthly_prepay = cpr_to_smm(rate).expect("valid CPR");
        let monthly_default = cpr_to_smm(rate).expect("valid CPR");

        // Should be identical
        assert!((monthly_prepay - monthly_default).abs() < 1e-15);
    }

    #[test]
    fn test_cpr_smm_roundtrip_via_new_names() {
        let cpr = 0.06;
        let smm = cpr_to_smm(cpr).expect("valid CPR");
        let cpr_back = smm_to_cpr(smm);
        assert!((cpr - cpr_back).abs() < 1e-10);
    }

    #[test]
    fn test_cpr_100_percent_clamped() {
        // 100% CPR should be clamped to MAX_CPR (0.999999) to avoid NaN
        // The clamped CPR still produces a high SMM but not 100%
        // SMM = 1 - (1 - 0.999999)^(1/12) ~ 0.683772...
        let smm = cpr_to_smm(1.0).expect("100% CPR should succeed (clamped)");
        assert!(smm.is_finite(), "100% CPR should produce finite SMM");
        let expected = 1.0 - (1.0 - MAX_CPR).powf(1.0 / 12.0);
        assert!(
            (smm - expected).abs() < 1e-12,
            "100% CPR (clamped) should match formula: expected {}, got {}",
            expected,
            smm
        );
        assert!(smm < 1.0, "SMM should be less than 1.0");
    }

    #[test]
    fn test_cpr_above_100_percent_clamped() {
        // CPR > 100% should be clamped to MAX_CPR to avoid NaN
        // Result should be same as 100% CPR
        let smm_100 = cpr_to_smm(1.0).expect("100% CPR should succeed");
        let smm_150 = cpr_to_smm(1.5).expect("150% CPR should succeed (clamped)");
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
    fn test_cpr_to_smm_rejects_negative() {
        let result = cpr_to_smm(-0.05);
        assert!(result.is_err(), "Negative CPR should return error");
    }

    #[test]
    fn test_cpr_to_smm_positive() {
        let result = cpr_to_smm(0.06).expect("positive CPR should succeed");
        // 6% CPR -> SMM ~ 0.005143
        assert!((result - 0.005143).abs() < 0.0001);
    }
}
