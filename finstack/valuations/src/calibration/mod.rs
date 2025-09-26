//! Comprehensive calibration framework for term structures and surfaces.
//!
//! Provides market-standard calibration methodologies for:
//! - Interest rate curves (discount/forward)
//! - Credit curves (survival/hazard)
//! - Inflation curves
//! - Volatility surfaces
//! - Base correlation curves
//!
//! Supports both sequential bootstrapping and global optimization approaches.
//!
//! ## Multi-Curve Framework
//!
//! The calibration framework follows the post-2008 multi-curve methodology where
//! discount and forward curves are calibrated separately to capture basis spreads.
//!
//! ```ignore
//! use finstack_valuations::calibration::{CalibrationConfig, MultiCurveConfig};
//! use finstack_valuations::calibration::methods::{
//!     DiscountCurveCalibrator, ForwardCurveCalibrator
//! };
//!
//! let config = CalibrationConfig::default()
//!     .with_multi_curve_config(MultiCurveConfig::new());
//!
//! // Step 1: Calibrate OIS discount curve using deposits and OIS swaps
//! let ois_quotes = vec![/* deposits and OIS swaps */];
//! let disc_calibrator = DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD)
//!     .with_config(config.clone());
//! let (discount_curve, _) = disc_calibrator.calibrate(&ois_quotes, &context)?;
//!
//! // Step 2: Add discount curve to context
//! let context = context.insert_discount(discount_curve);
//!
//! // Step 3: Calibrate forward curves using FRAs, futures, and tenor-specific swaps
//! let libor_quotes = vec![/* FRAs, futures, and LIBOR swaps */];
//! let fwd_calibrator = ForwardCurveCalibrator::new(
//!     "USD-SOFR-3M-FWD", 0.25, base_date, Currency::USD, "USD-OIS"
//! );
//! let (forward_curve, _) = fwd_calibrator.calibrate(&libor_quotes, &context)?;
//! ```
//!
//! ### Multi-Curve Calibration Notes
//!
//! 1. **Instrument Selection**:
//!    - For discount curve: Use deposits and OIS swaps (instruments that don't require forward curves)
//!    - For forward curves: Use FRAs, futures, and tenor-specific swaps
//!
//! 2. **Calibration Order**:
//!    - Always calibrate discount curve first
//!    - Then calibrate forward curves with discount curve in context

use finstack_core::F;

// Submodules
mod config;
pub mod derivatives;
pub mod methods;
mod quote;
mod report;
pub mod simple_calibration;
mod traits;
mod validation;

// Re-exports
pub use config::{CalibrationConfig, MultiCurveConfig, SolverKind};
pub use quote::{CreditQuote, FutureSpecs, InflationQuote, MarketQuote, RatesQuote, VolQuote};
pub use report::CalibrationReport;
pub use simple_calibration::SimpleCalibration;
pub use traits::Calibrator;
pub use validation::{CurveValidator, SurfaceValidator, ValidationConfig, ValidationError};

/// Finite penalty value used in objective functions instead of infinity.
/// Using a large finite value helps solvers behave more predictably and
/// documents intent while keeping diagnostics reasonable.
pub const PENALTY: F = 1e12;

pub fn penalize() -> F {
    PENALTY
}

// ------------------------- Solver Helper -------------------------
use finstack_core::Result;

/// Solve a 1D root-finding problem using the configured solver kind.
///
/// This replaces the former `with_solver!` macro with a plain helper function
/// to make control flow explicit and IDE-friendly.
pub fn solve_1d<Fun>(kind: SolverKind, tol: F, iters: usize, f: Fun, init: F) -> Result<F>
where
    Fun: Fn(F) -> F,
{
    use finstack_core::math::{BrentSolver, HybridSolver, NewtonSolver, Solver};

    match kind {
        SolverKind::Newton => {
            let solver = NewtonSolver::new()
                .with_tolerance(tol)
                .with_max_iterations(iters);
            solver.solve(f, init)
        }
        SolverKind::Brent => {
            let solver = BrentSolver::new().with_tolerance(tol);
            // BrentSolver currently does not expose a max-iteration builder; keep defaults
            solver.solve(f, init)
        }
        SolverKind::Hybrid => {
            let solver = HybridSolver::new()
                .with_tolerance(tol)
                .with_max_iterations(iters);
            solver.solve(f, init)
        }
        // For multi-dimensional kinds, fall back to Hybrid for 1D problems
        SolverKind::LevenbergMarquardt | SolverKind::DifferentialEvolution => {
            let solver = HybridSolver::new()
                .with_tolerance(tol)
                .with_max_iterations(iters);
            solver.solve(f, init)
        }
    }
}

/// Create a simple solver wrapper for calibration methods using `solve_1d` internally.
pub fn create_simple_solver(config: &CalibrationConfig) -> impl finstack_core::math::Solver {
    struct SimpleSolver {
        kind: SolverKind,
        tolerance: F,
        max_iterations: usize,
    }

    impl finstack_core::math::Solver for SimpleSolver {
        fn solve<Fun>(&self, f: Fun, initial_guess: F) -> finstack_core::Result<F>
        where
            Fun: Fn(F) -> F,
        {
            solve_1d(
                self.kind.clone(),
                self.tolerance,
                self.max_iterations,
                f,
                initial_guess,
            )
        }
    }

    SimpleSolver {
        kind: config.solver_kind.clone(),
        tolerance: config.tolerance,
        max_iterations: config.max_iterations,
    }
}
