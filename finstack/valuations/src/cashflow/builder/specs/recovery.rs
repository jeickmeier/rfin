//! Recovery model specifications for credit instruments.

/// Recovery model specification.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
}
