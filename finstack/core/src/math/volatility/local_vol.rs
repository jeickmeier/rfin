//! Dupire local volatility extraction from an implied volatility surface.
//!
//! The Dupire formula (1994) extracts a local (instantaneous) volatility surface
//! from observed European option prices (or equivalently, the implied volatility
//! surface). The local volatility model prices all European options consistently
//! with the market smile while being Markovian in the underlying.
//!
//! # Mathematical Foundation
//!
//! ## From call prices
//!
//! The Dupire formula in terms of call prices:
//!
//! ```text
//! σ_local²(K, T) = (∂C/∂T + rKC_K + qC) / ((1/2)K²∂²C/∂K²)
//! ```
//!
//! For zero dividends and zero rates (the case handled here for simplicity):
//!
//! ```text
//! σ_local²(K, T) ≈ (∂C/∂T) / ((1/2)K²∂²C/∂K²)
//! ```
//!
//! In practice we compute this via finite differences on Black-Scholes call
//! prices reconstructed from the implied vol surface.
//!
//! # Implementation Notes
//!
//! - Call prices are computed from the implied vol surface using Black-76
//! - Finite differences use central differences where possible, one-sided at boundaries
//! - A small floor is applied to the denominator (gamma term) to avoid division by zero
//! - The resulting local vol is stored on the same grid as the input implied vol surface
//!
//! # Reference
//!
//! - Dupire, B. (1994). "Pricing with a Smile." *Risk*, 7(1), 18-20.
//! - Gatheral, J. (2006). *The Volatility Surface: A Practitioner's Guide*.
//!   John Wiley & Sons. Chapter 2.

use crate::market_data::surfaces::VolSurface;
use crate::math::volatility::black_call;

/// Local volatility surface extracted from an implied volatility surface
/// via the Dupire formula.
///
/// The surface is represented on a rectangular grid of (expiry, strike) with
/// bilinear interpolation for off-grid queries.
///
/// # Examples
///
/// ```rust
/// use finstack_core::market_data::surfaces::VolSurface;
/// use finstack_core::math::volatility::local_vol::LocalVolSurface;
///
/// let surface = VolSurface::builder("SMILE")
///     .expiries(&[0.25, 0.5, 1.0, 2.0])
///     .strikes(&[80.0, 90.0, 95.0, 100.0, 105.0, 110.0, 120.0])
///     .row(&[0.30, 0.25, 0.22, 0.20, 0.21, 0.23, 0.28])
///     .row(&[0.28, 0.24, 0.21, 0.19, 0.20, 0.22, 0.26])
///     .row(&[0.26, 0.22, 0.20, 0.18, 0.19, 0.21, 0.24])
///     .row(&[0.24, 0.21, 0.19, 0.17, 0.18, 0.20, 0.22])
///     .build()
///     .expect("surface");
///
/// let local_vol = LocalVolSurface::from_implied_vol(&surface, 100.0, 0.03)
///     .expect("extraction should succeed");
///
/// let lv = local_vol.value(0.5, 100.0);
/// assert!(lv > 0.0 && lv.is_finite());
/// ```
#[derive(Clone, Debug)]
pub struct LocalVolSurface {
    expiries: Vec<f64>,
    strikes: Vec<f64>,
    /// Row-major storage: local_vols[expiry_idx * n_strikes + strike_idx]
    local_vols: Vec<f64>,
}

impl LocalVolSurface {
    /// Extract local volatility from an implied volatility surface using the Dupire formula.
    ///
    /// Computes call prices from Black-76 at each grid point, then applies finite
    /// differences to obtain ∂C/∂T and ∂²C/∂K² for the Dupire formula.
    ///
    /// # Arguments
    ///
    /// * `surface` — implied volatility surface (bilinear-interpolated)
    /// * `forward` — forward price (assumed constant across expiries for simplicity)
    /// * `rate` — continuously compounded risk-free rate (used for discounting)
    ///
    /// # Returns
    ///
    /// A `LocalVolSurface` on the same grid as the input.
    ///
    /// # Errors
    ///
    /// Returns an error if the surface has fewer than 2 expiries or 3 strikes
    /// (insufficient for finite differences).
    pub fn from_implied_vol(surface: &VolSurface, forward: f64, rate: f64) -> crate::Result<Self> {
        let expiries = surface.expiries().to_vec();
        let strikes = surface.strikes().to_vec();
        let n_exp = expiries.len();
        let n_str = strikes.len();

        if n_exp < 2 {
            return Err(crate::error::InputError::TooFewPoints.into());
        }
        if n_str < 3 {
            return Err(crate::error::InputError::TooFewPoints.into());
        }

        // Step 1: Compute Black-76 call prices at each grid point
        // C(T, K) = e^{-rT} × Black_Call(F, K, σ(T,K), T)
        let mut call_prices = vec![0.0; n_exp * n_str];
        for (ei, &t) in expiries.iter().enumerate() {
            let df = (-rate * t).exp();
            for (si, &k) in strikes.iter().enumerate() {
                let iv = surface
                    .value_checked(t, k)
                    .unwrap_or_else(|_| surface.value_clamped(t, k));
                call_prices[ei * n_str + si] = df * black_call(forward, k, iv, t);
            }
        }

        // Step 2: Apply Dupire formula using finite differences
        let mut local_vols = vec![0.0; n_exp * n_str];

        // Floor to prevent division by zero in gamma
        const GAMMA_FLOOR: f64 = 1e-14;

        for ei in 0..n_exp {
            let t = expiries[ei];

            for si in 0..n_str {
                let k = strikes[si];

                // ∂C/∂T via finite differences
                let dc_dt = if ei == 0 {
                    // Forward difference
                    let dt = expiries[1] - expiries[0];
                    if dt.abs() < 1e-14 {
                        0.0
                    } else {
                        (call_prices[n_str + si] - call_prices[si]) / dt
                    }
                } else if ei == n_exp - 1 {
                    // Backward difference
                    let dt = expiries[n_exp - 1] - expiries[n_exp - 2];
                    if dt.abs() < 1e-14 {
                        0.0
                    } else {
                        (call_prices[(n_exp - 1) * n_str + si]
                            - call_prices[(n_exp - 2) * n_str + si])
                            / dt
                    }
                } else {
                    // Central difference
                    let dt = expiries[ei + 1] - expiries[ei - 1];
                    if dt.abs() < 1e-14 {
                        0.0
                    } else {
                        (call_prices[(ei + 1) * n_str + si] - call_prices[(ei - 1) * n_str + si])
                            / dt
                    }
                };

                // Add rK × ∂C/∂K term (rate correction)
                // ∂C/∂K via central differences where possible
                let dc_dk = if si == 0 {
                    let dk = strikes[1] - strikes[0];
                    if dk.abs() < 1e-14 {
                        0.0
                    } else {
                        (call_prices[ei * n_str + 1] - call_prices[ei * n_str]) / dk
                    }
                } else if si == n_str - 1 {
                    let dk = strikes[n_str - 1] - strikes[n_str - 2];
                    if dk.abs() < 1e-14 {
                        0.0
                    } else {
                        (call_prices[ei * n_str + n_str - 1] - call_prices[ei * n_str + n_str - 2])
                            / dk
                    }
                } else {
                    let dk = strikes[si + 1] - strikes[si - 1];
                    if dk.abs() < 1e-14 {
                        0.0
                    } else {
                        (call_prices[ei * n_str + si + 1] - call_prices[ei * n_str + si - 1]) / dk
                    }
                };

                // ∂²C/∂K² via second-order central differences
                let d2c_dk2 = if si == 0 || si == n_str - 1 {
                    // At boundaries, use one-sided second difference if possible
                    if si == 0 && n_str >= 3 {
                        let dk01 = strikes[1] - strikes[0];
                        let dk12 = strikes[2] - strikes[1];
                        let dk02 = strikes[2] - strikes[0];
                        if dk01.abs() < 1e-14 || dk12.abs() < 1e-14 || dk02.abs() < 1e-14 {
                            0.0
                        } else {
                            // Non-uniform second derivative at left boundary
                            2.0 * (call_prices[ei * n_str + 2] / (dk12 * dk02)
                                - call_prices[ei * n_str + 1] / (dk01 * dk12)
                                + call_prices[ei * n_str] / (dk01 * dk02))
                        }
                    } else if si == n_str - 1 && n_str >= 3 {
                        let s0 = n_str - 3;
                        let dk01 = strikes[s0 + 1] - strikes[s0];
                        let dk12 = strikes[s0 + 2] - strikes[s0 + 1];
                        let dk02 = strikes[s0 + 2] - strikes[s0];
                        if dk01.abs() < 1e-14 || dk12.abs() < 1e-14 || dk02.abs() < 1e-14 {
                            0.0
                        } else {
                            2.0 * (call_prices[ei * n_str + s0 + 2] / (dk12 * dk02)
                                - call_prices[ei * n_str + s0 + 1] / (dk01 * dk12)
                                + call_prices[ei * n_str + s0] / (dk01 * dk02))
                        }
                    } else {
                        0.0
                    }
                } else {
                    // Standard central second difference (non-uniform spacing)
                    let dk_minus = strikes[si] - strikes[si - 1];
                    let dk_plus = strikes[si + 1] - strikes[si];
                    let dk_half = 0.5 * (dk_minus + dk_plus);
                    if dk_minus.abs() < 1e-14 || dk_plus.abs() < 1e-14 || dk_half.abs() < 1e-14 {
                        0.0
                    } else {
                        (call_prices[ei * n_str + si + 1] / dk_plus
                            - call_prices[ei * n_str + si] * (1.0 / dk_plus + 1.0 / dk_minus)
                            + call_prices[ei * n_str + si - 1] / dk_minus)
                            / dk_half
                    }
                };

                // Numerator: ∂C/∂T + r*K*∂C/∂K
                let numerator = dc_dt + rate * k * dc_dk;

                // Denominator: (1/2) × K² × ∂²C/∂K²
                let denominator = 0.5 * k * k * d2c_dk2;

                // Compute local variance
                let local_var = if denominator.abs() < GAMMA_FLOOR {
                    // Gamma is too small — fall back to implied vol
                    let iv = surface
                        .value_checked(t, k)
                        .unwrap_or_else(|_| surface.value_clamped(t, k));
                    iv * iv
                } else {
                    let lv2 = numerator / denominator;
                    if lv2 < 0.0 {
                        // Negative local variance — use implied vol as fallback
                        let iv = surface
                            .value_checked(t, k)
                            .unwrap_or_else(|_| surface.value_clamped(t, k));
                        iv * iv
                    } else {
                        lv2
                    }
                };

                local_vols[ei * n_str + si] = local_var.sqrt();
            }
        }

        Ok(Self {
            expiries,
            strikes,
            local_vols,
        })
    }

    /// Evaluate the local volatility at a given (expiry, strike) point.
    ///
    /// Uses bilinear interpolation on the local vol grid. For coordinates outside
    /// the grid, clamps to the nearest boundary value (flat extrapolation).
    ///
    /// # Arguments
    ///
    /// * `expiry` — time to expiry in years
    /// * `strike` — option strike
    ///
    /// # Returns
    ///
    /// Local volatility σ_local(T, K). Returns `NaN` for empty surfaces.
    pub fn value(&self, expiry: f64, strike: f64) -> f64 {
        let n_exp = self.expiries.len();
        let n_str = self.strikes.len();
        if n_exp == 0 || n_str == 0 {
            return f64::NAN;
        }

        // Clamp to grid bounds
        let exp_min = self.expiries[0];
        let exp_max = self.expiries[n_exp - 1];
        let str_min = self.strikes[0];
        let str_max = self.strikes[n_str - 1];

        let t = expiry.clamp(exp_min, exp_max);
        let k = strike.clamp(str_min, str_max);

        // Find segment indices
        let ei = find_segment(&self.expiries, t);
        let si = find_segment(&self.strikes, k);

        // Handle exact hits — intentional exact comparison against grid values.
        #[allow(clippy::float_cmp)]
        let exact_e = self.expiries[ei] == t;
        #[allow(clippy::float_cmp)]
        let exact_s = self.strikes[si] == k;

        if exact_e && exact_s {
            return self.local_vols[ei * n_str + si];
        }

        let ei1 = if exact_e { ei } else { (ei + 1).min(n_exp - 1) };
        let si1 = if exact_s { si } else { (si + 1).min(n_str - 1) };

        let e0 = self.expiries[ei];
        let e1 = self.expiries[ei1];
        let s0 = self.strikes[si];
        let s1 = self.strikes[si1];

        let q11 = self.local_vols[ei * n_str + si];
        let q21 = self.local_vols[ei1 * n_str + si];
        let q12 = self.local_vols[ei * n_str + si1];
        let q22 = self.local_vols[ei1 * n_str + si1];

        let u = if exact_e || (e1 - e0).abs() < 1e-14 {
            0.0
        } else {
            (t - e0) / (e1 - e0)
        };
        let v = if exact_s || (s1 - s0).abs() < 1e-14 {
            0.0
        } else {
            (k - s0) / (s1 - s0)
        };

        // Bilinear interpolation
        (1.0 - u) * (1.0 - v) * q11 + u * (1.0 - v) * q21 + (1.0 - u) * v * q12 + u * v * q22
    }

    /// Returns the expiry axis.
    pub fn expiries(&self) -> &[f64] {
        &self.expiries
    }

    /// Returns the strike axis.
    pub fn strikes(&self) -> &[f64] {
        &self.strikes
    }

    /// Grid shape as (n_expiries, n_strikes).
    pub fn grid_shape(&self) -> (usize, usize) {
        (self.expiries.len(), self.strikes.len())
    }
}

/// Find the segment index for a value in a sorted array.
/// Returns i such that arr[i] <= x < arr[i+1], or 0 / len-2 at boundaries.
fn find_segment(arr: &[f64], x: f64) -> usize {
    if arr.len() <= 1 {
        return 0;
    }
    if x <= arr[0] {
        return 0;
    }
    if x >= arr[arr.len() - 1] {
        return arr.len() - 1;
    }
    // Simple linear scan (grids are typically small)
    arr.windows(2)
        .position(|w| x >= w[0] && x <= w[1])
        .unwrap_or(arr.len() - 2)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn test_surface() -> VolSurface {
        VolSurface::builder("TEST-LV")
            .expiries(&[0.25, 0.5, 1.0, 2.0])
            .strikes(&[80.0, 90.0, 95.0, 100.0, 105.0, 110.0, 120.0])
            .row(&[0.30, 0.25, 0.22, 0.20, 0.21, 0.23, 0.28])
            .row(&[0.28, 0.24, 0.21, 0.19, 0.20, 0.22, 0.26])
            .row(&[0.26, 0.22, 0.20, 0.18, 0.19, 0.21, 0.24])
            .row(&[0.24, 0.21, 0.19, 0.17, 0.18, 0.20, 0.22])
            .build()
            .expect("test surface should build")
    }

    #[test]
    fn local_vol_from_implied_vol_succeeds() {
        let surface = test_surface();
        let lv = LocalVolSurface::from_implied_vol(&surface, 100.0, 0.03)
            .expect("extraction should succeed");

        assert_eq!(lv.grid_shape(), (4, 7));
    }

    #[test]
    fn local_vol_positive_and_finite() {
        let surface = test_surface();
        let lv = LocalVolSurface::from_implied_vol(&surface, 100.0, 0.03)
            .expect("extraction should succeed");

        for &t in lv.expiries() {
            for &k in lv.strikes() {
                let vol = lv.value(t, k);
                assert!(
                    vol > 0.0 && vol.is_finite(),
                    "Local vol at ({t}, {k}) = {vol} should be positive and finite"
                );
            }
        }
    }

    #[test]
    fn local_vol_interpolation_works() {
        let surface = test_surface();
        let lv = LocalVolSurface::from_implied_vol(&surface, 100.0, 0.03)
            .expect("extraction should succeed");

        // Interpolated point between grid nodes
        let vol = lv.value(0.75, 97.5);
        assert!(
            vol > 0.0 && vol.is_finite(),
            "Interpolated local vol should be valid: {vol}"
        );
    }

    #[test]
    fn local_vol_clamped_outside_grid() {
        let surface = test_surface();
        let lv = LocalVolSurface::from_implied_vol(&surface, 100.0, 0.03)
            .expect("extraction should succeed");

        // Outside grid bounds should clamp (flat extrapolation)
        let vol_low = lv.value(0.1, 60.0);
        let vol_high = lv.value(3.0, 150.0);

        assert!(
            vol_low > 0.0 && vol_low.is_finite(),
            "Clamped low vol: {vol_low}"
        );
        assert!(
            vol_high > 0.0 && vol_high.is_finite(),
            "Clamped high vol: {vol_high}"
        );
    }

    #[test]
    fn local_vol_reasonable_magnitude() {
        let surface = test_surface();
        let lv = LocalVolSurface::from_implied_vol(&surface, 100.0, 0.03)
            .expect("extraction should succeed");

        // Local vol should be of similar magnitude to implied vol (within 3x)
        let atm_local = lv.value(1.0, 100.0);
        let atm_implied = surface.value_checked(1.0, 100.0).expect("in-bounds lookup");

        assert!(
            atm_local > atm_implied * 0.3 && atm_local < atm_implied * 3.0,
            "ATM local vol {atm_local:.4} should be near implied {atm_implied:.4}"
        );
    }

    #[test]
    fn local_vol_flat_surface_is_flat() {
        // For a flat implied vol surface, local vol should also be approximately flat
        let flat = VolSurface::builder("FLAT")
            .expiries(&[0.25, 0.5, 1.0, 2.0])
            .strikes(&[80.0, 90.0, 100.0, 110.0, 120.0])
            .row(&[0.20, 0.20, 0.20, 0.20, 0.20])
            .row(&[0.20, 0.20, 0.20, 0.20, 0.20])
            .row(&[0.20, 0.20, 0.20, 0.20, 0.20])
            .row(&[0.20, 0.20, 0.20, 0.20, 0.20])
            .build()
            .expect("flat surface should build");

        let lv = LocalVolSurface::from_implied_vol(&flat, 100.0, 0.0)
            .expect("extraction should succeed");

        // Interior points (not boundaries) should be close to 0.20
        let vol_mid = lv.value(0.75, 100.0);
        assert!(
            (vol_mid - 0.20).abs() < 0.05,
            "Flat surface local vol should be ~0.20, got {vol_mid:.4}"
        );
    }

    #[test]
    fn local_vol_rejects_too_few_expiries() {
        let surface = VolSurface::builder("THIN")
            .expiries(&[1.0])
            .strikes(&[80.0, 90.0, 100.0, 110.0, 120.0])
            .row(&[0.25, 0.22, 0.20, 0.22, 0.25])
            .build()
            .expect("surface should build");

        let result = LocalVolSurface::from_implied_vol(&surface, 100.0, 0.03);
        assert!(result.is_err(), "Should require at least 2 expiries");
    }
}
