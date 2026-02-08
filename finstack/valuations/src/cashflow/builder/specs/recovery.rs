//! Recovery model specifications for credit instruments.

use finstack_core::types::Percentage;

/// Recovery model specification.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RecoveryModelSpec {
    /// Recovery rate as fraction (0.0 to 1.0, e.g., 0.40 for 40%)
    pub rate: f64,
    /// Recovery lag in months
    pub recovery_lag: u32,
}

impl RecoveryModelSpec {
    /// Standard recovery with lag.
    pub fn with_lag(rate: f64, recovery_lag: u32) -> Self {
        Self { rate, recovery_lag }
    }

    /// Standard recovery with lag using a typed percentage.
    pub fn with_lag_pct(rate: Percentage, recovery_lag: u32) -> Self {
        Self {
            rate: rate.as_decimal(),
            recovery_lag,
        }
    }

    /// Validate the recovery model parameters.
    ///
    /// # Errors
    ///
    /// Returns `Validation` error if:
    /// - `rate` is not in `[0.0, 1.0]`
    /// - `rate` is NaN or infinite
    pub fn validate(&self) -> finstack_core::Result<()> {
        if !self.rate.is_finite() || !(0.0..=1.0).contains(&self.rate) {
            return Err(finstack_core::Error::Validation(format!(
                "RecoveryModelSpec rate ({}) must be in [0.0, 1.0] and finite",
                self.rate
            )));
        }
        Ok(())
    }
}
