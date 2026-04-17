//! Sobol RNG re-exported from finstack_core with RandomStream integration.

use crate::traits::RandomStream;

pub use finstack_core::math::random::sobol::{SobolRng, MAX_SOBOL_DIMENSION};

impl RandomStream for SobolRng {
    /// Sobol sequences do not admit independent sub-streams. Any caller that
    /// reaches this method — regardless of `stream_id` — has a conceptual
    /// bug, so we panic unconditionally.
    ///
    /// The previous behaviour of returning a clone for `stream_id == 0`
    /// silently produced correlated "parallel" paths. Callers that need a
    /// single Sobol sequence should use the generator directly without
    /// splitting; callers that need stream splitting must switch to a
    /// pseudorandom generator such as [`crate::rng::philox::PhiloxRng`].
    fn split(&self, stream_id: u64) -> Self {
        panic!(
            "SobolRng::split called with stream_id={stream_id}; Sobol sequences cannot be split \
             into independent streams. Use a single Sobol sequence or switch to PhiloxRng for \
             parallel execution."
        );
    }

    fn fill_u01(&mut self, out: &mut [f64]) {
        SobolRng::fill_u01(self, out);
    }

    fn fill_std_normals(&mut self, out: &mut [f64]) {
        SobolRng::fill_std_normals(self, out);
    }

    /// Sobol sequences cannot be split into independent streams.
    /// Always returns false.
    fn supports_splitting(&self) -> bool {
        false
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "SobolRng::split called")]
    fn split_zero_is_rejected() {
        let rng = SobolRng::try_new(4, 0).expect("valid Sobol dimension");
        let _ = rng.split(0);
    }

    #[test]
    #[should_panic(expected = "SobolRng::split called")]
    fn split_nonzero_is_rejected() {
        let rng = SobolRng::try_new(4, 0).expect("valid Sobol dimension");
        let _ = rng.split(1);
    }
}
