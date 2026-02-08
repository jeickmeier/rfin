//! Standard calibration constants for structured credit stochastic models.
//!
//! This module provides industry-standard calibration parameters for RMBS,
//! CLO, and other structured credit instruments. Centralizing these constants
//! ensures consistency across default and prepayment models.
//!
//! # Asset Classes
//!
//! - **RMBS (Residential Mortgage-Backed Securities)**: Agency and non-agency
//! - **CLO (Collateralized Loan Obligations)**: Leveraged loan pools
#![allow(dead_code)] // Public API items may be used by external bindings
//! - **CMBS (Commercial Mortgage-Backed Securities)**: Commercial loans
//!
//! # References
//!
//! - Moody's Default Study (annual corporate default rates)
//! - PSA Standard Prepayment Model assumptions
//! - Basel IRB correlation formulas

/// RMBS standard calibration parameters.
///
/// Suitable for agency and prime non-agency RMBS pools.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RmbsCalibration {
    /// Base annual conditional default rate (CDR)
    pub base_cdr: f64,
    /// Default correlation (asset correlation)
    pub default_correlation: f64,
    /// Base annual conditional prepayment rate (CPR)
    pub base_cpr: f64,
    /// Prepayment factor loading (sensitivity to systematic factor)
    pub prepay_factor_loading: f64,
    /// CPR volatility
    pub cpr_volatility: f64,
    /// Default factor sensitivity (for intensity models)
    pub default_factor_sensitivity: f64,
    /// Default model mean reversion speed
    pub default_mean_reversion: f64,
    /// Default model volatility
    pub default_volatility: f64,
    /// Refinancing sensitivity (for Richard-Roll model)
    pub refi_sensitivity: f64,
    /// Burnout rate (for Richard-Roll model)
    pub burnout_rate: f64,
}

/// Standard RMBS calibration (prime/agency).
///
/// Based on historical agency RMBS performance:
/// - Low default rates (2% annual CDR)
/// - Low correlation (5%) due to pool diversification
/// - Moderate prepayment (6% CPR base)
/// - Standard PSA-style seasoning
pub const RMBS_STANDARD: RmbsCalibration = RmbsCalibration {
    base_cdr: 0.02,
    default_correlation: 0.05,
    base_cpr: 0.06,
    prepay_factor_loading: 0.4,
    cpr_volatility: 0.20,
    default_factor_sensitivity: 0.5,
    default_mean_reversion: 0.5,
    default_volatility: 0.30,
    refi_sensitivity: 2.0,
    burnout_rate: 0.10,
};

/// CLO standard calibration parameters.
///
/// Suitable for broadly syndicated loan CLOs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CloCalibration {
    /// Base annual conditional default rate (CDR)
    pub base_cdr: f64,
    /// Default correlation (asset correlation)
    pub default_correlation: f64,
    /// Base annual conditional prepayment rate (CPR)
    pub base_cpr: f64,
    /// Prepayment factor loading
    pub prepay_factor_loading: f64,
    /// CPR volatility
    pub cpr_volatility: f64,
    /// Default factor sensitivity
    pub default_factor_sensitivity: f64,
    /// Default model mean reversion speed
    pub default_mean_reversion: f64,
    /// Default model volatility
    pub default_volatility: f64,
}

/// Standard CLO calibration (broadly syndicated loans).
///
/// Based on historical CLO performance:
/// - Higher default rates (3% annual CDR)
/// - Higher correlation (20-25%) for corporate exposures
/// - Higher prepayment (15% CPR) due to refinancing
pub const CLO_STANDARD: CloCalibration = CloCalibration {
    base_cdr: 0.03,
    default_correlation: 0.20,
    base_cpr: 0.15,
    prepay_factor_loading: 0.25,
    cpr_volatility: 0.15,
    default_factor_sensitivity: 0.8,
    default_mean_reversion: 0.3,
    default_volatility: 0.40,
};

/// CMBS standard calibration parameters.
///
/// Suitable for conduit CMBS transactions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CmbsCalibration {
    /// Base annual conditional default rate (CDR)
    pub base_cdr: f64,
    /// Default correlation (asset correlation)
    pub default_correlation: f64,
    /// Base annual conditional prepayment rate (CPR)
    pub base_cpr: f64,
    /// Prepayment factor loading
    pub prepay_factor_loading: f64,
    /// CPR volatility
    pub cpr_volatility: f64,
}

/// Standard CMBS calibration (conduit).
///
/// Commercial mortgages have:
/// - Moderate default rates (2.5% annual CDR)
/// - Moderate correlation (15%)
/// - Low prepayment due to lockouts/defeasance (3% CPR)
pub const CMBS_STANDARD: CmbsCalibration = CmbsCalibration {
    base_cdr: 0.025,
    default_correlation: 0.15,
    base_cpr: 0.03,
    prepay_factor_loading: 0.20,
    cpr_volatility: 0.10,
};

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_rmbs_standard_values() {
        assert!((RMBS_STANDARD.base_cdr - 0.02).abs() < 1e-10);
        assert!((RMBS_STANDARD.default_correlation - 0.05).abs() < 1e-10);
        assert!((RMBS_STANDARD.base_cpr - 0.06).abs() < 1e-10);
    }

    #[test]
    fn test_clo_standard_values() {
        assert!((CLO_STANDARD.base_cdr - 0.03).abs() < 1e-10);
        assert!((CLO_STANDARD.default_correlation - 0.20).abs() < 1e-10);
        assert!((CLO_STANDARD.base_cpr - 0.15).abs() < 1e-10);
    }

    #[test]
    fn test_cmbs_standard_values() {
        assert!((CMBS_STANDARD.base_cdr - 0.025).abs() < 1e-10);
        assert!((CMBS_STANDARD.default_correlation - 0.15).abs() < 1e-10);
    }

    #[test]
    fn test_clo_higher_correlation_than_rmbs() {
        // Corporate loans have higher correlation than mortgages
        let clo_corr = CLO_STANDARD.default_correlation;
        let rmbs_corr = RMBS_STANDARD.default_correlation;
        assert!(
            clo_corr > rmbs_corr,
            "CLO correlation ({}) should exceed RMBS ({})",
            clo_corr,
            rmbs_corr
        );
    }

    #[test]
    fn test_cmbs_lower_prepayment_than_rmbs() {
        // Commercial mortgages have lockouts limiting prepayment
        let cmbs_cpr = CMBS_STANDARD.base_cpr;
        let rmbs_cpr = RMBS_STANDARD.base_cpr;
        assert!(
            cmbs_cpr < rmbs_cpr,
            "CMBS CPR ({}) should be less than RMBS ({})",
            cmbs_cpr,
            rmbs_cpr
        );
    }
}
