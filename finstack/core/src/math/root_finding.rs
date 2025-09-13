//! Root-finding algorithms: Brent, Newton-Raphson, and safeguarded Newton.
//!
//! **Note**: This module contains the raw root-finding implementations.
//! For general use, prefer the trait-based solvers in [`crate::math::solver`].
//!
//! # Examples
//!
//! Find the root of a quadratic with BrentSolver:
//!
//! ```
//! use finstack_core::math::solver::{BrentSolver, Solver};
//! let f = |x: f64| x * x - 2.0;
//! let solver = BrentSolver::new().with_tolerance(1e-12);
//! let r = solver.solve(f, 1.5).unwrap();
//! assert!((r - 2.0_f64.sqrt()).abs() < 1e-9);
//! ```
//!
//! Use HybridSolver for Newton-Raphson with Brent fallback:
//! ```
//! use finstack_core::math::solver::{HybridSolver, Solver};
//! let f = |x: f64| x * x * x - x;
//! let solver = HybridSolver::new().with_tolerance(1e-12);
//! let r = solver.solve(f, 0.85).unwrap();
//! assert!((r - 1.0).abs() < 1e-9);
//! ```

// This module intentionally left minimal - use trait-based solvers instead
