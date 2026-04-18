//! Prepayment model specifications for credit instruments.

use finstack_core::types::Percentage;

/// Prepayment curve shape.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "curve", rename_all = "snake_case")]
pub enum PrepaymentCurve {
    /// Constant CPR (no seasoning effect)
    Constant,
    /// PSA standard curve: ramps to 6% CPR over 30 months
    Psa {
        /// Speed multiplier (1.0 = 100% PSA)
        speed_multiplier: f64,
    },
    /// CMBS-style lockout: zero prepayment for an initial period, then constant CPR.
    ///
    /// Commercial mortgage-backed securities typically have prepayment lockout
    /// periods (defeasance/yield maintenance) lasting 5-10 years, after which
    /// voluntary prepayment resumes at the specified CPR.
    CmbsLockout {
        /// Number of months with zero prepayment (e.g., 60 for 5-year lockout)
        lockout_months: u32,
    },
}

/// Prepayment model specification.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct PrepaymentModelSpec {
    /// CPR: Constant Prepayment Rate (annual, e.g., 0.06 for 6%)
    pub cpr: f64,
    /// Optional curve shape (default: constant)
    #[serde(default)]
    pub curve: Option<PrepaymentCurve>,
}

impl PrepaymentModelSpec {
    /// Calculate SMM (single-month mortality) for the supplied seasoning.
    ///
    /// # Formula
    ///
    /// For the constant curve, the method converts annual CPR to monthly SMM
    /// using:
    ///
    /// `SMM = 1 - (1 - CPR)^(1/12)`
    ///
    /// For the PSA curve, the annual CPR is first derived from the seasoning:
    ///
    /// - months `1..=30`: `CPR = speed_multiplier * 0.06 * seasoning / 30`
    /// - months `> 30`: `CPR = speed_multiplier * 0.06`
    ///
    /// For the CMBS lockout curve:
    ///
    /// - months `<= lockout_months`: `CPR = 0`
    /// - months `> lockout_months`: `CPR = self.cpr`
    ///
    /// # Arguments
    ///
    /// * `seasoning_months` - Number of months since origination or pool start.
    ///
    /// # Returns
    ///
    /// Monthly prepayment rate as a decimal.
    ///
    /// # Errors
    ///
    /// Returns an error if the derived annual CPR is negative.
    ///
    /// # References
    ///
    /// - `docs/REFERENCES.md#tuckman-serrat-fixed-income`
    pub fn smm(&self, seasoning_months: u32) -> finstack_core::Result<f64> {
        let cpr = match &self.curve {
            None | Some(PrepaymentCurve::Constant) => self.cpr,
            Some(PrepaymentCurve::Psa { speed_multiplier }) => {
                // PSA: ramp to 6% CPR over 30 months, then flat
                const RAMP_MONTHS: u32 = 30;
                const TERMINAL_CPR: f64 = 0.06;

                let base = if seasoning_months <= RAMP_MONTHS {
                    (seasoning_months as f64 / RAMP_MONTHS as f64) * TERMINAL_CPR
                } else {
                    TERMINAL_CPR
                };
                base * speed_multiplier
            }
            Some(PrepaymentCurve::CmbsLockout { lockout_months }) => {
                // Zero prepayment during lockout, then constant CPR
                if seasoning_months <= *lockout_months {
                    0.0
                } else {
                    self.cpr
                }
            }
        };

        use super::super::credit_rates::cpr_to_smm;
        cpr_to_smm(cpr)
    }

    /// Constant CPR (no curve).
    pub fn constant_cpr(cpr: f64) -> Self {
        Self { cpr, curve: None }
    }

    /// Constant CPR (no curve) using a typed percentage.
    pub fn constant_cpr_pct(cpr: Percentage) -> Self {
        Self {
            cpr: cpr.as_decimal(),
            curve: None,
        }
    }

    /// PSA curve with multiplier (1.0 = 100% PSA).
    ///
    /// The implementation uses the standard PSA ramp to a 6% annual CPR over
    /// 30 months, then holds that terminal CPR flat.
    pub fn psa(speed_multiplier: f64) -> Self {
        Self {
            cpr: 0.06, // 100% PSA terminal rate
            curve: Some(PrepaymentCurve::Psa { speed_multiplier }),
        }
    }

    /// 100% PSA (standard prepayment assumption).
    pub fn psa_100() -> Self {
        Self::psa(1.0)
    }

    /// CMBS-style lockout: zero prepayment for `lockout_months`, then constant CPR.
    ///
    /// This models the common CMBS pattern where commercial mortgage borrowers
    /// face defeasance or yield maintenance penalties for an initial period,
    /// effectively preventing voluntary prepayment.
    ///
    /// # Arguments
    ///
    /// * `lockout_months` - Number of months with zero prepayment (e.g., 60 for 5 years)
    /// * `post_lockout_cpr` - Annual CPR after lockout expires (e.g., 0.10 for 10%)
    ///
    /// # Example
    ///
    /// ```text
    /// // 5-year lockout, then 10% CPR
    /// let spec = PrepaymentModelSpec::cmbs_with_lockout(60, 0.10);
    /// assert_eq!(spec.smm(30), 0.0);  // During lockout
    /// assert!(spec.smm(61) > 0.0);    // After lockout
    /// ```
    pub fn cmbs_with_lockout(lockout_months: u32, post_lockout_cpr: f64) -> Self {
        Self {
            cpr: post_lockout_cpr,
            curve: Some(PrepaymentCurve::CmbsLockout { lockout_months }),
        }
    }
}
