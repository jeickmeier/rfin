//! Bilinear-interpolated implied-volatility surface.
//!
//! The surface is defined on a rectangular grid of *option expiry* × *strike*
//! nodes.  Values between nodes are obtained via **bilinear interpolation**
//! which is sufficiently smooth and very fast for typical risk calculations.
//!
//! ## Construction Example
//! ```rust
//! use rfin_core::market_data::surfaces::vol_surface::VolSurface;
//! let vs = VolSurface::builder("EQ-FLAT")
//!     .expiries(&[1.0, 2.0])
//!     .strikes(&[90.0, 100.0, 110.0])
//!     .data(&[0.2, 0.2, 0.2])
//!     .data(&[0.2, 0.2, 0.2])
//!     .build()
//!     .unwrap();
//! assert!((vs.value(1.5, 95.0) - 0.2).abs() < 1e-12);
//! ```
#![allow(dead_code)]

extern crate alloc;
use alloc::{boxed::Box, vec::Vec};

use crate::{
    error::InputError,
    market_data::id::CurveId,
    market_data::traits::{Surface, TermStructure},
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

    fn locate(&self, xs: &[F], x: F) -> Result<(usize, bool), Error> {
        if x < xs[0] || x > xs[xs.len() - 1] {
            return Err(Error::InterpOutOfBounds);
        }
        // Find first element not strictly less than `x`.
        let idx = xs.partition_point(|v| *v < x);
        let exact = idx < xs.len() && xs[idx] == x;
        let seg_idx = if exact { idx } else { idx - 1 };
        Ok((seg_idx, exact))
    }

    /// Bilinear interpolation of vol for given expiry and strike.
    pub fn value(&self, expiry: F, strike: F) -> F {
        // Check bounds and maybe exact knot
        let ie = self.locate(&self.expiries, expiry).unwrap();
        let is = self.locate(&self.strikes, strike).unwrap();
        let (ie0, exact_e) = ie;
        let (is0, exact_s) = is;
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
        // Bilinear weights
        let t = (expiry - e0) / (e1 - e0);
        let u = (strike - s0) / (s1 - s0);
        (1.0 - t) * (1.0 - u) * q11 + t * (1.0 - u) * q21 + (1.0 - t) * u * q12 + t * u * q22
    }

    /// Unique identifier of the surface.
    pub fn id(&self) -> &CurveId {
        &self.id
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
            .data(&[0.2, 0.2, 0.2])
            .data(&[0.2, 0.2, 0.2])
            .build()
            .unwrap()
    }

    #[test]
    fn flat_returns_constant() {
        let vs = flat_surface();
        assert!((vs.value(1.5, 95.0) - 0.2).abs() < 1e-12);
    }
}
