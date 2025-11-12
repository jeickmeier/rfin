//! Deprecated shim; use `crate::constants` instead.
//!
//! This module re-exports constants from the unified `crate::constants` module
//! to maintain backward compatibility. New code should import directly from
//! `crate::constants::{isda, time, NUMERICAL_TOLERANCE}`.

// Re-export the isda module, but we need to extend it with the additional constants
// that were previously accessed through isda_constants::
pub mod isda_constants {
    // Re-export actual ISDA constants
    pub use crate::constants::isda::*;
    
    // Re-export business days constants that were previously in this module
    pub use crate::constants::time::{
        BUSINESS_DAYS_PER_YEAR_JP, BUSINESS_DAYS_PER_YEAR_UK, BUSINESS_DAYS_PER_YEAR_US,
    };
    
    // Re-export numerical tolerance
    pub use crate::constants::NUMERICAL_TOLERANCE;
}

// Also provide top-level re-exports for direct access
pub use crate::constants::time::{
    BUSINESS_DAYS_PER_YEAR_JP, BUSINESS_DAYS_PER_YEAR_UK, BUSINESS_DAYS_PER_YEAR_US,
};
pub use crate::constants::NUMERICAL_TOLERANCE;
