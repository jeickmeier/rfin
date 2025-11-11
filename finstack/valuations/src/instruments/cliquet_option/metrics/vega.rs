//! Vega calculator for cliquet options (generic FD).

use crate::instruments::cliquet_option::CliquetOption;
use crate::metrics::GenericFdVega;

/// Type alias to the generic finite-difference vega implementation.
pub type VegaCalculator = GenericFdVega<CliquetOption>;
