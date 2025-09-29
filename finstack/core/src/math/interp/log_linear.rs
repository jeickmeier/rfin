use super::InterpFn;
use crate::{error::InputError, math::interp::ExtrapolationPolicy};
use std::vec::Vec;

/// Interpolator that performs linear interpolation on the natural logarithm
/// of discount factors (i.e. piecewise‐constant zero rates).
///
/// See unit tests and `examples/` for usage.
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LogLinearDf {
    knots: Box<[f64]>,
    #[cfg_attr(
        feature = "serde",
        serde(
            serialize_with = "serialize_log_dfs",
            deserialize_with = "deserialize_log_dfs"
        )
    )]
    log_dfs: Box<[f64]>,
    extrapolation: ExtrapolationPolicy,
}

impl LogLinearDf {
    /// Construct a **log‐linear** DF interpolator (constant zero rate).
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
        crate::math::interp::utils::validate_knots(&knots)?;
        crate::math::interp::utils::validate_positive_series(&dfs)?;
        let log_dfs: Vec<f64> = dfs.iter().map(|d| d.ln()).collect();
        Ok(Self {
            knots,
            log_dfs: log_dfs.into_boxed_slice(),
            extrapolation,
        })
    }

    #[inline]
    fn segment_slope(&self, left_index: usize, right_index: usize) -> f64 {
        let x0 = self.knots[left_index];
        let x1 = self.knots[right_index];
        let y0 = self.log_dfs[left_index];
        let y1 = self.log_dfs[right_index];
        (y1 - y0) / (x1 - x0)
    }

    // Shared `locate_segment` from utils is used.

    /// Get the extrapolation policy for serialization
    #[cfg(feature = "serde")]
    pub(crate) fn extrapolation(&self) -> ExtrapolationPolicy {
        self.extrapolation
    }
}

impl InterpFn for LogLinearDf {
    fn interp(&self, x: f64) -> f64 {
        // Handle extrapolation based on policy
        if x <= self.knots[0] {
            return match self.extrapolation {
                ExtrapolationPolicy::FlatZero => (self.log_dfs[0]).exp(),
                ExtrapolationPolicy::FlatForward => {
                    let slope = self.segment_slope(0, 1);
                    let extrapolated_log_df = self.log_dfs[0] + slope * (x - self.knots[0]);
                    extrapolated_log_df.exp()
                }
            };
        }
        if x >= *self.knots.last().unwrap() {
            let n = self.knots.len();
            return match self.extrapolation {
                ExtrapolationPolicy::FlatZero => (*self.log_dfs.last().unwrap()).exp(),
                ExtrapolationPolicy::FlatForward => {
                    let slope = self.segment_slope(n - 2, n - 1);
                    let extrapolated_log_df = self.log_dfs[n - 1] + slope * (x - self.knots[n - 1]);
                    extrapolated_log_df.exp()
                }
            };
        }

        // Exact knot match
        if let Ok(idx_exact) = self.knots.binary_search_by(|k| k.partial_cmp(&x).unwrap()) {
            return (self.log_dfs[idx_exact]).exp();
        }

        // Interior interpolation
        let idx = crate::math::interp::utils::locate_segment(&self.knots, x).unwrap();
        let x0 = self.knots[idx];
        let x1 = self.knots[idx + 1];
        let y0 = self.log_dfs[idx];
        let y1 = self.log_dfs[idx + 1];
        let w = (x - x0) / (x1 - x0);
        (y0 + w * (y1 - y0)).exp()
    }

    fn interp_prime(&self, x: f64) -> f64 {
        // For log-linear interpolation: f(x) = exp(y0 + w*(y1-y0)) where w = (x-x0)/(x1-x0)
        // The derivative is: df/dx = f(x) * (y1-y0)/(x1-x0)

        // At boundaries, handle based on extrapolation policy
        if x <= self.knots[0] {
            return match self.extrapolation {
                ExtrapolationPolicy::FlatZero => 0.0,
                ExtrapolationPolicy::FlatForward => {
                    let slope = self.segment_slope(0, 1);
                    let f_val = self.interp(x);
                    f_val * slope
                }
            };
        }
        if x >= *self.knots.last().unwrap() {
            let n = self.knots.len();
            return match self.extrapolation {
                ExtrapolationPolicy::FlatZero => 0.0,
                ExtrapolationPolicy::FlatForward => {
                    let slope = self.segment_slope(n - 2, n - 1);
                    let f_val = self.interp(x);
                    f_val * slope
                }
            };
        }

        // Get the interpolated value and log-linear slope
        let f_val = self.interp(x);
        let idx = crate::math::interp::utils::locate_segment(&self.knots, x).unwrap();
        let x0 = self.knots[idx];
        let x1 = self.knots[idx + 1];
        let y0 = self.log_dfs[idx];
        let y1 = self.log_dfs[idx + 1];

        // Derivative: f(x) * (slope in log space)
        f_val * (y1 - y0) / (x1 - x0)
    }
}

#[cfg(feature = "serde")]
fn serialize_log_dfs<S>(log_dfs: &[f64], serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::Serialize;
    let dfs: Vec<f64> = log_dfs.iter().map(|log_df| log_df.exp()).collect();
    dfs.serialize(serializer)
}

#[cfg(feature = "serde")]
fn deserialize_log_dfs<'de, D>(deserializer: D) -> Result<Box<[f64]>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    let dfs: Vec<f64> = Vec::deserialize(deserializer)?;
    let log_dfs: Vec<f64> = dfs.iter().map(|df| df.ln()).collect();
    Ok(log_dfs.into_boxed_slice())
}
