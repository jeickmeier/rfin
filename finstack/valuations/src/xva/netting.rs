//! Netting and collateral logic for XVA exposure calculations.
//!
//! Implements close-out netting under ISDA master agreements and
//! CSA collateral reduction for counterparty credit exposure.
//!
//! # Close-Out Netting
//!
//! Under a valid ISDA master agreement, upon default all transactions
//! are terminated and a single net amount is determined:
//!
//! ```text
//! Net exposure = max(Σᵢ Vᵢ, 0)
//! ```
//!
//! This is significantly less than the sum of individual positive exposures:
//! ```text
//! Gross exposure = Σᵢ max(Vᵢ, 0) ≥ Net exposure
//! ```
//!
//! # References
//!
//! - ISDA (2002). "2002 ISDA Master Agreement." Section 6 (Close-Out Netting).
//! - Gregory, J. (2020). *The xVA Challenge*, Chapter 6.
//! - BCBS 279 (2014). SA-CCR: "The standardised approach for measuring
//!   counterparty credit risk exposures."

use super::types::CsaTerms;

/// Apply close-out netting to a set of instrument mark-to-market values.
///
/// Under a valid ISDA master agreement, the exposure is computed on the
/// net portfolio value rather than summing individual positive exposures.
///
/// # Arguments
///
/// * `instrument_values` - Individual instrument MtM values (positive or negative)
///
/// # Returns
///
/// Net positive exposure: `max(Σᵢ Vᵢ, 0)`.
///
/// # Examples
///
/// ```
/// use finstack_valuations::xva::netting::apply_netting;
///
/// // Two offsetting trades: net exposure is reduced
/// let values = [100.0, -80.0];
/// assert!((apply_netting(&values) - 20.0).abs() < 1e-12);
///
/// // All negative: no exposure
/// let values = [-50.0, -30.0];
/// assert!((apply_netting(&values)).abs() < 1e-12);
/// ```
#[inline]
pub fn apply_netting(instrument_values: &[f64]) -> f64 {
    let net: f64 = instrument_values.iter().sum();
    net.max(0.0)
}

/// Apply CSA collateral terms to reduce gross exposure.
///
/// Models the collateral mechanics of a Credit Support Annex:
///
/// ```text
/// over_threshold = max(exposure - threshold, 0)
/// collateral_call = max(over_threshold - MTA, 0)
/// net_exposure = max(exposure - collateral_call + IA, 0)
/// ```
///
/// The independent amount (IA) acts as an additional buffer that
/// increases the effective collateral requirement.
///
/// # Arguments
///
/// * `gross_exposure` - Portfolio exposure before collateral (non-negative)
/// * `csa` - CSA terms governing collateral exchange
///
/// # Returns
///
/// Net exposure after collateral, always non-negative.
///
/// # Examples
///
/// ```
/// use finstack_valuations::xva::netting::apply_collateral;
/// use finstack_valuations::xva::types::CsaTerms;
///
/// let csa = CsaTerms {
///     threshold: 10.0,
///     mta: 1.0,
///     mpor_days: 10,
///     independent_amount: 0.0,
/// };
///
/// // Exposure below threshold: no collateral called
/// assert!((apply_collateral(8.0, &csa) - 8.0).abs() < 1e-12);
///
/// // Exposure above threshold + MTA: collateral reduces exposure
/// assert!((apply_collateral(20.0, &csa) - 11.0).abs() < 1e-12);
/// ```
#[inline]
pub fn apply_collateral(gross_exposure: f64, csa: &CsaTerms) -> f64 {
    let over_threshold = (gross_exposure - csa.threshold).max(0.0);
    let collateral = (over_threshold - csa.mta).max(0.0);
    (gross_exposure - collateral + csa.independent_amount).max(0.0)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    // ── Netting tests ──────────────────────────────────────────────

    #[test]
    fn netting_reduces_exposure() {
        // Offsetting trades should reduce net exposure
        let gross: f64 = [100.0_f64, -80.0].iter().filter(|v| **v > 0.0).sum::<f64>();
        let net = apply_netting(&[100.0, -80.0]);
        assert!(
            net < gross,
            "Netting should reduce exposure: net={net}, gross={gross}"
        );
        assert!((net - 20.0).abs() < 1e-12);
    }

    #[test]
    fn netting_all_positive() {
        // All positive values: net equals sum
        let values = [10.0, 20.0, 30.0];
        assert!((apply_netting(&values) - 60.0).abs() < 1e-12);
    }

    #[test]
    fn netting_all_negative_gives_zero() {
        // All negative: no exposure
        let values = [-10.0, -20.0, -30.0];
        assert!(apply_netting(&values).abs() < 1e-12);
    }

    #[test]
    fn netting_empty_gives_zero() {
        assert!(apply_netting(&[]).abs() < 1e-12);
    }

    #[test]
    fn netting_single_positive() {
        assert!((apply_netting(&[42.0]) - 42.0).abs() < 1e-12);
    }

    #[test]
    fn netting_single_negative_gives_zero() {
        assert!(apply_netting(&[-42.0]).abs() < 1e-12);
    }

    // ── Collateral tests ───────────────────────────────────────────

    fn make_csa(threshold: f64, mta: f64, ia: f64) -> CsaTerms {
        CsaTerms {
            threshold,
            mta,
            mpor_days: 10,
            independent_amount: ia,
        }
    }

    #[test]
    fn collateral_below_threshold_unchanged() {
        let csa = make_csa(10.0, 1.0, 0.0);
        assert!((apply_collateral(8.0, &csa) - 8.0).abs() < 1e-12);
    }

    #[test]
    fn collateral_between_threshold_and_mta() {
        // Over threshold by 0.5, but below MTA (1.0) → no collateral called
        let csa = make_csa(10.0, 1.0, 0.0);
        assert!((apply_collateral(10.5, &csa) - 10.5).abs() < 1e-12);
    }

    #[test]
    fn collateral_above_threshold_plus_mta() {
        // Exposure = 20, threshold = 10, MTA = 1
        // over_threshold = 10, collateral = 10 - 1 = 9
        // net = 20 - 9 = 11
        let csa = make_csa(10.0, 1.0, 0.0);
        assert!((apply_collateral(20.0, &csa) - 11.0).abs() < 1e-12);
    }

    #[test]
    fn collateral_with_independent_amount() {
        // IA adds to the net exposure (represents additional margin held)
        let csa = make_csa(10.0, 1.0, 5.0);
        // Exposure = 20, over_threshold = 10, collateral = 9
        // net = 20 - 9 + 5 = 16
        assert!((apply_collateral(20.0, &csa) - 16.0).abs() < 1e-12);
    }

    #[test]
    fn collateral_zero_threshold() {
        // Zero threshold CSA (bilateral VM): all exposure is collateralized above MTA
        let csa = make_csa(0.0, 0.5, 0.0);
        // Exposure = 100, over_threshold = 100, collateral = 99.5
        // net = 100 - 99.5 = 0.5
        assert!((apply_collateral(100.0, &csa) - 0.5).abs() < 1e-12);
    }

    #[test]
    fn collateral_zero_exposure() {
        let csa = make_csa(10.0, 1.0, 0.0);
        assert!(apply_collateral(0.0, &csa).abs() < 1e-12);
    }

    #[test]
    fn collateral_never_negative() {
        // Even with large IA on zero exposure, result is non-negative
        let csa = make_csa(0.0, 0.0, 100.0);
        let result = apply_collateral(0.0, &csa);
        assert!(
            result >= 0.0,
            "Collateralized exposure must be non-negative"
        );
    }
}
