//! Vega calculator for range accrual instruments (generic FD).

use crate::instruments::common::metrics::GenericFdVega;
use crate::instruments::range_accrual::RangeAccrual;

/// Type alias to the generic finite-difference vega implementation.
pub type VegaCalculator = GenericFdVega<RangeAccrual>;
