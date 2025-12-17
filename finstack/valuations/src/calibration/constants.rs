//! Shared constants for calibration logic.
//!
//! Centralizes numerical tolerances and magic numbers to ensure consistency
//! across solvers, adapters, and validation.

/// Tolerance for detecting duplicate knot times or unsorted knots.
pub const TOLERANCE_DUP_KNOTS: f64 = 1e-10;

/// Relative tolerance for deduping grid points in scan grids.
pub const TOLERANCE_GRID_DEDUP: f64 = 0.001;

/// Minimum spacing between scan grid points to avoid numerical instability.
pub const MIN_GRID_SPACING: f64 = 1e-8;

/// Hard minimum for discount factors during solving (to prevent log(0) or negative DFs).
pub const DF_MIN_HARD: f64 = 1e-12;

/// Hard maximum for discount factors (sanity check against overflow).
pub const DF_MAX_HARD: f64 = 1e6;

/// Minimum weight floor to avoid division by zero or effectively ignoring valid quotes.
pub const WEIGHT_MIN_FLOOR: f64 = 1e-3;

/// Tolerance for floating point equality checks in validation.
pub const TOLERANCE_FLOAT_EQ: f64 = 1e-12;

/// Finite penalty value used in objective functions instead of infinity.
///
/// Using a moderate large finite value (1e6) helps solvers behave more predictably
/// than extremely large values like 1e12, which can cause numerical instability
/// with gradient-based methods.
pub const PENALTY: f64 = 1e6;

/// Maximum absolute objective value treated as "valid" during bracketing scans.
///
/// Values with `|f(x)| >= OBJECTIVE_VALID_ABS_MAX` are treated as penalized/infeasible
/// during the scan phase (but are still counted toward total evaluations).
pub const OBJECTIVE_VALID_ABS_MAX: f64 = PENALTY / 10.0;

/// Minimum absolute residual value treated as a "penalty" for reporting/diagnostics.
pub const RESIDUAL_PENALTY_ABS_MIN: f64 = PENALTY * 0.5;
