use super::problem::PortfolioOptimizationProblem;
use super::types::{MissingMetricPolicy, WeightingScheme};
use super::universe::PositionFilter;
use crate::error::{PortfolioError, Result};
use crate::position::Position;
use crate::types::PositionId;
use finstack_core::config::FinstackConfig;
use finstack_core::market_data::context::MarketContext;
use finstack_core::math::summation::neumaier_sum;
use finstack_valuations::metrics::MetricId;
use indexmap::IndexMap;

/// Result of building the decision space: items, features, current weights, and total PV.
type DecisionSpaceResult = (
    Vec<DecisionItem>,
    Vec<DecisionFeatures>,
    IndexMap<PositionId, f64>,
    f64,
);

/// Internal representation of a decision variable.
#[derive(Clone, Debug)]
pub(crate) struct DecisionItem {
    pub position_id: PositionId,
    /// Whether this represents an existing portfolio position.
    pub is_existing: bool,
    /// Whether this position is held (weight locked to current).
    pub is_held: bool,
    /// Current quantity held (if any).
    #[allow(dead_code)]
    pub current_quantity: f64,
}

/// Per‑decision‑variable features used to build linear forms.
#[derive(Clone, Debug)]
pub(crate) struct DecisionFeatures {
    /// Current base‑currency PV (scaled by quantity; 0 for candidates).
    pub pv_base: f64,
    /// Base‑currency PV per unit of quantity (for implied quantities).
    pub pv_per_unit: f64,
    /// Metric measures by string key (as in `ValuationResult::measures`).
    pub measures: IndexMap<String, f64>,
    /// Tags used by tag‑based constraints.
    pub tags: IndexMap<String, String>,
    /// Minimum weight bound.
    pub min_weight: f64,
    /// Maximum weight bound.
    pub max_weight: f64,
}

/// Check if a position matches a filter.
fn matches_filter(position: &Position, filter: &PositionFilter) -> bool {
    match filter {
        PositionFilter::All => true,
        PositionFilter::ByEntityId(id) => position.entity_id == *id,
        PositionFilter::ByTag { key, value } => position.tags.get(key) == Some(value),
        PositionFilter::ByPositionIds(ids) => ids.contains(&position.position_id),
        PositionFilter::Not(inner) => !matches_filter(position, inner),
    }
}

/// Build decision items and associated features from the portfolio and trade universe.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_decision_space(
    problem: &PortfolioOptimizationProblem,
    valuation: &crate::valuation::PortfolioValuation,
    required_metrics: &[MetricId],
    market: &MarketContext,
    config: &FinstackConfig,
) -> Result<DecisionSpaceResult> {
    let mut items = Vec::new();
    let mut features = Vec::new();
    let mut current_weights = IndexMap::new();

    // Map position id -> (pv_base, measures, tags, quantity)
    let mut total_pv_base = 0.0_f64;
    // Gross market value: sum of absolute PVs (for weight calculation with hedged portfolios)
    let mut gross_pv_base = 0.0_f64;
    // Track notional values for NotionalWeight scheme
    let mut position_notionals: IndexMap<PositionId, f64> = IndexMap::new();
    let mut gross_notional = 0.0_f64;

    for position in &problem.portfolio.positions {
        // Pull valuation for this position
        let pv_entry = valuation
            .position_values
            .get(&position.position_id)
            .ok_or_else(|| {
                PortfolioError::index_error(format!(
                    "missing valuation for position '{}'",
                    position.position_id
                ))
            })?;

        let pv_base = pv_entry.value_base.amount();
        total_pv_base = neumaier_sum([total_pv_base, pv_base].into_iter());
        gross_pv_base = neumaier_sum([gross_pv_base, pv_base.abs()].into_iter());
        // For NotionalWeight: use signed quantity as notional proxy
        position_notionals.insert(position.position_id.clone(), position.quantity);
        gross_notional = neumaier_sum([gross_notional, position.quantity.abs()].into_iter());

        // Extract measures
        let mut measures = IndexMap::new();
        if let Some(val_result) = &pv_entry.valuation_result {
            measures = val_result.measures.clone();
        } else if !required_metrics.is_empty()
            && matches!(problem.missing_metric_policy, MissingMetricPolicy::Strict)
        {
            return Err(PortfolioError::valuation(
                position.position_id.clone(),
                "valuation result missing required metrics",
            ));
        }

        // Decide if position is held / tradeable
        let is_held = if let Some(ref held) = problem.trade_universe.held_filter {
            matches_filter(position, held)
        } else {
            false
        };

        let is_tradeable = matches_filter(position, &problem.trade_universe.tradeable_filter);

        if !is_tradeable && !is_held {
            // Excluded from optimization entirely; still counted in total PV for weights.
            continue;
        }

        items.push(DecisionItem {
            position_id: position.position_id.clone(),
            is_existing: true,
            is_held,
            current_quantity: position.quantity,
        });

        // Default weight bounds: [0, 1]; will be refined by constraints later.
        features.push(DecisionFeatures {
            pv_base,
            // When quantity == 0, treat pv_per_unit as 0 to avoid division by zero.
            pv_per_unit: if position.quantity != 0.0 {
                pv_base / position.quantity
            } else {
                0.0
            },
            measures,
            tags: position.tags.clone(),
            min_weight: 0.0,
            max_weight: 1.0,
        });
    }

    // Batch price candidates using a temporary portfolio
    let candidate_valuation = if !problem.trade_universe.candidates.is_empty() {
        let mut builder = crate::builder::PortfolioBuilder::new("CANDIDATES")
            .base_ccy(problem.portfolio.base_ccy)
            .as_of(problem.portfolio.as_of);

        // Use a dummy entity for all candidates
        builder = builder.entity(crate::types::Entity::new("CANDIDATE_POOL"));

        for candidate in &problem.trade_universe.candidates {
            let pos = crate::position::Position::new(
                candidate.id.clone(),
                "CANDIDATE_POOL",
                candidate.instrument.id(),
                candidate.instrument.clone(),
                1.0, // Quantity 1.0 for unit pricing
                candidate.unit,
            )
            .map_err(|e| PortfolioError::invalid_input(e.to_string()))?
            .with_tags(candidate.tags.clone());

            builder = builder.position(pos);
        }

        let candidate_portfolio = builder
            .build()
            .map_err(|e| PortfolioError::invalid_input(e.to_string()))?;

        let options = crate::valuation::PortfolioValuationOptions {
            strict_risk: matches!(problem.missing_metric_policy, MissingMetricPolicy::Strict),
            additional_metrics: if required_metrics.is_empty() {
                None
            } else {
                Some(required_metrics.to_vec())
            },
            replace_standard_metrics: false,
        };

        Some(crate::valuation::value_portfolio_with_options(
            &candidate_portfolio,
            market,
            config,
            &options,
        )?)
    } else {
        None
    };

    for candidate in &problem.trade_universe.candidates {
        let val_entry = candidate_valuation
            .as_ref()
            .and_then(|v| v.position_values.get(&candidate.id))
            .ok_or_else(|| {
                PortfolioError::valuation(candidate.id.clone(), "failed to value candidate")
            })?;

        let pv_unit = val_entry.value_base.amount();
        // Candidates don't contribute to initial PV base but may have value

        let measures = val_entry
            .valuation_result
            .as_ref()
            .map(|r| r.measures.clone())
            .unwrap_or_default();

        items.push(DecisionItem {
            position_id: candidate.id.clone(),
            is_existing: false,
            is_held: false,
            current_quantity: 0.0,
        });

        features.push(DecisionFeatures {
            pv_base: 0.0, // Currently held value is 0
            pv_per_unit: pv_unit,
            measures,
            tags: candidate.tags.clone(),
            min_weight: candidate.min_weight,
            max_weight: candidate.max_weight, // Respect candidate constraints
        });
    }

    // Populate current weights based on weighting scheme.
    match problem.weighting {
        WeightingScheme::NotionalWeight => {
            // For NotionalWeight: use signed quantity / gross absolute quantity
            // This gives weights based on notional exposure, not PV
            if gross_notional.abs() > 1e-6 {
                for item in &items {
                    if item.is_existing {
                        let notional = position_notionals
                            .get(&item.position_id)
                            .copied()
                            .unwrap_or(0.0);
                        current_weights.insert(item.position_id.clone(), notional / gross_notional);
                    } else {
                        current_weights.insert(item.position_id.clone(), 0.0);
                    }
                }
            } else {
                for item in &items {
                    current_weights.insert(item.position_id.clone(), 0.0);
                }
            }
        }
        WeightingScheme::ValueWeight | WeightingScheme::UnitScaling => {
            // For ValueWeight/UnitScaling: use signed PV / gross market value
            // This handles hedged portfolios where net PV is ~0 but we still want meaningful weights.
            if gross_pv_base.abs() > 1e-6 {
                for (item, feat) in items.iter().zip(&features) {
                    if item.is_existing {
                        current_weights
                            .insert(item.position_id.clone(), feat.pv_base / gross_pv_base);
                    } else {
                        current_weights.insert(item.position_id.clone(), 0.0);
                    }
                }
            } else {
                for item in &items {
                    current_weights.insert(item.position_id.clone(), 0.0);
                }
            }
        }
    }

    Ok((items, features, current_weights, total_pv_base))
}
