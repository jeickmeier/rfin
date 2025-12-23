//! Rho calculator for autocallable structured products (generic).

use crate::instruments::autocallable::Autocallable;
use crate::metrics::GenericRho;

/// Type alias to the generic Rho implementation.
pub type RhoCalculator = GenericRho<Autocallable>;
