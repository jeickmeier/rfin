//! Volga calculator for range accrual instruments (generic FD).

use crate::instruments::range_accrual::RangeAccrual;
use crate::metrics::GenericFdVolga;

/// Type alias to the generic finite-difference volga implementation.
pub type VolgaCalculator = GenericFdVolga<RangeAccrual>;
