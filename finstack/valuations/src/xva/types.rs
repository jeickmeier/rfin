//! Core types for the XVA (Valuation Adjustments) framework.
//!
//! Defines configuration, result containers, netting set specifications,
//! and CSA (Credit Support Annex) terms used across all XVA calculations.

/// XVA calculation configuration.
///
/// Controls the time grid, recovery assumptions, and optional modeling
/// features for exposure simulation and CVA computation.
///
/// # Defaults
///
/// The default configuration provides a quarterly time grid out to 30 years
/// with a 40% recovery rate (ISDA standard for senior unsecured).
///
/// # References
///
/// - Gregory, J. (2020). *The xVA Challenge*, 4th ed. Wiley. Chapter 8 (Exposure).
/// - BCBS 325 (2014). "Fundamental review of the trading book."
#[derive(Clone, Debug)]
pub struct XvaConfig {
    /// Time grid for exposure simulation (years from today).
    ///
    /// Determines the granularity of exposure profiles. Finer grids
    /// improve accuracy but increase computation cost.
    pub time_grid: Vec<f64>,

    /// Recovery rate for counterparty default (typically 0.40).
    ///
    /// Market standard for senior unsecured is 40%, per ISDA conventions
    /// and CDS pricing practices.
    pub recovery_rate: f64,

    /// Whether to include wrong-way risk (placeholder for future implementation).
    ///
    /// When enabled, correlation between exposure and default probability
    /// is modeled, which can significantly increase CVA for certain portfolios.
    pub include_wrong_way_risk: bool,
}

impl Default for XvaConfig {
    /// Creates a standard quarterly grid to 30Y with 40% recovery.
    fn default() -> Self {
        // Quarterly grid out to 30 years: 0.25, 0.50, ..., 30.0
        let time_grid: Vec<f64> = (1..=120).map(|i| i as f64 * 0.25).collect();
        Self {
            time_grid,
            recovery_rate: 0.40,
            include_wrong_way_risk: false,
        }
    }
}

impl XvaConfig {
    /// Validate configuration parameters.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Time grid is empty
    /// - Time grid contains non-positive or non-finite values
    /// - Time grid is not strictly increasing
    /// - Recovery rate is not in `[0, 1]`
    pub fn validate(&self) -> finstack_core::Result<()> {
        if self.time_grid.is_empty() {
            return Err(finstack_core::Error::Validation(
                "XvaConfig: time_grid must not be empty".into(),
            ));
        }

        for (i, &t) in self.time_grid.iter().enumerate() {
            if !t.is_finite() || t <= 0.0 {
                return Err(finstack_core::Error::Validation(format!(
                    "XvaConfig: time_grid[{i}] = {t} must be positive and finite"
                )));
            }
            if i > 0 && t <= self.time_grid[i - 1] {
                return Err(finstack_core::Error::Validation(format!(
                    "XvaConfig: time_grid must be strictly increasing at index {i}"
                )));
            }
        }

        if !(0.0..=1.0).contains(&self.recovery_rate) {
            return Err(finstack_core::Error::Validation(format!(
                "XvaConfig: recovery_rate {} must be in [0, 1]",
                self.recovery_rate
            )));
        }

        Ok(())
    }
}

/// Result of XVA calculations.
///
/// Contains the CVA value along with the full exposure profile and
/// regulatory metrics used for reporting and risk management.
///
/// # Exposure Profiles
///
/// Each profile entry is a `(time, value)` pair where time is in years
/// from the valuation date and value is in the portfolio's base currency.
#[derive(Clone, Debug)]
pub struct XvaResult {
    /// Unilateral CVA (positive = cost to the desk).
    ///
    /// Represents the expected loss due to counterparty default,
    /// discounted to present value.
    pub cva: f64,

    /// Expected Positive Exposure profile: `(time, EPE(t))`.
    ///
    /// EPE(t) = E[max(V(t), 0)] — the average positive mark-to-market
    /// at each time point.
    pub epe_profile: Vec<(f64, f64)>,

    /// Expected Negative Exposure profile: `(time, ENE(t))`.
    ///
    /// ENE(t) = E[max(-V(t), 0)] — the average negative mark-to-market
    /// at each time point (own-default exposure).
    pub ene_profile: Vec<(f64, f64)>,

    /// Potential Future Exposure at 97.5% quantile: `(time, PFE(t))`.
    ///
    /// For the simplified deterministic model, PFE equals EPE since
    /// there is a single scenario. In a full Monte Carlo implementation,
    /// this would represent the 97.5th percentile of the exposure distribution.
    pub pfe_profile: Vec<(f64, f64)>,

    /// Maximum PFE across the profile.
    ///
    /// max_t PFE(t) — used for credit limit monitoring.
    pub max_pfe: f64,

    /// Effective EPE profile: `(time, Effective_EPE(t))`.
    ///
    /// Non-decreasing version of EPE, per Basel III SA-CCR:
    /// `Effective_EPE(t_k) = max(Effective_EPE(t_{k-1}), EPE(t_k))`
    ///
    /// # References
    ///
    /// - BCBS 279 (2014). "The standardised approach for measuring
    ///   counterparty credit risk exposures."
    pub effective_epe_profile: Vec<(f64, f64)>,

    /// Time-weighted average of Effective EPE (regulatory scalar metric).
    ///
    /// Computed as:
    /// ```text
    /// Effective_EPE_avg = (1 / min(1, M)) × Σₖ Effective_EPE(tₖ) × Δtₖ
    /// ```
    ///
    /// where `M` is the portfolio maturity and `Δtₖ = tₖ - tₖ₋₁`.
    /// This is the key input for EAD under SA-CCR.
    ///
    /// # References
    ///
    /// - BCBS 279 (2014). "The standardised approach for measuring
    ///   counterparty credit risk exposures."
    pub effective_epe: f64,
}

/// Exposure profile computed at each time grid point.
///
/// This is the intermediate result from exposure simulation,
/// consumed by the CVA calculator.
#[derive(Clone, Debug)]
pub struct ExposureProfile {
    /// Time points in years from valuation date.
    pub times: Vec<f64>,

    /// Portfolio mark-to-market value at each time point (may be negative).
    pub mtm_values: Vec<f64>,

    /// Expected Positive Exposure at each time point: max(V(t), 0).
    pub epe: Vec<f64>,

    /// Expected Negative Exposure at each time point: max(-V(t), 0).
    pub ene: Vec<f64>,
}

impl ExposureProfile {
    /// Validate that the exposure profile is internally consistent.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Any vector lengths are inconsistent
    /// - Times are not strictly increasing
    /// - EPE or ENE contain negative or non-finite values
    /// - MtM values are non-finite
    pub fn validate(&self) -> finstack_core::Result<()> {
        let n = self.times.len();

        if self.mtm_values.len() != n || self.epe.len() != n || self.ene.len() != n {
            return Err(finstack_core::Error::Validation(format!(
                "ExposureProfile: vector lengths must be equal \
                 (times={}, mtm={}, epe={}, ene={})",
                n,
                self.mtm_values.len(),
                self.epe.len(),
                self.ene.len()
            )));
        }

        for (i, &t) in self.times.iter().enumerate() {
            if !t.is_finite() || t <= 0.0 {
                return Err(finstack_core::Error::Validation(format!(
                    "ExposureProfile: times[{i}] = {t} must be positive and finite"
                )));
            }
            if i > 0 && t <= self.times[i - 1] {
                return Err(finstack_core::Error::Validation(format!(
                    "ExposureProfile: times must be strictly increasing at index {i}"
                )));
            }
        }

        for (i, (&epe_v, &ene_v)) in self.epe.iter().zip(self.ene.iter()).enumerate() {
            if !epe_v.is_finite() || epe_v < 0.0 {
                return Err(finstack_core::Error::Validation(format!(
                    "ExposureProfile: epe[{i}] = {epe_v} must be non-negative and finite"
                )));
            }
            if !ene_v.is_finite() || ene_v < 0.0 {
                return Err(finstack_core::Error::Validation(format!(
                    "ExposureProfile: ene[{i}] = {ene_v} must be non-negative and finite"
                )));
            }
        }

        for (i, &mtm) in self.mtm_values.iter().enumerate() {
            if !mtm.is_finite() {
                return Err(finstack_core::Error::Validation(format!(
                    "ExposureProfile: mtm_values[{i}] = {mtm} must be finite"
                )));
            }
        }

        Ok(())
    }
}

/// A netting set: collection of trades under a single ISDA master agreement.
///
/// Under a valid ISDA master agreement, upon counterparty default the
/// portfolio is closed out on a net basis — positive and negative values
/// offset each other before determining the credit exposure.
///
/// # References
///
/// - ISDA (2002). "2002 ISDA Master Agreement."
/// - Gregory, J. (2020). *The xVA Challenge*, Chapter 6.
#[derive(Clone, Debug)]
pub struct NettingSet {
    /// Unique identifier for this netting set.
    pub id: String,

    /// Counterparty identifier (maps to hazard curve).
    pub counterparty_id: String,

    /// CSA terms (if any) governing collateral exchange.
    ///
    /// `None` means uncollateralized — full exposure is at risk.
    pub csa: Option<CsaTerms>,
}

/// Credit Support Annex terms for collateralization.
///
/// Models the key economic terms of an ISDA CSA that determine
/// how collateral reduces counterparty credit exposure.
///
/// # Collateral Mechanics
///
/// ```text
/// Net exposure = Portfolio MtM - Collateral held
/// Collateral call = max(MtM - Threshold - MTA, 0)
/// Effective exposure = max(MtM - Collateral - IA, 0)
/// ```
///
/// The independent amount (IA) is additional collateral posted by the
/// counterparty that further reduces the credit exposure beyond
/// the variation margin collateral call.
///
/// # References
///
/// - ISDA (2016). "Credit Support Annex for Variation Margin."
/// - Gregory, J. (2020). *The xVA Challenge*, Chapter 7.
#[derive(Clone, Debug)]
pub struct CsaTerms {
    /// Threshold below which no collateral is required.
    ///
    /// For investment-grade counterparties, typical thresholds
    /// range from $0 (zero threshold) to $50M.
    pub threshold: f64,

    /// Minimum transfer amount (MTA).
    ///
    /// Collateral is only exchanged when the call amount exceeds
    /// this minimum. Typical values: $250K–$1M.
    pub mta: f64,

    /// Margin period of risk (MPOR) in calendar days.
    ///
    /// The time needed to close out the portfolio after default.
    /// Regulatory standard: 10 days for bilateral, 5 days for cleared.
    ///
    /// **Note**: This field is stored for future MPOR-aware exposure modeling.
    /// The current deterministic exposure engine does not yet incorporate MPOR
    /// into collateral dynamics (the gap risk during the close-out period).
    pub mpor_days: u32,

    /// Independent amount (initial margin).
    ///
    /// Additional collateral posted regardless of MtM,
    /// reducing exposure by a fixed buffer.
    pub independent_amount: f64,
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        let config = XvaConfig::default();
        config.validate().expect("Default config should be valid");
        assert_eq!(config.time_grid.len(), 120); // quarterly to 30Y
        assert!((config.recovery_rate - 0.40).abs() < f64::EPSILON);
        assert!(!config.include_wrong_way_risk);
    }

    #[test]
    fn config_validation_rejects_empty_grid() {
        let config = XvaConfig {
            time_grid: vec![],
            recovery_rate: 0.40,
            include_wrong_way_risk: false,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn config_validation_rejects_non_increasing_grid() {
        let config = XvaConfig {
            time_grid: vec![1.0, 0.5, 2.0],
            recovery_rate: 0.40,
            include_wrong_way_risk: false,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn config_validation_rejects_bad_recovery() {
        let config = XvaConfig {
            time_grid: vec![1.0, 2.0],
            recovery_rate: 1.5,
            include_wrong_way_risk: false,
        };
        assert!(config.validate().is_err());

        let config_neg = XvaConfig {
            time_grid: vec![1.0, 2.0],
            recovery_rate: -0.1,
            include_wrong_way_risk: false,
        };
        assert!(config_neg.validate().is_err());
    }

    #[test]
    fn config_validation_rejects_non_positive_times() {
        let config = XvaConfig {
            time_grid: vec![0.0, 1.0],
            recovery_rate: 0.40,
            include_wrong_way_risk: false,
        };
        assert!(config.validate().is_err());
    }

    // ── ExposureProfile validation tests ─────────────────────────

    #[test]
    fn profile_validate_valid() {
        let profile = ExposureProfile {
            times: vec![0.25, 0.5, 1.0],
            mtm_values: vec![100.0, -50.0, 25.0],
            epe: vec![100.0, 0.0, 25.0],
            ene: vec![0.0, 50.0, 0.0],
        };
        profile.validate().expect("Valid profile should pass");
    }

    #[test]
    fn profile_validate_rejects_mismatched_lengths() {
        let profile = ExposureProfile {
            times: vec![0.25, 0.5],
            mtm_values: vec![100.0],
            epe: vec![100.0, 0.0],
            ene: vec![0.0, 50.0],
        };
        assert!(profile.validate().is_err());
    }

    #[test]
    fn profile_validate_rejects_negative_epe() {
        let profile = ExposureProfile {
            times: vec![0.25],
            mtm_values: vec![100.0],
            epe: vec![-1.0],
            ene: vec![0.0],
        };
        assert!(profile.validate().is_err());
    }

    #[test]
    fn profile_validate_rejects_nan_mtm() {
        let profile = ExposureProfile {
            times: vec![0.25],
            mtm_values: vec![f64::NAN],
            epe: vec![0.0],
            ene: vec![0.0],
        };
        assert!(profile.validate().is_err());
    }

    #[test]
    fn profile_validate_rejects_non_increasing_times() {
        let profile = ExposureProfile {
            times: vec![1.0, 0.5],
            mtm_values: vec![100.0, 50.0],
            epe: vec![100.0, 50.0],
            ene: vec![0.0, 0.0],
        };
        assert!(profile.validate().is_err());
    }
}
