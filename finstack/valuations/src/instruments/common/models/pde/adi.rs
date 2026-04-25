//! Craig-Sneyd ADI time-stepping for 2D PDEs.
//!
//! Implements the modified Craig-Sneyd (MCS) scheme from In 't Hout & Welfert
//! (2009) for solving 2D convection-diffusion-reaction PDEs. Each time step
//! splits into fractional steps along each spatial direction, with the
//! cross-derivative term treated explicitly.
//!
//! # Algorithm
//!
//! Given operators `A_x`, `A_y` (directional tridiag) and `A_xy` (cross-deriv):
//!
//! ```text
//! Y_0 = u^n + dt * (A_x + A_y + A_xy) * u^n          [explicit predictor]
//! Y_1 = Y_0 + theta * dt * (A_x * Y_1 - A_x * u^n)   [implicit x-sweep]
//! Y_2 = Y_1 + theta * dt * (A_y * Y_2 - A_y * u^n)   [implicit y-sweep]
//! u^{n+1} = Y_2
//! ```
//!
//! For theta = 0.5 this gives the standard Craig-Sneyd scheme (second-order
//! in time). The implicit sweeps are tridiagonal solves reusing the 1D
//! Thomas algorithm from Phase 1.
//!
//! # References
//!
//! - In 't Hout, K. J. & Welfert, B. D. (2009). "Unconditional stability of
//!   second-order ADI schemes applied to multi-dimensional diffusion equations
//!   with mixed derivative terms." *Applied Numerical Mathematics*, 59(3-4).
//! - Craig, I. J. D. & Sneyd, A. D. (1988). "An alternating-direction implicit
//!   scheme for parabolic equations with mixed derivatives."

use super::boundary::BoundaryCondition;
use super::grid2d::Grid2D;
use super::operator2d::{apply_cross_derivative, Operators2D};
use super::problem2d::PdeProblem2D;

/// Reusable per-step scratch buffers for Craig-Sneyd ADI.
///
/// Sized to the largest grid the caller intends to step. Reusing this across
/// timesteps (and across grids of identical shape) lets `Solver2D` skip the
/// six allocations the previous implementation performed every call to
/// [`CraigSneydStepper::step_with_buffers`].
#[derive(Clone, Default)]
pub struct AdiWorkBuffers {
    x_line: Vec<f64>,
    y_line: Vec<f64>,
    line_out: Vec<f64>,
    rhs_buf: Vec<f64>,
    y0: Vec<f64>,
    y1: Vec<f64>,
    ax_u: Vec<f64>,
    ay_u: Vec<f64>,
}

impl AdiWorkBuffers {
    /// Pre-allocate buffers sized to a particular grid.
    pub fn for_grid(grid: &Grid2D) -> Self {
        let nx_int = grid.nx_interior();
        let ny_int = grid.ny_interior();
        let interior = nx_int * ny_int;
        let line_max = nx_int.max(ny_int);
        Self {
            x_line: vec![0.0; nx_int],
            y_line: vec![0.0; ny_int],
            line_out: vec![0.0; line_max],
            rhs_buf: vec![0.0; line_max],
            y0: vec![0.0; interior],
            y1: vec![0.0; interior],
            ax_u: vec![0.0; interior],
            ay_u: vec![0.0; interior],
        }
    }

    /// Resize all buffers in place if the grid shape changed.
    fn resize_for(&mut self, grid: &Grid2D) {
        let nx_int = grid.nx_interior();
        let ny_int = grid.ny_interior();
        let interior = nx_int * ny_int;
        let line_max = nx_int.max(ny_int);
        if self.x_line.len() != nx_int {
            self.x_line.resize(nx_int, 0.0);
        }
        if self.y_line.len() != ny_int {
            self.y_line.resize(ny_int, 0.0);
        }
        if self.line_out.len() != line_max {
            self.line_out.resize(line_max, 0.0);
        }
        if self.rhs_buf.len() != line_max {
            self.rhs_buf.resize(line_max, 0.0);
        }
        if self.y0.len() != interior {
            self.y0.resize(interior, 0.0);
        }
        if self.y1.len() != interior {
            self.y1.resize(interior, 0.0);
        }
        if self.ax_u.len() != interior {
            self.ax_u.resize(interior, 0.0);
        }
        if self.ay_u.len() != interior {
            self.ay_u.resize(interior, 0.0);
        }
    }
}

/// Craig-Sneyd ADI time stepper for 2D problems.
///
/// Supports both standard Craig-Sneyd (`theta = 0.5`) and a Rannacher-like
/// variant that uses `theta = 1.0` for the first few steps to smooth
/// discontinuities near the terminal condition.
pub struct CraigSneydStepper {
    /// Theta parameter for the implicit weight (0.5 = standard CS).
    theta: f64,
    /// Number of initial fully-implicit steps (Rannacher smoothing).
    implicit_start_steps: usize,
    /// Total number of time steps.
    n_steps: usize,
}

impl CraigSneydStepper {
    /// Standard Craig-Sneyd with `theta = 0.5`.
    pub fn new(n_steps: usize) -> Self {
        Self {
            theta: 0.5,
            implicit_start_steps: 0,
            n_steps,
        }
    }

    /// Craig-Sneyd with Rannacher-style smoothing: use `theta = 1.0` for the
    /// first `implicit_start` steps, then switch to `theta = 0.5`.
    pub fn with_rannacher(implicit_start: usize, n_steps: usize) -> Self {
        Self {
            theta: 0.5,
            implicit_start_steps: implicit_start,
            n_steps,
        }
    }

    /// Total number of time steps.
    pub fn n_steps(&self) -> usize {
        self.n_steps
    }

    /// Generate time levels from maturity backward to 0.
    pub fn time_levels(&self, maturity: f64) -> Vec<f64> {
        let n = self.n_steps;
        let dt = maturity / n as f64;
        (0..=n).map(|i| maturity - i as f64 * dt).collect()
    }

    /// Execute one Craig-Sneyd ADI step from `t_from` to `t_to` (backward).
    ///
    /// Allocates fresh work buffers on every call; prefer
    /// [`CraigSneydStepper::step_with_buffers`] when stepping in a loop.
    ///
    /// `u_full` is the full (boundary-inclusive) solution of length `nx * ny`.
    /// `u_int` is the interior solution of length `nx_int * ny_int` (row-major).
    /// Both are updated in place.
    #[allow(clippy::too_many_arguments)]
    pub fn step(
        &self,
        problem: &dyn PdeProblem2D,
        grid: &Grid2D,
        u_full: &mut [f64],
        u_int: &mut [f64],
        t_from: f64,
        t_to: f64,
        step_index: usize,
    ) {
        let mut buffers = AdiWorkBuffers::for_grid(grid);
        self.step_with_buffers(
            problem,
            grid,
            u_full,
            u_int,
            t_from,
            t_to,
            step_index,
            &mut buffers,
        );
    }

    /// Like [`CraigSneydStepper::step`], but reuses caller-owned scratch
    /// buffers instead of allocating six fresh `Vec`s on every call.
    #[allow(clippy::too_many_arguments)]
    pub fn step_with_buffers(
        &self,
        problem: &dyn PdeProblem2D,
        grid: &Grid2D,
        u_full: &mut [f64],
        u_int: &mut [f64],
        t_from: f64,
        t_to: f64,
        step_index: usize,
        buffers: &mut AdiWorkBuffers,
    ) {
        let dt = t_from - t_to;
        debug_assert!(dt > 0.0);

        let theta = if step_index < self.implicit_start_steps {
            1.0
        } else {
            self.theta
        };

        let nx_int = grid.nx_interior();
        let ny_int = grid.ny_interior();

        buffers.resize_for(grid);
        let AdiWorkBuffers {
            x_line,
            y_line,
            line_out,
            rhs_buf,
            y0,
            y1,
            ax_u,
            ay_u,
        } = buffers;

        // Assemble operators at t_from (for the explicit evaluation)
        let ops = Operators2D::assemble(problem, grid, t_from);

        // --- Step 0: Explicit predictor ---
        // Y_0 = u^n + dt * (A_x * u^n + A_y * u^n + A_xy * u^n + c * u^n)
        let cross = apply_cross_derivative(&ops.cross_deriv, u_full, grid);

        // Compute A_x * u^n for each y-line
        for jj in 0..ny_int {
            for ii in 0..nx_int {
                x_line[ii] = u_int[ii * ny_int + jj];
            }
            ops.op_x[jj].apply_into(x_line, &mut line_out[..nx_int]);
            for ii in 0..nx_int {
                ax_u[ii * ny_int + jj] = line_out[ii];
            }
        }

        // Compute A_y * u^n for each x-line
        for ii in 0..nx_int {
            for jj in 0..ny_int {
                y_line[jj] = u_int[ii * ny_int + jj];
            }
            ops.op_y[ii].apply_into(y_line, &mut line_out[..ny_int]);
            for jj in 0..ny_int {
                ay_u[ii * ny_int + jj] = line_out[jj];
            }
        }

        // Y_0 = u^n + dt * (A_x*u^n + A_y*u^n + A_xy*u^n)
        // Note: the reaction term c(x,y,t)*u is already split into the
        // directional operators (c/2 in A_x, c/2 in A_y), so ax_u + ay_u
        // already includes the full reaction contribution.
        for idx in 0..nx_int * ny_int {
            y0[idx] = u_int[idx] + dt * (ax_u[idx] + ay_u[idx] + cross[idx]);
        }

        // --- Step 1: Implicit x-sweep ---
        // Reassemble operators at t_to for the implicit side
        let ops_impl = if problem.is_time_homogeneous() {
            ops
        } else {
            Operators2D::assemble(problem, grid, t_to)
        };

        let alpha_x = theta * dt;

        for jj in 0..ny_int {
            for ii in 0..nx_int {
                rhs_buf[ii] = y0[ii * ny_int + jj] - theta * dt * ax_u[ii * ny_int + jj];
            }
            ops_impl.op_x[jj].add_implicit_corrections(alpha_x, &mut rhs_buf[..nx_int]);
            ops_impl.op_x[jj].solve_thomas_into(
                alpha_x,
                &rhs_buf[..nx_int],
                &mut line_out[..nx_int],
            );
            for ii in 0..nx_int {
                y1[ii * ny_int + jj] = line_out[ii];
            }
        }

        // --- Step 2: Implicit y-sweep ---
        let alpha_y = theta * dt;
        // Reuse y0 as output for y-sweep (y0 is fully consumed by x-sweep)
        for ii in 0..nx_int {
            for jj in 0..ny_int {
                rhs_buf[jj] = y1[ii * ny_int + jj] - theta * dt * ay_u[ii * ny_int + jj];
            }
            ops_impl.op_y[ii].add_implicit_corrections(alpha_y, &mut rhs_buf[..ny_int]);
            ops_impl.op_y[ii].solve_thomas_into(
                alpha_y,
                &rhs_buf[..ny_int],
                &mut line_out[..ny_int],
            );
            for jj in 0..ny_int {
                y0[ii * ny_int + jj] = line_out[jj];
            }
        }

        // Update interior solution
        u_int.copy_from_slice(y0);

        // Reconstruct full solution from interior + boundary conditions
        fill_boundaries(problem, grid, u_full, u_int, t_to);
    }
}

/// Fill boundary values in the full grid from boundary conditions and the
/// interior solution.
///
/// `u_full` has length `nx * ny`, `u_int` has length `nx_int * ny_int`.
pub fn fill_boundaries(
    problem: &dyn PdeProblem2D,
    grid: &Grid2D,
    u_full: &mut [f64],
    u_int: &[f64],
    t: f64,
) {
    let nx = grid.nx();
    let ny = grid.ny();
    let nx_int = grid.nx_interior();
    let ny_int = grid.ny_interior();
    let x_pts = grid.x().points();
    let y_pts = grid.y().points();

    // Copy interior into full grid
    for ii in 0..nx_int {
        for jj in 0..ny_int {
            u_full[(ii + 1) * ny + (jj + 1)] = u_int[ii * ny_int + jj];
        }
    }

    // x-boundaries (left and right edges): all y-values
    for j in 0..ny {
        let y = y_pts[j];
        // Lower x-boundary (i = 0)
        u_full[j] = boundary_value_2d(
            problem.boundary_x_lower(y, t),
            u_full,
            grid,
            0,
            j,
            true,
            true,
        );
        // Upper x-boundary (i = nx-1)
        u_full[(nx - 1) * ny + j] = boundary_value_2d(
            problem.boundary_x_upper(y, t),
            u_full,
            grid,
            nx - 1,
            j,
            true,
            false,
        );
    }

    // y-boundaries (bottom and top edges): interior x-values only
    // (corners already set by x-boundary pass)
    for i in 1..nx - 1 {
        let x = x_pts[i];
        // Lower y-boundary (j = 0)
        u_full[i * ny] = boundary_value_2d(
            problem.boundary_y_lower(x, t),
            u_full,
            grid,
            i,
            0,
            false,
            true,
        );
        // Upper y-boundary (j = ny-1)
        u_full[i * ny + ny - 1] = boundary_value_2d(
            problem.boundary_y_upper(x, t),
            u_full,
            grid,
            i,
            ny - 1,
            false,
            false,
        );
    }
}

/// Extract a boundary value from a boundary condition.
///
/// For Dirichlet, returns the fixed value. For Neumann, extrapolates using
/// the derivative value. For Linear, extrapolates from two interior points
/// (vanishing second derivative). `is_x_dir` indicates which direction the
/// boundary is on; `is_lower` indicates lower vs upper edge.
fn boundary_value_2d(
    bc: BoundaryCondition,
    u_full: &[f64],
    grid: &Grid2D,
    i: usize,
    j: usize,
    is_x_dir: bool,
    is_lower: bool,
) -> f64 {
    let ny = grid.ny();
    match bc {
        BoundaryCondition::Dirichlet(g) => g,
        BoundaryCondition::Neumann(g) => {
            // du/dn = g: first-order extrapolation using the derivative value
            if is_x_dir {
                if is_lower {
                    let h = grid.x().h_left(1);
                    let u1 = u_full[ny + j];
                    u1 - h * g
                } else {
                    let h = grid.x().h_right(grid.nx() - 2);
                    let u1 = u_full[(i - 1) * ny + j];
                    u1 + h * g
                }
            } else if is_lower {
                let h = grid.y().h_left(1);
                let u1 = u_full[i * ny + 1];
                u1 - h * g
            } else {
                let h = grid.y().h_right(grid.ny() - 2);
                let u1 = u_full[i * ny + (j - 1)];
                u1 + h * g
            }
        }
        BoundaryCondition::Linear => {
            // d²u/dx² = 0: linear extrapolation from two interior neighbors
            if is_x_dir {
                if is_lower {
                    // i=0: extrapolate from i=1, i=2
                    let u1 = u_full[ny + j];
                    let u2 = u_full[2 * ny + j];
                    2.0 * u1 - u2
                } else {
                    // i=nx-1: extrapolate from i=nx-2, i=nx-3
                    let u1 = u_full[(i - 1) * ny + j];
                    let u2 = u_full[(i - 2) * ny + j];
                    2.0 * u1 - u2
                }
            } else if is_lower {
                // j=0: extrapolate from j=1, j=2
                let u1 = u_full[i * ny + 1];
                let u2 = u_full[i * ny + 2];
                2.0 * u1 - u2
            } else {
                // j=ny-1: extrapolate from j=ny-2, j=ny-3
                let u1 = u_full[i * ny + (j - 1)];
                let u2 = u_full[i * ny + (j - 2)];
                2.0 * u1 - u2
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::boundary::BoundaryCondition;
    use super::super::grid::Grid1D;
    use super::super::grid2d::Grid2D;
    use super::super::problem2d::PdeProblem2D;
    use super::*;

    /// 2D heat equation: u_t = u_xx + u_yy on [0,pi] x [0,pi]
    /// Terminal: u(x,y,T) = sin(x) * sin(y)
    /// Exact: u(x,y,t) = exp(-2*(T-t)) * sin(x) * sin(y)
    struct Heat2D {
        maturity: f64,
    }

    impl PdeProblem2D for Heat2D {
        fn diffusion_xx(&self, _x: f64, _y: f64, _t: f64) -> f64 {
            1.0
        }
        fn diffusion_yy(&self, _x: f64, _y: f64, _t: f64) -> f64 {
            1.0
        }
        fn mixed_diffusion(&self, _x: f64, _y: f64, _t: f64) -> f64 {
            0.0
        }
        fn convection_x(&self, _x: f64, _y: f64, _t: f64) -> f64 {
            0.0
        }
        fn convection_y(&self, _x: f64, _y: f64, _t: f64) -> f64 {
            0.0
        }
        fn reaction(&self, _x: f64, _y: f64, _t: f64) -> f64 {
            0.0
        }
        fn terminal_condition(&self, x: f64, y: f64) -> f64 {
            x.sin() * y.sin()
        }
        fn boundary_x_lower(&self, _y: f64, _t: f64) -> BoundaryCondition {
            BoundaryCondition::Dirichlet(0.0)
        }
        fn boundary_x_upper(&self, _y: f64, _t: f64) -> BoundaryCondition {
            BoundaryCondition::Dirichlet(0.0)
        }
        fn boundary_y_lower(&self, _x: f64, _t: f64) -> BoundaryCondition {
            BoundaryCondition::Dirichlet(0.0)
        }
        fn boundary_y_upper(&self, _x: f64, _t: f64) -> BoundaryCondition {
            BoundaryCondition::Dirichlet(0.0)
        }
        fn is_time_homogeneous(&self) -> bool {
            true
        }
    }

    #[test]
    fn heat_2d_adi_converges() {
        let t_mat = 0.25;
        let problem = Heat2D { maturity: t_mat };
        let pi = std::f64::consts::PI;

        let n_space = 41;
        let n_time = 200;

        let gx = Grid1D::uniform(0.0, pi, n_space).expect("valid grid");
        let gy = Grid1D::uniform(0.0, pi, n_space).expect("valid grid");
        let grid = Grid2D::new(gx, gy);

        let nx = grid.nx();
        let ny = grid.ny();
        let nx_int = grid.nx_interior();
        let ny_int = grid.ny_interior();

        // Initialize full solution with terminal condition
        let mut u_full = vec![0.0; nx * ny];
        for i in 0..nx {
            for j in 0..ny {
                u_full[i * ny + j] =
                    problem.terminal_condition(grid.x().points()[i], grid.y().points()[j]);
            }
        }

        // Extract interior
        let mut u_int = vec![0.0; nx_int * ny_int];
        for ii in 0..nx_int {
            for jj in 0..ny_int {
                u_int[ii * ny_int + jj] = u_full[(ii + 1) * ny + (jj + 1)];
            }
        }

        let stepper = CraigSneydStepper::new(n_time);
        let levels = stepper.time_levels(t_mat);

        for step in 0..n_time {
            stepper.step(
                &problem,
                &grid,
                &mut u_full,
                &mut u_int,
                levels[step],
                levels[step + 1],
                step,
            );
        }

        // Check at (pi/2, pi/2)
        let exact = (-2.0 * t_mat).exp() * (pi / 2.0).sin() * (pi / 2.0).sin();
        let computed = grid.interpolate(&u_full, pi / 2.0, pi / 2.0);
        let error = (computed - exact).abs();
        assert!(
            error < 0.01,
            "2D heat CS error = {error:.6e}, exact = {exact:.6}, computed = {computed:.6}"
        );
    }
}
