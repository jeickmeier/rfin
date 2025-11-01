//! Volga calculator for autocallables (generic FD).

use crate::instruments::autocallable::Autocallable;
use crate::instruments::common::metrics::GenericFdVolga;

/// Type alias to the generic finite-difference volga implementation.
pub type VolgaCalculator = GenericFdVolga<Autocallable>;
