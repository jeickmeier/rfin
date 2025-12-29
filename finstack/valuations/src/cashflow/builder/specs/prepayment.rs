//! Prepayment model specifications for credit instruments.

use finstack_core::types::Percentage;

/// Prepayment curve shape.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "curve", rename_all = "snake_case"))]
pub enum PrepaymentCurve {
    /// Constant CPR (no seasoning effect)
    Constant,
    /// PSA standard curve: ramps to 6% CPR over 30 months
    Psa {
        /// Speed multiplier (1.0 = 100% PSA)
        speed_multiplier: f64,
    },
}

/// Prepayment model specification.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PrepaymentModelSpec {
    /// CPR: Constant Prepayment Rate (annual, e.g., 0.06 for 6%)
    pub cpr: f64,
    /// Optional curve shape (default: constant)
    #[cfg_attr(feature = "serde", serde(default))]
    pub curve: Option<PrepaymentCurve>,
}

impl PrepaymentModelSpec {
    /// Calculate SMM (monthly prepayment rate) for given seasoning.
    pub fn smm(&self, seasoning_months: u32) -> f64 {
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
}
