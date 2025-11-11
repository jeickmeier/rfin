//! Rate conversion utilities for structured credit instruments.
//!
//! Provides standard conversions between different rate conventions:
//! - CPR (Constant Prepayment Rate) ↔ SMM (Single Monthly Mortality)
//! - CDR (Constant Default Rate) ↔ MDR (Monthly Default Rate)
//! - PSA to CPR conversions
//!
//! # Mathematical Foundations
//!
//! ## CPR ↔ SMM
//!
//! CPR is an annualized prepayment rate, while SMM is the monthly equivalent.
//!
//! ```text
//! SMM = 1 - (1 - CPR)^(1/12)
//! CPR = 1 - (1 - SMM)^12
//! ```
//!
//! ## CDR ↔ MDR
//!
//! Similarly, CDR is annualized and MDR is monthly:
//!
//! ```text
//! MDR = 1 - (1 - CDR)^(1/12)
//! CDR = 1 - (1 - MDR)^12
//! ```
//!
//! ## PSA Model
//!
//! The PSA (Public Securities Association) prepayment model defines a standard
//! prepayment curve for residential mortgages:
//!
//! - Months 1-30: CPR ramps linearly from 0% to 6%
//! - Month 30+: CPR stays at 6%
//!
//! PSA speeds are multiples of this curve (e.g., 150% PSA = 1.5x the standard curve).

use super::super::config::{PSA_RAMP_MONTHS, PSA_TERMINAL_CPR};

/// Converts annual CPR to monthly SMM.
///
/// # Arguments
///
/// * `cpr` - Annual constant prepayment rate (as decimal, e.g., 0.06 for 6%)
///
/// # Returns
///
/// Monthly single mortality rate (as decimal)
///
/// # Formula
///
/// ```text
/// SMM = 1 - (1 - CPR)^(1/12)
/// ```
///
/// # Examples
///
/// ```
/// use finstack_valuations::instruments::structured_credit::cpr_to_smm;
///
/// let cpr = 0.06; // 6% annual CPR
/// let smm = cpr_to_smm(cpr);
/// assert!((smm - 0.005143).abs() < 0.0001);
/// ```
#[inline]
pub fn cpr_to_smm(cpr: f64) -> f64 {
    if cpr == 0.0 {
        return 0.0;
    }
    1.0 - (1.0 - cpr).powf(1.0 / 12.0)
}

/// Converts monthly SMM to annual CPR.
///
/// # Arguments
///
/// * `smm` - Monthly single mortality rate (as decimal, e.g., 0.005 for 0.5%)
///
/// # Returns
///
/// Annual constant prepayment rate (as decimal)
///
/// # Formula
///
/// ```text
/// CPR = 1 - (1 - SMM)^12
/// ```
///
/// # Examples
///
/// ```
/// use finstack_valuations::instruments::structured_credit::smm_to_cpr;
///
/// let smm = 0.005; // 0.5% monthly SMM
/// let cpr = smm_to_cpr(smm);
/// assert!((cpr - 0.0584).abs() < 0.001);
/// ```
#[inline]
pub fn smm_to_cpr(smm: f64) -> f64 {
    if smm == 0.0 {
        return 0.0;
    }
    1.0 - (1.0 - smm).powi(12)
}

/// Converts annual CDR to monthly MDR.
///
/// # Arguments
///
/// * `cdr` - Annual constant default rate (as decimal, e.g., 0.02 for 2%)
///
/// # Returns
///
/// Monthly default rate (as decimal)
///
/// # Formula
///
/// ```text
/// MDR = 1 - (1 - CDR)^(1/12)
/// ```
///
/// # Examples
///
/// ```
/// use finstack_valuations::instruments::structured_credit::cdr_to_mdr;
///
/// let cdr = 0.02; // 2% annual CDR
/// let mdr = cdr_to_mdr(cdr);
/// assert!((mdr - 0.001679).abs() < 0.0001);
/// ```
#[inline]
pub fn cdr_to_mdr(cdr: f64) -> f64 {
    if cdr == 0.0 {
        return 0.0;
    }
    1.0 - (1.0 - cdr).powf(1.0 / 12.0)
}

/// Converts monthly MDR to annual CDR.
///
/// # Arguments
///
/// * `mdr` - Monthly default rate (as decimal, e.g., 0.002 for 0.2%)
///
/// # Returns
///
/// Annual constant default rate (as decimal)
///
/// # Formula
///
/// ```text
/// CDR = 1 - (1 - MDR)^12
/// ```
///
/// # Examples
///
/// ```
/// use finstack_valuations::instruments::structured_credit::mdr_to_cdr;
///
/// let mdr = 0.002; // 0.2% monthly MDR
/// let cdr = mdr_to_cdr(mdr);
/// assert!((cdr - 0.02375).abs() < 0.0001);
/// ```
#[inline]
pub fn mdr_to_cdr(mdr: f64) -> f64 {
    if mdr == 0.0 {
        return 0.0;
    }
    1.0 - (1.0 - mdr).powi(12)
}

/// Converts PSA speed to CPR at a given month.
///
/// # Arguments
///
/// * `psa_speed` - PSA speed multiplier (e.g., 1.0 for 100% PSA, 1.5 for 150% PSA)
/// * `month` - Month number (1-indexed, i.e., month 1 is the first month)
///
/// # Returns
///
/// Annual CPR at the given month (as decimal)
///
/// # PSA Model
///
/// The standard PSA model (100% PSA):
/// - Month 1: CPR = 0.2%
/// - Month 2: CPR = 0.4%
/// - ...
/// - Month 30: CPR = 6.0%
/// - Month 31+: CPR = 6.0%
///
/// PSA speeds scale this curve linearly. For example, 150% PSA at month 30 = 9% CPR.
///
/// # Examples
///
/// ```
/// use finstack_valuations::instruments::structured_credit::psa_to_cpr;
///
/// // 100% PSA at month 15
/// let cpr = psa_to_cpr(1.0, 15);
/// assert!((cpr - 0.03).abs() < 0.0001); // 3% CPR
///
/// // 150% PSA at month 30
/// let cpr = psa_to_cpr(1.5, 30);
/// assert!((cpr - 0.09).abs() < 0.0001); // 9% CPR
/// ```
pub fn psa_to_cpr(psa_speed: f64, month: u32) -> f64 {
    if month == 0 {
        return 0.0;
    }

    // Standard PSA ramps from 0% to 6% CPR over first 30 months
    let base_cpr = if month <= PSA_RAMP_MONTHS {
        (month as f64 / PSA_RAMP_MONTHS as f64) * PSA_TERMINAL_CPR
    } else {
        PSA_TERMINAL_CPR
    };

    psa_speed * base_cpr
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpr_smm_roundtrip() {
        let test_cprs = vec![0.0, 0.01, 0.05, 0.10, 0.15, 0.20, 0.30];
        for cpr in test_cprs {
            let smm = cpr_to_smm(cpr);
            let cpr_back = smm_to_cpr(smm);
            assert!((cpr - cpr_back).abs() < 1e-10);
        }
    }

    #[test]
    fn test_cdr_mdr_roundtrip() {
        let test_cdrs = vec![0.0, 0.01, 0.02, 0.05, 0.10];
        for cdr in test_cdrs {
            let mdr = cdr_to_mdr(cdr);
            let cdr_back = mdr_to_cdr(mdr);
            assert!((cdr - cdr_back).abs() < 1e-10);
        }
    }

    #[test]
    fn test_psa_curve() {
        // 100% PSA at month 1 should be 0.2% CPR
        assert!((psa_to_cpr(1.0, 1) - 0.002).abs() < 0.0001);

        // 100% PSA at month 15 should be 3% CPR
        assert!((psa_to_cpr(1.0, 15) - 0.03).abs() < 0.0001);

        // 100% PSA at month 30 should be 6% CPR
        assert!((psa_to_cpr(1.0, 30) - 0.06).abs() < 0.0001);

        // 100% PSA after month 30 should stay at 6% CPR
        assert!((psa_to_cpr(1.0, 100) - 0.06).abs() < 0.0001);

        // 150% PSA at month 30 should be 9% CPR
        assert!((psa_to_cpr(1.5, 30) - 0.09).abs() < 0.0001);
    }
}

