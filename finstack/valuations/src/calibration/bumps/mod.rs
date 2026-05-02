//! Re-calibration helpers for curve and surface bumping.
//!
//! Supports "what-if" risk analysis: apply a `BumpRequest` (parallel or
//! per-tenor) to a calibrated market object and re-run the matching
//! calibration step to produce a new curve/surface. Used by the scenarios
//! engine and per-instrument risk metrics (CS01, key-rate duration, vega).
//!
//! # Entry points by asset class
//!
//! | Asset class       | Functions                                                   |
//! |-------------------|-------------------------------------------------------------|
//! | Discount rates    | `bump_discount_curve`, `bump_discount_curve_synthetic`      |
//! | Credit hazard     | `bump_hazard_spreads`, `bump_hazard_shift`                  |
//! | Inflation         | `bump_inflation_rates`                                      |
//! | Volatility        | `bump_vol_surface` (uses `VolBumpRequest`)                  |
//!
//! # Convention
//!
//! Rate/spread bumps are specified in basis points. Vol bumps use
//! `VolBumpRequest` because the semantics differ (absolute vol points vs
//! relative shifts) — see that type for details.

mod currency;
pub(crate) mod hazard;
pub(crate) mod inflation;
pub(crate) mod rates;
pub(crate) mod vol;

#[doc(hidden)]
pub use hazard::bump_hazard_spreads_with_doc_clause_and_valuation_convention;
pub use hazard::{bump_hazard_shift, bump_hazard_spreads};
pub use inflation::{
    bump_inflation_rates, infer_currency_from_curve_id, observation_lag_from_curve,
};
pub use rates::{
    bump_discount_curve, bump_discount_curve_synthetic, infer_currency_from_discount_curve_id,
};
pub use vol::{bump_vol_surface, VolBumpRequest};

/// Request for a curve bump operation.
///
/// Defines the type and magnitude of a shift to be applied to market quotes
/// before re-calibration.
#[derive(Debug, Clone, PartialEq)]
pub enum BumpRequest {
    /// Parallel shift in basis points (additive to rates/spreads).
    ///
    /// # Example
    /// ```rust
    /// # use finstack_valuations::calibration::bumps::BumpRequest;
    /// let bp_10 = BumpRequest::Parallel(10.0);
    /// ```
    Parallel(f64),
    /// Node-specific shifts in basis points.
    ///
    /// Vector of `(Tenor in Years, Shift in BP)`. Used for key-rate durations
    /// and bucketed risk.
    Tenors(Vec<(f64, f64)>),
}
