//! Vega calculator for Asian options (generic FD).

use crate::instruments::asian_option::AsianOption;
use crate::instruments::common::metrics::GenericFdVega;

/// Type alias to the generic finite-difference vega implementation.
pub type VegaCalculator = GenericFdVega<AsianOption>;
