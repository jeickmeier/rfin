//! Numerical helpers: root finding, summation, statistics, distributions, and mathematical functions.
//!
//! The implementations avoid heap allocation where possible. When the
//! `deterministic` feature is enabled, functions prefer numerically stable,
//! order-preserving algorithms.
//!
//! # Root Finding
//!
//! The `solver` module provides multiple root-finding algorithms:
//! - `NewtonSolver`: Fast convergence, supports analytic derivatives via `solve_with_derivative`
//! - `BrentSolver`: Robust bracketing method, guaranteed convergence
//!
//! **Performance Tip:** When analytic derivatives are available (e.g., for XIRR, implied volatility),
//! use `NewtonSolver::solve_with_derivative` for 2× fewer function evaluations and better numerical stability.
//!
//! # Examples
//!
//! ## Root finding with finite differences
//!
//! ```rust
//! use finstack_core::math::{Solver, mean, variance};
//! use finstack_core::math::solver::NewtonSolver;
//! # fn main() -> finstack_core::Result<()> {
//!
//! let solver = NewtonSolver::new();
//! let root = solver.solve(|x| x * x - 2.0, 1.0)?;
//! assert!((root - 2f64.sqrt()).abs() < 1e-9);
//! # Ok(())
//! # }
//! ```
//!
//! ## Root finding with analytic derivatives (recommended when available)
//!
//! ```rust
//! use finstack_core::math::solver::NewtonSolver;
//! # fn main() -> finstack_core::Result<()> {
//!
//! let solver = NewtonSolver::new();
//! let f = |x: f64| x * x - 2.0;
//! let f_prime = |x: f64| 2.0 * x;  // Analytic derivative
//!
//! let root = solver.solve_with_derivative(f, f_prime, 1.0)?;
//! assert!((root - 2f64.sqrt()).abs() < 1e-10);
//! # Ok(())
//! # }
//! ```
//!
//! ## Basic statistics
//!
//! ```rust
//! use finstack_core::math::{mean, variance};
//!
//! let data = [1.0, 2.0, 3.0, 4.0];
//! assert_eq!(mean(&data), 2.5);
//! assert_eq!(variance(&data), 1.25);
//! ```

pub mod distributions;
pub mod integration;
pub mod interp;
pub mod linalg;
pub mod probability;
pub mod random;
pub mod solver;
pub mod solver_multi;
pub mod special_functions;
pub mod stats;
pub mod summation;
pub mod time_grid;
pub mod volatility;

// Re-exports for ergonomic access
pub use distributions::{
    binomial_distribution, binomial_probability, log_binomial_coefficient, log_factorial,
    sample_beta,
};
pub use integration::{
    adaptive_simpson, gauss_legendre_integrate, gauss_legendre_integrate_adaptive,
    gauss_legendre_integrate_composite, simpson_rule, trapezoidal_rule, GaussHermiteQuadrature,
};
pub use interp::{
    CubicHermite, ExtrapolationPolicy, InterpFn, LinearDf, LogLinearDf, MonotoneConvex,
};
pub use linalg::{
    apply_correlation, build_correlation_matrix, cholesky_decomposition,
    validate_correlation_matrix, CholeskyError,
};
pub use probability::{correlation_bounds, joint_probabilities, CorrelatedBernoulli};
pub use random::{box_muller_transform, RandomNumberGenerator, TestRng};
pub use random::sobol::{SobolRng, MAX_SOBOL_DIMENSION};
// Raw root finding functions are no longer exported - use trait-based solvers instead
pub use solver::{BrentSolver, NewtonSolver, Solver};
pub use solver_multi::{AnalyticalDerivatives, LevenbergMarquardtSolver, MultiSolver};
pub use special_functions::{
    erf, norm_cdf, norm_pdf, standard_normal_inv_cdf, student_t_cdf, student_t_inv_cdf,
};
pub use stats::{
    correlation, covariance, mean, mean_var, moment_match, required_samples, variance,
    OnlineCovariance, OnlineStats,
};
pub use summation::{
    kahan_sum, neumaier_sum, pairwise_sum, stable_sum, KahanAccumulator, NeumaierAccumulator,
};
pub use time_grid::{
    map_date_to_step, map_dates_to_steps, map_exercise_dates_to_steps, TimeGrid, TimeGridError,
};
