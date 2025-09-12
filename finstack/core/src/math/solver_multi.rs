//! Multi-dimensional optimization and root-finding algorithms.
//!
//! This module provides solvers for multi-dimensional problems commonly
//! encountered in financial calibration, such as SABR parameter fitting
//! and multi-curve bootstrapping.
//!
//! # Examples
//!
//! ```
//! use finstack_core::math::solver_multi::{LevenbergMarquardtSolver, MultiSolver};
//!
//! let solver = LevenbergMarquardtSolver::new().with_tolerance(1e-8);
//! 
//! // Minimize sum of squares: (x-2)^2 + (y-3)^2
//! let objective = |params: &[f64]| -> f64 {
//!     (params[0] - 2.0).powi(2) + (params[1] - 3.0).powi(2)
//! };
//! 
//! let initial = vec![0.0, 0.0];
//! let result = solver.minimize(objective, &initial, None).unwrap();
//! assert!((result[0] - 2.0).abs() < 1e-6);
//! assert!((result[1] - 3.0).abs() < 1e-6);
//! ```

use crate::{Result, F};
use crate::error::InputError;
use crate::math::random::RandomNumberGenerator;

/// Trait for functions that can provide analytical derivatives.
///
/// This trait allows optimization algorithms to use exact derivatives
/// when available, significantly improving convergence speed and accuracy.
pub trait AnalyticalDerivatives {
    /// Compute the gradient of the objective function.
    ///
    /// # Arguments
    /// * `params` - Current parameter values
    /// * `gradient` - Output buffer for gradient (must be same length as params)
    fn gradient(&self, params: &[F], gradient: &mut [F]);
    
    /// Compute the Jacobian matrix for a system of equations.
    ///
    /// # Arguments
    /// * `params` - Current parameter values
    /// * `jacobian` - Output buffer for Jacobian matrix (rows = equations, cols = params)
    ///
    /// Default implementation returns None, indicating Jacobian is not available.
    fn jacobian(&self, _params: &[F], _jacobian: &mut [Vec<F>]) -> Option<()> {
        None
    }
    
    /// Returns true if analytical gradient is available.
    fn has_gradient(&self) -> bool {
        true
    }
    
    /// Returns true if analytical Jacobian is available.
    fn has_jacobian(&self) -> bool {
        false
    }
}

/// Multi-dimensional optimization/root-finding trait.
///
/// Provides a unified interface for solvers that can handle multiple
/// parameters simultaneously, essential for calibrating complex models
/// like SABR volatility surfaces.
pub trait MultiSolver: Send + Sync {
    /// Minimize objective function starting from initial guess.
    ///
    /// # Arguments
    /// * `objective` - Function to minimize, takes parameter vector and returns scalar
    /// * `initial` - Initial parameter guess
    /// * `bounds` - Optional box constraints for each parameter
    ///
    /// # Returns
    /// Optimal parameter vector that minimizes the objective
    fn minimize<Obj>(
        &self,
        objective: Obj,
        initial: &[F],
        bounds: Option<&[(F, F)]>,
    ) -> Result<Vec<F>>
    where
        Obj: Fn(&[F]) -> F;

    /// Solve system of equations f(x) = 0 using least squares.
    ///
    /// # Arguments
    /// * `residuals` - Function that computes residual vector
    /// * `initial` - Initial parameter guess
    ///
    /// # Returns
    /// Parameter vector that minimizes ||f(x)||²
    fn solve_system<Res>(&self, residuals: Res, initial: &[F]) -> Result<Vec<F>>
    where
        Res: Fn(&[F], &mut [F]),
    {
        // Default implementation: convert to minimization problem
        let objective = |params: &[F]| -> F {
            let mut resid = vec![0.0; initial.len()];
            residuals(params, &mut resid);
            resid.iter().map(|r| r * r).sum()
        };
        self.minimize(objective, initial, None)
    }
}

/// Levenberg-Marquardt solver for non-linear least squares.
///
/// This algorithm is particularly effective for problems where the objective
/// is a sum of squares, common in curve fitting and calibration tasks.
/// It adaptively switches between Gauss-Newton (fast convergence near solution)
/// and gradient descent (robust far from solution).
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LevenbergMarquardtSolver {
    /// Convergence tolerance for gradient norm
    pub tolerance: F,
    /// Maximum iterations
    pub max_iterations: usize,
    /// Initial damping parameter (lambda)
    pub lambda_init: F,
    /// Factor for adjusting lambda (increase on failure, decrease on success)
    pub lambda_factor: F,
    /// Finite difference step size for Jacobian approximation
    pub fd_step: F,
    /// Minimum allowed step size (for numerical stability)
    pub min_step_size: F,
}

impl Default for LevenbergMarquardtSolver {
    fn default() -> Self {
        Self {
            tolerance: 1e-8,
            max_iterations: 100,
            lambda_init: 1e-3,
            lambda_factor: 10.0,
            fd_step: 1e-8,
            min_step_size: 1e-12,
        }
    }
}

impl LevenbergMarquardtSolver {
    /// Create a new Levenberg-Marquardt solver with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set convergence tolerance.
    pub fn with_tolerance(mut self, tolerance: F) -> Self {
        self.tolerance = tolerance;
        self
    }

    /// Set maximum iterations.
    pub fn with_max_iterations(mut self, max_iterations: usize) -> Self {
        self.max_iterations = max_iterations;
        self
    }

    /// Set initial damping parameter.
    pub fn with_lambda_init(mut self, lambda: F) -> Self {
        self.lambda_init = lambda;
        self
    }
    
    /// Set finite difference step size.
    pub fn with_fd_step(mut self, step: F) -> Self {
        self.fd_step = step;
        self
    }

    /// Compute Jacobian matrix using finite differences.
    fn compute_jacobian<Obj>(&self, objective: &Obj, params: &[F]) -> Vec<Vec<F>>
    where
        Obj: Fn(&[F]) -> F,
    {
        let n = params.len();
        let mut jacobian = vec![vec![0.0; n]; 1]; // For scalar objective, Jacobian is 1×n
        
        let f0 = objective(params);
        let mut params_plus = params.to_vec();
        
        for j in 0..n {
            let h = (params[j].abs() * self.fd_step).max(self.fd_step);
            params_plus[j] = params[j] + h;
            let f_plus = objective(&params_plus);
            jacobian[0][j] = (f_plus - f0) / h;
            params_plus[j] = params[j]; // Reset
        }
        
        jacobian
    }
    
    /// Compute gradient using analytical derivatives if available, otherwise finite differences.
    fn compute_gradient_with_analytical<Obj, D>(
        &self,
        objective: &Obj,
        params: &[F],
        derivatives: Option<&D>,
    ) -> Vec<F>
    where
        Obj: Fn(&[F]) -> F,
        D: AnalyticalDerivatives,
    {
        if let Some(deriv) = derivatives {
            if deriv.has_gradient() {
                let mut gradient = vec![0.0; params.len()];
                deriv.gradient(params, &mut gradient);
                return gradient;
            }
        }
        
        // Fall back to finite differences
        let jacobian = self.compute_jacobian(objective, params);
        jacobian[0].clone()
    }

    /// Compute Jacobian for a system of residuals.
    fn compute_jacobian_system<Res>(
        &self,
        residuals: &Res,
        params: &[F],
        n_residuals: usize,
    ) -> Vec<Vec<F>>
    where
        Res: Fn(&[F], &mut [F]),
    {
        let n_params = params.len();
        let mut jacobian = vec![vec![0.0; n_params]; n_residuals];
        
        let mut r0 = vec![0.0; n_residuals];
        residuals(params, &mut r0);
        
        let mut params_plus = params.to_vec();
        let mut r_plus = vec![0.0; n_residuals];
        
        for j in 0..n_params {
            let h = (params[j].abs() * self.fd_step).max(self.fd_step);
            params_plus[j] = params[j] + h;
            residuals(&params_plus, &mut r_plus);
            
            for i in 0..n_residuals {
                jacobian[i][j] = (r_plus[i] - r0[i]) / h;
            }
            params_plus[j] = params[j]; // Reset
        }
        
        jacobian
    }

    /// Solve the normal equations (J^T J + λI) δ = -J^T r
    fn solve_normal_equations(
        &self,
        jacobian: &[Vec<F>],
        residuals: &[F],
        lambda: F,
    ) -> Result<Vec<F>> {
        let n = jacobian[0].len(); // Number of parameters
        let m = jacobian.len();     // Number of residuals
        
        // Compute J^T J
        let mut jtj = vec![vec![0.0; n]; n];
        for (i, row) in jtj.iter_mut().enumerate().take(n) {
            for (j, entry) in row.iter_mut().enumerate().take(n) {
                for jacobian_row in jacobian.iter().take(m) {
                    *entry += jacobian_row[i] * jacobian_row[j];
                }
            }
        }
        
        // Add damping term λI
        #[allow(clippy::needless_range_loop)]
        for i in 0..n {
            jtj[i][i] += lambda;
        }
        
        // Compute -J^T r
        let mut jtr = vec![0.0; n];
        for (i, jtr_entry) in jtr.iter_mut().enumerate().take(n) {
            for jacobian_row in jacobian.iter().take(m).enumerate() {
                let (k, row) = jacobian_row;
                *jtr_entry -= row[i] * residuals[k];
            }
        }
        
        // Solve using Gaussian elimination with partial pivoting
        self.solve_linear_system(&jtj, &jtr)
    }

    /// Simple Gaussian elimination with partial pivoting.
    #[allow(clippy::needless_range_loop)]
    fn solve_linear_system(&self, a: &[Vec<F>], b: &[F]) -> Result<Vec<F>> {
        let n = a.len();
        let mut aug = a.to_vec();
        let mut x = b.to_vec();
        
        // Forward elimination with partial pivoting
        for k in 0..n {
            // Find pivot
            let mut max_idx = k;
            let mut max_val = aug[k][k].abs();
            for i in (k + 1)..n {
                if aug[i][k].abs() > max_val {
                    max_val = aug[i][k].abs();
                    max_idx = i;
                }
            }
            
            if max_val < 1e-15 {
                return Err(InputError::Invalid.into());
            }
            
            // Swap rows
            if max_idx != k {
                aug.swap(k, max_idx);
                x.swap(k, max_idx);
            }
            
            // Eliminate
            for i in (k + 1)..n {
                let factor = aug[i][k] / aug[k][k];
                for j in (k + 1)..n {
                    aug[i][j] -= factor * aug[k][j];
                }
                x[i] -= factor * x[k];
            }
        }
        
        // Back substitution
        for i in (0..n).rev() {
            for j in (i + 1)..n {
                x[i] -= aug[i][j] * x[j];
            }
            x[i] /= aug[i][i];
        }
        
        Ok(x)
    }

    /// Apply box constraints to parameters.
    fn apply_bounds(&self, params: &mut [F], bounds: Option<&[(F, F)]>) {
        if let Some(bounds) = bounds {
            for (i, (lo, hi)) in bounds.iter().enumerate().take(params.len()) {
                params[i] = params[i].clamp(*lo, *hi);
            }
        }
    }
    
    /// Minimize objective function with analytical derivatives.
    ///
    /// # Arguments
    /// * `objective` - Function to minimize
    /// * `derivatives` - Provider of analytical derivatives
    /// * `initial` - Initial parameter guess
    /// * `bounds` - Optional box constraints
    ///
    /// # Returns
    /// Optimal parameter vector
    pub fn minimize_with_derivatives<Obj, D>(
        &self,
        objective: Obj,
        derivatives: &D,
        initial: &[F],
        bounds: Option<&[(F, F)]>,
    ) -> Result<Vec<F>>
    where
        Obj: Fn(&[F]) -> F,
        D: AnalyticalDerivatives,
    {
        if initial.is_empty() {
            return Err(InputError::Invalid.into());
        }

        let mut params = initial.to_vec();
        let mut lambda = self.lambda_init;
        let mut best_value = objective(&params);
        let mut best_params = params.clone();

        for _iter in 0..self.max_iterations {
            // Use analytical gradient if available
            let gradient = self.compute_gradient_with_analytical(&objective, &params, Some(derivatives));
            
            // Check convergence
            let grad_norm: F = gradient.iter().map(|g| g * g).sum::<F>().sqrt();
            if grad_norm < self.tolerance {
                return Ok(params);
            }
            
            // For scalar objective, create Jacobian from gradient
            let jacobian = vec![gradient.clone()];
            let residual = vec![objective(&params)];
            
            // Solve for step
            let step = self.solve_normal_equations(&jacobian, &residual, lambda)?;
            
            // Check step size
            let step_norm: F = step.iter().map(|s| s * s).sum::<F>().sqrt();
            if step_norm < self.min_step_size {
                return Ok(params);
            }
            
            // Try the step
            let mut new_params = params.clone();
            for (i, &s) in step.iter().enumerate() {
                new_params[i] += s;
            }
            self.apply_bounds(&mut new_params, bounds);
            
            let new_value = objective(&new_params);
            
            // Accept or reject step
            if new_value < best_value {
                params = new_params;
                best_value = new_value;
                best_params = params.clone();
                lambda /= self.lambda_factor;
                lambda = lambda.max(1e-15);
            } else {
                lambda *= self.lambda_factor;
                lambda = lambda.min(1e15);
            }
        }
        
        Ok(best_params)
    }
    
    /// Solve system with analytical Jacobian.
    pub fn solve_system_with_jacobian<Res, D>(
        &self,
        residuals: Res,
        derivatives: &D,
        initial: &[F],
    ) -> Result<Vec<F>>
    where
        Res: Fn(&[F], &mut [F]),
        D: AnalyticalDerivatives,
    {
        if initial.is_empty() {
            return Err(InputError::Invalid.into());
        }

        // Determine number of residuals
        let mut test_residuals = vec![0.0; initial.len() * 2];
        residuals(initial, &mut test_residuals);
        let n_residuals = test_residuals.iter().position(|&r| r == 0.0).unwrap_or(test_residuals.len());
        let mut resid_vec = vec![0.0; n_residuals];

        let mut params = initial.to_vec();
        let mut lambda = self.lambda_init;

        for _iter in 0..self.max_iterations {
            // Compute residuals
            residuals(&params, &mut resid_vec);
            
            // Use analytical Jacobian if available
            let jacobian = if derivatives.has_jacobian() {
                let mut jac = vec![vec![0.0; params.len()]; n_residuals];
                if derivatives.jacobian(&params, &mut jac).is_some() {
                    jac
                } else {
                    // Fall back to finite differences
                    self.compute_jacobian_system(&residuals, &params, n_residuals)
                }
            } else {
                self.compute_jacobian_system(&residuals, &params, n_residuals)
            };
            
            // Check convergence
            let resid_norm: F = resid_vec.iter().map(|r| r * r).sum::<F>().sqrt();
            if resid_norm < self.tolerance {
                return Ok(params);
            }
            
            // Solve for step
            let step = self.solve_normal_equations(&jacobian, &resid_vec, lambda)?;
            
            // Check step size
            let step_norm: F = step.iter().map(|s| s * s).sum::<F>().sqrt();
            if step_norm < self.min_step_size {
                return Ok(params);
            }
            
            // Try the step
            let mut new_params = params.clone();
            for (i, &s) in step.iter().enumerate() {
                new_params[i] += s;
            }
            
            // Evaluate new residuals
            let mut new_resid = vec![0.0; n_residuals];
            residuals(&new_params, &mut new_resid);
            let new_norm: F = new_resid.iter().map(|r| r * r).sum::<F>().sqrt();
            
            // Accept or reject
            if new_norm < resid_norm {
                params = new_params;
                lambda /= self.lambda_factor;
                lambda = lambda.max(1e-15);
            } else {
                lambda *= self.lambda_factor;
                lambda = lambda.min(1e15);
            }
        }
        
        Ok(params)
    }
}

impl MultiSolver for LevenbergMarquardtSolver {
    fn minimize<Obj>(
        &self,
        objective: Obj,
        initial: &[F],
        bounds: Option<&[(F, F)]>,
    ) -> Result<Vec<F>>
    where
        Obj: Fn(&[F]) -> F,
    {
        if initial.is_empty() {
            return Err(InputError::Invalid.into());
        }

        let mut params = initial.to_vec();
        let mut lambda = self.lambda_init;
        let mut best_value = objective(&params);
        let mut best_params = params.clone();

        for _iter in 0..self.max_iterations {
            // Compute gradient (Jacobian for scalar function)
            let jacobian = self.compute_jacobian(&objective, &params);
            let gradient: Vec<F> = jacobian[0].clone();
            
            // Check convergence
            let grad_norm: F = gradient.iter().map(|g| g * g).sum::<F>().sqrt();
            if grad_norm < self.tolerance {
                return Ok(params);
            }
            
            // For scalar objective, residual is just the objective value
            let residual = vec![objective(&params)];
            
            // Solve for step
            let step = self.solve_normal_equations(&jacobian, &residual, lambda)?;
            
            // Check step size
            let step_norm: F = step.iter().map(|s| s * s).sum::<F>().sqrt();
            if step_norm < self.min_step_size {
                return Ok(params);
            }
            
            // Try the step
            let mut new_params = params.clone();
            for (i, &s) in step.iter().enumerate() {
                new_params[i] += s;
            }
            self.apply_bounds(&mut new_params, bounds);
            
            let new_value = objective(&new_params);
            
            // Accept or reject step
            if new_value < best_value {
                // Accept: decrease lambda (more Newton-like)
                params = new_params;
                best_value = new_value;
                best_params = params.clone();
                lambda /= self.lambda_factor;
                lambda = lambda.max(1e-15);
            } else {
                // Reject: increase lambda (more gradient descent-like)
                lambda *= self.lambda_factor;
                lambda = lambda.min(1e15);
            }
        }
        
        Ok(best_params)
    }

    fn solve_system<Res>(&self, residuals: Res, initial: &[F]) -> Result<Vec<F>>
    where
        Res: Fn(&[F], &mut [F]),
    {
        if initial.is_empty() {
            return Err(InputError::Invalid.into());
        }

        // Determine number of residuals
        let mut test_residuals = vec![0.0; initial.len() * 2]; // Assume at most 2x params
        residuals(initial, &mut test_residuals);
        let n_residuals = test_residuals.iter().position(|&r| r == 0.0).unwrap_or(test_residuals.len());
        let mut resid_vec = vec![0.0; n_residuals];

        let mut params = initial.to_vec();
        let mut lambda = self.lambda_init;

        for _iter in 0..self.max_iterations {
            // Compute residuals and Jacobian
            residuals(&params, &mut resid_vec);
            let jacobian = self.compute_jacobian_system(&residuals, &params, n_residuals);
            
            // Check convergence
            let resid_norm: F = resid_vec.iter().map(|r| r * r).sum::<F>().sqrt();
            if resid_norm < self.tolerance {
                return Ok(params);
            }
            
            // Solve for step
            let step = self.solve_normal_equations(&jacobian, &resid_vec, lambda)?;
            
            // Check step size
            let step_norm: F = step.iter().map(|s| s * s).sum::<F>().sqrt();
            if step_norm < self.min_step_size {
                return Ok(params);
            }
            
            // Try the step
            let mut new_params = params.clone();
            for (i, &s) in step.iter().enumerate() {
                new_params[i] += s;
            }
            
            // Evaluate new residuals
            let mut new_resid = vec![0.0; n_residuals];
            residuals(&new_params, &mut new_resid);
            let new_norm: F = new_resid.iter().map(|r| r * r).sum::<F>().sqrt();
            
            // Accept or reject
            if new_norm < resid_norm {
                params = new_params;
                lambda /= self.lambda_factor;
                lambda = lambda.max(1e-15);
            } else {
                lambda *= self.lambda_factor;
                lambda = lambda.min(1e15);
            }
        }
        
        Ok(params)
    }
}

/// Differential Evolution solver for global optimization.
///
/// DE is a stochastic population-based optimization algorithm that doesn't
/// require gradient information, making it suitable for non-smooth or
/// multi-modal objective functions.
///
/// This implementation uses the DE/rand/1/bin strategy, which is robust
/// and widely applicable.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DifferentialEvolutionSolver {
    /// Population size (number of candidate solutions)
    pub population_size: usize,
    /// Maximum number of generations
    pub max_generations: usize,
    /// Mutation factor F (typically 0.5-2.0)
    pub mutation_factor: F,
    /// Crossover probability CR (typically 0.1-1.0)
    pub crossover_prob: F,
    /// Convergence tolerance
    pub tolerance: F,
    /// Random seed for reproducibility
    pub seed: Option<u64>,
}

impl Default for DifferentialEvolutionSolver {
    fn default() -> Self {
        Self {
            population_size: 50,
            max_generations: 200,
            mutation_factor: 0.8,
            crossover_prob: 0.9,
            tolerance: 1e-6,
            seed: None,
        }
    }
}

impl DifferentialEvolutionSolver {
    /// Create a new Differential Evolution solver with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set population size (recommended: 10 * number of parameters).
    pub fn with_population_size(mut self, size: usize) -> Self {
        self.population_size = size;
        self
    }

    /// Set maximum generations.
    pub fn with_max_generations(mut self, generations: usize) -> Self {
        self.max_generations = generations;
        self
    }

    /// Set mutation factor F.
    pub fn with_mutation_factor(mut self, factor: F) -> Self {
        self.mutation_factor = factor;
        self
    }

    /// Set crossover probability.
    pub fn with_crossover_prob(mut self, prob: F) -> Self {
        self.crossover_prob = prob;
        self
    }

    /// Set convergence tolerance.
    pub fn with_tolerance(mut self, tolerance: F) -> Self {
        self.tolerance = tolerance;
        self
    }

    /// Set random seed for reproducibility.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Initialize population within bounds.
    fn initialize_population(
        &self,
        n_params: usize,
        bounds: Option<&[(F, F)]>,
        rng: &mut crate::math::random::SimpleRng,
    ) -> Vec<Vec<F>> {
        let mut population = Vec::with_capacity(self.population_size);
        
        for _ in 0..self.population_size {
            let mut individual = Vec::with_capacity(n_params);
            for i in 0..n_params {
                let value = if let Some(bounds) = bounds {
                    if i < bounds.len() {
                        let (lo, hi) = bounds[i];
                        lo + rng.uniform() * (hi - lo)
                    } else {
                        // No bounds for this parameter
                        rng.uniform() * 10.0 - 5.0 // Default range [-5, 5]
                    }
                } else {
                    rng.uniform() * 10.0 - 5.0
                };
                individual.push(value);
            }
            population.push(individual);
        }
        
        population
    }

    /// Apply bounds to a candidate solution.
    fn apply_bounds(&self, candidate: &mut [F], bounds: Option<&[(F, F)]>) {
        if let Some(bounds) = bounds {
            for (i, value) in candidate.iter_mut().enumerate() {
                if i < bounds.len() {
                    let (lo, hi) = bounds[i];
                    *value = value.clamp(lo, hi);
                }
            }
        }
    }

    /// Perform mutation and crossover to create trial vector.
    fn mutate_and_crossover(
        &self,
        population: &[Vec<F>],
        target_idx: usize,
        rng: &mut crate::math::random::SimpleRng,
    ) -> Vec<F> {
        let n = population.len();
        let dim = population[0].len();
        
        // Select three distinct random individuals (not target)
        let mut indices = Vec::new();
        while indices.len() < 3 {
            let idx = (rng.uniform() * n as F) as usize % n;
            if idx != target_idx && !indices.contains(&idx) {
                indices.push(idx);
            }
        }
        
        let r1 = indices[0];
        let r2 = indices[1];
        let r3 = indices[2];
        
        // Create trial vector using DE/rand/1/bin strategy
        let mut trial = vec![0.0; dim];
        let j_rand = (rng.uniform() * dim as F) as usize % dim; // Ensure at least one parameter from mutant
        
        for (j, trial_entry) in trial.iter_mut().enumerate().take(dim) {
            if rng.uniform() < self.crossover_prob || j == j_rand {
                // Mutation: v = x_r1 + F * (x_r2 - x_r3)
                *trial_entry = population[r1][j] + self.mutation_factor * (population[r2][j] - population[r3][j]);
            } else {
                // Keep original
                *trial_entry = population[target_idx][j];
            }
        }
        
        trial
    }
}

impl MultiSolver for DifferentialEvolutionSolver {
    fn minimize<Obj>(
        &self,
        objective: Obj,
        initial: &[F],
        bounds: Option<&[(F, F)]>,
    ) -> Result<Vec<F>>
    where
        Obj: Fn(&[F]) -> F,
    {
        use crate::math::random::SimpleRng;
        
        if initial.is_empty() {
            return Err(InputError::Invalid.into());
        }

        let n_params = initial.len();
        let mut rng = SimpleRng::new(self.seed.unwrap_or(42));
        
        // Initialize population
        let mut population = self.initialize_population(n_params, bounds, &mut rng);
        
        // Include initial guess in population
        population[0] = initial.to_vec();
        self.apply_bounds(&mut population[0], bounds);
        
        // Evaluate initial population
        let mut fitness: Vec<F> = population.iter().map(|ind| objective(ind)).collect();
        
        // Find best individual
        let mut best_idx = 0;
        let mut best_fitness = fitness[0];
        for (i, &f) in fitness.iter().enumerate() {
            if f < best_fitness {
                best_fitness = f;
                best_idx = i;
            }
        }
        
        // Evolution loop
        for _gen in 0..self.max_generations {
            let mut new_population = population.clone();
            let mut new_fitness = fitness.clone();
            
            for i in 0..self.population_size {
                // Create trial vector
                let mut trial = self.mutate_and_crossover(&population, i, &mut rng);
                self.apply_bounds(&mut trial, bounds);
                
                // Evaluate trial
                let trial_fitness = objective(&trial);
                
                // Selection: keep better solution
                if trial_fitness < fitness[i] {
                    new_population[i] = trial;
                    new_fitness[i] = trial_fitness;
                    
                    // Update global best
                    if trial_fitness < best_fitness {
                        best_fitness = trial_fitness;
                        best_idx = i;
                    }
                }
            }
            
            population = new_population;
            fitness = new_fitness;
            
            // Check convergence (population diversity)
            let fitness_std: F = {
                let mean = fitness.iter().sum::<F>() / fitness.len() as F;
                let variance = fitness.iter().map(|f| (f - mean).powi(2)).sum::<F>() / fitness.len() as F;
                variance.sqrt()
            };
            
            if fitness_std < self.tolerance {
                break;
            }
        }
        
        Ok(population[best_idx].clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_levenberg_marquardt_simple_minimum() {
        let solver = LevenbergMarquardtSolver::new().with_tolerance(1e-10);
        
        // Minimize (x-2)^2 + (y-3)^2
        let objective = |params: &[F]| -> F {
            (params[0] - 2.0).powi(2) + (params[1] - 3.0).powi(2)
        };
        
        let initial = vec![0.0, 0.0];
        let result = solver.minimize(objective, &initial, None).unwrap();
        
        assert!((result[0] - 2.0).abs() < 1e-6);
        assert!((result[1] - 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_levenberg_marquardt_with_bounds() {
        let solver = LevenbergMarquardtSolver::new();
        
        // Minimize (x-5)^2 + (y-5)^2 with bounds
        let objective = |params: &[F]| -> F {
            (params[0] - 5.0).powi(2) + (params[1] - 5.0).powi(2)
        };
        
        let initial = vec![0.0, 0.0];
        let bounds = vec![(0.0, 3.0), (0.0, 3.0)]; // Constrain solution
        let result = solver.minimize(objective, &initial, Some(&bounds)).unwrap();
        
        // Solution should be at boundary (3, 3)
        assert!((result[0] - 3.0).abs() < 1e-6);
        assert!((result[1] - 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_levenberg_marquardt_system() {
        let solver = LevenbergMarquardtSolver::new().with_tolerance(1e-8);
        
        // Solve system: x + y = 5, x - y = 1 (solution: x=3, y=2)
        let residuals = |params: &[F], resid: &mut [F]| {
            resid[0] = params[0] + params[1] - 5.0;
            resid[1] = params[0] - params[1] - 1.0;
        };
        
        let initial = vec![0.0, 0.0];
        let result = solver.solve_system(residuals, &initial).unwrap();
        
        assert!((result[0] - 3.0).abs() < 1e-6);
        assert!((result[1] - 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_least_squares_fitting() {
        let solver = LevenbergMarquardtSolver::new()
            .with_tolerance(1e-8)
            .with_max_iterations(100);
        
        // Fit a quadratic y = a*x^2 + b*x + c to some data points
        // True parameters: a=1, b=-2, c=3
        let x_data = [-2.0, -1.0, 0.0, 1.0, 2.0];
        let y_data = [11.0, 6.0, 3.0, 2.0, 3.0]; // y = x^2 - 2*x + 3
        
        // Least squares objective
        let objective = move |params: &[F]| -> F {
            let a = params[0];
            let b = params[1];
            let c = params[2];
            
            x_data.iter().zip(y_data.iter())
                .map(|(&x, &y_true)| {
                    let y_pred = a * x * x + b * x + c;
                    (y_pred - y_true).powi(2)
                })
                .sum()
        };
        
        let initial = vec![0.5, 0.5, 0.5];
        let result = solver.minimize(objective, &initial, None).unwrap();
        
        // Should recover the true parameters (within reasonable tolerance)
        assert!((result[0] - 1.0).abs() < 0.01, "a = {}, expected 1.0", result[0]);
        assert!((result[1] - (-2.0)).abs() < 0.01, "b = {}, expected -2.0", result[1]);
        assert!((result[2] - 3.0).abs() < 0.01, "c = {}, expected 3.0", result[2]);
    }

    #[test]
    fn test_differential_evolution_simple() {
        let solver = DifferentialEvolutionSolver::new()
            .with_population_size(20)
            .with_max_generations(50)
            .with_seed(42) // For reproducibility
            .with_tolerance(1e-6);
        
        // Minimize simple quadratic (x-2)^2 + (y-3)^2
        let objective = |params: &[F]| -> F {
            (params[0] - 2.0).powi(2) + (params[1] - 3.0).powi(2)
        };
        
        let initial = vec![0.0, 0.0];
        let result = solver.minimize(objective, &initial, None).unwrap();
        
        assert!((result[0] - 2.0).abs() < 0.01, "x = {}, expected 2.0", result[0]);
        assert!((result[1] - 3.0).abs() < 0.01, "y = {}, expected 3.0", result[1]);
    }

    #[test]
    fn test_differential_evolution_with_bounds() {
        let solver = DifferentialEvolutionSolver::new()
            .with_population_size(30)
            .with_max_generations(100)
            .with_seed(123)
            .with_tolerance(1e-6);
        
        // Minimize Rastrigin function (multi-modal test function)
        // f(x) = 10n + Σ[x_i^2 - 10*cos(2π*x_i)]
        let objective = |params: &[F]| -> F {
            let n = params.len() as F;
            let pi2 = 2.0 * std::f64::consts::PI;
            10.0 * n + params.iter()
                .map(|&x| x * x - 10.0 * (pi2 * x).cos())
                .sum::<F>()
        };
        
        let initial = vec![2.5, 2.5];
        let bounds = vec![(-5.12, 5.12), (-5.12, 5.12)];
        let result = solver.minimize(objective, &initial, Some(&bounds)).unwrap();
        
        // Global minimum is at (0, 0) with value 0
        // DE should get reasonably close
        assert!(result[0].abs() < 0.1, "x = {}, expected ~0.0", result[0]);
        assert!(result[1].abs() < 0.1, "y = {}, expected ~0.0", result[1]);
    }

    #[test]
    fn test_differential_evolution_deterministic() {
        // Test that with fixed seed, we get reproducible results
        let solver1 = DifferentialEvolutionSolver::new()
            .with_population_size(10)
            .with_max_generations(20)
            .with_seed(999);
        
        let solver2 = DifferentialEvolutionSolver::new()
            .with_population_size(10)
            .with_max_generations(20)
            .with_seed(999);
        
        let objective = |params: &[F]| -> F {
            params.iter().map(|x| x * x).sum()
        };
        
        let initial = vec![1.0, 2.0, 3.0];
        let result1 = solver1.minimize(objective, &initial, None).unwrap();
        let result2 = solver2.minimize(objective, &initial, None).unwrap();
        
        // Results should be identical with same seed
        for i in 0..3 {
            assert_eq!(result1[i], result2[i], "Results differ at index {}", i);
        }
    }
    
    #[test]
    fn test_analytical_derivatives_simple() {
        // Simple quadratic with analytical derivatives
        struct QuadraticDerivatives;
        
        impl AnalyticalDerivatives for QuadraticDerivatives {
            fn gradient(&self, params: &[F], gradient: &mut [F]) {
                // f(x,y) = (x-2)^2 + (y-3)^2
                gradient[0] = 2.0 * (params[0] - 2.0);
                gradient[1] = 2.0 * (params[1] - 3.0);
            }
        }
        
        let solver = LevenbergMarquardtSolver::new()
            .with_tolerance(1e-10);
        
        let objective = |params: &[F]| -> F {
            (params[0] - 2.0).powi(2) + (params[1] - 3.0).powi(2)
        };
        
        let derivatives = QuadraticDerivatives;
        let initial = vec![0.0, 0.0];
        
        let result = solver.minimize_with_derivatives(
            objective, 
            &derivatives, 
            &initial, 
            None
        ).unwrap();
        
        assert!((result[0] - 2.0).abs() < 1e-8);
        assert!((result[1] - 3.0).abs() < 1e-8);
    }
    
    #[test]
    fn test_jacobian_system() {
        // System: x^2 + y^2 = 4, x - y = 0
        struct CircleLineJacobian;
        
        impl AnalyticalDerivatives for CircleLineJacobian {
            fn gradient(&self, _params: &[F], _gradient: &mut [F]) {
                // Not used for this test
            }
            
            fn jacobian(&self, params: &[F], jacobian: &mut [Vec<F>]) -> Option<()> {
                let x = params[0];
                let y = params[1];
                
                jacobian[0][0] = 2.0 * x;  // df1/dx
                jacobian[0][1] = 2.0 * y;  // df1/dy
                jacobian[1][0] = 1.0;      // df2/dx
                jacobian[1][1] = -1.0;     // df2/dy
                
                Some(())
            }
            
            fn has_jacobian(&self) -> bool {
                true
            }
            
            fn has_gradient(&self) -> bool {
                false
            }
        }
        
        let solver = LevenbergMarquardtSolver::new()
            .with_tolerance(1e-10);
        
        let residuals = |params: &[F], resid: &mut [F]| {
            resid[0] = params[0] * params[0] + params[1] * params[1] - 4.0;
            resid[1] = params[0] - params[1];
        };
        
        let derivatives = CircleLineJacobian;
        let initial = vec![1.0, 0.0];
        
        let result = solver.solve_system_with_jacobian(
            residuals, 
            &derivatives, 
            &initial
        ).unwrap();
        
        let expected = 2.0_f64.sqrt();
        assert!((result[0] - expected).abs() < 1e-8);
        assert!((result[1] - expected).abs() < 1e-8);
    }
}
