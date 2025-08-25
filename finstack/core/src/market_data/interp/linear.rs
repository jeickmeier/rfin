use crate::{error::InputError, market_data::interp::InterpFn, F};

/// Piece-wise linear interpolation on discount factors.
///
/// # Example
/// ```text
/// use crate::market_data::interp::LinearDf;
/// let interp = LinearDf::new(
///     vec![0.0, 1.0].into_boxed_slice(),
///     vec![1.0, 0.95].into_boxed_slice(),
/// ).unwrap();
/// let _ = interp.interp(0.5);
/// ```
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
}
