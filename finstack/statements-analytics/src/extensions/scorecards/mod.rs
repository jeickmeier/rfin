//! Credit scorecard analysis extension.
//!
//! This extension provides credit rating assignment based on financial metrics
//! and configurable thresholds.
//!
//! # Migration (v0.5)
//!
//! The `Extension` trait implementation in this module is deprecated;
//! prefer calling [`CreditScorecardExtension`] directly.
//!
//! # Features
//!
//! - ✅ Credit rating assignment based on financial metrics
//! - ✅ Configurable rating scales and thresholds
//! - ✅ Weighted scoring across multiple metrics
//! - ✅ Support for multiple rating agencies (S&P, Moody's, Fitch)
//! - ✅ Minimum rating compliance checks
//! - ✅ Detailed metric evaluation with scores and weights
//!
//! # Configuration Schema
//!
//! ```json
//! {
//!   "rating_scale": "S&P",
//!   "metrics": [
//!     {
//!       "name": "debt_to_ebitda",
//!       "formula": "total_debt / ttm(ebitda)",
//!       "weight": 0.3,
//!       "thresholds": {
//!         "AAA": [0.0, 1.0],
//!         "AA": [1.0, 2.0],
//!         "A": [2.0, 3.0],
//!         "BBB": [3.0, 4.0],
//!         "BB": [4.0, 5.0],
//!         "B": [5.0, 6.0],
//!         "CCC": [6.0, 999.0]
//!       }
//!     },
//!     {
//!       "name": "interest_coverage",
//!       "formula": "ebitda / interest_expense",
//!       "weight": 0.25,
//!       "thresholds": {
//!         "AAA": [8.0, 999.0],
//!         "AA": [6.0, 8.0],
//!         "A": [4.5, 6.0],
//!         "BBB": [3.0, 4.5],
//!         "BB": [2.0, 3.0],
//!         "B": [1.0, 2.0],
//!         "CCC": [0.0, 1.0]
//!       }
//!     }
//!   ]
//! }
//! ```
//!
//! # Example Usage
//!
//! ```rust,no_run
//! use finstack_statements_analytics::extensions::CreditScorecardExtension;
//! use finstack_statements::extensions::{ExtensionRegistry, ExtensionContext};
//!
//! # fn main() -> finstack_statements::Result<()> {
//! let config = serde_json::json!({
//!   "rating_scale": "S&P",
//!   "metrics": [{
//!     "name": "debt_to_ebitda",
//!     "formula": "total_debt / ttm(ebitda)"
//!   }]
//! });
//! let mut registry = ExtensionRegistry::new();
//! registry.register(Box::new(CreditScorecardExtension::new()))?;
//!
//! # let context: ExtensionContext = unimplemented!("build ExtensionContext from a model and StatementResult");
//! let results = registry.execute("credit_scorecard", &context.with_config(&config))?;
//! # let _ = results;
//! # Ok(())
//! # }
//! ```

#![allow(deprecated)]

use finstack_statements::extensions::{
    Extension, ExtensionContext, ExtensionMetadata, ExtensionResult,
};
use finstack_statements::Result;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// Rating level for credit rating scales.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RatingLevel {
    /// Rating name (e.g., "AAA", "Aaa")
    pub name: String,
    /// Numeric score (0-100 scale)
    pub score: f64,
    /// Minimum score threshold for this rating
    pub min_score: f64,
}

/// Rating scale definition (for JSON deserialization).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RatingScale {
    /// Scale name (e.g., "S&P", "Moody's")
    pub scale_name: String,
    /// Human-readable description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Ordered list of rating levels (best to worst)
    pub ratings: Vec<RatingLevel>,
}

/// Lazy-loaded S&P rating scale.
static SP_SCALE: OnceLock<RatingScale> = OnceLock::new();

/// Lazy-loaded Moody's rating scale.
static MOODYS_SCALE: OnceLock<RatingScale> = OnceLock::new();

/// Get the S&P rating scale (loads from embedded JSON on first access).
#[allow(clippy::expect_used)] // Embedded JSON validated at compile time; parse failure is a build bug
fn get_sp_scale() -> &'static RatingScale {
    SP_SCALE.get_or_init(|| {
        let json = include_str!("../../../data/rating_scales/sp.json");
        serde_json::from_str(json).expect("Failed to parse embedded S&P rating scale")
    })
}

/// Get the Moody's rating scale (loads from embedded JSON on first access).
#[allow(clippy::expect_used)] // Embedded JSON validated at compile time; parse failure is a build bug
fn get_moodys_scale() -> &'static RatingScale {
    MOODYS_SCALE.get_or_init(|| {
        let json = include_str!("../../../data/rating_scales/moodys.json");
        serde_json::from_str(json).expect("Failed to parse embedded Moody's rating scale")
    })
}

/// Default score when no threshold matches.
///
/// A mid-scale fallback keeps scorecard execution usable for partially specified
/// threshold grids, but the extension emits a warning so callers can fix the
/// configuration rather than silently relying on this value.
const DEFAULT_SCORECARD_SCORE: f64 = 50.0;

/// Get the appropriate rating scale based on name.
fn get_rating_scale(scale_name: &str) -> &'static RatingScale {
    match scale_name {
        "Moody's" | "MOODYS" | "Moodys" => get_moodys_scale(),
        "Fitch" | "FITCH" => get_sp_scale(), // Fitch uses same notation as S&P
        "S&P" | "S&P Global" | "SP" | "sp" | "s&p" => get_sp_scale(),
        _ => get_sp_scale(),
    }
}

fn is_supported_rating_scale(scale_name: &str) -> bool {
    matches!(
        scale_name,
        "S&P"
            | "S&P Global"
            | "SP"
            | "sp"
            | "s&p"
            | "Moody's"
            | "MOODYS"
            | "Moodys"
            | "Fitch"
            | "FITCH"
    )
}

/// Credit scorecard analysis extension for rating and stress testing.
///
/// **Features:**
/// - Credit rating assignment using weighted metric scores
/// - Support for multiple rating scales (S&P, Moody's, Fitch)
/// - Configurable thresholds per rating level
/// - Minimum rating compliance checks
pub struct CreditScorecardExtension {
    /// Extension configuration
    config: Option<ScorecardConfig>,
}

/// Configuration for credit scorecard analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScorecardConfig {
    /// Rating scale to use (e.g., "S&P", "Moody's", "Fitch")
    #[serde(default = "default_rating_scale")]
    pub rating_scale: String,

    /// List of metrics to evaluate
    #[serde(default)]
    pub metrics: Vec<ScorecardMetric>,

    /// Minimum acceptable rating (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_rating: Option<String>,
}

/// Definition of a scorecard metric.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScorecardMetric {
    /// Metric name
    pub name: String,

    /// Formula to calculate the metric (DSL syntax)
    pub formula: String,

    /// Weight in overall score (0.0 to 1.0)
    #[serde(default = "default_weight")]
    pub weight: f64,

    /// Rating thresholds: rating → [min, max]
    #[serde(default)]
    pub thresholds: indexmap::IndexMap<String, (f64, f64)>,

    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

fn default_rating_scale() -> String {
    "S&P".into()
}

fn default_weight() -> f64 {
    1.0
}

impl CreditScorecardExtension {
    /// Create a new credit scorecard extension with default configuration.
    ///
    /// # Example
    /// ```rust
    /// # use finstack_statements_analytics::extensions::CreditScorecardExtension;
    /// let extension = CreditScorecardExtension::new();
    /// assert!(extension.config().is_none());
    /// ```
    pub fn new() -> Self {
        Self { config: None }
    }

    /// Create a new credit scorecard extension with the given configuration.
    ///
    /// # Arguments
    /// * `config` - Pre-built [`ScorecardConfig`] to use
    pub fn with_config(config: ScorecardConfig) -> Self {
        Self {
            config: Some(config),
        }
    }

    /// Get the current configuration.
    pub fn config(&self) -> Option<&ScorecardConfig> {
        self.config.as_ref()
    }

    /// Set the configuration.
    ///
    /// # Arguments
    /// * `config` - New configuration to assign
    pub fn set_config(&mut self, config: ScorecardConfig) {
        self.config = Some(config);
    }

    fn resolve_config<'a>(&'a self, context: &'a ExtensionContext) -> Result<ScorecardConfig> {
        if let Some(config) = context.config {
            serde_json::from_value(config.clone()).map_err(|e| {
                finstack_statements::error::Error::invalid_input(format!(
                    "Invalid scorecard configuration: {}",
                    e
                ))
            })
        } else {
            self.config.clone().ok_or_else(|| {
                finstack_statements::error::Error::registry(
                    "Credit scorecard extension requires configuration",
                )
            })
        }
    }

    /// Evaluate a single metric.
    fn evaluate_metric(
        &self,
        metric: &ScorecardMetric,
        context: &ExtensionContext,
        config: &ScorecardConfig,
    ) -> Result<MetricEvaluation> {
        // Parse and evaluate the formula
        let expr = finstack_statements::dsl::parse_and_compile(&metric.formula)?;

        // Create evaluation context for the last period (or average across all)
        let last_period =
            context.model.periods.last().ok_or_else(|| {
                finstack_statements::error::Error::registry("No periods in model")
            })?;

        let node_to_column: indexmap::IndexMap<finstack_statements::types::NodeId, usize> = context
            .model
            .nodes
            .keys()
            .enumerate()
            .map(|(i, k)| (k.clone(), i))
            .collect();

        let mut historical_results = indexmap::IndexMap::new();
        for period in &context.model.periods {
            if period.id == last_period.id {
                continue;
            }
            let mut period_values = indexmap::IndexMap::new();
            for (node_id, node_periods) in &context.results.nodes {
                if let Some(value) = node_periods.get(&period.id) {
                    period_values.insert(node_id.clone(), *value);
                }
            }
            if !period_values.is_empty() {
                historical_results.insert(period.id, period_values);
            }
        }

        let mut eval_context = finstack_statements::evaluator::EvaluationContext::new(
            last_period.id,
            std::sync::Arc::new(node_to_column),
            std::sync::Arc::new(historical_results),
        );

        if let Some(ref cs) = context.results.cs_cashflows {
            eval_context.capital_structure_cashflows = Some(cs.clone());
        }

        for (node_id, node_values) in &context.results.nodes {
            if let Some(value) = node_values.get(&last_period.id) {
                if eval_context.node_to_column.contains_key(node_id.as_str()) {
                    eval_context.set_value(node_id, *value)?;
                }
            }
        }

        // Evaluate the formula
        let value = finstack_statements::evaluator::formula::evaluate_formula(
            &expr,
            &mut eval_context,
            Some(metric.name.as_str()),
        )?;

        // Calculate score based on thresholds
        let score = self.calculate_metric_score(value, &metric.thresholds, &config.rating_scale);
        let warning = if self
            .matching_threshold_score(value, &metric.thresholds, &config.rating_scale)
            .is_none()
        {
            Some(format!(
                "Credit scorecard metric '{}' thresholds did not match value {} for {}; using fallback score {}",
                metric.name, value, config.rating_scale, DEFAULT_SCORECARD_SCORE
            ))
        } else {
            None
        };

        Ok(MetricEvaluation {
            score: MetricScore {
                metric_name: metric.name.clone(),
                value,
                score,
                weight: metric.weight,
            },
            warning,
        })
    }

    fn matching_threshold_score(
        &self,
        value: f64,
        thresholds: &indexmap::IndexMap<String, (f64, f64)>,
        rating_scale: &str,
    ) -> Option<f64> {
        let scale = get_rating_scale(rating_scale);

        scale.ratings.iter().find_map(|level| {
            thresholds.get(&level.name).and_then(|(min, max)| {
                if value >= *min && value <= *max {
                    Some(level.score)
                } else {
                    None
                }
            })
        })
    }

    /// Calculate score based on thresholds.
    ///
    /// Uses the configured rating scale (S&P by default) to map metric values
    /// to numeric scores based on user-provided thresholds.
    fn calculate_metric_score(
        &self,
        value: f64,
        thresholds: &indexmap::IndexMap<String, (f64, f64)>,
        rating_scale: &str,
    ) -> f64 {
        if let Some(score) = self.matching_threshold_score(value, thresholds, rating_scale) {
            return score;
        }

        // Default score if no threshold matches
        tracing::warn!(
            rating_scale,
            value,
            default_score = DEFAULT_SCORECARD_SCORE,
            "credit scorecard thresholds did not match metric value; using fallback score"
        );
        DEFAULT_SCORECARD_SCORE
    }

    /// Calculate weighted average score.
    fn calculate_weighted_score(&self, scores: &[MetricScore]) -> f64 {
        if scores.is_empty() {
            return 0.0;
        }

        let total_weight: f64 = scores.iter().map(|s| s.weight).sum();
        if total_weight.abs() < f64::EPSILON {
            return 0.0;
        }

        scores.iter().map(|s| s.score * s.weight).sum::<f64>() / total_weight
    }

    /// Determine rating based on total score.
    ///
    /// Uses the configured rating scale to map a numeric score to a credit rating.
    /// Supports S&P, Moody's, and Fitch scales.
    fn determine_rating(&self, score: f64, rating_scale: &str) -> String {
        let scale = get_rating_scale(rating_scale);

        // Find the rating by checking score thresholds
        for level in &scale.ratings {
            if score >= level.min_score {
                return level.name.clone();
            }
        }

        // Fallback to lowest rating
        scale
            .ratings
            .last()
            .map(|l| l.name.clone())
            .unwrap_or_else(|| "D".to_string())
    }

    /// Check if rating meets minimum requirement.
    ///
    /// Compares ratings using the configured rating scale with exact matching.
    /// Returns true if the rating is equal to or better than the minimum.
    fn meets_minimum_rating(&self, rating: &str, min_rating: &str, rating_scale: &str) -> bool {
        let scale = get_rating_scale(rating_scale);

        // Find positions in the rating scale (lower index = better rating).
        // Use exact string matching to avoid false matches (e.g., "AA" matching "A").
        let rating_pos = scale.ratings.iter().position(|l| l.name == rating);
        let min_pos = scale.ratings.iter().position(|l| l.name == min_rating);

        match (rating_pos, min_pos) {
            (Some(r), Some(m)) => r <= m, // Lower index = better rating
            _ => false,
        }
    }
}

/// Score for a single metric.
struct MetricScore {
    metric_name: String,
    value: f64,
    score: f64,
    weight: f64,
}

struct MetricEvaluation {
    score: MetricScore,
    warning: Option<String>,
}

impl Default for CreditScorecardExtension {
    fn default() -> Self {
        Self::new()
    }
}

impl Extension for CreditScorecardExtension {
    fn metadata(&self) -> ExtensionMetadata {
        ExtensionMetadata {
            name: "credit_scorecard".into(),
            version: "0.1.0".into(),
            description: Some("Credit rating and stress testing based on financial metrics".into()),
            author: Some("Finstack Team".into()),
        }
    }

    fn execute(&mut self, context: &ExtensionContext) -> Result<ExtensionResult> {
        // Credit scorecard analysis implementation
        let config = self.resolve_config(context)?;

        let mut scores = Vec::new();
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Evaluate each metric
        for metric_config in &config.metrics {
            match self.evaluate_metric(metric_config, context, &config) {
                Ok(evaluation) => {
                    if let Some(warning) = evaluation.warning {
                        warnings.push(warning);
                    }
                    scores.push(evaluation.score);
                }
                Err(e) => errors.push(format!("Metric '{}': {}", metric_config.name, e)),
            }
        }

        // Calculate weighted average score
        let total_score = self.calculate_weighted_score(&scores);

        // Determine rating based on scale
        let rating = self.determine_rating(total_score, &config.rating_scale);

        // Check minimum rating requirement
        if let Some(min_rating) = &config.min_rating {
            if !self.meets_minimum_rating(&rating, min_rating, &config.rating_scale) {
                warnings.push(format!(
                    "Credit rating {} is below minimum required {}",
                    rating, min_rating
                ));
            }
        }

        // Build result
        let mut result = if errors.is_empty() {
            ExtensionResult::success(format!(
                "Credit scorecard complete. Rating: {} (Score: {:.2})",
                rating, total_score
            ))
        } else {
            ExtensionResult::failure(format!(
                "Credit scorecard failed with {} errors",
                errors.len()
            ))
        };

        // Add scorecard data
        result = result
            .with_data("rating", serde_json::json!(rating))
            .with_data("total_score", serde_json::json!(total_score))
            .with_data(
                "metric_scores",
                serde_json::json!(scores
                    .iter()
                    .map(|s| {
                        serde_json::json!({
                            "metric": s.metric_name,
                            "value": s.value,
                            "score": s.score,
                            "weight": s.weight,
                            "weighted_score": s.score * s.weight,
                        })
                    })
                    .collect::<Vec<_>>()),
            )
            .with_data("rating_scale", serde_json::json!(config.rating_scale));

        // Add warnings and errors
        for warning in warnings {
            result = result.with_warning(warning);
        }
        for error in errors {
            result = result.with_error(error);
        }

        Ok(result)
    }

    fn is_enabled(&self) -> bool {
        // Extension is always available but returns NotImplemented
        true
    }

    fn config_schema(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "title": "ScorecardConfig",
            "type": "object",
            "properties": {
                "rating_scale": {
                    "type": "string",
                    "default": "S&P",
                    "description": "Rating scale to use (e.g., 'S&P', 'Moody's', 'Fitch')"
                },
                "metrics": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "required": ["name", "formula"],
                        "properties": {
                            "name": {
                                "type": "string",
                                "description": "Metric name"
                            },
                            "formula": {
                                "type": "string",
                                "description": "Formula to calculate the metric (DSL syntax)"
                            },
                            "weight": {
                                "type": "number",
                                "default": 1.0,
                                "description": "Weight in overall score (0.0 to 1.0)"
                            },
                            "thresholds": {
                                "type": "object",
                                "description": "Rating thresholds: rating → [min, max]"
                            },
                            "description": {
                                "type": "string",
                                "description": "Metric description"
                            }
                        }
                    }
                },
                "min_rating": {
                    "type": "string",
                    "description": "Minimum acceptable rating (optional)"
                }
            }
        }))
    }

    fn validate_config(&self, config: &serde_json::Value) -> Result<()> {
        // Validate configuration structure
        let scorecard_config: ScorecardConfig =
            serde_json::from_value(config.clone()).map_err(|e| {
                finstack_statements::error::Error::invalid_input(format!(
                    "Invalid scorecard configuration: {}",
                    e
                ))
            })?;

        if !is_supported_rating_scale(&scorecard_config.rating_scale) {
            return Err(finstack_statements::error::Error::invalid_input(format!(
                "Unsupported rating_scale '{}'. Expected one of: S&P, Moody's, Fitch",
                scorecard_config.rating_scale
            )));
        }

        // Validate metric weights sum to reasonable values
        let total_weight: f64 = scorecard_config.metrics.iter().map(|m| m.weight).sum();
        if total_weight > 0.0 && !(0.01..=100.0).contains(&total_weight) {
            return Err(finstack_statements::error::Error::invalid_input(format!(
                "Total metric weights ({}) should be between 0.01 and 100.0",
                total_weight
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn calculate_metric_score_falls_back_when_thresholds_miss() {
        let extension = CreditScorecardExtension::new();
        let mut thresholds = indexmap::IndexMap::new();
        thresholds.insert("AAA".to_string(), (0.0, 1.0));

        let score = extension.calculate_metric_score(2.5, &thresholds, "S&P");

        assert_eq!(score, DEFAULT_SCORECARD_SCORE);
    }

    #[test]
    fn calculate_weighted_score_treats_sub_epsilon_weights_as_zero() {
        let extension = CreditScorecardExtension::new();
        let scores = vec![MetricScore {
            metric_name: "leverage".to_string(),
            value: 2.5,
            score: 80.0,
            weight: f64::EPSILON / 4.0,
        }];

        let weighted = extension.calculate_weighted_score(&scores);

        assert_eq!(weighted, 0.0);
    }
}
