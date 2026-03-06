//! Sobol RNG re-exported from finstack_core with RandomStream integration.

use crate::instruments::common_impl::models::monte_carlo::traits::RandomStream;

pub use finstack_core::math::random::sobol::{SobolRng, MAX_SOBOL_DIMENSION};

impl RandomStream for SobolRng {
    fn split(&self, stream_id: u64) -> Self {
        assert_eq!(
            stream_id, 0,
            "SobolRng::split is unsupported for nonzero stream IDs; use a single Sobol sequence \
             or switch to a pseudorandom generator with true stream splitting"
        );
        self.clone()
    }

    fn fill_u01(&mut self, out: &mut [f64]) {
        SobolRng::fill_u01(self, out);
    }

    fn fill_std_normals(&mut self, out: &mut [f64]) {
        SobolRng::fill_std_normals(self, out);
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn split_zero_preserves_sequence() {
        let rng = SobolRng::new(4, 0);
        let mut split = rng.split(0);
        let mut original = rng.clone();
        let mut a = [0.0; 4];
        let mut b = [0.0; 4];
        original.fill_u01(&mut a);
        split.fill_u01(&mut b);
        assert_eq!(a, b);
    }

    #[test]
    #[should_panic(expected = "SobolRng::split is unsupported")]
    fn split_nonzero_is_rejected() {
        let rng = SobolRng::new(4, 0);
        let _ = rng.split(1);
    }
}
