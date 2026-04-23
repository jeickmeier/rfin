//! Time-stepping schemes for backward PDE evolution.
//!
//! Provides theta-scheme time stepping (explicit, implicit, Crank-Nicolson)
//! and Rannacher smoothing (implicit start + CN continuation) to eliminate
//! spurious oscillations near payoff discontinuities.

use super::grid::Grid1D;
use super::operator::TridiagOperator;
use super::problem::PdeProblem1D;

/// Time-stepping strategy for advancing the PDE solution backward from maturity.
///
/// Implementors define how each time step is executed, potentially with
/// different theta parameters at different stages (e.g., Rannacher smoothing).
pub trait TimeStepper {
    /// Advance the solution one step backward in time from `t_from` to `t_to`.
    ///
    /// * `problem` — PDE coefficients and boundary conditions
    /// * `grid` — spatial grid
    /// * `u` — solution vector (interior points only, length `grid.n_interior()`)
    /// * `t_from` — current time (closer to maturity)
    /// * `t_to` — target time (closer to t=0)
    /// * `step_index` — zero-based step counter (for Rannacher switching)
    fn step(
        &self,
        problem: &dyn PdeProblem1D,
        grid: &Grid1D,
        u: &mut [f64],
        t_from: f64,
        t_to: f64,
        step_index: usize,
    );

    /// Total number of time steps.
    fn n_steps(&self) -> usize;

    /// Generate time levels from maturity (T) backward to 0.
    ///
    /// Returns a vector of length `n_steps + 1` with `levels[0] = T`
    /// and `levels[n_steps] = 0`. Steps proceed backward: the solver
    /// evolves from `levels[i]` to `levels[i+1]` for `i = 0..n_steps-1`.
    fn time_levels(&self, maturity: f64) -> Vec<f64> {
        let n = self.n_steps();
        let dt = maturity / n as f64;
        (0..=n).map(|i| maturity - i as f64 * dt).collect()
    }
}

/// Standard theta-scheme time stepper.
///
/// - `theta = 0.0`: fully explicit (forward Euler) — conditionally stable, for debugging only
/// - `theta = 0.5`: Crank-Nicolson — second-order in time, the production workhorse
/// - `theta = 1.0`: fully implicit (backward Euler) — first-order, unconditionally stable
pub struct ThetaStepper {
    /// Theta parameter (0 = explicit, 0.5 = CN, 1 = implicit).
    theta: f64,
    /// Number of time steps.
    n_steps: usize,
}

impl ThetaStepper {
    /// Create a Crank-Nicolson stepper (theta = 0.5).
    pub fn crank_nicolson(n_steps: usize) -> Self {
        Self {
            theta: 0.5,
            n_steps,
        }
    }

    /// Create a fully implicit stepper (theta = 1.0).
    pub fn implicit(n_steps: usize) -> Self {
        Self {
            theta: 1.0,
            n_steps,
        }
    }

    /// Create a fully explicit stepper (theta = 0.0). Use for debugging only.
    pub fn explicit(n_steps: usize) -> Self {
        Self {
            theta: 0.0,
            n_steps,
        }
    }

    /// Create a stepper with custom theta.
    pub fn custom(theta: f64, n_steps: usize) -> Self {
        Self { theta, n_steps }
    }
}

impl TimeStepper for ThetaStepper {
    fn step(
        &self,
        problem: &dyn PdeProblem1D,
        grid: &Grid1D,
        u: &mut [f64],
        t_from: f64,
        t_to: f64,
        _step_index: usize,
    ) {
        theta_step(problem, grid, u, t_from, t_to, self.theta);
    }

    fn n_steps(&self) -> usize {
        self.n_steps
    }
}

/// Rannacher time stepper: runs a few fully implicit steps at the start
/// (near the terminal condition) then switches to Crank-Nicolson.
///
/// Eliminates the spurious oscillations that Crank-Nicolson produces near
/// payoff discontinuities (e.g., at the strike of a digital option or at
/// barrier knock-out levels). Typically 2–4 implicit steps suffice.
pub struct RannacherStepper {
    /// Number of initial fully-implicit steps (typically 2–4).
    implicit_steps: usize,
    /// Theta for remaining steps (usually 0.5 for Crank-Nicolson).
    theta: f64,
    /// Total number of time steps.
    n_steps: usize,
}

impl RannacherStepper {
    /// Create a Rannacher stepper with `implicit_steps` initial implicit steps
    /// followed by Crank-Nicolson for the remainder.
    pub fn new(implicit_steps: usize, n_steps: usize) -> Self {
        Self {
            implicit_steps,
            theta: 0.5,
            n_steps,
        }
    }
}

impl TimeStepper for RannacherStepper {
    fn step(
        &self,
        problem: &dyn PdeProblem1D,
        grid: &Grid1D,
        u: &mut [f64],
        t_from: f64,
        t_to: f64,
        step_index: usize,
    ) {
        let theta = if step_index < self.implicit_steps {
            1.0
        } else {
            self.theta
        };
        theta_step(problem, grid, u, t_from, t_to, theta);
    }

    fn n_steps(&self) -> usize {
        self.n_steps
    }
}

/// Execute a single theta-scheme time step from `t_from` to `t_to` (backward).
///
/// The scheme:
/// ```text
/// (I - theta * dt * A_to) * u_to = (I + (1-theta) * dt * A_from) * u_from
///                                  + dt * [theta * (source_to + bc_to)
///                                         + (1-theta) * (source_from + bc_from)]
/// ```
///
/// For time-homogeneous problems, `A_from == A_to` and only one assembly is needed.
fn theta_step(
    problem: &dyn PdeProblem1D,
    grid: &Grid1D,
    u: &mut [f64],
    t_from: f64,
    t_to: f64,
    theta: f64,
) {
    let dt = t_from - t_to;
    debug_assert!(dt > 0.0);

    let is_homogeneous = problem.is_time_homogeneous();

    // Assemble the operator at t_from (explicit side)
    let op_from = TridiagOperator::assemble(problem, grid, t_from);

    // Explicit part: y = (I + (1-theta)*dt * A_from) * u + (1-theta)*dt*(source + bc)
    let beta = (1.0 - theta) * dt;
    let mut rhs = op_from.apply_explicit(beta, u);

    // If time-homogeneous, reuse op_from for the implicit side
    let op_to = if is_homogeneous {
        op_from
    } else {
        TridiagOperator::assemble(problem, grid, t_to)
    };

    // Add implicit-side source and boundary corrections: theta * dt * (source_to + bc_to)
    let alpha = theta * dt;
    op_to.add_implicit_corrections(alpha, &mut rhs);

    // Solve (I - theta*dt * A_to) * u_new = rhs
    let u_new = op_to.solve_thomas(alpha, &rhs);
    u.copy_from_slice(&u_new);
}

#[cfg(test)]
mod tests {
    use super::super::boundary::BoundaryCondition;
    use super::*;

    /// Heat equation: u_t = u_xx on [0, pi] with u(0,t) = u(pi,t) = 0
    /// Terminal: u(x, T) = sin(x)
    /// Exact: u(x, t) = exp(-(T-t)) * sin(x)
    struct HeatSinProblem {
        maturity: f64,
    }

    impl PdeProblem1D for HeatSinProblem {
        fn diffusion(&self, _x: f64, _t: f64) -> f64 {
            1.0
        }
        fn convection(&self, _x: f64, _t: f64) -> f64 {
            0.0
        }
        fn reaction(&self, _x: f64, _t: f64) -> f64 {
            0.0
        }
        fn terminal_condition(&self, x: f64) -> f64 {
            x.sin()
        }
        fn lower_boundary(&self, _t: f64) -> BoundaryCondition {
            BoundaryCondition::Dirichlet(0.0)
        }
        fn upper_boundary(&self, _t: f64) -> BoundaryCondition {
            BoundaryCondition::Dirichlet(0.0)
        }
        fn is_time_homogeneous(&self) -> bool {
            true
        }
    }

    #[test]
    fn heat_equation_implicit_converges() {
        let t_mat = 0.5;
        let problem = HeatSinProblem { maturity: t_mat };
        let n_space = 101;
        let n_time = 200;
        let grid = Grid1D::uniform(0.0, std::f64::consts::PI, n_space).expect("valid grid");
        let stepper = ThetaStepper::implicit(n_time);

        // Initialize terminal condition (interior points only)
        let mut u: Vec<f64> = grid.points()[1..grid.n() - 1]
            .iter()
            .map(|&x| problem.terminal_condition(x))
            .collect();

        // Step backward
        let levels = stepper.time_levels(t_mat);
        for i in 0..n_time {
            stepper.step(&problem, &grid, &mut u, levels[i], levels[i + 1], i);
        }

        // Compare with exact solution at t=0
        let exact_factor = (-t_mat).exp();
        let mid = grid.n_interior() / 2;
        let x_mid = grid.points()[mid + 1];
        let exact = exact_factor * x_mid.sin();
        let error = (u[mid] - exact).abs();
        assert!(
            error < 0.005,
            "Implicit Euler error too large: {error:.6} at x={x_mid:.4}"
        );
    }

    #[test]
    fn heat_equation_cn_converges_faster() {
        let t_mat = 0.5;
        let problem = HeatSinProblem { maturity: t_mat };
        let n_space = 51;
        let n_time = 100;
        let grid = Grid1D::uniform(0.0, std::f64::consts::PI, n_space).expect("valid grid");
        let stepper = ThetaStepper::crank_nicolson(n_time);

        let mut u: Vec<f64> = grid.points()[1..grid.n() - 1]
            .iter()
            .map(|&x| problem.terminal_condition(x))
            .collect();

        let levels = stepper.time_levels(t_mat);
        for i in 0..n_time {
            stepper.step(&problem, &grid, &mut u, levels[i], levels[i + 1], i);
        }

        let exact_factor = (-t_mat).exp();
        let mid = grid.n_interior() / 2;
        let x_mid = grid.points()[mid + 1];
        let exact = exact_factor * x_mid.sin();
        let error = (u[mid] - exact).abs();
        // CN should be much more accurate than implicit Euler with same grid
        assert!(
            error < 1e-4,
            "CN error too large: {error:.6e} at x={x_mid:.4}"
        );
    }
}
