//! Vega calculator for autocallable structured products (generic FD).

use crate::instruments::autocallable::Autocallable;
use crate::instruments::common::metrics::GenericFdVega;

/// Type alias to the generic finite-difference vega implementation.
pub type VegaCalculator = GenericFdVega<Autocallable>;
