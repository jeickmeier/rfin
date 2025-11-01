//! Volga calculator for barrier options (generic FD).

use crate::instruments::barrier_option::BarrierOption;
use crate::instruments::common::metrics::GenericFdVolga;

/// Type alias to the generic finite-difference volga implementation.
pub type VolgaCalculator = GenericFdVolga<BarrierOption>;
