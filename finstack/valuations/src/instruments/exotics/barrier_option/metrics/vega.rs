//! Vega calculator for barrier options (generic FD).

use crate::instruments::exotics::barrier_option::BarrierOption;
use crate::metrics::GenericFdVega;

/// Type alias to the generic finite-difference vega implementation.
pub type VegaCalculator = GenericFdVega<BarrierOption>;
