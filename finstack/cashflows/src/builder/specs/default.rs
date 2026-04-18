//! Default model specifications for credit instruments.

use finstack_core::dates::{BusinessDayConvention, Date};

/// Default curve shape.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "curve", rename_all = "snake_case")]
pub enum DefaultCurve {
    /// Constant CDR (no seasoning effect)
    Constant,
    /// SDA standard curve: ramps to peak then declines
    Sda {
        /// Speed multiplier (1.0 = 100% SDA)
        speed_multiplier: f64,
    },
}

/// Default model specification.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct DefaultModelSpec {
    /// CDR: Constant Default Rate (annual, e.g., 0.02 for 2%)
    pub cdr: f64,
    /// Optional curve shape (default: constant)
    #[serde(default)]
    pub curve: Option<DefaultCurve>,
}

impl DefaultModelSpec {
    /// Calculate MDR (monthly default rate) for the supplied seasoning.
    ///
    /// # Formula
    ///
    /// For the constant curve, the method converts annual CDR to monthly MDR
    /// using:
    ///
    /// `MDR = 1 - (1 - CDR)^(1/12)`
    ///
    /// For the SDA curve, the annual CDR is first derived from seasoning:
    ///
    /// - months `1..=30`: linear ramp to a 6% annual CDR peak
    /// - months `31..=60`: linear decline from 6% to a 3% terminal annual CDR
    /// - months `> 60`: flat 3% annual CDR terminal level
    ///
    /// The `speed_multiplier` scales the resulting annual CDR before conversion
    /// into MDR.
    ///
    /// # Arguments
    ///
    /// * `seasoning_months` - Number of months since origination or pool start.
    ///
    /// # Returns
    ///
    /// Monthly default rate as a decimal.
    ///
    /// # Errors
    ///
    /// Returns an error if the derived annual CDR is negative.
    ///
    /// # References
    ///
    /// - `docs/REFERENCES.md#isda-cds-standard-model`
    /// - `docs/REFERENCES.md#tuckman-serrat-fixed-income`
    pub fn mdr(&self, seasoning_months: u32) -> finstack_core::Result<f64> {
        let cdr = match &self.curve {
            None | Some(DefaultCurve::Constant) => self.cdr,
            Some(DefaultCurve::Sda { speed_multiplier }) => {
                // SDA: peak at month 30, decline to terminal
                const PEAK_MONTH: u32 = 30;
                const PEAK_CDR: f64 = 0.06;
                const TERMINAL_CDR: f64 = 0.03;

                let base = if seasoning_months <= PEAK_MONTH {
                    (seasoning_months as f64 / PEAK_MONTH as f64) * PEAK_CDR
                } else if seasoning_months <= PEAK_MONTH + 30 {
                    let past_peak = (seasoning_months - PEAK_MONTH) as f64;
                    PEAK_CDR - (past_peak / 30.0) * (PEAK_CDR - TERMINAL_CDR)
                } else {
                    TERMINAL_CDR
                };
                base * speed_multiplier
            }
        };

        use super::super::credit_rates::cpr_to_smm;
        cpr_to_smm(cdr)
    }

    /// Constant CDR (no curve).
    pub fn constant_cdr(cdr: f64) -> Self {
        Self { cdr, curve: None }
    }

    /// SDA curve with multiplier (1.0 = 100% SDA).
    ///
    /// The implementation ramps annual CDR to a 6% peak by month 30, then
    /// decays linearly to a 3% terminal annual CDR by month 60.
    pub fn sda(speed_multiplier: f64) -> Self {
        Self {
            cdr: 0.03, // 100% SDA terminal rate
            curve: Some(DefaultCurve::Sda { speed_multiplier }),
        }
    }

    /// 2% CDR (common baseline).
    pub fn cdr_2pct() -> Self {
        Self::constant_cdr(0.02)
    }
}

/// Default event specification.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct DefaultEvent {
    /// Date when default occurs
    #[schemars(with = "String")]
    pub default_date: Date,
    /// Amount that defaults
    pub defaulted_amount: f64,
    /// Recovery rate (0.0 to 1.0)
    pub recovery_rate: f64,
    /// Recovery lag in months
    pub recovery_lag: u32,
    /// Optional business-day convention for recovery date adjustment.
    ///
    /// When `None`, recovery dates are computed using a simple calendar
    /// month offset with no adjustment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovery_bdc: Option<BusinessDayConvention>,
    /// Optional holiday calendar identifier used for recovery date adjustment.
    ///
    /// When `None`, calendar-aware adjustment is skipped and the recovery
    /// date is left as the raw lagged date.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovery_calendar_id: Option<String>,
    /// Pre-computed accrued interest amount at default (ISDA standard).
    ///
    /// When `Some(amt)` and `amt > 0.0`, an additional `AccruedOnDefault`
    /// cashflow is emitted on the default date. The accrued amount should
    /// be computed by the caller using `accrued_interest_amount()`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accrued_on_default: Option<f64>,
}

impl DefaultEvent {
    /// Validate the default event parameters.
    ///
    /// # Errors
    ///
    /// Returns `InputError::Invalid` if:
    /// - `recovery_rate` is not in `[0.0, 1.0]`
    /// - `defaulted_amount` is negative
    pub fn validate(&self) -> finstack_core::Result<()> {
        use finstack_core::InputError;

        if !(0.0..=1.0).contains(&self.recovery_rate) {
            return Err(finstack_core::Error::Input(InputError::Invalid));
        }
        if self.defaulted_amount < 0.0 {
            return Err(finstack_core::Error::Input(InputError::Invalid));
        }
        Ok(())
    }
}
