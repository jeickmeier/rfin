//! Volga calculator for lookback options (generic FD).

use crate::metrics::GenericFdVolga;
use crate::instruments::lookback_option::LookbackOption;

/// Type alias to the generic finite-difference volga implementation.
pub type VolgaCalculator = GenericFdVolga<LookbackOption>;
