use crate::{error::InputError, market_data::interp::{InterpFn, ExtrapolationPolicy}, F};

/// Piece-wise linear interpolation on discount factors.
///
/// See unit tests and `examples/` for usage.
#[derive(Debug)]
pub struct LinearDf {
    knots: Box<[F]>,
    dfs: Box<[F]>,
    extrapolation: ExtrapolationPolicy,
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
        Ok(Self { 
            knots, 
            dfs, 
            extrapolation: ExtrapolationPolicy::default() 
        })
    }







    // Shared `locate_segment` is used via utils.
}

impl LinearDf {
    #[inline]
    fn segment_slope(&self, left_index: usize, right_index: usize) -> F {
        let x0 = self.knots[left_index];
        let x1 = self.knots[right_index];
        let y0 = self.dfs[left_index];
        let y1 = self.dfs[right_index];
        (y1 - y0) / (x1 - x0)
    }
}

impl InterpFn for LinearDf {
    fn interp(&self, x: F) -> F {
        // Handle extrapolation based on policy
        if x <= self.knots[0] {
            return match self.extrapolation {
                ExtrapolationPolicy::FlatZero => self.dfs[0],
                ExtrapolationPolicy::FlatForward => {
                    let slope = self.segment_slope(0, 1);
                    let x0 = self.knots[0];
                    self.dfs[0] + slope * (x - x0)
                }
            };
        }
        if x >= *self.knots.last().unwrap() {
            let n = self.knots.len();
            return match self.extrapolation {
                ExtrapolationPolicy::FlatZero => *self.dfs.last().unwrap(),
                ExtrapolationPolicy::FlatForward => {
                    let slope = self.segment_slope(n - 2, n - 1);
                    let x1 = self.knots[n - 1];
                    self.dfs[n - 1] + slope * (x - x1)
                }
            };
        }
        
        // Exact knot match
        if let Ok(idx_exact) = self.knots.binary_search_by(|k| k.partial_cmp(&x).unwrap()) {
            return self.dfs[idx_exact];
        }
        
        // Interior linear interpolation
        let idx = crate::market_data::utils::locate_segment(&self.knots, x).unwrap();
        let x0 = self.knots[idx];
        let x1 = self.knots[idx + 1];
        let df0 = self.dfs[idx];
        let df1 = self.dfs[idx + 1];
        let w = (x - x0) / (x1 - x0);
        df0 + w * (df1 - df0)
    }

    fn interp_prime(&self, x: F) -> F {
        // Handle extrapolation based on policy
        if x <= self.knots[0] {
            return match self.extrapolation {
                ExtrapolationPolicy::FlatZero => 0.0,
                ExtrapolationPolicy::FlatForward => self.segment_slope(0, 1),
            };
        }
        if x >= *self.knots.last().unwrap() {
            let n = self.knots.len();
            return match self.extrapolation {
                ExtrapolationPolicy::FlatZero => 0.0,
                ExtrapolationPolicy::FlatForward => self.segment_slope(n - 2, n - 1),
            };
        }
        
        // Interior linear interpolation derivative
        let idx = crate::market_data::utils::locate_segment(&self.knots, x).unwrap();
        let x0 = self.knots[idx];
        let x1 = self.knots[idx + 1];
        let df0 = self.dfs[idx];
        let df1 = self.dfs[idx + 1];
        
        // Linear interpolation derivative: constant slope within segment
        (df1 - df0) / (x1 - x0)
    }

    fn set_extrapolation_policy(&mut self, policy: ExtrapolationPolicy) {
        self.extrapolation = policy;
    }

    fn extrapolation_policy(&self) -> ExtrapolationPolicy {
        self.extrapolation
    }
}
