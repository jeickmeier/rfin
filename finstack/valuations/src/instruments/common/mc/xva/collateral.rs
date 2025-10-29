//! Collateral modeling for xVA.
//!
//! Handles ISDA-style collateral agreements (CSA) including:
//! - Threshold: Minimum exposure before collateral required
//! - Minimum Transfer Amount (MTA): Minimum collateral movement
//! - Independent Amount (IA): Fixed collateral posted upfront

use super::exposure::ExposureProfile;

/// Collateral agreement (Credit Support Annex parameters).
#[derive(Clone, Debug)]
pub struct CollateralAgreement {
    /// Threshold: No collateral required if exposure < threshold
    pub threshold: f64,
    /// Minimum Transfer Amount: Collateral only moved if change > MTA
    pub mta: f64,
    /// Independent Amount: Fixed collateral posted at inception
    pub independent_amount: f64,
    /// Haircut: Percentage discount on collateral value (e.g., 0.02 for 2%)
    pub haircut: f64,
}

impl CollateralAgreement {
    /// Create a new collateral agreement.
    pub fn new(threshold: f64, mta: f64, independent_amount: f64) -> Self {
        Self {
            threshold,
            mta,
            independent_amount,
            haircut: 0.0,
        }
    }

    /// Create with haircut.
    pub fn with_haircut(mut self, haircut: f64) -> Self {
        self.haircut = haircut;
        self
    }

    /// No collateral agreement (bilateral uncollateralized).
    pub fn uncollateralized() -> Self {
        Self {
            threshold: f64::INFINITY,
            mta: f64::INFINITY,
            independent_amount: 0.0,
            haircut: 0.0,
        }
    }

    /// Fully collateralized (zero threshold, zero MTA).
    pub fn fully_collateralized() -> Self {
        Self {
            threshold: 0.0,
            mta: 0.0,
            independent_amount: 0.0,
            haircut: 0.0,
        }
    }
}

/// Apply collateral agreement to exposure.
///
/// Returns collateralized exposure (after applying threshold, MTA, etc.).
///
/// # Arguments
///
/// * `exposure` - Uncollateralized exposure
/// * `prev_collateral` - Previously posted collateral
/// * `agreement` - Collateral agreement terms
///
/// # Returns
///
/// (collateralized_exposure, new_collateral_posted)
pub fn apply_collateral(
    exposure: f64,
    prev_collateral: f64,
    agreement: &CollateralAgreement,
) -> (f64, f64) {
    // Exposure net of threshold
    let net_exposure = if exposure > agreement.threshold {
        exposure - agreement.threshold
    } else if exposure < -agreement.threshold {
        exposure + agreement.threshold
    } else {
        0.0
    };

    // Required collateral (considering independent amount)
    let required_collateral = net_exposure + agreement.independent_amount;

    // Check if change exceeds MTA
    let collateral_change = required_collateral - prev_collateral;
    let new_collateral = if collateral_change.abs() > agreement.mta {
        required_collateral
    } else {
        prev_collateral // No transfer
    };

    // Apply haircut to collateral value
    let effective_collateral = new_collateral * (1.0 - agreement.haircut);

    // Collateralized exposure = exposure - effective collateral
    let coll_exposure = (exposure - effective_collateral).max(0.0);

    (coll_exposure, new_collateral)
}

/// Apply collateral to an exposure profile.
///
/// Returns new exposure profile with collateral applied.
pub fn apply_collateral_to_profile(
    exposure_profile: &ExposureProfile,
    agreement: &CollateralAgreement,
) -> ExposureProfile {
    let num_points = exposure_profile.num_points();
    let mut coll_profile = ExposureProfile::new(exposure_profile.times.clone());

    let mut prev_collateral = 0.0;

    for i in 0..num_points {
        // Raw exposure (could be positive or negative)
        let raw_exposure = exposure_profile.epe[i] - exposure_profile.ene[i];

        // Apply collateral
        let (coll_exp, new_coll) = apply_collateral(raw_exposure, prev_collateral, agreement);

        coll_profile.epe[i] = coll_exp;
        coll_profile.ene[i] = 0.0; // After collateral, only positive exposure remains

        prev_collateral = new_coll;
    }

    coll_profile
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collateral_below_threshold() {
        let agreement = CollateralAgreement::new(100.0, 10.0, 0.0);

        // Exposure below threshold: no collateral
        let (coll_exp, new_coll) = apply_collateral(50.0, 0.0, &agreement);

        assert_eq!(coll_exp, 50.0); // Full exposure (no collateral)
        assert_eq!(new_coll, 0.0); // No collateral posted
    }

    #[test]
    fn test_collateral_above_threshold() {
        let agreement = CollateralAgreement::new(100.0, 10.0, 0.0);

        // Exposure above threshold: collateral required
        let (coll_exp, new_coll) = apply_collateral(150.0, 0.0, &agreement);

        // Should post collateral for 150 - 100 = 50
        assert_eq!(new_coll, 50.0);
        // Collateralized exposure should be 150 - 50 = 100 (threshold)
        assert_eq!(coll_exp, 100.0);
    }

    #[test]
    fn test_collateral_mta() {
        let agreement = CollateralAgreement::new(0.0, 10.0, 0.0); // Zero threshold, 10 MTA

        // Small exposure change (< MTA): no collateral movement
        let (_, new_coll1) = apply_collateral(5.0, 0.0, &agreement);
        assert_eq!(new_coll1, 0.0); // Change < MTA, no transfer

        // Large exposure change (> MTA): collateral moved
        let (_, new_coll2) = apply_collateral(15.0, 0.0, &agreement);
        assert_eq!(new_coll2, 15.0); // Change > MTA, transfer occurs
    }

    #[test]
    fn test_collateral_independent_amount() {
        let agreement = CollateralAgreement::new(0.0, 0.0, 20.0);

        // Even with zero exposure, IA is posted
        let (coll_exp, new_coll) = apply_collateral(0.0, 0.0, &agreement);

        assert_eq!(new_coll, 20.0); // IA posted
                                    // Exposure is negative (we're over-collateralized)
        assert_eq!(coll_exp, 0.0); // Can't be negative
    }

    #[test]
    fn test_uncollateralized() {
        let agreement = CollateralAgreement::uncollateralized();

        let (coll_exp, new_coll) = apply_collateral(100.0, 0.0, &agreement);

        // No collateral
        assert_eq!(new_coll, 0.0);
        // Full exposure
        assert_eq!(coll_exp, 100.0);
    }

    #[test]
    fn test_fully_collateralized() {
        let agreement = CollateralAgreement::fully_collateralized();

        let (coll_exp, new_coll) = apply_collateral(100.0, 0.0, &agreement);

        // Fully collateralized
        assert_eq!(new_coll, 100.0);
        // Zero residual exposure
        assert_eq!(coll_exp, 0.0);
    }
}
