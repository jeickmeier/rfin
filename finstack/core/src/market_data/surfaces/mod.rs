//! Two-dimensional market data surfaces.
//!
//! Provides 2D interpolation structures for market observables that vary by
//! two parameters (e.g., volatility by strike and maturity). Currently supports
//! volatility surfaces with planned expansion for correlation and dividend surfaces.
//!
//! # Surface Types
//!
//! - `VolSurface`: Implied volatility by strike and maturity (bilinear interpolation)
//!
//! # Examples
//! ```rust
//! use finstack_core::market_data::surfaces::VolSurface;
//! use finstack_core::types::CurveId;
//! # fn main() -> finstack_core::Result<()> {
//!
//! let surface = VolSurface::builder("EQ-FLAT")
//!     .expiries(&[1.0, 2.0])
//!     .strikes(&[90.0, 100.0])
//!     .row(&[0.2, 0.2])
//!     .row(&[0.2, 0.2])
//!     .build()
//!     ?;
//! assert_eq!(surface.id(), &CurveId::from("EQ-FLAT"));
//! # Ok(())
//! # }
//! ```

mod delta_vol_surface;
pub mod fx_delta_vol_surface;
mod vol_surface;

#[inline]
pub(crate) fn recover_fx_wing_vols(atm: f64, rr: f64, bf: f64) -> (f64, f64) {
    let sigma_call = atm + bf + 0.5 * rr;
    let sigma_put = atm + bf - 0.5 * rr;
    (sigma_put, sigma_call)
}

#[inline]
pub(crate) fn fx_forward(spot: f64, domestic_rate: f64, foreign_rate: f64, expiry: f64) -> f64 {
    spot * ((domestic_rate - foreign_rate) * expiry).exp()
}

#[inline]
pub(crate) fn fx_atm_dns_strike(forward: f64, vol: f64, expiry: f64) -> f64 {
    forward * (0.5 * vol * vol * expiry).exp()
}

#[inline]
pub(crate) fn fx_put_call_25d_strikes(
    forward: f64,
    sigma_put: f64,
    sigma_call: f64,
    expiry: f64,
) -> (f64, f64) {
    let sqrt_t = expiry.sqrt();
    let z_25d = crate::math::special_functions::standard_normal_inv_cdf(0.25);
    let k_put = forward * (z_25d * sigma_put * sqrt_t + 0.5 * sigma_put * sigma_put * expiry).exp();
    let k_call =
        forward * (-z_25d * sigma_call * sqrt_t + 0.5 * sigma_call * sigma_call * expiry).exp();
    (k_put, k_call)
}

/// Piecewise-linear interpolation on sorted knots with flat extrapolation.
pub(crate) fn interp_linear_clamp(xs: &[f64], ys: &[f64], x: f64) -> f64 {
    debug_assert!(!xs.is_empty());
    debug_assert_eq!(xs.len(), ys.len());

    if x <= xs[0] {
        return ys[0];
    }
    let n = xs.len();
    if x >= xs[n - 1] {
        return ys[n - 1];
    }

    for i in 0..n - 1 {
        if x >= xs[i] && x <= xs[i + 1] {
            let t = (x - xs[i]) / (xs[i + 1] - xs[i]);
            return ys[i] + t * (ys[i + 1] - ys[i]);
        }
    }

    ys[n - 1]
}

// Re-export for ergonomic access (curated list)
pub use delta_vol_surface::FxDeltaVolSurfaceBuilder;
pub use fx_delta_vol_surface::FxDeltaVolSurface;
pub use vol_surface::{VolSurface, VolSurfaceAxis, VolSurfaceBuilder};
