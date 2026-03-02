//! Toggle exercise models for PIK/cash coupon decisions.
//!
//! Models the borrower's decision to pay-in-kind (PIK) or pay cash at each
//! coupon date. The toggle decision depends on observable credit state and can
//! follow a hard threshold rule, a stochastic (sigmoid) model, or an optimal
//! exercise strategy (stub for nested Monte Carlo).
//!
//! # Supported Models
//!
//! - **Threshold**: PIK when a credit metric crosses a boundary (above or below).
//! - **Stochastic**: PIK probability is a smooth sigmoid function of credit state.
//! - **OptimalExercise**: Nested MC for optimal toggle (stub -- see Task 14).
//!
//! # Examples
//!
//! ```
//! use finstack_valuations::instruments::common::models::credit::toggle_exercise::{
//!     CreditState, CreditStateVariable, ThresholdDirection, ToggleExerciseModel,
//! };
//! use finstack_core::math::random::Pcg64Rng;
//!
//! let model = ToggleExerciseModel::threshold(
//!     CreditStateVariable::HazardRate, 0.15, ThresholdDirection::Above,
//! );
//! let state = CreditState { hazard_rate: 0.20, ..Default::default() };
//! let mut rng = Pcg64Rng::new(42);
//! assert!(model.should_pik(&state, &mut rng));
//! ```

use finstack_core::math::random::RandomNumberGenerator;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Observable credit state at a point in time.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreditState {
    /// Current hazard rate (annualised instantaneous default intensity).
    pub hazard_rate: f64,
    /// Distance-to-default (number of standard deviations from the default point).
    pub distance_to_default: Option<f64>,
    /// Leverage ratio (debt / assets).
    pub leverage: f64,
    /// Accreted (PIK-augmented) notional outstanding.
    pub accreted_notional: f64,
    /// Fair value of the firm's assets, if available.
    pub asset_value: Option<f64>,
}

/// Which credit metric drives the toggle decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CreditStateVariable {
    /// Use the hazard rate.
    HazardRate,
    /// Use the distance-to-default.
    DistanceToDefault,
    /// Use the leverage ratio.
    Leverage,
}

/// Direction for threshold comparison.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ThresholdDirection {
    /// PIK when state > threshold (e.g., hazard rate above limit).
    Above,
    /// PIK when state < threshold (e.g., distance-to-default below limit).
    Below,
}

/// Toggle exercise model for PIK/cash decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToggleExerciseModel {
    /// Hard threshold: PIK when credit metric crosses boundary.
    Threshold(ThresholdToggle),
    /// Stochastic: PIK probability is smooth sigmoid of credit state.
    Stochastic(StochasticToggle),
    /// Optimal exercise: nested MC (stub for now, implemented in Task 14).
    OptimalExercise(OptimalToggle),
}

/// Hard threshold toggle configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdToggle {
    /// Credit metric to observe.
    pub state_variable: CreditStateVariable,
    /// Threshold value.
    pub threshold: f64,
    /// Direction for comparison.
    pub direction: ThresholdDirection,
}

/// Stochastic (sigmoid) toggle configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StochasticToggle {
    /// Credit metric to observe.
    pub state_variable: CreditStateVariable,
    /// Intercept of the logistic function: `P(PIK) = 1 / (1 + exp(-(intercept + sensitivity * state)))`.
    pub intercept: f64,
    /// Sensitivity (slope) of the logistic function with respect to the state variable.
    pub sensitivity: f64,
}

/// Optimal toggle configuration (stub -- requires nested MC from Task 14).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimalToggle {
    /// Number of nested Monte Carlo paths for continuation value estimation.
    pub nested_paths: usize,
    /// Equity holder discount rate for NPV of toggle decision.
    pub equity_discount_rate: f64,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Extract the relevant state value for the toggle decision.
fn extract_state_value(state: &CreditState, variable: &CreditStateVariable) -> f64 {
    match variable {
        CreditStateVariable::HazardRate => state.hazard_rate,
        CreditStateVariable::DistanceToDefault => state.distance_to_default.unwrap_or(0.0),
        CreditStateVariable::Leverage => state.leverage,
    }
}

// ---------------------------------------------------------------------------
// Implementation
// ---------------------------------------------------------------------------

impl ToggleExerciseModel {
    /// Create a threshold toggle model.
    #[must_use]
    pub fn threshold(
        variable: CreditStateVariable,
        threshold: f64,
        direction: ThresholdDirection,
    ) -> Self {
        Self::Threshold(ThresholdToggle {
            state_variable: variable,
            threshold,
            direction,
        })
    }

    /// Create a stochastic (sigmoid) toggle model.
    #[must_use]
    pub fn stochastic(variable: CreditStateVariable, intercept: f64, sensitivity: f64) -> Self {
        Self::Stochastic(StochasticToggle {
            state_variable: variable,
            intercept,
            sensitivity,
        })
    }

    /// Returns `true` if the borrower elects PIK at this coupon date.
    pub fn should_pik(&self, state: &CreditState, rng: &mut dyn RandomNumberGenerator) -> bool {
        match self {
            Self::Threshold(t) => {
                let value = extract_state_value(state, &t.state_variable);
                match t.direction {
                    ThresholdDirection::Above => value > t.threshold,
                    ThresholdDirection::Below => value < t.threshold,
                }
            }
            Self::Stochastic(s) => {
                let value = extract_state_value(state, &s.state_variable);
                let p = 1.0 / (1.0 + (-s.intercept - s.sensitivity * value).exp());
                rng.uniform() < p
            }
            Self::OptimalExercise(_) => {
                // Stub -- will be implemented in Task 14 (nested MC).
                false
            }
        }
    }

    /// Returns the PIK fraction in `[0, 1]`.
    ///
    /// For threshold: returns `0.0` or `1.0`.
    /// For stochastic: returns `0.0` or `1.0` (sampled from probability).
    /// For optimal exercise: returns `0.0` (stub).
    pub fn pik_fraction(&self, state: &CreditState, rng: &mut dyn RandomNumberGenerator) -> f64 {
        if self.should_pik(state, rng) {
            1.0
        } else {
            0.0
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use finstack_core::math::random::Pcg64Rng;

    #[test]
    fn threshold_piks_above_threshold() {
        let model = ToggleExerciseModel::threshold(
            CreditStateVariable::HazardRate,
            0.15,
            ThresholdDirection::Above,
        );
        let mut rng = Pcg64Rng::new(42);
        let state_low = CreditState {
            hazard_rate: 0.10,
            ..Default::default()
        };
        let state_high = CreditState {
            hazard_rate: 0.20,
            ..Default::default()
        };
        assert!(!model.should_pik(&state_low, &mut rng));
        assert!(model.should_pik(&state_high, &mut rng));
    }

    #[test]
    fn threshold_piks_below_threshold() {
        let model = ToggleExerciseModel::threshold(
            CreditStateVariable::DistanceToDefault,
            2.0,
            ThresholdDirection::Below,
        );
        let mut rng = Pcg64Rng::new(42);
        let state_safe = CreditState {
            distance_to_default: Some(3.0),
            ..Default::default()
        };
        let state_stressed = CreditState {
            distance_to_default: Some(1.5),
            ..Default::default()
        };
        assert!(!model.should_pik(&state_safe, &mut rng));
        assert!(model.should_pik(&state_stressed, &mut rng));
    }

    #[test]
    fn stochastic_toggle_probability_increases_with_hazard() {
        let model = ToggleExerciseModel::stochastic(CreditStateVariable::HazardRate, -3.0, 20.0);
        // Run 10k samples at lambda=0.10 and lambda=0.20
        let count_low: usize = (0..10_000)
            .filter(|i| {
                let mut rng = Pcg64Rng::new(42 + *i as u64);
                let state = CreditState {
                    hazard_rate: 0.10,
                    ..Default::default()
                };
                model.should_pik(&state, &mut rng)
            })
            .count();
        let count_high: usize = (0..10_000)
            .filter(|i| {
                let mut rng = Pcg64Rng::new(42 + *i as u64);
                let state = CreditState {
                    hazard_rate: 0.20,
                    ..Default::default()
                };
                model.should_pik(&state, &mut rng)
            })
            .count();
        assert!(
            count_high > count_low,
            "Higher hazard should have more PIK: low={count_low}, high={count_high}"
        );
    }

    #[test]
    fn pik_fraction_returns_0_or_1_for_threshold() {
        let model = ToggleExerciseModel::threshold(
            CreditStateVariable::HazardRate,
            0.15,
            ThresholdDirection::Above,
        );
        let mut rng = Pcg64Rng::new(42);
        let state_above = CreditState {
            hazard_rate: 0.20,
            ..Default::default()
        };
        let state_below = CreditState {
            hazard_rate: 0.10,
            ..Default::default()
        };
        assert!((model.pik_fraction(&state_above, &mut rng) - 1.0).abs() < 1e-10);
        assert!((model.pik_fraction(&state_below, &mut rng) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn optimal_toggle_stub_returns_false() {
        let model = ToggleExerciseModel::OptimalExercise(OptimalToggle {
            nested_paths: 100,
            equity_discount_rate: 0.10,
        });
        let mut rng = Pcg64Rng::new(42);
        let state = CreditState {
            hazard_rate: 0.20,
            ..Default::default()
        };
        // Stub should return false (not panic)
        assert!(!model.should_pik(&state, &mut rng));
    }
}
