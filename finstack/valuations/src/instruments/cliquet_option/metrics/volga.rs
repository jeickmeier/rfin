//! Volga calculator for cliquet options (generic FD).

use crate::instruments::cliquet_option::CliquetOption;
use crate::metrics::GenericFdVolga;

/// Type alias to the generic finite-difference volga implementation.
pub type VolgaCalculator = GenericFdVolga<CliquetOption>;
