//! Vega calculator for range accrual instruments (generic FD).

use crate::instruments::rates::range_accrual::RangeAccrual;
use crate::metrics::GenericFdVega;

/// Type alias to the generic finite-difference vega implementation.
pub type VegaCalculator = GenericFdVega<RangeAccrual>;
