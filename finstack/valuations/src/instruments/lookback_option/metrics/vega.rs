//! Vega calculator for lookback options (generic FD).

use crate::instruments::lookback_option::LookbackOption;
use crate::metrics::GenericFdVega;

/// Type alias to the generic finite-difference vega implementation.
pub type VegaCalculator = GenericFdVega<LookbackOption>;
