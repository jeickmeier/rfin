use super::types::MetricExpr;
use super::universe::PositionFilter;

/// Inequality/equality operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Inequality {
    /// Less‑than or equal: `lhs <= rhs`.
    Le,
    /// Greater‑than or equal: `lhs >= rhs`.
    Ge,
    /// Equality: `lhs == rhs`.
    Eq,
}

/// Declarative constraint specification.
#[derive(Clone, Debug)]
pub enum Constraint {
    /// General metric bound, e.g. duration `<= 4.0`.
    MetricBound {
        /// Human‑readable label for debugging/diagnostics.
        label: Option<String>,
        /// Metric expression on the left‑hand side.
        metric: MetricExpr,
        /// Operator (<=, >=, ==).
        op: Inequality,
        /// Right‑hand side constant.
        rhs: f64,
    },

    /// Tag exposure limit, e.g. rating=CCC weight `<= 0.10`.
    TagExposureLimit {
        /// Human‑readable label for debugging/diagnostics.
        label: Option<String>,
        /// Tag key to match (e.g., "rating").
        tag_key: String,
        /// Tag value to match (e.g., "CCC").
        tag_value: String,
        /// Maximum share in `[0, 1]`.
        max_share: f64,
    },

    /// Minimum tag exposure, e.g. rating=IG weight `>= 0.50`.
    TagExposureMinimum {
        /// Human‑readable label for debugging/diagnostics.
        label: Option<String>,
        /// Tag key to match (e.g., "rating").
        tag_key: String,
        /// Tag value to match (e.g., "IG").
        tag_value: String,
        /// Minimum share in `[0, 1]`.
        min_share: f64,
    },

    /// Weight bounds for all positions matching the filter.
    WeightBounds {
        /// Human‑readable label for debugging/diagnostics.
        label: Option<String>,
        /// Filter to select positions for this constraint.
        filter: PositionFilter,
        /// Inclusive minimum weight.
        min: f64,
        /// Inclusive maximum weight.
        max: f64,
    },

    /// Maximum turnover constraint: `Σ |w_new - w_current| <= max_turnover`.
    MaxTurnover {
        /// Human‑readable label for debugging/diagnostics.
        label: Option<String>,
        /// Maximum allowed turnover (sum of absolute weight changes).
        max_turnover: f64,
    },

    /// Maximum single position weight change: `|w_new - w_current| <= max_delta`.
    MaxPositionDelta {
        /// Human‑readable label for debugging/diagnostics.
        label: Option<String>,
        /// Filter to select positions for this constraint.
        filter: PositionFilter,
        /// Maximum allowed absolute weight change per position.
        max_delta: f64,
    },

    /// Budget/normalization constraint: usually `∑ w_i == 1.0`.
    Budget {
        /// Right‑hand side constant (typically 1.0 for normalization).
        rhs: f64,
    },
}

impl Constraint {
    /// Get the constraint label (for diagnostics).
    #[must_use]
    pub fn label(&self) -> Option<&str> {
        match self {
            Self::MetricBound { label, .. } => label.as_deref(),
            Self::TagExposureLimit { label, .. } => label.as_deref(),
            Self::TagExposureMinimum { label, .. } => label.as_deref(),
            Self::WeightBounds { label, .. } => label.as_deref(),
            Self::MaxTurnover { label, .. } => label.as_deref(),
            Self::MaxPositionDelta { label, .. } => label.as_deref(),
            Self::Budget { .. } => Some("budget"),
        }
    }
}
