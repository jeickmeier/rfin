//! Volga calculator for Asian options (generic FD).

use crate::instruments::asian_option::AsianOption;
use crate::metrics::GenericFdVolga;

/// Type alias to the generic finite-difference volga implementation.
pub type VolgaCalculator = GenericFdVolga<AsianOption>;
