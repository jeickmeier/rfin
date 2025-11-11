//! Default model specifications for credit instruments.

use finstack_core::dates::Date;

/// Default curve shape.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "curve", rename_all = "snake_case"))]
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
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DefaultModelSpec {
    /// CDR: Constant Default Rate (annual, e.g., 0.02 for 2%)
    pub cdr: f64,
    /// Optional curve shape (default: constant)
    #[cfg_attr(feature = "serde", serde(default))]
    pub curve: Option<DefaultCurve>,
}

impl DefaultModelSpec {
    /// Calculate MDR (monthly default rate) for given seasoning.
    pub fn mdr(&self, seasoning_months: u32) -> f64 {
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

        use super::super::credit_rates::annual_to_monthly;
        annual_to_monthly(cdr)
    }

    /// Constant CDR (no curve).
    pub fn constant_cdr(cdr: f64) -> Self {
        Self { cdr, curve: None }
    }

    /// SDA curve with multiplier (1.0 = 100% SDA).
    pub fn sda(speed_multiplier: f64) -> Self {
        Self {
            cdr: 0.03, // 100% SDA terminal rate
            curve: Some(DefaultCurve::Sda { speed_multiplier }),
        }
    }

    /// 100% SDA (standard default assumption).
    pub fn sda_100() -> Self {
        Self::sda(1.0)
    }

    /// 2% CDR (common baseline).
    pub fn cdr_2pct() -> Self {
        Self::constant_cdr(0.02)
    }
}

/// Default event specification.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DefaultEvent {
    /// Date when default occurs
    pub default_date: Date,
    /// Amount that defaults
    pub defaulted_amount: f64,
    /// Recovery rate (0.0 to 1.0)
    pub recovery_rate: f64,
    /// Recovery lag in months
    pub recovery_lag: u32,
}

