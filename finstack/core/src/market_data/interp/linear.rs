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

    /// Extrapolate to the left of the first knot based on the extrapolation policy.
    fn extrapolate_left(&self, x: F) -> F {
        match self.extrapolation {
            ExtrapolationPolicy::FlatZero => self.dfs[0],
            ExtrapolationPolicy::FlatForward => {
                // Flat-forward: extend using the slope from the first segment
                let x0 = self.knots[0];
                let x1 = self.knots[1];
                let df0 = self.dfs[0];
                let df1 = self.dfs[1];
                let slope = (df1 - df0) / (x1 - x0);
                // Linear extrapolation: f(x) = f(x0) + slope * (x - x0)
                df0 + slope * (x - x0)
            }
        }
    }

    /// Extrapolate to the right of the last knot based on the extrapolation policy.
    fn extrapolate_right(&self, x: F) -> F {
        match self.extrapolation {
            ExtrapolationPolicy::FlatZero => *self.dfs.last().unwrap(),
            ExtrapolationPolicy::FlatForward => {
                // Flat-forward: extend using the slope from the last segment
                let n = self.knots.len();
                let x0 = self.knots[n - 2];
                let x1 = self.knots[n - 1];
                let df0 = self.dfs[n - 2];
                let df1 = self.dfs[n - 1];
                let slope = (df1 - df0) / (x1 - x0);
                // Linear extrapolation: f(x) = f(x1) + slope * (x - x1)
                df1 + slope * (x - x1)
            }
        }
    }

    /// Compute the derivative for left extrapolation.
    fn extrapolate_left_prime(&self, _x: F) -> F {
        match self.extrapolation {
            ExtrapolationPolicy::FlatZero => 0.0, // Flat extrapolation has zero derivative
            ExtrapolationPolicy::FlatForward => {
                // Flat-forward: constant slope from first segment
                let x0 = self.knots[0];
                let x1 = self.knots[1];
                let df0 = self.dfs[0];
                let df1 = self.dfs[1];
                (df1 - df0) / (x1 - x0)
            }
        }
    }

    /// Compute the derivative for right extrapolation.
    fn extrapolate_right_prime(&self, _x: F) -> F {
        match self.extrapolation {
            ExtrapolationPolicy::FlatZero => 0.0, // Flat extrapolation has zero derivative
            ExtrapolationPolicy::FlatForward => {
                // Flat-forward: constant slope from last segment
                let n = self.knots.len();
                let x0 = self.knots[n - 2];
                let x1 = self.knots[n - 1];
                let df0 = self.dfs[n - 2];
                let df1 = self.dfs[n - 1];
                (df1 - df0) / (x1 - x0)
            }
        }
    }

    // Shared `locate_segment` is used via utils.
}

impl InterpFn for LinearDf {
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
            return self.extrapolate_left_prime(x);
        }
        if x >= *self.knots.last().unwrap() {
            return self.extrapolate_right_prime(x);
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
