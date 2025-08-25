use crate::{
    market_data::interp::{InterpFn, LogLinearDf},
    F,
};

/// Flat-forward DF interpolator – constant instantaneous forward rate between knots.
/// Implemented via linear interpolation on log DF (equivalent behaviour).
///
/// # Example
/// ```ignore
/// use crate::market_data::interp::FlatFwd;
/// let interp = FlatFwd::new(
///     vec![0.0, 1.0].into_boxed_slice(),
///     vec![1.0, 0.95].into_boxed_slice(),
/// ).unwrap();
/// let _ = interp.interp(0.5);
/// ```
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
    pub fn new(knots: Box<[F]>, dfs: Box<[F]>) -> crate::Result<Self> {
        Ok(Self {
            inner: LogLinearDf::new(knots, dfs)?,
        })
    }
}

impl InterpFn for FlatFwd {
    fn interp(&self, x: F) -> F {
        self.inner.interp(x)
    }
}
