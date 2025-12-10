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

// Submodules
mod config;
pub mod derivatives;
pub mod methods;
mod quote;
mod report;
mod solver_config;
pub mod spec;
mod traits;
mod validation;

// Re-exports
pub use config::{CalibrationConfig, MultiCurveConfig, RateBounds, SolverKind, ValidationMode};
pub use derivatives::sabr_derivatives::{SABRCalibrationDerivatives, SABRMarketData};
pub use derivatives::sabr_model_params::SABRModelParams;
pub use quote::{CreditQuote, FutureSpecs, InflationQuote, MarketQuote, RatesQuote, VolQuote};
pub use report::CalibrationReport;
pub use solver_config::SolverConfig;
pub use spec::{
    CalibrationEnvelope, CalibrationResult, CalibrationResultEnvelope, CalibrationSpec,
    CalibrationStep, CALIBRATION_SCHEMA_V1,
};
pub use traits::Calibrator;
pub use validation::{CurveValidator, SurfaceValidator, ValidationConfig, ValidationError};

// Re-export test helpers for swap repricing
pub use methods::create_ois_swap_from_quote;

/// Finite penalty value used in objective functions instead of infinity.
///
/// Using a moderate large finite value (1e6) helps solvers behave more predictably
/// than extremely large values like 1e12, which can cause numerical instability
/// with gradient-based methods. The value is chosen to be:
/// - Large enough to clearly indicate failure/infeasibility
/// - Small enough to avoid gradient explosion issues
/// - Proportional to typical financial quantities (notional-normalized PVs)
pub const PENALTY: f64 = 1e6;

// ------------------------- Solver Helper -------------------------
use finstack_core::Result;

/// Solve a 1D root-finding problem using the configured solver kind.
///
/// This replaces the former `with_solver!` macro with a plain helper function
/// to make control flow explicit and IDE-friendly.
pub fn solve_1d<Fun>(kind: SolverKind, tol: f64, iters: usize, f: Fun, init: f64) -> Result<f64>
where
    Fun: Fn(f64) -> f64,
{
    use finstack_core::math::{BrentSolver, NewtonSolver, Solver};

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
        // For multi-dimensional kinds, fall back to Brent for 1D problems
        SolverKind::LevenbergMarquardt => {
            let solver = BrentSolver::new().with_tolerance(tol);
            solver.solve(f, init)
        }
    }
}

/// Scan a set of points to bracket a root, then refine with the configured 1D solver.
/// Returns Ok(Some(root)) when a bracket is found and solved; Ok(None) if no bracket was found.
pub(crate) fn bracket_solve_1d(
    objective: &dyn Fn(f64) -> f64,
    initial: f64,
    scan_points: &[f64],
    tol: f64,
    max_iters: usize,
) -> Result<Option<f64>> {
    let v0 = objective(initial);
    if v0.is_finite() && v0.abs() < tol {
        return Ok(Some(initial));
    }

    let mut last_valid: Option<(f64, f64)> = None;
    for &point in scan_points {
        let value = objective(point);
        if !value.is_finite() || value.abs() >= crate::calibration::PENALTY / 10.0 {
            continue;
        }

        if let Some((prev_point, prev_value)) = last_valid {
            if prev_value == 0.0 {
                return Ok(Some(prev_point));
            }
            if value == 0.0 {
                return Ok(Some(point));
            }
            if prev_value.signum() != value.signum() {
                let guess = (prev_point + point) * 0.5;
                let root = solve_1d(SolverKind::Brent, tol, max_iters.max(50), objective, guess)?;
                return Ok(Some(root));
            }
        }

        last_valid = Some((point, value));
    }

    Ok(None)
}

/// Create a simple solver wrapper for calibration methods using `solve_1d` internally.
pub fn create_simple_solver(config: &CalibrationConfig) -> impl finstack_core::math::Solver {
    struct SimpleSolver {
        kind: SolverKind,
        tolerance: f64,
        max_iterations: usize,
    }

    impl finstack_core::math::Solver for SimpleSolver {
        fn solve<Fun>(&self, f: Fun, initial_guess: f64) -> finstack_core::Result<f64>
        where
            Fun: Fn(f64) -> f64,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bracket_solve_1d_finds_root() {
        // f(x) = x - 0.5 has root at 0.5
        let f = |x: f64| x - 0.5;
        let scan = [-1.0, 0.0, 0.25, 0.75, 1.0];
        let root = bracket_solve_1d(&f, 0.0, &scan, 1e-12, 100).expect("solver error");
        assert!(root.is_some());
        let r = root.expect("root should be Some");
        assert!((r - 0.5).abs() < 1e-9, "root inaccurate: {}", r);
    }
}
