use super::InterpFn;
use crate::math::interp::LogLinearDf;

/// Flat-forward discount factor interpolation.
///
/// Assumes constant instantaneous forward rates between knots. Mathematically
/// equivalent to log-linear interpolation. Commonly used for OIS curves and
/// ensures no-arbitrage by construction (positive forwards guaranteed).
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
        knots: Box<[f64]>,
        dfs: Box<[f64]>,
        extrapolation: crate::math::interp::ExtrapolationPolicy,
    ) -> crate::Result<Self> {
        Ok(Self {
            inner: LogLinearDf::new(knots, dfs, extrapolation)?,
        })
    }

    /// Get the extrapolation policy for serialization
    #[cfg(feature = "serde")]
    pub(crate) fn extrapolation(&self) -> crate::math::interp::ExtrapolationPolicy {
        self.inner.extrapolation()
    }
}

impl InterpFn for FlatFwd {
    fn interp(&self, x: f64) -> f64 {
        self.inner.interp(x)
    }

    fn interp_prime(&self, x: f64) -> f64 {
        self.inner.interp_prime(x)
    }
}
