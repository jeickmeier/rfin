use super::InterpFn;
use crate::{error::InputError, math::interp::ExtrapolationPolicy};

/// Piecewise linear interpolation on discount factors.
///
/// Simple linear interpolation between knot points. Fast and straightforward
/// but may produce negative forward rates (arbitrage) if discount factors
/// aren't carefully spaced. Prefer LogLinear or MonotoneConvex for yield curves.
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LinearDf {
    knots: Box<[f64]>,
    dfs: Box<[f64]>,
    extrapolation: ExtrapolationPolicy,
}

impl LinearDf {
    /// Build a **piecewise‐linear** discount‐factor interpolator.
    ///
    /// # Errors
    /// Returns [`InputError`] variants if fewer than two points are supplied
    /// or the data is invalid.
    #[allow(clippy::boxed_local)]
    pub fn new(
        knots: Box<[f64]>,
        dfs: Box<[f64]>,
        extrapolation: ExtrapolationPolicy,
    ) -> crate::Result<Self> {
        debug_assert_eq!(knots.len(), dfs.len());
        if knots.len() < 2 {
            return Err(InputError::TooFewPoints.into());
        }
        // Ensure strictly ascending times via shared helper
        crate::math::interp::utils::validate_knots(&knots)?;
        // Validate values (positive).
        crate::math::interp::utils::validate_positive_series(&dfs)?;
        Ok(Self {
            knots,
            dfs,
            extrapolation,
        })
    }

    // Shared `locate_segment` is used via utils.

    /// Get the extrapolation policy for serialization
    #[cfg(feature = "serde")]
    pub(crate) fn extrapolation(&self) -> ExtrapolationPolicy {
        self.extrapolation
    }
}

impl LinearDf {
    #[inline]
    fn segment_slope(&self, left_index: usize, right_index: usize) -> f64 {
        let x0 = self.knots[left_index];
        let x1 = self.knots[right_index];
        let y0 = self.dfs[left_index];
        let y1 = self.dfs[right_index];
        (y1 - y0) / (x1 - x0)
    }
}

impl InterpFn for LinearDf {
    fn interp(&self, x: f64) -> f64 {
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
        let idx = crate::math::interp::utils::locate_segment(&self.knots, x).unwrap();
        let x0 = self.knots[idx];
        let x1 = self.knots[idx + 1];
        let df0 = self.dfs[idx];
        let df1 = self.dfs[idx + 1];
        let w = (x - x0) / (x1 - x0);
        df0 + w * (df1 - df0)
    }

    fn interp_prime(&self, x: f64) -> f64 {
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
        let idx = crate::math::interp::utils::locate_segment(&self.knots, x).unwrap();
        let x0 = self.knots[idx];
        let x1 = self.knots[idx + 1];
        let df0 = self.dfs[idx];
        let df1 = self.dfs[idx + 1];

        // Linear interpolation derivative: constant slope within segment
        (df1 - df0) / (x1 - x0)
    }
}
