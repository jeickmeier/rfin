use crate::types::AttributeTest;
use finstack_valuations::metrics::MetricId;
use serde::{Deserialize, Serialize};

/// How optimization weights are defined.
///
/// # Conventions
///
/// `ValueWeight` and `NotionalWeight` are normalized portfolio shares, whereas
/// `UnitScaling` is a direct multiplier on current quantity for existing
/// positions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WeightingScheme {
    /// `w_i` is share of portfolio base currency PV; `∑ w_i = 1`.
    ValueWeight,

    /// `w_i` is share of some notional exposure; still normalized so `∑ w_i = 1`.
    NotionalWeight,

    /// `w_i` scales the current quantity (e.g. units or face value).
    ///
    /// Unlike `ValueWeight` and `NotionalWeight`, this is not a PV share.
    /// For existing positions, `w_i = 1.0` means "keep the current quantity",
    /// `w_i = 0.5` means "halve it", and `w_i = 0.0` means "close it".
    /// For new candidates, `w_i` is interpreted directly as the target quantity
    /// because there is no live quantity to scale.
    UnitScaling,
}

/// Where a per‑position scalar metric comes from.
#[derive(Clone, Debug, Serialize, Deserialize)]
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
    ///
    /// **Two behaviors** (historical naming: "native" refers to the stored native PV field):
    ///
    /// - When lowering a scalar through the generic per-metric path, this resolves to the
    ///   **native-currency** PV.
    /// - When this variant appears in [`MetricExpr::WeightedSum`] or
    ///   [`MetricExpr::ValueWeightedAverage`], the linear coefficient builder uses
    ///   **base-currency PV** (FX-converted to the portfolio base) so mixed-currency
    ///   positions remain comparable inside the linear program.
    ///
    /// Use [`PerPositionMetric::PvBase`] when you need base-currency PV in every context.
    PvNative,

    /// Numeric attribute value from position attributes.
    Attribute(String),

    /// 1.0 if the attribute test passes, 0.0 otherwise.
    AttributeIndicator(AttributeTest),

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
///
/// These expressions are intentionally restricted to linear or linearized forms
/// so they can be represented by the LP-based optimizer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MetricExpr {
    /// `sum_i w_i * m_i`, where `m_i` comes from a `PerPositionMetric`.
    WeightedSum {
        /// Per‑position metric to aggregate.
        metric: PerPositionMetric,
        /// Optional filter restricting which positions contribute.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        filter: Option<super::universe::PositionFilter>,
    },

    /// Value‑weighted average: `sum_i w_i * m_i`, with implicit `sum_i w_i == 1`.
    /// This is appropriate for duration or yield when weights are `ValueWeight`.
    ValueWeightedAverage {
        /// Per‑position metric to average.
        metric: PerPositionMetric,
        /// Optional filter restricting which positions contribute.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        filter: Option<super::universe::PositionFilter>,
    },
}

/// Optimization direction and target.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Objective {
    /// Maximize a portfolio‑level metric.
    Maximize(MetricExpr),
    /// Minimize a portfolio‑level metric.
    Minimize(MetricExpr),
}
