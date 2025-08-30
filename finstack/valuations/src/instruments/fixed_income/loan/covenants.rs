//! Covenant specifications for loans.

use finstack_core::dates::{Date, Frequency};
use finstack_core::F;

/// Financial covenant specification.
#[derive(Clone, Debug)]
pub struct Covenant {
    /// Type of covenant
    pub covenant_type: CovenantType,
    /// Frequency of testing
    pub test_frequency: Frequency,
    /// Grace period in days after breach
    pub cure_period_days: Option<i32>,
    /// Consequences if breached and not cured
    pub consequences: Vec<CovenantConsequence>,
    /// Whether the covenant is currently in effect
    pub is_active: bool,
}

/// Type of financial covenant.
#[derive(Clone, Debug)]
pub enum CovenantType {
    /// Maximum debt to EBITDA ratio
    MaxDebtToEBITDA {
        /// Maximum allowed ratio
        threshold: F,
    },
    /// Minimum interest coverage ratio
    MinInterestCoverage {
        /// Minimum required ratio
        threshold: F,
    },
    /// Minimum fixed charge coverage ratio
    MinFixedChargeCoverage {
        /// Minimum required ratio
        threshold: F,
    },
    /// Maximum total leverage ratio
    MaxTotalLeverage {
        /// Maximum allowed ratio
        threshold: F,
    },
    /// Maximum senior leverage ratio  
    MaxSeniorLeverage {
        /// Maximum allowed ratio
        threshold: F,
    },
    /// Minimum asset coverage ratio
    MinAssetCoverage {
        /// Minimum required ratio
        threshold: F,
    },
    /// Negative covenant (restriction)
    Negative {
        /// Description of restriction
        restriction: String,
    },
    /// Affirmative covenant (requirement)
    Affirmative {
        /// Description of requirement
        requirement: String,
    },
    /// Custom financial metric test
    Custom {
        /// Metric name
        metric: String,
        /// Test operator and threshold
        test: ThresholdTest,
    },
}

/// Direction of threshold test.
#[derive(Clone, Copy, Debug)]
pub enum ThresholdTest {
    /// Value must be <= threshold
    Maximum(F),
    /// Value must be >= threshold
    Minimum(F),
}

/// Consequences of covenant breach.
#[derive(Clone, Debug)]
pub enum CovenantConsequence {
    /// Event of default
    Default,
    /// Increase in interest rate
    RateIncrease {
        /// Basis points to add to rate
        bp_increase: F,
    },
    /// Mandatory cash sweep
    CashSweep {
        /// Percentage of excess cash to sweep
        sweep_percentage: F,
    },
    /// Block on distributions to equity
    BlockDistributions,
    /// Require additional collateral
    RequireCollateral {
        /// Description of required collateral
        description: String,
    },
    /// Accelerate maturity
    AccelerateMaturity {
        /// New maturity date
        new_maturity: Date,
    },
}

impl Covenant {
    /// Creates a new covenant.
    pub fn new(covenant_type: CovenantType, test_frequency: Frequency) -> Self {
        Self {
            covenant_type,
            test_frequency,
            cure_period_days: Some(30), // Default 30 day cure period
            consequences: Vec::new(),
            is_active: true,
        }
    }

    /// Sets the cure period.
    pub fn with_cure_period(mut self, days: Option<i32>) -> Self {
        self.cure_period_days = days;
        self
    }

    /// Adds a consequence for breach.
    pub fn with_consequence(mut self, consequence: CovenantConsequence) -> Self {
        self.consequences.push(consequence);
        self
    }

    /// Sets whether the covenant is active.
    pub fn set_active(mut self, active: bool) -> Self {
        self.is_active = active;
        self
    }

    /// Returns a description of the covenant.
    pub fn description(&self) -> String {
        match &self.covenant_type {
            CovenantType::MaxDebtToEBITDA { threshold } => {
                format!("Debt/EBITDA ≤ {:.2}x", threshold)
            }
            CovenantType::MinInterestCoverage { threshold } => {
                format!("Interest Coverage ≥ {:.2}x", threshold)
            }
            CovenantType::MinFixedChargeCoverage { threshold } => {
                format!("Fixed Charge Coverage ≥ {:.2}x", threshold)
            }
            CovenantType::MaxTotalLeverage { threshold } => {
                format!("Total Leverage ≤ {:.2}x", threshold)
            }
            CovenantType::MaxSeniorLeverage { threshold } => {
                format!("Senior Leverage ≤ {:.2}x", threshold)
            }
            CovenantType::MinAssetCoverage { threshold } => {
                format!("Asset Coverage ≥ {:.2}x", threshold)
            }
            CovenantType::Negative { restriction } => {
                format!("Negative: {}", restriction)
            }
            CovenantType::Affirmative { requirement } => {
                format!("Affirmative: {}", requirement)
            }
            CovenantType::Custom { metric, test } => match test {
                ThresholdTest::Maximum(v) => format!("{} ≤ {:.2}", metric, v),
                ThresholdTest::Minimum(v) => format!("{} ≥ {:.2}", metric, v),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_covenant_creation() {
        let covenant = Covenant::new(
            CovenantType::MaxDebtToEBITDA { threshold: 3.5 },
            Frequency::quarterly(),
        )
        .with_cure_period(Some(15))
        .with_consequence(CovenantConsequence::RateIncrease { bp_increase: 50.0 });

        assert_eq!(covenant.cure_period_days, Some(15));
        assert_eq!(covenant.consequences.len(), 1);
        assert!(covenant.is_active);
    }

    #[test]
    fn test_covenant_description() {
        let covenant = Covenant::new(
            CovenantType::MinInterestCoverage { threshold: 2.0 },
            Frequency::quarterly(),
        );

        assert_eq!(covenant.description(), "Interest Coverage ≥ 2.00x");
    }
}
