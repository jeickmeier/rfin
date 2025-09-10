//! Calibration constraints and optimization targets.

use finstack_core::F;

/// Calibration constraint for optimization.
#[derive(Clone, Debug)]
pub struct CalibrationConstraint {
    /// Instrument identifier
    pub instrument_id: String,
    /// Target value (rate, price, spread, etc.)
    pub target_value: F,
    /// Weight in objective function
    pub weight: F,
    /// Constraint type
    pub constraint_type: ConstraintType,
}

/// Type of calibration constraint.
#[derive(Clone, Debug)]
pub enum ConstraintType {
    /// Exact match (zero PV for par instruments)
    Exact,
    /// Weighted least squares fit
    WeightedFit,
    /// Inequality constraint (e.g., no-arbitrage)
    Inequality {
        bound: F,
        direction: InequalityDirection,
    },
}

/// Direction for inequality constraints.
#[derive(Clone, Debug)]
pub enum InequalityDirection {
    /// Value >= bound
    GreaterEqual,
    /// Value <= bound  
    LessEqual,
}

impl CalibrationConstraint {
    /// Create an exact constraint.
    pub fn exact(instrument_id: impl Into<String>, target_value: F) -> Self {
        Self {
            instrument_id: instrument_id.into(),
            target_value,
            weight: 1.0,
            constraint_type: ConstraintType::Exact,
        }
    }

    /// Create a weighted least squares constraint.
    pub fn weighted(instrument_id: impl Into<String>, target_value: F, weight: F) -> Self {
        Self {
            instrument_id: instrument_id.into(),
            target_value,
            weight,
            constraint_type: ConstraintType::WeightedFit,
        }
    }

    /// Set constraint weight.
    pub fn with_weight(mut self, weight: F) -> Self {
        self.weight = weight;
        self
    }
}
