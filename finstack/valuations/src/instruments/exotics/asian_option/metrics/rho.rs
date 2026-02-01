//! Rho calculator for Asian options (generic).

use crate::instruments::exotics::asian_option::AsianOption;
use crate::metrics::GenericRho;

/// Type alias to the generic Rho implementation.
pub type RhoCalculator = GenericRho<AsianOption>;
