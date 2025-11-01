//! Trait interface for interpolation functions.
//!
//! Defines the object-safe trait used by all interpolators, enabling
//! polymorphic interpolation with static or dynamic dispatch.

use core::fmt::Debug;

use super::types::DERIVATIVE_EPSILON;

/// Object-safe interpolation trait.
pub trait InterpFn: Send + Sync + Debug {
    /// Interpolate at coordinate `x`.
    fn interp(&self, x: f64) -> f64;

    /// First derivative at `x`. Default via central finite differences.
    fn interp_prime(&self, x: f64) -> f64 {
        let h = (x.abs() * DERIVATIVE_EPSILON).max(1e-8);
        (self.interp(x + h) - self.interp(x - h)) / (2.0 * h)
    }
}
