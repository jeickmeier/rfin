//! Vega calculator for autocallable structured products (generic FD).

use crate::instruments::autocallable::Autocallable;
use crate::metrics::GenericFdVega;

/// Type alias to the generic finite-difference vega implementation.
pub type VegaCalculator = GenericFdVega<Autocallable>;
