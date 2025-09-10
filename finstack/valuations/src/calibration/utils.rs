//! Utility types for calibration framework.

use finstack_core::F;
use ordered_float::OrderedFloat;

/// Type alias for hashable floating point values used as HashMap keys.
///
/// Uses OrderedFloat which provides total ordering and hashing for f64 values.
/// This simplifies the code compared to a custom HashableFloat implementation.
pub type HashableFloat = OrderedFloat<F>;
