//! Numerical helpers: root finding, summation, statistics, distributions, and mathematical functions.
//!
//! The implementations avoid heap allocation where possible. When the
//! `deterministic` feature is enabled, functions prefer numerically stable,
//! order-preserving algorithms.
//!
//! # Examples
//! ```rust
//! use finstack_core::math::{Solver, mean, variance};
//! use finstack_core::math::solver::NewtonSolver;
//!
//! // Root finding with Brent's method
//! let solver = NewtonSolver::new();
//! let root = solver.solve(|x| x * x - 2.0, 1.0).unwrap();
//! assert!((root - 2f64.sqrt()).abs() < 1e-9);
//!
//! // Basic statistics helpers
//! let data = [1.0, 2.0, 3.0, 4.0];
//! assert_eq!(mean(&data), 2.5);
//! assert_eq!(variance(&data), 1.25);
//! ```

pub mod distributions;
pub mod integration;
pub mod interp;
pub mod random;
pub mod root_finding;
pub mod solver;
pub mod solver_multi;
pub mod special_functions;
pub mod stats;
pub mod summation;

// Re-exports for ergonomic access
pub use distributions::{
    binomial_probability, log_binomial_coefficient, log_factorial, sample_beta,
};
pub use integration::{
    adaptive_quadrature, simpson_rule, trapezoidal_rule, GaussHermiteQuadrature,
    gauss_legendre_integrate, gauss_legendre_integrate_adaptive, gauss_legendre_integrate_composite,
    adaptive_simpson,
};
pub use interp::{
    CubicHermite, ExtrapolationPolicy, FlatFwd, InterpFn, LinearDf, LogLinearDf, MonotoneConvex,
};
pub use random::{RandomNumberGenerator, SimpleRng};
// Raw root finding functions are no longer exported - use trait-based solvers instead
pub use solver::{BrentSolver, HybridSolver, NewtonSolver, Solver};
pub use solver_multi::{
    AnalyticalDerivatives, DifferentialEvolutionSolver, LevenbergMarquardtSolver, MultiSolver,
};
pub use special_functions::{
    erf, norm_cdf, norm_pdf, standard_normal_cdf, standard_normal_inv_cdf,
};
pub use stats::{correlation, covariance, mean, mean_var, variance};
pub use summation::{kahan_sum, pairwise_sum, stable_sum};
