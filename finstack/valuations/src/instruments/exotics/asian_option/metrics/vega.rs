//! Vega calculator for Asian options (generic FD).

use crate::instruments::exotics::asian_option::AsianOption;
use crate::metrics::GenericFdVega;

/// Type alias to the generic finite-difference vega implementation.
pub type VegaCalculator = GenericFdVega<AsianOption>;
