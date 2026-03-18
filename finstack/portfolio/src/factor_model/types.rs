use crate::PositionId;
use finstack_core::factor_model::{FactorId, RiskMeasure};
use serde::{Deserialize, Serialize};

/// Portfolio-level decomposition of total risk across common factors and residuals.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RiskDecomposition {
    /// Total portfolio risk under the selected `measure`.
    pub total_risk: f64,
    /// Risk measure used to aggregate and report the decomposition.
    pub measure: RiskMeasure,
    /// Aggregate factor-level contributions to portfolio risk.
    pub factor_contributions: Vec<FactorContribution>,
    /// Unattributed or idiosyncratic risk left after factor aggregation.
    pub residual_risk: f64,
    /// Per-position, per-factor contributions that roll up into the portfolio view.
    pub position_factor_contributions: Vec<PositionFactorContribution>,
}

/// Contribution of a single factor to portfolio risk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FactorContribution {
    /// Identifier of the factor being reported.
    pub factor_id: FactorId,
    /// Absolute contribution of the factor to the chosen risk measure.
    pub absolute_risk: f64,
    /// Contribution expressed as a share of total portfolio risk.
    pub relative_risk: f64,
    /// Marginal sensitivity of portfolio risk to the factor.
    pub marginal_risk: f64,
}

/// Contribution of a single position to a specific factor bucket.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PositionFactorContribution {
    /// Portfolio position identifier.
    pub position_id: PositionId,
    /// Identifier of the contributing factor.
    pub factor_id: FactorId,
    /// Risk attributed to this position-factor pair.
    pub risk_contribution: f64,
}

#[cfg(test)]
mod tests {
    use super::{FactorContribution, RiskDecomposition};
    use finstack_core::factor_model::{FactorId, RiskMeasure};

    #[test]
    fn test_risk_decomposition_total_matches_sum() {
        let decomp = RiskDecomposition {
            total_risk: 100.0,
            measure: RiskMeasure::Variance,
            factor_contributions: vec![
                FactorContribution {
                    factor_id: FactorId::new("Rates"),
                    absolute_risk: 60.0,
                    relative_risk: 0.6,
                    marginal_risk: 0.3,
                },
                FactorContribution {
                    factor_id: FactorId::new("Credit"),
                    absolute_risk: 40.0,
                    relative_risk: 0.4,
                    marginal_risk: 0.2,
                },
            ],
            residual_risk: 0.0,
            position_factor_contributions: vec![],
        };

        let sum: f64 = decomp
            .factor_contributions
            .iter()
            .map(|c| c.absolute_risk)
            .sum();
        assert!((sum + decomp.residual_risk - decomp.total_risk).abs() < 1e-10);
    }

    #[test]
    fn test_relative_risk_sums_to_one_when_residual_risk_is_zero() {
        let decomp = RiskDecomposition {
            total_risk: 100.0,
            measure: RiskMeasure::Variance,
            factor_contributions: vec![
                FactorContribution {
                    factor_id: FactorId::new("Rates"),
                    absolute_risk: 60.0,
                    relative_risk: 0.6,
                    marginal_risk: 0.3,
                },
                FactorContribution {
                    factor_id: FactorId::new("Credit"),
                    absolute_risk: 40.0,
                    relative_risk: 0.4,
                    marginal_risk: 0.2,
                },
            ],
            residual_risk: 0.0,
            position_factor_contributions: vec![],
        };

        let sum_rel: f64 = decomp
            .factor_contributions
            .iter()
            .map(|c| c.relative_risk)
            .sum();
        assert!((sum_rel - 1.0).abs() < 1e-10);
    }
}
