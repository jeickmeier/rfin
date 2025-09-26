//! Bilinear-interpolated implied-volatility surface.
//!
//! The surface is defined on a rectangular grid of *option expiry* × *strike*
//! nodes.  Values between nodes are obtained via **bilinear interpolation**
//! which is smooth enough for risk engines while staying computationally cheap.
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
//!     .unwrap();
//! assert_eq!(surface.id(), &CurveId::from("EQ-FLAT"));
//! let v = surface.value_checked(1.5, 100.0).unwrap();
//! assert!(v > 0.2);
//! ```

// Box and Vec are available from the standard prelude; no explicit alloc import needed.

use crate::{
    error::InputError, market_data::traits::TermStructure, math::interp::utils::locate_segment,
    types::CurveId, Error, F,
};
use ndarray::Array2;

/// Volatility surface defined on expiry × strike grid.
///
/// Note: Use `to_state()` and `from_state()` for serialization.
pub struct VolSurface {
    id: CurveId,
    expiries: Box<[F]>,
    strikes: Box<[F]>,
    vols: Array2<F>, // shape: (expiries.len(), strikes.len())
}

/// Serializable state of a VolSurface
#[cfg(feature = "serde")]
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct VolSurfaceState {
    /// Surface identifier
    pub id: String,
    /// Expiry times in years
    pub expiries: Vec<F>,
    /// Strike prices
    pub strikes: Vec<F>,
    /// Volatility values in row-major order
    pub vols_row_major: Vec<F>,
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
    ///     .unwrap();
    /// assert!(surface.value(1.5, 0.015) > 0.22);
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
    fn bilinear(q11: F, q21: F, q12: F, q22: F, t: F, u: F) -> F {
        (1.0 - t) * (1.0 - u) * q11 + t * (1.0 - u) * q21 + (1.0 - t) * u * q12 + t * u * q22
    }

    /// Bilinear interpolation of vol for given expiry and strike.
    /// Panics if coordinates are out of bounds - prefer value_checked for safety.
    pub fn value(&self, expiry: F, strike: F) -> F {
        self.value_checked(expiry, strike)
            .expect("expiry or strike out of bounds")
    }

    /// Safe evaluation: returns `Err` if either coordinate is out of bounds.
    pub fn value_checked(&self, expiry: F, strike: F) -> crate::Result<F> {
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
    pub fn value_clamped(&self, mut expiry: F, mut strike: F) -> F {
        if expiry < self.expiries[0] {
            expiry = self.expiries[0];
        } else if expiry > *self.expiries.last().unwrap() {
            expiry = *self.expiries.last().unwrap();
        }
        if strike < self.strikes[0] {
            strike = self.strikes[0];
        } else if strike > *self.strikes.last().unwrap() {
            strike = *self.strikes.last().unwrap();
        }
        self.value(expiry, strike)
    }

    #[inline]
    fn value_indices(&self, xs: &[F], x: F) -> Result<(usize, bool), Error> {
        let i = locate_segment(xs, x)?;
        let exact = xs[i] == x;
        Ok((i, exact))
    }

    /// Unique identifier of the surface.
    pub fn id(&self) -> &CurveId {
        &self.id
    }

    /// Returns the expiries axis (years).
    pub fn expiries(&self) -> &[F] {
        &self.expiries
    }

    /// Returns the strikes axis.
    pub fn strikes(&self) -> &[F] {
        &self.strikes
    }

    /// Grid shape as (n_expiries, n_strikes).
    pub fn grid_shape(&self) -> (usize, usize) {
        (self.expiries.len(), self.strikes.len())
    }

    #[cfg(feature = "serde")]
    /// Extract serializable state
    pub fn to_state(&self) -> VolSurfaceState {
        let vols_flat: Vec<F> = self.vols.iter().copied().collect();
        VolSurfaceState {
            id: self.id.to_string(),
            expiries: self.expiries.to_vec(),
            strikes: self.strikes.to_vec(),
            vols_row_major: vols_flat,
        }
    }

    #[cfg(feature = "serde")]
    /// Create from serialized state
    pub fn from_state(state: VolSurfaceState) -> crate::Result<Self> {
        Self::from_grid(
            &state.id,
            &state.expiries,
            &state.strikes,
            &state.vols_row_major,
        )
    }
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
    expiries: Vec<F>,
    strikes: Vec<F>,
    vols: Vec<Vec<F>>, // row-major expiries
}

impl VolSurfaceBuilder {
    /// Set the vector of option **expiries** (years).
    pub fn expiries(mut self, exps: &[F]) -> Self {
        self.expiries.extend_from_slice(exps);
        self
    }
    /// Set the vector of option **strikes**.
    pub fn strikes(mut self, ks: &[F]) -> Self {
        self.strikes.extend_from_slice(ks);
        self
    }
    /// Append a row of implied volatilities corresponding to the previously
    /// set strikes. Rows must be added in the **same order** as expiries.
    pub fn data(mut self, row: &[F]) -> Self {
        self.vols.push(row.to_vec());
        self
    }

    /// Alias for data(): clearer intent that each call appends a row.
    pub fn row(self, row: &[F]) -> Self {
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
        let flat: Vec<F> = self.vols.into_iter().flatten().collect();
        let array =
            Array2::from_shape_vec((self.expiries.len(), self.strikes.len()), flat).unwrap();
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
        expiries: &[F],
        strikes: &[F],
        vols_row_major: &[F],
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
            .unwrap()
    }

    #[test]
    fn flat_returns_constant() {
        let vs = flat_surface();
        assert!((vs.value(1.5, 95.0) - 0.2).abs() < 1e-12);
        // checked path
        assert!((vs.value_checked(1.5, 95.0).unwrap() - 0.2).abs() < 1e-12);
        // clamped path (below min strike/expiry)
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
