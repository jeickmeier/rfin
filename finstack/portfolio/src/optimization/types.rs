use finstack_valuations::metrics::MetricId;
use serde::{Deserialize, Serialize};

/// How optimization weights are defined.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WeightingScheme {
    /// `w_i` is share of portfolio base currency PV; `∑ w_i = 1`.
    ValueWeight,

    /// `w_i` is share of some notional exposure; still normalized so `∑ w_i = 1`.
    NotionalWeight,

    /// `w_i` scales the current quantity (e.g. units or face value).
    /// For MVP we treat `w_i` as a fraction of current position size.
    UnitScaling,
}

/// Where a per‑position scalar metric comes from.
#[derive(Clone, Debug)]
pub enum PerPositionMetric {
    /// Directly from `ValuationResult::measures` using a standard `MetricId`.
    ///
    /// Examples:
    /// - `Metric(MetricId::DurationMod)` for modified duration
    /// - `Metric(MetricId::Ytm)` for yield to maturity
    /// - `Metric(MetricId::Dv01)` for DV01
    Metric(MetricId),

    /// From `ValuationResult::measures` using a string key (for custom or
    /// bucketed metrics stored by name).
    CustomKey(String),

    /// Use the base currency PV of the position (after scaling).
    PvBase,

    /// Use the native‑currency PV of the position (after scaling).
    PvNative,

    /// Tag‑based 0/1 indicator: 1.0 if tag matches, else 0.0.
    TagEquals {
        /// Tag key to match.
        key: String,
        /// Tag value to match.
        value: String,
    },

    /// Constant scalar for all positions.
    Constant(f64),
}

/// How to handle missing metrics for a position.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum MissingMetricPolicy {
    /// Treat missing as 0.0 (default, appropriate for duration‑like metrics).
    #[default]
    Zero,

    /// Exclude position from constraint evaluation (position keeps current weight).
    Exclude,

    /// Fail with error if any required metric is missing.
    Strict,
}

/// Portfolio‑level scalar metric expressed in terms of position metrics + weights.
#[derive(Clone, Debug)]
pub enum MetricExpr {
    /// `sum_i w_i * m_i`, where `m_i` comes from a `PerPositionMetric`.
    WeightedSum {
        /// Per‑position metric to aggregate.
        metric: PerPositionMetric,
    },

    /// Value‑weighted average: `sum_i w_i * m_i`, with implicit `sum_i w_i == 1`.
    /// This is appropriate for duration or yield when weights are `ValueWeight`.
    ValueWeightedAverage {
        /// Per‑position metric to average.
        metric: PerPositionMetric,
    },

    /// Exposure share for a tag bucket: `sum_i w_i * I[tag == value]`.
    /// Assumes weights are already normalized (e.g. `ValueWeight`).
    TagExposureShare {
        /// Tag key to match.
        tag_key: String,
        /// Tag value to match.
        tag_value: String,
    },
}

/// Optimization direction and target.
#[derive(Clone, Debug)]
pub enum Objective {
    /// Maximize a portfolio‑level metric.
    Maximize(MetricExpr),
    /// Minimize a portfolio‑level metric.
    Minimize(MetricExpr),
}
