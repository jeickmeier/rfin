//! Volga calculator for lookback options (generic FD).

use crate::instruments::lookback_option::LookbackOption;
use crate::instruments::common::metrics::GenericFdVolga;

/// Type alias to the generic finite-difference volga implementation.
pub type VolgaCalculator = GenericFdVolga<LookbackOption>;
