//! Vega calculator for range accrual instruments (generic FD).

use crate::instruments::range_accrual::RangeAccrual;
use crate::instruments::common::metrics::GenericFdVega;

/// Type alias to the generic finite-difference vega implementation.
pub type VegaCalculator = GenericFdVega<RangeAccrual>;
