//! Vega calculator for barrier options (generic FD).

use crate::instruments::barrier_option::BarrierOption;
use crate::metrics::GenericFdVega;

/// Type alias to the generic finite-difference vega implementation.
pub type VegaCalculator = GenericFdVega<BarrierOption>;
