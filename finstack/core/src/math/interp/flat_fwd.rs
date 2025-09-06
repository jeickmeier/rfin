use crate::{math::interp::LogLinearDf, F};
use super::InterpFn;

/// Flat-forward DF interpolator – constant instantaneous forward rate between knots.
/// Implemented via linear interpolation on log DF (equivalent behaviour).
///
/// See unit tests and `examples/` for usage.
#[derive(Debug)]
pub struct FlatFwd {
    inner: LogLinearDf,
}

impl FlatFwd {
    /// Create a **flat‐forward** interpolator (constant inst. forward rate).
    ///
    /// Internally this is equivalent to [`LogLinearDf`] which provides the
    /// same mathematical behaviour.
    #[allow(clippy::boxed_local)]
    pub fn new(
        knots: Box<[F]>,
        dfs: Box<[F]>,
        extrapolation: crate::math::interp::ExtrapolationPolicy,
    ) -> crate::Result<Self> {
        Ok(Self {
            inner: LogLinearDf::new(knots, dfs, extrapolation)?,
        })
    }
}

impl InterpFn for FlatFwd {
    fn interp(&self, x: F) -> F {
        self.inner.interp(x)
    }

    fn interp_prime(&self, x: F) -> F {
        self.inner.interp_prime(x)
    }
}
