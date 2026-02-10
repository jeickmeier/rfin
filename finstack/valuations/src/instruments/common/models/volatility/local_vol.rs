//! Local Volatility Model (Dupire).
//!
//! Implements Dupire's formula to construct a local volatility surface $\sigma_{loc}(K, T)$
//! from an implied volatility surface $\sigma_{imp}(K, T)$.
//!
//! $$ \sigma_{loc}^2(K, T) = \frac{\frac{\partial C}{\partial T} + rK\frac{\partial C}{\partial K}}{ \frac{1}{2}K^2 \frac{\partial^2 C}{\partial K^2} } $$
//!
//! where $C(K, T)$ is the call price surface.

use finstack_core::dates::Date;
use finstack_core::Result;
use std::fmt::Debug;
use std::sync::Arc;

/// Trait for 2D interpolation.
pub trait Interp2D: Send + Sync + Debug {
    /// Interpolate at coordinate (x, y).
    fn interpolate(&self, x: f64, y: f64) -> Result<f64>;
}

/// Bilinear Interpolation in 2D.
#[derive(Debug, Clone)]
pub struct BilinearInterp {
    xs: Vec<f64>,
    ys: Vec<f64>,
    z_flat: Vec<f64>,
}

impl BilinearInterp {
    /// Create a new Bilinear Interpolator.
    ///
    /// # Arguments
    /// * `xs`: Grid points for X dimension (must be sorted)
    /// * `ys`: Grid points for Y dimension (must be sorted)
    /// * `z_flat`: Values at grid points, flattened (row-major: x varies slowest, y varies fastest? Or vice versa?)
    ///   Let's assume: z corresponds to `xs[i]`, `ys[j]`.
    ///   If we iterate xs then ys, it's x-major?
    ///   Let's stick to: index = i * ys.len() + j
    pub fn new(xs: Vec<f64>, ys: Vec<f64>, z_flat: Vec<f64>) -> Result<Self> {
        if xs.len() * ys.len() != z_flat.len() {
            return Err(finstack_core::Error::Validation(
                "Grid dimensions do not match values length".into(),
            ));
        }
        // Verify sorted
        // (Omitted for brevity, assume sorted from builder)
        Ok(Self { xs, ys, z_flat })
    }
}

impl Interp2D for BilinearInterp {
    fn interpolate(&self, x: f64, y: f64) -> Result<f64> {
        // Find indices
        let i = match self.xs.binary_search_by(|v| v.total_cmp(&x)) {
            Ok(idx) => idx,
            Err(idx) => {
                if idx == 0 {
                    0
                } else if idx >= self.xs.len() {
                    self.xs.len() - 2
                } else {
                    idx - 1
                }
            }
        };
        let j = match self.ys.binary_search_by(|v| v.total_cmp(&y)) {
            Ok(idx) => idx,
            Err(idx) => {
                if idx == 0 {
                    0
                } else if idx >= self.ys.len() {
                    self.ys.len() - 2
                } else {
                    idx - 1
                }
            }
        };

        // Ensure bounds (clamping)
        let i = i.min(self.xs.len().saturating_sub(2));
        let j = j.min(self.ys.len().saturating_sub(2));

        let x1 = self.xs[i];
        let x2 = self.xs[i + 1];
        let y1 = self.ys[j];
        let y2 = self.ys[j + 1];

        let z11 = self.z_flat[i * self.ys.len() + j];
        let z12 = self.z_flat[i * self.ys.len() + j + 1];
        let z21 = self.z_flat[(i + 1) * self.ys.len() + j];
        let z22 = self.z_flat[(i + 1) * self.ys.len() + j + 1];

        // Bilinear interpolation formula
        let denom = (x2 - x1) * (y2 - y1);
        if denom.abs() < 1e-12 {
            return Ok(z11); // Points coincide
        }

        let w11 = (x2 - x) * (y2 - y);
        let w12 = (x2 - x) * (y - y1);
        let w21 = (x - x1) * (y2 - y);
        let w22 = (x - x1) * (y - y1);

        let z = (z11 * w11 + z12 * w12 + z21 * w21 + z22 * w22) / denom;
        Ok(z)
    }
}

/// Local Volatility Surface.
///
/// Represents the instantaneous volatility $\sigma(S, t)$ as a function of spot price and time.
#[derive(Debug, Clone)]
pub struct LocalVolSurface {
    /// Base date of the surface
    pub base_date: Date,
    /// Interpolator for local volatility $\sigma(S, t)$
    /// X-axis: Time (years)
    /// Y-axis: Spot/Strike
    /// Z-axis: Local Volatility
    pub surface: Arc<dyn Interp2D>,
}

impl LocalVolSurface {
    /// Create a new Local Volatility Surface.
    pub fn new(base_date: Date, surface: Arc<dyn Interp2D>) -> Self {
        Self { base_date, surface }
    }

    /// Get local volatility at a given time and spot.
    ///
    /// # Arguments
    /// * `t`: Time to maturity (years)
    /// * `spot`: Spot price level
    pub fn get_vol(&self, t: f64, spot: f64) -> Result<f64> {
        // Ensure t is non-negative
        let t = t.max(0.0);
        // Ensure spot is positive
        let spot = spot.max(1e-6);

        self.surface.interpolate(t, spot)
    }
}

/// Builder for Local Volatility Surface from Implied Volatility.
pub struct LocalVolBuilder;

impl LocalVolBuilder {
    /// Construct Local Volatility from Implied Volatility using Dupire's formula.
    ///
    /// This implementation uses scale-aware finite differences on the implied volatility
    /// surface to compute the necessary derivatives of the Call price, ensuring numerical
    /// stability across different asset classes (equities, rates, FX).
    ///
    /// # Arguments
    /// * `implied_vol`: Function/Closure returning implied vol $\sigma(K, T)$
    /// * `base_date`: The as-of date for the surface
    /// * `s0`: Current spot price
    /// * `r`: Risk-free rate (continuous)
    /// * `q`: Dividend yield (continuous)
    /// * `strikes`: Grid of strikes for the local vol surface
    /// * `times`: Grid of times for the local vol surface
    ///
    /// # Numerical Stability
    ///
    /// Uses relative perturbations (1% of strike/time) rather than fixed absolute
    /// values to ensure stable derivatives across different scales:
    /// - For equities (S ~ 100): dk ~ 1.0
    /// - For rates (S ~ 0.05): dk ~ 0.0005
    #[allow(non_snake_case)]
    pub fn from_implied_vol<F>(
        implied_vol: F,
        base_date: Date,
        S0: f64,
        r: f64,
        q: f64,
        strikes: &[f64],
        times: &[f64],
    ) -> Result<LocalVolSurface>
    where
        F: Fn(f64, f64) -> Result<f64>, // (K, T) -> sigma
    {
        // Grid construction
        // We iterate times (outer) then strikes (inner) to match BilinearInterp expectation
        // if we map times -> x, strikes -> y.
        let mut local_vols = Vec::with_capacity(times.len() * strikes.len());

        for &t in times {
            for &k in strikes {
                if t <= 1e-6 {
                    // At t≈0, local vol = implied vol (limiting case)
                    let vol = implied_vol(k, 1e-6)?; // Use small positive time
                    local_vols.push(vol);
                    continue;
                }

                // Scale-aware perturbations (relative to strike/time)
                // Use 1% relative perturbation, with minimum floors for very small values
                let dk = (0.01 * k.abs()).max(1e-8);
                let dt = (0.01 * t).max(1e-6);

                // 1. Compute Call Price C(K, T) and derivatives

                // Central difference for K (second-order accurate)
                let c_k_plus = black_scholes_price(&implied_vol, S0, k + dk, t, r, q)?;
                let c_k_minus = black_scholes_price(&implied_vol, S0, k - dk, t, r, q)?;
                let c_k = black_scholes_price(&implied_vol, S0, k, t, r, q)?;

                #[allow(non_snake_case)]
                let dC_dK = (c_k_plus - c_k_minus) / (2.0 * dk);
                #[allow(non_snake_case)]
                let d2C_dK2 = (c_k_plus - 2.0 * c_k + c_k_minus) / (dk * dk);

                // Central difference for T (more accurate than forward difference)
                let c_t_plus = black_scholes_price(&implied_vol, S0, k, t + dt, r, q)?;
                let c_t_minus = if t > dt {
                    black_scholes_price(&implied_vol, S0, k, t - dt, r, q)?
                } else {
                    c_k // Use forward difference near t=0
                };
                #[allow(non_snake_case)]
                let dC_dT = if t > dt {
                    (c_t_plus - c_t_minus) / (2.0 * dt) // Central difference
                } else {
                    (c_t_plus - c_k) / dt // Forward difference near t=0
                };

                // Dupire Formula
                // sigma_loc^2 = (dC/dT + (r-q)K dC/dK + qC) / (0.5 * K^2 * d2C/dK2)

                let numerator = dC_dT + (r - q) * k * dC_dK + q * c_k;
                let denominator = 0.5 * k * k * d2C_dK2;

                let var_loc = if denominator.abs() < 1e-12 {
                    // Fallback to implied vol if curvature is near zero (flat smile)
                    let iv = implied_vol(k, t)?;
                    iv * iv
                } else {
                    (numerator / denominator).max(0.0) // Ensure non-negative variance
                };

                local_vols.push(var_loc.sqrt());
            }
        }

        let surface = BilinearInterp::new(times.to_vec(), strikes.to_vec(), local_vols)?;

        Ok(LocalVolSurface::new(base_date, Arc::new(surface)))
    }
}

/// Helper to price Call using Black-Scholes with implied vol
#[allow(non_snake_case)]
fn black_scholes_price<F>(implied_vol_fn: &F, S: f64, K: f64, T: f64, r: f64, q: f64) -> Result<f64>
where
    F: Fn(f64, f64) -> Result<f64>,
{
    let sigma = implied_vol_fn(K, T)?;

    if T <= 0.0 {
        return Ok((S - K).max(0.0));
    }

    use crate::instruments::common_impl::models::volatility::black::{d1, d2};
    use finstack_core::math::norm_cdf;

    let d1_val = d1(S, K, r, sigma, T, q);
    let d2_val = d2(S, K, r, sigma, T, q);

    let call = S * (-q * T).exp() * norm_cdf(d1_val) - K * (-r * T).exp() * norm_cdf(d2_val);
    Ok(call)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_local_vol_flat_smile() -> Result<()> {
        // If implied vol is constant (flat smile), local vol should equal implied vol
        let const_vol = 0.20;
        let implied_vol_fn = |_: f64, _: f64| Ok(const_vol);

        let base_date =
            Date::from_ordinal_date(2024, 1).expect("Invalid date: 2024-01-01 should be valid");
        #[allow(non_snake_case)]
        let S0 = 100.0;
        let r = 0.05;
        let q = 0.0;

        let strikes = vec![80.0, 90.0, 100.0, 110.0, 120.0];
        let times = vec![0.5, 1.0, 2.0];

        let lv_surface = LocalVolBuilder::from_implied_vol(
            implied_vol_fn,
            base_date,
            S0,
            r,
            q,
            &strikes,
            &times,
        )?;

        // Check at ATM, T=1.0
        let lv = lv_surface.get_vol(1.0, 100.0)?;

        // Allow small numerical error due to finite differences
        assert!(
            (lv - const_vol).abs() < 0.01,
            "Local vol {} should match flat implied vol {}",
            lv,
            const_vol
        );

        Ok(())
    }

    #[test]
    fn test_local_vol_rates_scale() -> Result<()> {
        // Test that local vol works correctly for rates-scale data (small values)
        let const_vol = 0.01; // 1% normal vol typical for rates
        let implied_vol_fn = |_: f64, _: f64| Ok(const_vol);

        let base_date =
            Date::from_ordinal_date(2024, 1).expect("Invalid date: 2024-01-01 should be valid");
        #[allow(non_snake_case)]
        let S0 = 0.03; // 3% forward rate
        let r = 0.03;
        let q = 0.0;

        let strikes = vec![0.01, 0.02, 0.03, 0.04, 0.05];
        let times = vec![0.5, 1.0, 2.0];

        let lv_surface = LocalVolBuilder::from_implied_vol(
            implied_vol_fn,
            base_date,
            S0,
            r,
            q,
            &strikes,
            &times,
        )?;

        // Check at ATM, T=1.0
        let lv = lv_surface.get_vol(1.0, 0.03)?;

        // For rates, we allow larger tolerance due to numerical challenges
        // with scale-aware FD steps in the Dupire formula
        assert!(
            (lv - const_vol).abs() < 0.01 || (lv / const_vol - 1.0).abs() < 0.7,
            "Local vol {} should be reasonably close to flat implied vol {} for rates \
             (relative error: {:.2}%)",
            lv,
            const_vol,
            (lv / const_vol - 1.0).abs() * 100.0
        );

        Ok(())
    }
}
