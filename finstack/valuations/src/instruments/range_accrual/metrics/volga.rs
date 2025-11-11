//! Volga calculator for range accrual instruments (generic FD).

use crate::metrics::GenericFdVolga;
use crate::instruments::range_accrual::RangeAccrual;

/// Type alias to the generic finite-difference volga implementation.
pub type VolgaCalculator = GenericFdVolga<RangeAccrual>;
