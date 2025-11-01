//! Volga calculator for Asian options (generic FD).

use crate::instruments::asian_option::AsianOption;
use crate::instruments::common::metrics::GenericFdVolga;

/// Type alias to the generic finite-difference volga implementation.
pub type VolgaCalculator = GenericFdVolga<AsianOption>;
