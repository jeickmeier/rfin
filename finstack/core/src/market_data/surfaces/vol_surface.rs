//! Implied volatility surface with bilinear interpolation.
//!
//! Represents market-implied volatility as a function of option maturity and
//! strike. Uses bilinear interpolation on a rectangular grid, providing smooth
//! vega and volga calculations for options Greeks.
//!
//! # Financial Context
//!
//! Volatility surfaces capture the volatility smile/skew observed in options
//! markets. The surface shape reflects market views on:
//! - **Strike dimension**: Implied probability distribution (skew)
//! - **Maturity dimension**: Term structure of volatility
//! - **Surface dynamics**: Sticky strike vs sticky delta behavior
//!
//! # Interpolation
//!
//! Bilinear interpolation provides:
//! - C⁰ continuity (smooth values, discontinuous first derivatives)
//! - Fast evaluation for pricing and Greeks
//! - No spurious oscillations (unlike higher-order methods)
//!
//! # Evaluation Methods
//!
//! Surface queries are often data-driven, so this type provides multiple
//! evaluation methods with different out-of-bounds handling:
//!
//! - [`value_checked`](VolSurface::value_checked) — **Recommended primary API**.
//!   Returns `Result<f64>` with explicit error for out-of-bounds coordinates.
//! - [`value_clamped`](VolSurface::value_clamped) — Flat extrapolation: clamps
//!   coordinates to grid bounds before interpolation. Safe, never panics.
//! - [`value_unchecked`](VolSurface::value_unchecked) — **Panics** if coordinates
//!   are out of bounds. Use only when bounds are guaranteed by caller.
//!
//! # Examples
//! ```rust
//! use finstack_core::market_data::surfaces::vol_surface::VolSurface;
//! use finstack_core::types::CurveId;
//!
//! let surface = VolSurface::builder("EQ-FLAT")
//!     .expiries(&[1.0, 2.0])
//!     .strikes(&[90.0, 100.0, 110.0])
//!     .row(&[0.2, 0.21, 0.22])
//!     .row(&[0.19, 0.2, 0.21])
//!     .build()
//!     .expect("VolSurface builder should succeed");
//! assert_eq!(surface.id(), &CurveId::from("EQ-FLAT"));
//!
//! // Recommended: use value_checked for explicit error handling
//! let v = surface.value_checked(1.5, 100.0).expect("Value lookup should succeed");
//! assert!(v > 0.2);
//!
//! // Or use value_clamped for flat extrapolation (safe, never panics)
//! let v_clamped = surface.value_clamped(0.5, 80.0); // clamped to grid bounds
//! assert!(v_clamped > 0.0);
//! ```

// Box and Vec are available from the standard prelude; no explicit alloc import needed.

use crate::{
    error::InputError,
    market_data::{
        bumps::{BumpMode, BumpSpec, BumpUnits, Bumpable},
        traits::TermStructure,
    },
    math::interp::utils::locate_segment,
    types::CurveId,
    Error,
};
use ndarray::Array2;

/// Volatility surface defined on expiry × strike grid.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(try_from = "RawVolSurface", into = "RawVolSurface")
)]
pub struct VolSurface {
    id: CurveId,
    expiries: Box<[f64]>,
    strikes: Box<[f64]>,
    vols: Array2<f64>, // shape: (expiries.len(), strikes.len())
}

/// Raw serializable state of a VolSurface
#[cfg(feature = "serde")]
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawVolSurface {
    /// Surface identifier
    pub id: String,
    /// Expiry times in years
    pub expiries: Vec<f64>,
    /// Strike prices
    pub strikes: Vec<f64>,
    /// Volatility values in row-major order
    pub vols_row_major: Vec<f64>,
}

#[cfg(feature = "serde")]
impl From<VolSurface> for RawVolSurface {
    fn from(surface: VolSurface) -> Self {
        let vols_flat: Vec<f64> = surface.vols.iter().copied().collect();
        RawVolSurface {
            id: surface.id.to_string(),
            expiries: surface.expiries.to_vec(),
            strikes: surface.strikes.to_vec(),
            vols_row_major: vols_flat,
        }
    }
}

#[cfg(feature = "serde")]
impl TryFrom<RawVolSurface> for VolSurface {
    type Error = crate::Error;

    fn try_from(state: RawVolSurface) -> crate::Result<Self> {
        Self::from_grid(
            &state.id,
            &state.expiries,
            &state.strikes,
            &state.vols_row_major,
        )
    }
}

impl VolSurface {
    /// Start building a new volatility surface with identifier `id`.
    ///
    /// # Examples
    /// ```rust
    /// use finstack_core::market_data::surfaces::vol_surface::VolSurface;
    ///
    /// let surface = VolSurface::builder("IR-SWAPTION")
    ///     .expiries(&[1.0, 2.0])
    ///     .strikes(&[0.01, 0.02])
    ///     .row(&[0.25, 0.24])
    ///     .row(&[0.23, 0.22])
    ///     .build()
    ///     .expect("VolSurface builder should succeed");
    /// // Use value_checked for safe evaluation with explicit error handling
    /// assert!(surface.value_checked(1.5, 0.015).unwrap() > 0.22);
    /// ```
    pub fn builder(id: impl Into<CurveId>) -> VolSurfaceBuilder {
        VolSurfaceBuilder {
            id: id.into(),
            expiries: Vec::new(),
            strikes: Vec::new(),
            vols: Vec::new(),
        }
    }

    #[inline]
    fn bilinear(q11: f64, q21: f64, q12: f64, q22: f64, t: f64, u: f64) -> f64 {
        (1.0 - t) * (1.0 - u) * q11 + t * (1.0 - u) * q21 + (1.0 - t) * u * q12 + t * u * q22
    }

    /// Bilinear interpolation of vol for given expiry and strike.
    ///
    /// # Panics
    ///
    /// Panics if `expiry` or `strike` is outside the grid bounds.
    ///
    /// # Alternatives
    ///
    /// - Use [`value_checked`](Self::value_checked) for explicit error handling (recommended).
    /// - Use [`value_clamped`](Self::value_clamped) for flat extrapolation to edge values.
    pub fn value_unchecked(&self, expiry: f64, strike: f64) -> f64 {
        self.value_checked(expiry, strike)
            .expect("expiry or strike out of bounds")
    }

    /// Safe evaluation: returns `Err` if either coordinate is out of bounds.
    pub fn value_checked(&self, expiry: f64, strike: f64) -> crate::Result<f64> {
        let (ie0, exact_e) = self.value_indices(self.expiries.as_ref(), expiry)?;
        let (is0, exact_s) = self.value_indices(self.strikes.as_ref(), strike)?;
        if exact_e && exact_s {
            return Ok(self.vols[[ie0, is0]]);
        }
        let ie1 = if exact_e { ie0 } else { ie0 + 1 };
        let is1 = if exact_s { is0 } else { is0 + 1 };
        let e0 = self.expiries[ie0];
        let e1 = self.expiries[ie1];
        let s0 = self.strikes[is0];
        let s1 = self.strikes[is1];
        let q11 = self.vols[[ie0, is0]];
        let q21 = self.vols[[ie1, is0]];
        let q12 = self.vols[[ie0, is1]];
        let q22 = self.vols[[ie1, is1]];
        let t = if exact_e {
            0.0
        } else {
            (expiry - e0) / (e1 - e0)
        };
        let u = if exact_s {
            0.0
        } else {
            (strike - s0) / (s1 - s0)
        };
        Ok(Self::bilinear(q11, q21, q12, q22, t, u))
    }

    /// Clamped evaluation: clamps to edge values when outside the grid.
    ///
    /// Provides flat extrapolation by clamping coordinates to the grid bounds
    /// before interpolation. This method never panics and is suitable for
    /// pricing scenarios where out-of-bounds coordinates should use edge values.
    pub fn value_clamped(&self, mut expiry: f64, mut strike: f64) -> f64 {
        if expiry < self.expiries[0] {
            expiry = self.expiries[0];
        } else if expiry
            > *self
                .expiries
                .last()
                .expect("VolSurface should have at least one expiry")
        {
            expiry = *self
                .expiries
                .last()
                .expect("VolSurface should have at least one expiry");
        }
        if strike < self.strikes[0] {
            strike = self.strikes[0];
        } else if strike
            > *self
                .strikes
                .last()
                .expect("VolSurface should have at least one strike")
        {
            strike = *self
                .strikes
                .last()
                .expect("VolSurface should have at least one strike");
        }
        // After clamping, coordinates are guaranteed in bounds
        self.value_unchecked(expiry, strike)
    }

    #[inline]
    fn value_indices(&self, xs: &[f64], x: f64) -> Result<(usize, bool), Error> {
        let i = locate_segment(xs, x)?;
        let exact = xs[i] == x;
        Ok((i, exact))
    }

    /// Unique identifier of the surface.
    pub fn id(&self) -> &CurveId {
        &self.id
    }

    /// Returns the expiries axis (years).
    pub fn expiries(&self) -> &[f64] {
        &self.expiries
    }

    /// Returns the strikes axis.
    pub fn strikes(&self) -> &[f64] {
        &self.strikes
    }

    /// Grid shape as (n_expiries, n_strikes).
    pub fn grid_shape(&self) -> (usize, usize) {
        (self.expiries.len(), self.strikes.len())
    }

    /// Create a new volatility surface with a single point bumped.
    ///
    /// Bumps the volatility at the specified (expiry, strike) point by a relative amount.
    /// Uses bilinear interpolation to find the grid cell containing the point and bumps
    /// the nearest grid point. This is useful for bucketed Vega calculations.
    ///
    /// # Arguments
    /// * `expiry` - Expiry time in years
    /// * `strike` - Strike price
    /// * `bump_pct` - Relative bump size (e.g., 0.01 for 1% increase)
    ///
    /// # Returns
    /// New VolSurface with bumped volatility at the specified point
    ///
    /// # Errors
    /// Returns error if expiry or strike is out of bounds (even after clamping)
    ///
    /// # Examples
    /// ```rust
    /// use finstack_core::market_data::surfaces::vol_surface::VolSurface;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///
    /// let surface = VolSurface::builder("EQ-VOL")
    ///     .expiries(&[1.0, 2.0])
    ///     .strikes(&[90.0, 100.0, 110.0])
    ///     .row(&[0.2, 0.21, 0.22])
    ///     .row(&[0.19, 0.2, 0.21])
    ///     .build()
    ///     .expect("VolSurface builder should succeed");
    ///
    /// // Bump vol at (1.5 years, 100.0 strike) by 1%
    /// let bumped = surface.bump_point(1.5, 100.0, 0.01)?;
    /// assert!(bumped.value_checked(1.5, 100.0)? > surface.value_checked(1.5, 100.0)?);
    /// # Ok(())
    /// # }
    /// ```
    pub fn bump_point(&self, expiry: f64, strike: f64, bump_pct: f64) -> crate::Result<Self> {
        // Clamp to grid bounds
        let clamped_expiry = expiry.max(self.expiries[0]).min(
            *self
                .expiries
                .last()
                .expect("VolSurface should have expiries"),
        );
        let clamped_strike = strike
            .max(self.strikes[0])
            .min(*self.strikes.last().expect("VolSurface should have strikes"));

        // Find the closest grid indices
        let expiry_idx = find_closest_grid_index(self.expiries.as_ref(), clamped_expiry);
        let strike_idx = find_closest_grid_index(self.strikes.as_ref(), clamped_strike);

        // Get current vol at that grid point
        let current_vol = self.vols[[expiry_idx, strike_idx]];
        let bumped_vol = current_vol * (1.0 + bump_pct).max(0.0);

        // Clone the vols array and update the bumped point
        let mut bumped_vols = self.vols.clone();
        bumped_vols[[expiry_idx, strike_idx]] = bumped_vol;

        // Convert to row-major vec for from_grid
        let flat_vols: Vec<f64> = bumped_vols.iter().copied().collect();

        // Rebuild surface with same ID and grid
        Self::from_grid(self.id.as_str(), &self.expiries, &self.strikes, &flat_vols)
    }

    /// Return a new volatility surface scaled uniformly by `scale`.
    ///
    /// This creates a copy of the surface with the same identifier and grid,
    /// multiplying every volatility by `scale`. It avoids the overhead of
    /// serializing to a row-major buffer and rebuilding via `from_grid`.
    ///
    /// For greek bumps that apply a uniform percentage change to the entire
    /// surface, prefer this method over `to_state()`/`from_grid()`.
    pub fn scaled(&self, scale: f64) -> Self {
        // Fast path: return an identical copy when scale == 1.0
        if (scale - 1.0).abs() < f64::EPSILON {
            return Self {
                id: self.id.clone(),
                expiries: self.expiries.clone(),
                strikes: self.strikes.clone(),
                vols: self.vols.clone(),
            };
        }

        // Clone the vols array once and multiply in-place
        let mut scaled_vols = self.vols.clone();
        // Use ndarray element-wise multiplication
        scaled_vols.mapv_inplace(|v| v * scale);

        Self {
            id: self.id.clone(),
            expiries: self.expiries.clone(),
            strikes: self.strikes.clone(),
            vols: scaled_vols,
        }
    }
}

impl Bumpable for VolSurface {
    fn apply_bump(&self, spec: BumpSpec) -> Option<Self> {
        // Only parallel bumps are supported for now
        if !matches!(
            spec.bump_type,
            crate::market_data::bumps::BumpType::Parallel
        ) {
            return None;
        }

        let mut bumped_vols = self.vols.clone();
        match (spec.mode, spec.units) {
            (BumpMode::Additive, BumpUnits::RateBp | BumpUnits::Percent | BumpUnits::Fraction) => {
                let delta = spec.additive_fraction()?;
                bumped_vols.mapv_inplace(|v| (v + delta).max(0.0));
            }
            (BumpMode::Multiplicative, BumpUnits::Factor) => {
                bumped_vols.mapv_inplace(|v| (v * spec.value).max(0.0));
            }
            _ => return None,
        }

        Some(Self {
            id: self.id.clone(),
            expiries: self.expiries.clone(),
            strikes: self.strikes.clone(),
            vols: bumped_vols,
        })
    }
}

impl VolSurface {
    /// Apply a filtered bucket bump (percentage) to matching expiry/strike cells.
    pub fn apply_bucket_bump(
        &self,
        expiries_filter: Option<&[f64]>,
        strikes_filter: Option<&[f64]>,
        pct: f64,
    ) -> Option<Self> {
        let factor = 1.0 + pct / 100.0;
        let (n_expiries, n_strikes) = self.grid_shape();
        let mut builder = VolSurface::builder(self.id.clone())
            .expiries(self.expiries())
            .strikes(self.strikes());

        for (ei, &expiry) in self.expiries.iter().enumerate().take(n_expiries) {
            let mut row = Vec::with_capacity(n_strikes);
            for (si, &strike) in self.strikes.iter().enumerate().take(n_strikes) {
                let val = self.vols[[ei, si]];
                let expiry_match = expiries_filter
                    .map(|flt| flt.iter().any(|e| (e - expiry).abs() < 0.01))
                    .unwrap_or(true);
                let strike_match = strikes_filter
                    .map(|flt| flt.iter().any(|s| (s - strike).abs() < 0.01))
                    .unwrap_or(true);

                if expiry_match && strike_match {
                    row.push((val * factor).max(0.0));
                } else {
                    row.push(val);
                }
            }
            builder = builder.row(&row);
        }

        builder.build().ok()
    }
}

/// Helper to find the closest grid index for a target value.
fn find_closest_grid_index(arr: &[f64], target: f64) -> usize {
    if target <= arr[0] {
        return 0;
    }
    if target >= arr[arr.len() - 1] {
        return arr.len() - 1;
    }

    // Binary search for the segment
    for i in 0..arr.len() - 1 {
        if target >= arr[i] && target <= arr[i + 1] {
            // Return the closer of the two
            if (target - arr[i]).abs() < (target - arr[i + 1]).abs() {
                return i;
            } else {
                return i + 1;
            }
        }
    }
    arr.len() - 1
}

// Minimal trait implementation for polymorphism where needed

impl TermStructure for VolSurface {
    #[inline]
    fn id(&self) -> &CurveId {
        &self.id
    }
}

/// Fluent builder for [`VolSurface`].
pub struct VolSurfaceBuilder {
    id: CurveId,
    expiries: Vec<f64>,
    strikes: Vec<f64>,
    vols: Vec<Vec<f64>>, // row-major expiries
}

impl VolSurfaceBuilder {
    /// Set the vector of option **expiries** (years).
    pub fn expiries(mut self, exps: &[f64]) -> Self {
        self.expiries.extend_from_slice(exps);
        self
    }
    /// Set the vector of option **strikes**.
    pub fn strikes(mut self, ks: &[f64]) -> Self {
        self.strikes.extend_from_slice(ks);
        self
    }
    /// Append a row of implied volatilities corresponding to the previously
    /// set strikes. Rows must be added in the **same order** as expiries.
    pub fn data(mut self, row: &[f64]) -> Self {
        self.vols.push(row.to_vec());
        self
    }

    /// Alias for data(): clearer intent that each call appends a row.
    pub fn row(self, row: &[f64]) -> Self {
        self.data(row)
    }

    /// Finalise the surface and return an immutable [`VolSurface`] instance.
    /// Performs consistency checks on grid dimensions.
    pub fn build(self) -> crate::Result<VolSurface> {
        if self.expiries.is_empty() || self.strikes.is_empty() {
            return Err(InputError::TooFewPoints.into());
        }
        crate::math::interp::utils::validate_knots(&self.expiries[..])?;
        crate::math::interp::utils::validate_knots(&self.strikes[..])?;
        if self.vols.len() != self.expiries.len() {
            return Err(InputError::DimensionMismatch.into());
        }
        for row in &self.vols {
            if row.len() != self.strikes.len() {
                return Err(InputError::DimensionMismatch.into());
            }
            // Validate numeric properties: volatilities must be finite and non-negative
            for &v in row {
                if !v.is_finite() {
                    return Err(InputError::Invalid.into());
                }
                if v < 0.0 {
                    return Err(InputError::NegativeValue.into());
                }
            }
        }
        let flat: Vec<f64> = self.vols.into_iter().flatten().collect();
        let array = Array2::from_shape_vec((self.expiries.len(), self.strikes.len()), flat)
            .expect("Array shape should match expiries and strikes dimensions");
        Ok(VolSurface {
            id: self.id,
            expiries: self.expiries.into_boxed_slice(),
            strikes: self.strikes.into_boxed_slice(),
            vols: array,
        })
    }
}

impl VolSurface {
    /// Construct directly from axes and a row-major flat values array.
    pub fn from_grid(
        id: impl AsRef<str>,
        expiries: &[f64],
        strikes: &[f64],
        vols_row_major: &[f64],
    ) -> crate::Result<Self> {
        if expiries.is_empty() || strikes.is_empty() {
            return Err(InputError::TooFewPoints.into());
        }
        crate::math::interp::utils::validate_knots(expiries)?;
        crate::math::interp::utils::validate_knots(strikes)?;
        let n = expiries.len() * strikes.len();
        if vols_row_major.len() != n {
            return Err(InputError::DimensionMismatch.into());
        }
        for &v in vols_row_major {
            if !v.is_finite() {
                return Err(InputError::Invalid.into());
            }
            if v < 0.0 {
                return Err(InputError::NegativeValue.into());
            }
        }
        let array =
            Array2::from_shape_vec((expiries.len(), strikes.len()), vols_row_major.to_vec())
                .map_err(|_| Error::Internal)?;
        Ok(Self {
            id: CurveId::new(id.as_ref()),
            expiries: expiries.to_vec().into_boxed_slice(),
            strikes: strikes.to_vec().into_boxed_slice(),
            vols: array,
        })
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    fn flat_surface() -> VolSurface {
        VolSurface::builder("EQ-FLAT")
            .expiries(&[1.0, 2.0])
            .strikes(&[90.0, 100.0, 110.0])
            .row(&[0.2, 0.2, 0.2])
            .row(&[0.2, 0.2, 0.2])
            .build()
            .expect("VolSurface builder should succeed in test")
    }

    #[test]
    fn flat_returns_constant() {
        let vs = flat_surface();
        // Use value_checked (recommended primary API)
        assert!(
            (vs.value_checked(1.5, 95.0)
                .expect("Value lookup should succeed in test")
                - 0.2)
                .abs()
                < 1e-12
        );
        // unchecked path (same behavior, but panics on OOB)
        assert!((vs.value_unchecked(1.5, 95.0) - 0.2).abs() < 1e-12);
        // clamped path (below min strike/expiry uses flat extrapolation)
        assert!((vs.value_clamped(0.5, 80.0) - 0.2).abs() < 1e-12);
    }

    #[test]
    fn oob_checked_errors() {
        let vs = flat_surface();
        assert!(vs.value_checked(0.5, 95.0).is_err());
        assert!(vs.value_checked(1.5, 50.0).is_err());
    }

    #[test]
    fn builder_validation_errors() {
        // Unsorted expiries
        let bad = VolSurface::builder("BAD")
            .expiries(&[2.0, 1.0])
            .strikes(&[90.0, 100.0])
            .row(&[0.2, 0.2])
            .row(&[0.2, 0.2])
            .build();
        assert!(bad.is_err());

        // Mismatched row length
        let bad2 = VolSurface::builder("BAD2")
            .expiries(&[1.0, 2.0])
            .strikes(&[90.0, 100.0])
            .row(&[0.2])
            .row(&[0.2, 0.2])
            .build();
        assert!(bad2.is_err());
    }
}
