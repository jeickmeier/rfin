//! Bilinear-interpolated implied-volatility surface.
//!
//! The surface is defined on a rectangular grid of *option expiry* × *strike*
//! nodes.  Values between nodes are obtained via **bilinear interpolation**
//! which is sufficiently smooth and very fast for typical risk calculations.
//!
//! ## Construction Example
//! ```rust
//! use finstack_core::market_data::surfaces::vol_surface::VolSurface;
//! let vs = VolSurface::builder("EQ-FLAT")
//!     .expiries(&[1.0, 2.0])
//!     .strikes(&[90.0, 100.0, 110.0])
//!     .row(&[0.2, 0.2, 0.2])
//!     .row(&[0.2, 0.2, 0.2])
//!     .build()
//!     .unwrap();
//! assert!((vs.value(1.5, 95.0) - 0.2).abs() < 1e-12);
//! assert!((vs.value_checked(1.5, 95.0).unwrap() - 0.2).abs() < 1e-12);
//! assert!((vs.value_clamped(0.5, 80.0) - 0.2).abs() < 1e-12);
//! ```

// Box and Vec are available from the standard prelude; no explicit alloc import needed.

use crate::{
    error::InputError,
    market_data::id::CurveId,
    market_data::traits::{Surface, TermStructure},
    market_data::utils::locate_segment,
    Error, F,
};
use ndarray::Array2;

/// Volatility surface defined on expiry × strike grid.
pub struct VolSurface {
    id: CurveId,
    expiries: Box<[F]>,
    strikes: Box<[F]>,
    vols: Array2<F>, // shape: (expiries.len(), strikes.len())
}

impl VolSurface {
    /// Start building a new volatility surface with identifier `id`.
    pub fn builder(id: &'static str) -> VolSurfaceBuilder {
        VolSurfaceBuilder {
            id,
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
    pub fn value(&self, expiry: F, strike: F) -> F {
        // Unchecked: will panic on OOB. Prefer value_checked for safety.
        let (ie0, exact_e) = self
            .value_indices(self.expiries.as_ref(), expiry)
            .expect("expiry out of bounds");
        let (is0, exact_s) = self
            .value_indices(self.strikes.as_ref(), strike)
            .expect("strike out of bounds");
        if exact_e && exact_s {
            return self.vols[[ie0, is0]];
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
        Self::bilinear(q11, q21, q12, q22, t, u)
    }

    /// Safe evaluation: returns Err if either coordinate is out of bounds.
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
}

impl Surface for VolSurface {
    fn value(&self, x: F, y: F) -> F {
        self.value(x, y)
    }
}

impl TermStructure for VolSurface {
    fn id(&self) -> &CurveId {
        &self.id
    }
}

/// Fluent builder for [`VolSurface`].
pub struct VolSurfaceBuilder {
    id: &'static str,
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
        crate::market_data::utils::validate_knots(&self.expiries[..])?;
        crate::market_data::utils::validate_knots(&self.strikes[..])?;
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
            id: CurveId::new(self.id),
            expiries: self.expiries.into_boxed_slice(),
            strikes: self.strikes.into_boxed_slice(),
            vols: array,
        })
    }
}

impl VolSurface {
    /// Construct directly from axes and a row-major flat values array.
    pub fn from_grid(
        id: &'static str,
        expiries: &[F],
        strikes: &[F],
        vols_row_major: &[F],
    ) -> crate::Result<Self> {
        if expiries.is_empty() || strikes.is_empty() {
            return Err(InputError::TooFewPoints.into());
        }
        crate::market_data::utils::validate_knots(expiries)?;
        crate::market_data::utils::validate_knots(strikes)?;
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
            id: CurveId::new(id),
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
