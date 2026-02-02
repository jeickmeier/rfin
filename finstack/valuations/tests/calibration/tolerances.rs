//! Shared tolerance policy for the calibration v2 test suite.
//!
//! These values aim to be "vendor grade" (Bloomberg/FINCAD-style): tight, explicit,
//! and consistent with the units used by each calibration adapter.
//!
//! ## Residual Units (important!)
//!
//! - **Rates discount/forward adapters**: residual = PV / CALIBRATION_NOTIONAL
//!   (PV is in currency units; CALIBRATION_NOTIONAL is typically 1_000_000).
//!   Example: tolerance `1e-8` ⇒ PV ≤ `0.01` per $1mm notional.
//!
//! - **Base correlation adapter**: residual = upfront_fraction_model - upfront_fraction_market
//!   (dimensionless, where upfront_fraction = upfront_pct / 100).
//!
//! - **Swaption vol adapter**: residual = |model_vol - market_vol| in **decimal** units.
//!   For Normal vols, quotes are in bp and are normalized to decimals (50bp → 0.0050).

/// Strict absolute tolerance for pure floating-point identity checks.
#[allow(dead_code)]
pub const F64_ABS_TOL_STRICT: f64 = 1e-12;

/// Looser absolute tolerance for time-axis/year-fraction computations that involve multiple
/// floating-point steps and convention adjustments.
pub const F64_ABS_TOL_LOOSE: f64 = 1e-10;

/// Repricing PV tolerance (absolute dollars) used by v2 external repricing tests.
pub const REPRICE_PV_ABS_TOL_DOLLARS: f64 = 1.0;

/// Swap repricing tolerance expressed as an implied par-rate error in basis points.
#[allow(dead_code)]
pub const SWAP_REPRICE_TOL_BP: f64 = 0.1;

/// Minimum absolute PV tolerance for swap repricing, to avoid over-tight constraints when DV01 is
/// extremely small.
#[allow(dead_code)]
pub const SWAP_REPRICE_MIN_ABS_DOLLARS: f64 = 1.0;

/// FRA repricing tolerance (absolute dollars) for a $1mm notional FRA.
pub const FRA_REPRICE_ABS_TOL_DOLLARS: f64 = 5.0;

/// Bloomberg DF difference tolerances (basis points of DF, i.e. (df - bbg_df) * 10_000).
#[allow(dead_code)]
pub const BBG_DF_TOL_BP_SHORT: f64 = 2.0;
#[allow(dead_code)]
pub const BBG_DF_TOL_BP_MID: f64 = 4.0;
#[allow(dead_code)]
pub const BBG_DF_TOL_BP_LONG: f64 = 5.0;

/// Bloomberg zero-rate tolerance (basis points).
#[allow(dead_code)]
pub const BBG_ZERO_TOL_BP: f64 = 2.0;

/// Bloomberg DF tolerance for ultra-long end (>= 25Y), in DF basis points.
#[allow(dead_code)]
pub const BBG_DF_TOL_BP_ULTRA_LONG: f64 = 100.0;

/// Bloomberg zero-rate tolerance for ultra-long end (>= 25Y), in basis points.
#[allow(dead_code)]
pub const BBG_ZERO_TOL_BP_ULTRA_LONG: f64 = 15.0;

/// Forward rate absolute tolerance used by parity tests (in rate decimals).
pub const FWD_RATE_ABS_TOL: f64 = 1e-8;

/// Base correlation upfront-fraction tolerance (dimensionless).
pub const BASE_CORR_UPFRONT_FRAC_TOL: f64 = 1e-10;

/// Swaption normal-vol fit tolerance (decimal). 5bp → 0.0005.
pub const SWAPTION_VOL_FIT_TOL_NORMAL_DECIMAL: f64 = 0.0005;

/// Assert two f64 values are close in absolute terms with a high-signal error message.
pub fn assert_close_abs(actual: f64, expected: f64, tol: f64, label: &str) {
    let diff = (actual - expected).abs();
    assert!(
        diff <= tol,
        "{label}: |actual-expected|={diff:.3e} > tol={tol:.3e} (actual={actual:.12}, expected={expected:.12})"
    );
}

/// Convert a basis-point tolerance to a PV tolerance using an instrument DV01 (dollars per bp).
#[allow(dead_code)]
#[inline]
pub fn pv_tolerance_from_dv01(dv01_dollars_per_bp: f64, tol_bp: f64) -> f64 {
    (tol_bp * dv01_dollars_per_bp.abs()).max(SWAP_REPRICE_MIN_ABS_DOLLARS)
}
