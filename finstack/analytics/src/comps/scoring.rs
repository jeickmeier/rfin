//! Composite rich/cheap scoring across multiple valuation dimensions.
//!
//! Combines percentile rank, z-score, and regression residual signals
//! into a weighted composite relative value score.

use super::multiples::compute_multiple;
use super::peer_set::PeerSet;
use super::stats::{percentile_rank, regression_fair_value, z_score};
use super::types::{CompanyId, CompanyMetrics, Multiple};
use crate::Result;
use finstack_core::Error;
use serde::{Deserialize, Serialize};

/// Configuration for a single rich/cheap scoring dimension.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringDimension {
    /// Human-readable label (e.g., "Spread vs Leverage").
    pub label: String,
    /// How to extract the Y variable (dependent) from CompanyMetrics.
    pub y_extractor: MetricExtractor,
    /// How to extract the X variable(s) (explanatory) from CompanyMetrics.
    pub x_extractors: Vec<MetricExtractor>,
    /// Weight of this dimension in the composite score (0.0 to 1.0).
    pub weight: f64,
}

/// Identifies which metric to extract from CompanyMetrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricExtractor {
    /// A named field (e.g., "leverage", "oas_bps", "ebitda_margin").
    Named(String),
    /// A valuation multiple.
    Multiple(Multiple),
    /// A custom metric key from the `custom` map.
    Custom(String),
}

/// Decomposed score for a single dimension.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionScore {
    /// Label of the dimension.
    pub label: String,
    /// Percentile rank within peers (0-1, higher = richer).
    pub percentile: f64,
    /// Z-score relative to peer distribution.
    pub z_score: f64,
    /// Regression residual (positive = cheap).
    pub regression_residual: Option<f64>,
    /// R-squared of the regression (confidence measure).
    pub r_squared: Option<f64>,
    /// Dimension weight in composite.
    pub weight: f64,
}

/// Composite relative value result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelativeValueResult {
    /// Company being scored.
    pub company_id: CompanyId,
    /// Composite rich/cheap score.
    ///
    /// Positive = cheap (trading below fair value across dimensions).
    /// Negative = rich (trading above fair value).
    /// Magnitude indicates conviction (bounded by number of dimensions
    /// and their weights).
    pub composite_score: f64,
    /// Per-dimension decomposition.
    pub dimensions: Vec<DimensionScore>,
    /// Confidence in the composite score (average R-squared across
    /// regression-based dimensions, weighted by dimension weight).
    pub confidence: f64,
    /// Number of peers used in the analysis.
    pub peer_count: usize,
}

/// Score a subject company against its peer set across multiple dimensions.
///
/// For each `ScoringDimension`:
/// 1. Extract Y and X metrics from peers and subject.
/// 2. Compute percentile rank of subject's Y among peers' Y values.
/// 3. Compute z-score of subject's Y in the peer distribution.
/// 4. If X extractors are provided, run regression(s) and compute residual.
/// 5. Combine into a dimension score.
///
/// The composite score is the weighted average of per-dimension residual
/// z-scores (for regression-based dimensions) or raw z-scores (for
/// univariate dimensions), with sign convention: positive = cheap.
pub fn score_relative_value(
    peer_set: &PeerSet,
    dimensions: &[ScoringDimension],
) -> Result<RelativeValueResult> {
    if dimensions.is_empty() {
        return Err(Error::Validation(
            "at least one scoring dimension is required".into(),
        ));
    }

    let mut dim_scores = Vec::with_capacity(dimensions.len());
    let mut weighted_sum = 0.0;
    let mut total_weight = 0.0;
    let mut confidence_num = 0.0;
    let mut confidence_den = 0.0;

    for dim in dimensions {
        // Extract Y values from peers and subject
        let peer_y = extract_values(peer_set, &dim.y_extractor);
        let subject_y = extract_subject_value(peer_set, &dim.y_extractor);
        let (peer_vals, subject_val) = match (peer_y.as_slice(), subject_y) {
            (vals, Some(sv)) if !vals.is_empty() => (vals, sv),
            _ => continue, // Skip dimension if data is insufficient
        };

        let pctile = percentile_rank(peer_vals, subject_val).unwrap_or(0.5);
        let zs = z_score(peer_vals, subject_val).unwrap_or(0.0);

        let (reg_residual, r_sq) = if !dim.x_extractors.is_empty() {
            // Run single-factor regression using the first X extractor
            let peer_x = extract_values(peer_set, &dim.x_extractors[0]);
            let subject_x = extract_subject_value(peer_set, &dim.x_extractors[0]);
            match (peer_x.as_slice(), subject_x) {
                (xv, Some(sx)) if xv.len() >= 3 => {
                    let reg = regression_fair_value(xv, peer_vals, sx, subject_val);
                    (
                        reg.as_ref().map(|r| r.residual),
                        reg.as_ref().map(|r| r.r_squared),
                    )
                }
                _ => (None, None),
            }
        } else {
            (None, None)
        };

        // Sign convention: negative residual or negative z-score means trading rich
        // We negate so positive composite = cheap
        let score = reg_residual.map(|r| -r).unwrap_or(-zs);
        weighted_sum += dim.weight * score;
        total_weight += dim.weight;
        if let Some(rsq) = r_sq {
            confidence_num += dim.weight * rsq;
            confidence_den += dim.weight;
        }

        dim_scores.push(DimensionScore {
            label: dim.label.clone(),
            percentile: pctile,
            z_score: zs,
            regression_residual: reg_residual,
            r_squared: r_sq,
            weight: dim.weight,
        });
    }

    let composite = if total_weight > 0.0 {
        weighted_sum / total_weight
    } else {
        0.0
    };
    let confidence = if confidence_den > 0.0 {
        confidence_num / confidence_den
    } else {
        0.0
    };

    Ok(RelativeValueResult {
        company_id: peer_set.subject.id.clone(),
        composite_score: composite,
        dimensions: dim_scores,
        confidence,
        peer_count: peer_set.peer_count(),
    })
}

/// Extract metric values from all peers in the set.
fn extract_values(peer_set: &PeerSet, extractor: &MetricExtractor) -> Vec<f64> {
    peer_set
        .peers
        .iter()
        .filter_map(|c| extract_single(c, extractor))
        .collect()
}

/// Extract the subject's metric value.
fn extract_subject_value(peer_set: &PeerSet, extractor: &MetricExtractor) -> Option<f64> {
    extract_single(&peer_set.subject, extractor)
}

/// Extract a single metric value from a `CompanyMetrics`.
fn extract_single(metrics: &CompanyMetrics, extractor: &MetricExtractor) -> Option<f64> {
    match extractor {
        MetricExtractor::Named(name) => match name.as_str() {
            "enterprise_value" => metrics.enterprise_value,
            "market_cap" => metrics.market_cap,
            "share_price" => metrics.share_price,
            "oas_bps" => metrics.oas_bps,
            "yield_pct" => metrics.yield_pct,
            "ebitda" => metrics.ebitda,
            "revenue" => metrics.revenue,
            "ebit" => metrics.ebit,
            "ufcf" => metrics.ufcf,
            "lfcf" => metrics.lfcf,
            "net_income" => metrics.net_income,
            "book_value" => metrics.book_value,
            "tangible_book_value" => metrics.tangible_book_value,
            "dividends_per_share" => metrics.dividends_per_share,
            "leverage" => metrics.leverage,
            "interest_coverage" => metrics.interest_coverage,
            "revenue_growth" => metrics.revenue_growth,
            "ebitda_margin" => metrics.ebitda_margin,
            _ => None,
        },
        MetricExtractor::Multiple(multiple) => compute_multiple(metrics, *multiple),
        MetricExtractor::Custom(key) => metrics.custom.get(key).copied(),
    }
}
