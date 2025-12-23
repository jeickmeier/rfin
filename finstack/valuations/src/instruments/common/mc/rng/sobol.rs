//! Sobol RNG re-exported from finstack_core with RandomStream integration.

use crate::instruments::common::mc::traits::RandomStream;

pub use finstack_core::math::random::sobol::{SobolRng, MAX_SOBOL_DIMENSION};

impl RandomStream for SobolRng {
    fn split(&self, stream_id: u64) -> Self {
        let mut new_rng = self.clone();
        new_rng.skip(stream_id * 10000);
        new_rng
    }

    fn fill_u01(&mut self, out: &mut [f64]) {
        SobolRng::fill_u01(self, out);
    }

    fn fill_std_normals(&mut self, out: &mut [f64]) {
        SobolRng::fill_std_normals(self, out);
    }
}
