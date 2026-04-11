use super::types::MetricExpr;
use super::universe::PositionFilter;
use serde::{Deserialize, Serialize};

/// Inequality/equality operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Inequality {
    /// Less‑than or equal: `lhs <= rhs`.
    Le,
    /// Greater‑than or equal: `lhs >= rhs`.
    Ge,
    /// Equality: `lhs == rhs`.
    Eq,
}

/// Declarative constraint specification.
#[derive(Clone, Debug, Serialize, Deserialize)]
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

/// Error returned when constraint parameters are invalid.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstraintValidationError {
    /// Description of the validation failure.
    pub message: String,
}

impl std::fmt::Display for ConstraintValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Constraint validation error: {}", self.message)
    }
}

impl std::error::Error for ConstraintValidationError {}

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

    /// Create a tag exposure limit constraint with validation.
    ///
    /// # Arguments
    ///
    /// * `tag_key` - Tag key to match (e.g., "rating")
    /// * `tag_value` - Tag value to match (e.g., "CCC")
    /// * `max_share` - Maximum share in `[0, 1]`
    ///
    /// # Errors
    ///
    /// Returns [`ConstraintValidationError`] if `max_share` is not in `[0, 1]`.
    pub fn tag_exposure_limit(
        tag_key: impl Into<String>,
        tag_value: impl Into<String>,
        max_share: f64,
    ) -> Result<Self, ConstraintValidationError> {
        Self::tag_exposure_limit_with_label(None, tag_key, tag_value, max_share)
    }

    /// Create a tag exposure limit constraint with a label and validation.
    ///
    /// # Errors
    ///
    /// Returns [`ConstraintValidationError`] if `max_share` is not in `[0, 1]`.
    pub fn tag_exposure_limit_with_label(
        label: Option<String>,
        tag_key: impl Into<String>,
        tag_value: impl Into<String>,
        max_share: f64,
    ) -> Result<Self, ConstraintValidationError> {
        if !(0.0..=1.0).contains(&max_share) {
            return Err(ConstraintValidationError {
                message: format!("max_share must be in [0, 1], got {}", max_share),
            });
        }

        Ok(Self::TagExposureLimit {
            label,
            tag_key: tag_key.into(),
            tag_value: tag_value.into(),
            max_share,
        })
    }

    /// Create a tag exposure minimum constraint with validation.
    ///
    /// # Arguments
    ///
    /// * `tag_key` - Tag key to match (e.g., "rating")
    /// * `tag_value` - Tag value to match (e.g., "IG")
    /// * `min_share` - Minimum share in `[0, 1]`
    ///
    /// # Errors
    ///
    /// Returns [`ConstraintValidationError`] if `min_share` is not in `[0, 1]`.
    pub fn tag_exposure_minimum(
        tag_key: impl Into<String>,
        tag_value: impl Into<String>,
        min_share: f64,
    ) -> Result<Self, ConstraintValidationError> {
        Self::tag_exposure_minimum_with_label(None, tag_key, tag_value, min_share)
    }

    /// Create a tag exposure minimum constraint with a label and validation.
    ///
    /// # Errors
    ///
    /// Returns [`ConstraintValidationError`] if `min_share` is not in `[0, 1]`.
    pub fn tag_exposure_minimum_with_label(
        label: Option<String>,
        tag_key: impl Into<String>,
        tag_value: impl Into<String>,
        min_share: f64,
    ) -> Result<Self, ConstraintValidationError> {
        if !(0.0..=1.0).contains(&min_share) {
            return Err(ConstraintValidationError {
                message: format!("min_share must be in [0, 1], got {}", min_share),
            });
        }

        Ok(Self::TagExposureMinimum {
            label,
            tag_key: tag_key.into(),
            tag_value: tag_value.into(),
            min_share,
        })
    }

    /// Create a weight bounds constraint with validation.
    ///
    /// # Arguments
    ///
    /// * `filter` - Filter to select positions
    /// * `min` - Inclusive minimum weight
    /// * `max` - Inclusive maximum weight
    ///
    /// # Errors
    ///
    /// Returns [`ConstraintValidationError`] if `min > max`.
    pub fn weight_bounds(
        filter: PositionFilter,
        min: f64,
        max: f64,
    ) -> Result<Self, ConstraintValidationError> {
        Self::weight_bounds_with_label(None, filter, min, max)
    }

    /// Create a weight bounds constraint with a label and validation.
    ///
    /// # Errors
    ///
    /// Returns [`ConstraintValidationError`] if `min > max`.
    pub fn weight_bounds_with_label(
        label: Option<String>,
        filter: PositionFilter,
        min: f64,
        max: f64,
    ) -> Result<Self, ConstraintValidationError> {
        if min > max {
            return Err(ConstraintValidationError {
                message: format!("weight bounds min ({}) must be <= max ({})", min, max),
            });
        }

        Ok(Self::WeightBounds {
            label,
            filter,
            min,
            max,
        })
    }

    /// Create a max turnover constraint with validation.
    ///
    /// # Arguments
    ///
    /// * `max_turnover` - Maximum allowed turnover (must be non-negative)
    ///
    /// # Errors
    ///
    /// Returns [`ConstraintValidationError`] if `max_turnover` is negative.
    pub fn max_turnover(max_turnover: f64) -> Result<Self, ConstraintValidationError> {
        Self::max_turnover_with_label(None, max_turnover)
    }

    /// Create a max turnover constraint with a label and validation.
    ///
    /// # Errors
    ///
    /// Returns [`ConstraintValidationError`] if `max_turnover` is negative.
    pub fn max_turnover_with_label(
        label: Option<String>,
        max_turnover: f64,
    ) -> Result<Self, ConstraintValidationError> {
        if max_turnover < 0.0 {
            return Err(ConstraintValidationError {
                message: format!("max_turnover must be non-negative, got {}", max_turnover),
            });
        }

        Ok(Self::MaxTurnover {
            label,
            max_turnover,
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_tag_exposure_limit_validation() {
        // Valid: 0.0
        assert!(Constraint::tag_exposure_limit("rating", "CCC", 0.0).is_ok());

        // Valid: 1.0
        assert!(Constraint::tag_exposure_limit("rating", "CCC", 1.0).is_ok());

        // Valid: 0.5
        assert!(Constraint::tag_exposure_limit("rating", "CCC", 0.5).is_ok());

        // Invalid: negative
        let result = Constraint::tag_exposure_limit("rating", "CCC", -0.1);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("max_share"));

        // Invalid: > 1.0
        let result = Constraint::tag_exposure_limit("rating", "CCC", 1.5);
        assert!(result.is_err());
    }

    #[test]
    fn test_tag_exposure_minimum_validation() {
        // Valid
        assert!(Constraint::tag_exposure_minimum("rating", "IG", 0.5).is_ok());

        // Invalid: negative
        let result = Constraint::tag_exposure_minimum("rating", "IG", -0.1);
        assert!(result.is_err());

        // Invalid: > 1.0
        let result = Constraint::tag_exposure_minimum("rating", "IG", 1.1);
        assert!(result.is_err());
    }

    #[test]
    fn test_weight_bounds_validation() {
        // Valid: min < max
        assert!(Constraint::weight_bounds(PositionFilter::All, 0.0, 0.1).is_ok());

        // Valid: min == max
        assert!(Constraint::weight_bounds(PositionFilter::All, 0.05, 0.05).is_ok());

        // Invalid: min > max
        let result = Constraint::weight_bounds(PositionFilter::All, 0.2, 0.1);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("min"));
    }

    #[test]
    fn test_max_turnover_validation() {
        // Valid
        assert!(Constraint::max_turnover(0.5).is_ok());
        assert!(Constraint::max_turnover(0.0).is_ok());

        // Invalid: negative
        let result = Constraint::max_turnover(-0.1);
        assert!(result.is_err());
    }
}
