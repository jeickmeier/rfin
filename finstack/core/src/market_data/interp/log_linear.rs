use crate::{error::InputError, market_data::interp::{InterpFn, ExtrapolationPolicy}, F};
use std::vec::Vec;

/// Interpolator that performs linear interpolation on the natural logarithm
/// of discount factors (i.e. piecewise‐constant zero rates).
///
/// See unit tests and `examples/` for usage.
#[derive(Debug)]
pub struct LogLinearDf {
    knots: Box<[F]>,
    log_dfs: Box<[F]>,
    extrapolation: ExtrapolationPolicy,
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
            extrapolation: ExtrapolationPolicy::default(),
        })
    }

    /// Extrapolate to the left of the first knot based on the extrapolation policy.
    fn extrapolate_left(&self, x: F) -> F {
        match self.extrapolation {
            ExtrapolationPolicy::FlatZero => (self.log_dfs[0]).exp(),
            ExtrapolationPolicy::FlatForward => {
                // Flat-forward: extend the zero rate from the first segment
                let x0 = self.knots[0];
                let x1 = self.knots[1];
                let y0 = self.log_dfs[0];
                let y1 = self.log_dfs[1];
                let slope = (y1 - y0) / (x1 - x0);
                let extrapolated_log_df = y0 + slope * (x - x0);
                extrapolated_log_df.exp()
            }
        }
    }

    /// Extrapolate to the right of the last knot based on the extrapolation policy.
    fn extrapolate_right(&self, x: F) -> F {
        match self.extrapolation {
            ExtrapolationPolicy::FlatZero => (*self.log_dfs.last().unwrap()).exp(),
            ExtrapolationPolicy::FlatForward => {
                // Flat-forward: extend the zero rate from the last segment
                let n = self.knots.len();
                let x0 = self.knots[n - 2];
                let x1 = self.knots[n - 1];
                let y0 = self.log_dfs[n - 2];
                let y1 = self.log_dfs[n - 1];
                let slope = (y1 - y0) / (x1 - x0);
                let extrapolated_log_df = y1 + slope * (x - x1);
                extrapolated_log_df.exp()
            }
        }
    }

    /// Compute the derivative for left extrapolation.
    fn extrapolate_left_prime(&self, x: F) -> F {
        match self.extrapolation {
            ExtrapolationPolicy::FlatZero => 0.0, // Flat extrapolation has zero derivative
            ExtrapolationPolicy::FlatForward => {
                // Flat-forward: constant slope from first segment
                let x0 = self.knots[0];
                let x1 = self.knots[1];
                let y0 = self.log_dfs[0];
                let y1 = self.log_dfs[1];
                let slope = (y1 - y0) / (x1 - x0);
                let f_val = self.extrapolate_left(x);
                f_val * slope
            }
        }
    }

    /// Compute the derivative for right extrapolation.
    fn extrapolate_right_prime(&self, x: F) -> F {
        match self.extrapolation {
            ExtrapolationPolicy::FlatZero => 0.0, // Flat extrapolation has zero derivative
            ExtrapolationPolicy::FlatForward => {
                // Flat-forward: constant slope from last segment
                let n = self.knots.len();
                let x0 = self.knots[n - 2];
                let x1 = self.knots[n - 1];
                let y0 = self.log_dfs[n - 2];
                let y1 = self.log_dfs[n - 1];
                let slope = (y1 - y0) / (x1 - x0);
                let f_val = self.extrapolate_right(x);
                f_val * slope
            }
        }
    }

    // Shared `locate_segment` from utils is used.
}

impl InterpFn for LogLinearDf {
    fn interp(&self, x: F) -> F {
        // Handle extrapolation based on policy
        if x <= self.knots[0] {
            return self.extrapolate_left(x);
        }
        if x >= *self.knots.last().unwrap() {
            return self.extrapolate_right(x);
        }
        
        // Exact knot match
        if let Ok(idx_exact) = self.knots.binary_search_by(|k| k.partial_cmp(&x).unwrap()) {
            return (self.log_dfs[idx_exact]).exp();
        }
        
        // Interior interpolation
        let idx = crate::market_data::utils::locate_segment(&self.knots, x).unwrap();
        let x0 = self.knots[idx];
        let x1 = self.knots[idx + 1];
        let y0 = self.log_dfs[idx];
        let y1 = self.log_dfs[idx + 1];
        let w = (x - x0) / (x1 - x0);
        (y0 + w * (y1 - y0)).exp()
    }

    fn interp_prime(&self, x: F) -> F {
        // For log-linear interpolation: f(x) = exp(y0 + w*(y1-y0)) where w = (x-x0)/(x1-x0)
        // The derivative is: df/dx = f(x) * (y1-y0)/(x1-x0)
        
        // At boundaries, handle based on extrapolation policy
        if x <= self.knots[0] {
            return self.extrapolate_left_prime(x);
        }
        if x >= *self.knots.last().unwrap() {
            return self.extrapolate_right_prime(x);
        }
        
        // Get the interpolated value and log-linear slope
        let f_val = self.interp(x);
        let idx = crate::market_data::utils::locate_segment(&self.knots, x).unwrap();
        let x0 = self.knots[idx];
        let x1 = self.knots[idx + 1];
        let y0 = self.log_dfs[idx];
        let y1 = self.log_dfs[idx + 1];
        
        // Derivative: f(x) * (slope in log space)
        f_val * (y1 - y0) / (x1 - x0)
    }

    fn set_extrapolation_policy(&mut self, policy: ExtrapolationPolicy) {
        self.extrapolation = policy;
    }

    fn extrapolation_policy(&self) -> ExtrapolationPolicy {
        self.extrapolation
    }
}
