//! Penalty method for American and Bermudan early exercise.
//!
//! After each time step, the penalty method enforces `u >= payoff` by adding
//! a large penalty term to the main diagonal at nodes where the constraint
//! is violated. This is simpler than PSOR and works naturally with all theta
//! schemes without inner iteration tuning.

/// Early exercise constraint enforced via the penalty method.
///
/// At exercise-eligible time steps, nodes where `u_i < payoff_i` get a large
/// penalty `lambda` added to the diagonal, driving the solution toward the
/// intrinsic value. One penalty iteration usually suffices; the solver optionally
/// does 2–3 for convergence assurance.
#[derive(Debug, Clone)]
pub struct PenaltyExercise {
    /// Penalty scaling factor (default `1e8`). The effective penalty per step
    /// is `penalty_factor / dt`.
    pub penalty_factor: f64,
    /// Intrinsic payoff value at each interior grid node.
    pub payoff_values: Vec<f64>,
    /// Exercise schedule (American = every step, Bermudan = specific times).
    pub exercise_type: ExerciseType,
    /// Number of penalty iterations per step (default 1; 2–3 for convergence assurance).
    pub iterations: usize,
}

/// Exercise schedule type.
#[derive(Debug, Clone)]
pub enum ExerciseType {
    /// Exercisable at every time step.
    American,
    /// Exercisable only at specified times (must align with time grid).
    Bermudan {
        /// Exercise times (year fractions from valuation date).
        exercise_times: Vec<f64>,
    },
}

impl PenaltyExercise {
    /// Create an American exercise constraint.
    ///
    /// # Arguments
    ///
    /// * `payoff_values` — intrinsic value at each interior grid node
    pub fn american(payoff_values: Vec<f64>) -> Self {
        Self {
            penalty_factor: 1e8,
            payoff_values,
            exercise_type: ExerciseType::American,
            iterations: 1,
        }
    }

    /// Create a Bermudan exercise constraint.
    ///
    /// # Arguments
    ///
    /// * `payoff_values` — intrinsic value at each interior grid node
    /// * `exercise_times` — times at which exercise is allowed
    pub fn bermudan(payoff_values: Vec<f64>, exercise_times: Vec<f64>) -> Self {
        Self {
            penalty_factor: 1e8,
            payoff_values,
            exercise_type: ExerciseType::Bermudan { exercise_times },
            iterations: 1,
        }
    }

    /// Check whether exercise is allowed at time `t`.
    pub fn is_exercise_time(&self, t: f64) -> bool {
        match &self.exercise_type {
            ExerciseType::American => true,
            ExerciseType::Bermudan { exercise_times } => {
                exercise_times.iter().any(|&et| (et - t).abs() < 1e-10)
            }
        }
    }

    /// Apply the penalty method to enforce the exercise constraint.
    ///
    /// After the linear solve, nodes where `u_i < payoff_i` are pushed
    /// toward the intrinsic value. Modifies `u` in place.
    ///
    /// Returns the early exercise boundary (leftmost grid index where
    /// continuation value exceeds intrinsic, or `None` if fully exercised).
    pub fn apply(&self, u: &mut [f64], dt: f64) -> Option<usize> {
        debug_assert_eq!(u.len(), self.payoff_values.len());

        let lambda = self.penalty_factor / dt;
        let mut boundary_idx = None;

        for _ in 0..self.iterations {
            for (i, (&payoff, u_val)) in self.payoff_values.iter().zip(u.iter_mut()).enumerate() {
                if *u_val < payoff {
                    // Apply penalty: push u toward payoff
                    // In continuous limit: u = (u + lambda*dt*payoff) / (1 + lambda*dt)
                    // With lambda*dt = penalty_factor >> 1, this ≈ payoff
                    *u_val = (*u_val + lambda * dt * payoff) / (1.0 + lambda * dt);
                } else if boundary_idx.is_none() {
                    boundary_idx = Some(i);
                }
            }
        }

        boundary_idx
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn american_penalty_enforces_floor() {
        let payoff = vec![5.0, 3.0, 1.0, 0.0, 0.0];
        let exercise = PenaltyExercise::american(payoff.clone());

        // Solution below intrinsic should be pushed up
        let mut u = vec![4.0, 2.0, 0.5, 1.0, 2.0];
        exercise.apply(&mut u, 0.01);

        for (i, (&u_val, &p_val)) in u.iter().zip(payoff.iter()).enumerate() {
            if p_val > 0.0 {
                assert!(
                    u_val >= p_val - 0.01,
                    "u[{i}]={u_val} should be near payoff={p_val}"
                );
            }
        }
    }

    #[test]
    fn bermudan_respects_schedule() {
        let payoff = vec![1.0, 1.0, 1.0];
        let exercise = PenaltyExercise::bermudan(payoff, vec![0.5, 1.0]);
        assert!(exercise.is_exercise_time(0.5));
        assert!(exercise.is_exercise_time(1.0));
        assert!(!exercise.is_exercise_time(0.75));
    }
}
