use crate::{error::InputError, market_data::interp::InterpFn, F};

/// Piece-wise linear interpolation on discount factors.
///
/// See unit tests and `examples/` for usage.
#[derive(Debug)]
pub struct LinearDf {
    knots: Box<[F]>,
    dfs: Box<[F]>,
}

impl LinearDf {
    /// Build a **piecewise‐linear** discount‐factor interpolator.
    ///
    /// # Errors
    /// Returns [`InputError`] variants if fewer than two points are supplied
    /// or the data is invalid.
    #[allow(clippy::boxed_local)]
    pub fn new(knots: Box<[F]>, dfs: Box<[F]>) -> crate::Result<Self> {
        debug_assert_eq!(knots.len(), dfs.len());
        if knots.len() < 2 {
            return Err(InputError::TooFewPoints.into());
        }
        // Ensure strictly ascending times via shared helper
        crate::market_data::utils::validate_knots(&knots)?;
        // Validate discount factors (positive).
        crate::market_data::utils::validate_dfs(&dfs, false)?;
        Ok(Self { knots, dfs })
    }

    // Shared `locate_segment` is used via utils.
}

impl InterpFn for LinearDf {
    fn interp(&self, x: F) -> F {
        // Clamp to bounds to avoid out-of-range evaluations due to
        // small day-count or floating-point discrepancies.
        if x <= self.knots[0] {
            return self.dfs[0];
        }
        if x >= *self.knots.last().unwrap() {
            return *self.dfs.last().unwrap();
        }
        if let Ok(idx_exact) = self.knots.binary_search_by(|k| k.partial_cmp(&x).unwrap()) {
            return self.dfs[idx_exact];
        }
        let idx = crate::market_data::utils::locate_segment(&self.knots, x).unwrap();
        let x0 = self.knots[idx];
        let x1 = self.knots[idx + 1];
        let df0 = self.dfs[idx];
        let df1 = self.dfs[idx + 1];
        let w = (x - x0) / (x1 - x0);
        df0 + w * (df1 - df0)
    }

    fn interp_prime(&self, x: F) -> F {
        // For linear interpolation, the derivative is constant within each segment
        // and equals the slope of the line segment.
        
        // At boundaries, return the slope of the adjacent segment
        if x <= self.knots[0] {
            let x0 = self.knots[0];
            let x1 = self.knots[1];
            let df0 = self.dfs[0];
            let df1 = self.dfs[1];
            return (df1 - df0) / (x1 - x0);
        }
        if x >= *self.knots.last().unwrap() {
            let n = self.knots.len();
            let x0 = self.knots[n - 2];
            let x1 = self.knots[n - 1];
            let df0 = self.dfs[n - 2];
            let df1 = self.dfs[n - 1];
            return (df1 - df0) / (x1 - x0);
        }
        
        let idx = crate::market_data::utils::locate_segment(&self.knots, x).unwrap();
        let x0 = self.knots[idx];
        let x1 = self.knots[idx + 1];
        let df0 = self.dfs[idx];
        let df1 = self.dfs[idx + 1];
        
        // Linear interpolation derivative: constant slope within segment
        (df1 - df0) / (x1 - x0)
    }
}
