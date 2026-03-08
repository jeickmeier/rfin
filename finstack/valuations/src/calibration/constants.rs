//! Shared constants for calibration logic.
//!
//! Centralizes numerical tolerances and magic numbers to ensure consistency
//! across solvers, adapters, and validation.
//!
//! # Constants
//! - [`TOLERANCE_DUP_KNOTS`]: Tolerance for detecting duplicate knot times.
//! - [`PENALTY`]: Finite penalty value for objective functions.
//! - [`DF_MIN_HARD`]: Lower bound for discount factors.

/// Tolerance for detecting duplicate knot times or unsorted knots.
///
/// Used to ensure numerical stability in spline interpolation and sequential bootstrapping.
/// Knots closer than this value are treated as occurring at the same time.
pub const TOLERANCE_DUP_KNOTS: f64 = 1e-10;

/// Relative tolerance for deduping grid points in scan grids.
///
/// Ensures scan grids don't have clusters of points that could cause
/// redundant objective function evaluations.
pub const TOLERANCE_GRID_DEDUP: f64 = 0.001;

/// Minimum spacing between scan grid points to avoid numerical instability.
///
/// Expressed in year-fraction units. Spacing below this threshold can lead
/// to poorly conditioned matrices in global optimization.
pub const MIN_GRID_SPACING: f64 = 1e-8;

/// Hard minimum for discount factors during solving (to prevent log(0) or negative DFs).
///
/// Prevents the solver from exploring regions where interest rates become physically
/// impossible or result in non-finite logarithms.
pub const DF_MIN_HARD: f64 = 1e-12;

/// Minimum weight floor to avoid division by zero or effectively ignoring valid quotes.
///
/// Used when weighting residuals by inverse duration or other dynamic schemes.
pub const WEIGHT_MIN_FLOOR: f64 = 1e-3;

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
///
/// If the final residual exceeds this value, the calibration is considered to have
/// hit a hard constraint or failed significantly.
pub const RESIDUAL_PENALTY_ABS_MIN: f64 = PENALTY * 0.5;

/// Newtype wrapper for `f64` that implements `Ord` for use as `BTreeMap` keys.
///
/// Uses `f64::total_cmp` so NaN values sort consistently (greater than all finite values).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct OrderedF64(pub f64);

impl Eq for OrderedF64 {}

impl PartialOrd for OrderedF64 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OrderedF64 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}

impl From<f64> for OrderedF64 {
    fn from(value: f64) -> Self {
        Self(value)
    }
}

impl OrderedF64 {
    pub fn into_inner(self) -> f64 {
        self.0
    }
}
