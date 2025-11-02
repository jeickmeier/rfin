//! Vega calculator for lookback options (generic FD).

use crate::instruments::common::metrics::GenericFdVega;
use crate::instruments::lookback_option::LookbackOption;

/// Type alias to the generic finite-difference vega implementation.
pub type VegaCalculator = GenericFdVega<LookbackOption>;
