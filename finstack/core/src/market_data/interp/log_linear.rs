use crate::{error::InputError, market_data::interp::InterpFn, F};
use std::vec::Vec;

/// Interpolator that performs linear interpolation on the natural logarithm
/// of discount factors (i.e. piecewise‐constant zero rates).
///
/// See unit tests and `examples/` for usage.
#[derive(Debug)]
pub struct LogLinearDf {
    knots: Box<[F]>,
    log_dfs: Box<[F]>,
}

impl LogLinearDf {
    /// Construct a **log‐linear** DF interpolator (constant zero rate).
    #[allow(clippy::boxed_local)]
    pub fn new(knots: Box<[F]>, dfs: Box<[F]>) -> crate::Result<Self> {
        debug_assert_eq!(knots.len(), dfs.len());
        if knots.len() < 2 {
            return Err(InputError::TooFewPoints.into());
        }
        crate::market_data::utils::validate_knots(&knots)?;
        crate::market_data::utils::validate_dfs(&dfs, false)?;
        let log_dfs: Vec<F> = dfs.iter().map(|d| d.ln()).collect();
        Ok(Self {
            knots,
            log_dfs: log_dfs.into_boxed_slice(),
        })
    }

    // Shared `locate_segment` from utils is used.
}

impl InterpFn for LogLinearDf {
    fn interp(&self, x: F) -> F {
        // Clamp to bounds to avoid out-of-range evaluations due to
        // small day-count or floating-point discrepancies.
        if x <= self.knots[0] {
            return (self.log_dfs[0]).exp();
        }
        if x >= *self.knots.last().unwrap() {
            return (*self.log_dfs.last().unwrap()).exp();
        }
        if let Ok(idx_exact) = self.knots.binary_search_by(|k| k.partial_cmp(&x).unwrap()) {
            return (self.log_dfs[idx_exact]).exp();
        }
        let idx = crate::market_data::utils::locate_segment(&self.knots, x).unwrap();
        let x0 = self.knots[idx];
        let x1 = self.knots[idx + 1];
        let y0 = self.log_dfs[idx];
        let y1 = self.log_dfs[idx + 1];
        let w = (x - x0) / (x1 - x0);
        (y0 + w * (y1 - y0)).exp()
    }
}
