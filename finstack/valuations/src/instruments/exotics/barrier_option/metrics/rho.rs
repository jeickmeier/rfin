//! Rho calculator for barrier options (generic).

use crate::instruments::exotics::barrier_option::BarrierOption;
use crate::metrics::GenericRho;

/// Type alias to the generic Rho implementation.
pub type RhoCalculator = GenericRho<BarrierOption>;
