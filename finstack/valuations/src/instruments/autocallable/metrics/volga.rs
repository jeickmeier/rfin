//! Volga calculator for autocallables (generic FD).

use crate::instruments::autocallable::Autocallable;
use crate::metrics::GenericFdVolga;

/// Type alias to the generic finite-difference volga implementation.
pub type VolgaCalculator = GenericFdVolga<Autocallable>;
