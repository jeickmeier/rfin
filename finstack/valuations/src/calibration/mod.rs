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
//! The calibration framework supports both single-curve and multi-curve calibration:
//!
//! ### Single-Curve Mode (Pre-2008 Methodology)
//! In single-curve mode, forward curves are derived from discount curves using
//! no-arbitrage relationships. This is suitable for markets where basis spreads
//! are negligible.
//!
//! ```ignore
//! use finstack_valuations::calibration::{CalibrationConfig, MultiCurveConfig};
//! use finstack_valuations::calibration::methodså::DiscountCurveCalibrator;
//!
//! let config = CalibrationConfig::default()
//!     .with_multi_curve(MultiCurveConfig::single_curve(0.25)); // 3M tenor
//!
//! // Calibrate discount curve - forward curve will be derived automatically
//! let calibrator = DiscountCurveCalibrator::new(base_date, Currency::USD, config);
//! let (discount_curve, _) = calibrator.calibrate(&quotes, &context)?;
//! ```
//!
//! ### Multi-Curve Mode (Post-2008 Methodology)
//! In multi-curve mode, discount and forward curves are calibrated separately
//! to capture basis spreads and credit/liquidity effects.
//!
//! ```ignore
//! use finstack_valuations::calibration::{CalibrationConfig, MultiCurveConfig};
//! use finstack_valuations::calibration::methods::{
//!     DiscountCurveCalibrator, ForwardCurveCalibrator
//! };
//!
//! let config = CalibrationConfig::default()
//!     .with_multi_curve(MultiCurveConfig::multi_curve());
//!
//! // Step 1: Calibrate OIS discount curve using deposits and OIS swaps
//! let ois_quotes = vec![/* deposits and OIS swaps */];
//! let disc_calibrator = DiscountCurveCalibrator::new(base_date, Currency::USD, config.clone());
//! let (discount_curve, _) = disc_calibrator.calibrate(&ois_quotes, &context)?;
//!
//! // Step 2: Add discount curve to context
//! let context = context.insert_discount(discount_curve);
//!
//! // Step 3: Calibrate forward curves using FRAs, futures, and LIBOR swaps
//! let libor_quotes = vec![/* FRAs, futures, and LIBOR swaps */];
//! let fwd_calibrator = ForwardCurveCalibrator::new(
//!     "3M-LIBOR", 0.25, base_date, Currency::USD, "OIS", config
//! );
//! let (forward_curve, _) = fwd_calibrator.calibrate(&libor_quotes, &context)?;
//! ```
//!
//! ### Important Notes for Multi-Curve Calibration
//!
//! 1. **Instrument Selection**:
//!    - For discount curve: Use deposits and OIS swaps (instruments that don't require forward curves)
//!    - For forward curves: Use FRAs, futures, and tenor-specific swaps
//!
//! 2. **Calibration Order**:
//!    - Always calibrate discount curve first
//!    - Then calibrate forward curves with discount curve in context
//!
//! 3. **Validation**:
//!    - The framework will warn if using forward-dependent instruments for discount curve calibration
//!    - In multi-curve mode, forward curves won't be automatically derived

use finstack_core::F;

// Submodules
mod config;
mod constraints;
pub mod derivatives;
pub mod methods;
mod quote;
mod report;
pub mod simple_calibration;
mod traits;
mod validation;

// Re-exports
pub use config::{CalibrationConfig, MultiCurveConfig, MultiCurveMode, SolverKind};
pub use constraints::{CalibrationConstraint, ConstraintType, InequalityDirection};
pub use quote::{
    CreditQuote, FutureSpecs, InflationQuote, MarketQuote, QuoteWithMetadata, RatesQuote, VolQuote,
};
pub use report::CalibrationReport;
pub use simple_calibration::SimpleCalibration;
pub use traits::Calibrator;
pub use validation::{
    CurveValidator, MarketValidator, SurfaceValidator, ValidationConfig, ValidationError,
};

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
pub fn solve_1d<Fun>(
    kind: SolverKind,
    tol: F,
    iters: usize,
    f: Fun,
    init: F,
) -> Result<F>
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
