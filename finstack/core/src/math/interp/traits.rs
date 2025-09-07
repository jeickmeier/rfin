//! Interpolation traits.

use core::fmt::Debug;

use crate::F;

use super::types::DERIVATIVE_EPSILON;

/// Object-safe interpolation trait.
pub trait InterpFn: Send + Sync + Debug {
    /// Interpolate at coordinate `x`.
    fn interp(&self, x: F) -> F;

    /// First derivative at `x`. Default via central finite differences.
    fn interp_prime(&self, x: F) -> F {
        let h = (x.abs() * DERIVATIVE_EPSILON).max(1e-8);
        (self.interp(x + h) - self.interp(x - h)) / (2.0 * h)
    }
}
