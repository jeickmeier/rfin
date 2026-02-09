//! Normalization Engine implementation.

use crate::adjustments::types::{
    Adjustment, AdjustmentCap, AdjustmentValue, AppliedAdjustment, NormalizationConfig,
    NormalizationResult,
};
use crate::error::{Error, Result};
use crate::evaluator::StatementResult;
use finstack_core::dates::PeriodId;
use indexmap::IndexMap;

/// Engine for calculating normalized metrics.
pub struct NormalizationEngine;

impl NormalizationEngine {
    /// Normalize a target node across all periods in the results.
    pub fn normalize(
        results: &StatementResult,
        config: &NormalizationConfig,
    ) -> Result<Vec<NormalizationResult>> {
        let mut normalization_results = Vec::new();

        // Get the target node values
        let target_values = results.nodes.get(&config.target_node).ok_or_else(|| {
            Error::eval(format!(
                "Target node '{}' not found in results",
                config.target_node
            ))
        })?;

        // Iterate over all periods where the target node has a value
        for (period_id, &base_value) in target_values {
            let mut applied_adjustments = Vec::new();
            let mut total_adjustments = 0.0;

            for adjustment in &config.adjustments {
                // Calculate raw adjustment amount
                let raw_amount = Self::calculate_adjustment_value(adjustment, *period_id, results)?;

                // Apply cap if present
                let (capped_amount, is_capped) = if let Some(cap) = &adjustment.cap {
                    Self::apply_cap(cap, raw_amount, *period_id, results)?
                } else {
                    (raw_amount, false)
                };

                applied_adjustments.push(AppliedAdjustment {
                    adjustment_id: adjustment.id.clone(),
                    name: adjustment.name.clone(),
                    raw_amount,
                    capped_amount,
                    is_capped,
                });

                total_adjustments += capped_amount;
            }

            normalization_results.push(NormalizationResult {
                period: *period_id,
                base_value,
                adjustments: applied_adjustments,
                final_value: base_value + total_adjustments,
            });
        }

        // Sort by period for consistent output
        normalization_results.sort_by_key(|r| r.period);

        Ok(normalization_results)
    }

    /// Merge normalization results back into the main StatementResult object as a new node.
    pub fn merge_into_results(
        results: &mut StatementResult,
        normalization_results: &[NormalizationResult],
        output_node_id: &str,
    ) {
        let mut period_map = IndexMap::new();
        for res in normalization_results {
            period_map.insert(res.period, res.final_value);
        }
        results.nodes.insert(output_node_id.to_string(), period_map);
    }

    /// Calculate the raw value of an adjustment for a specific period.
    fn calculate_adjustment_value(
        adjustment: &Adjustment,
        period_id: PeriodId,
        results: &StatementResult,
    ) -> Result<f64> {
        match &adjustment.value {
            AdjustmentValue::Fixed { amounts } => Ok(*amounts.get(&period_id).unwrap_or(&0.0)),
            AdjustmentValue::PercentageOfNode {
                node_id,
                percentage,
            } => {
                let node_values = results.nodes.get(node_id).ok_or_else(|| {
                    Error::eval(format!("Reference node '{}' not found", node_id))
                })?;

                let value = node_values.get(&period_id).unwrap_or(&0.0);
                Ok(value * percentage)
            }
            AdjustmentValue::Formula { .. } => {
                // Formula evaluation requires more complex context setup which we'll defer for now
                // or implement if strictly needed. For now returning error as per plan focus.
                Err(Error::eval("Formula adjustments not yet implemented"))
            }
        }
    }

    /// Apply capping logic to an adjustment amount.
    fn apply_cap(
        cap: &AdjustmentCap,
        raw_amount: f64,
        period_id: PeriodId,
        results: &StatementResult,
    ) -> Result<(f64, bool)> {
        let cap_limit = if let Some(base_node) = &cap.base_node {
            // Percentage of base node (e.g., 20% of EBITDA)
            let base_values = results
                .nodes
                .get(base_node)
                .ok_or_else(|| Error::eval(format!("Cap base node '{}' not found", base_node)))?;

            let base_value = base_values.get(&period_id).unwrap_or(&0.0);
            base_value.abs() * cap.value // Usually caps are based on absolute magnitude or positive EBITDA
        } else {
            // Fixed absolute cap
            cap.value
        };

        if raw_amount > cap_limit {
            Ok((cap_limit, true))
        } else {
            Ok((raw_amount, false))
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::adjustments::types::Adjustment;
    use crate::evaluator::StatementResult;
    use finstack_core::dates::PeriodId;
    use indexmap::IndexMap;

    fn mock_results() -> StatementResult {
        let mut results = StatementResult::new();
        let mut ebitda = IndexMap::new();
        ebitda.insert(PeriodId::quarter(2025, 1), 100.0);
        ebitda.insert(PeriodId::quarter(2025, 2), 120.0);
        results.nodes.insert("EBITDA".to_string(), ebitda);

        let mut revenue = IndexMap::new();
        revenue.insert(PeriodId::quarter(2025, 1), 1000.0);
        revenue.insert(PeriodId::quarter(2025, 2), 1200.0);
        results.nodes.insert("Revenue".to_string(), revenue);

        results
    }

    #[test]
    fn test_fixed_adjustment() {
        let results = mock_results();
        let mut amounts = IndexMap::new();
        amounts.insert(PeriodId::quarter(2025, 1), 10.0);
        amounts.insert(PeriodId::quarter(2025, 2), 15.0);

        let adj = Adjustment::fixed("addback1", "Addback 1", amounts);
        let config = NormalizationConfig::new("EBITDA").add_adjustment(adj);

        let normalized = NormalizationEngine::normalize(&results, &config)
            .expect("normalization should succeed for fixed adjustment");

        assert_eq!(normalized.len(), 2);
        assert_eq!(normalized[0].final_value, 110.0); // 100 + 10
        assert_eq!(normalized[1].final_value, 135.0); // 120 + 15
    }

    #[test]
    fn test_percentage_adjustment() {
        let results = mock_results();
        // 5% of Revenue
        let adj = Adjustment::percentage("perc1", "Perc 1", "Revenue", 0.05);
        let config = NormalizationConfig::new("EBITDA").add_adjustment(adj);

        let normalized = NormalizationEngine::normalize(&results, &config)
            .expect("normalization should succeed for percentage adjustment");

        // Q1: Revenue 1000 * 0.05 = 50. EBITDA 100 + 50 = 150.
        assert_eq!(normalized[0].adjustments[0].raw_amount, 50.0);
        assert_eq!(normalized[0].final_value, 150.0);
    }

    #[test]
    fn test_capped_adjustment() {
        let results = mock_results();
        // Synergies: 50.0 fixed. Cap: 20% of EBITDA.
        // Q1 EBITDA = 100. Cap = 20. Raw = 50. Result = 20 (Capped).
        let mut amounts = IndexMap::new();
        amounts.insert(PeriodId::quarter(2025, 1), 50.0);

        let adj = Adjustment::fixed("syn", "Synergies", amounts)
            .with_cap(Some("EBITDA".to_string()), 0.20);

        let config = NormalizationConfig::new("EBITDA").add_adjustment(adj);
        let normalized = NormalizationEngine::normalize(&results, &config)
            .expect("normalization should succeed for capped adjustment");

        let q1 = &normalized[0];
        assert_eq!(q1.adjustments[0].raw_amount, 50.0);
        assert_eq!(q1.adjustments[0].capped_amount, 20.0);
        assert!(q1.adjustments[0].is_capped);
        assert_eq!(q1.final_value, 120.0); // 100 + 20
    }

    #[test]
    fn test_merge_into_results() {
        let mut results = mock_results();
        let mut amounts = IndexMap::new();
        amounts.insert(PeriodId::quarter(2025, 1), 10.0);
        amounts.insert(PeriodId::quarter(2025, 2), 15.0);

        let adj = Adjustment::fixed("addback1", "Addback 1", amounts);
        let config = NormalizationConfig::new("EBITDA").add_adjustment(adj);

        let normalized = NormalizationEngine::normalize(&results, &config)
            .expect("normalization should succeed for merge test");

        NormalizationEngine::merge_into_results(&mut results, &normalized, "Adjusted EBITDA");

        assert!(results.nodes.contains_key("Adjusted EBITDA"));
        let adjusted = results
            .nodes
            .get("Adjusted EBITDA")
            .expect("Adjusted EBITDA should be added");
        assert_eq!(
            *adjusted
                .get(&PeriodId::quarter(2025, 1))
                .expect("Q1 adjusted value present"),
            110.0
        );
        assert_eq!(
            *adjusted
                .get(&PeriodId::quarter(2025, 2))
                .expect("Q2 adjusted value present"),
            135.0
        );
    }
}
