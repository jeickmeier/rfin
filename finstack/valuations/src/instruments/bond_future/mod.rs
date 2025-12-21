//! Bond Future instrument implementation.
//!
//! This module provides comprehensive support for bond futures (e.g., UST Treasury futures,
//! German Bund futures, UK Gilt futures) with deliverable basket mechanics.
//!
//! # Features
//!
//! - Deliverable basket with conversion factors
//! - Cheapest-to-deliver (CTD) bond selection
//! - Invoice price calculation
//! - Contract DV01 and bucketed risk metrics
//!
//! # Example
//!
//! ```ignore
//! // TODO: Add example once implementation is complete
//! ```

pub mod types;
pub mod pricer;
pub mod metrics;

// Re-export commonly used types
pub use types::{BondFuture, BondFutureBuilder, BondFutureSpecs, DeliverableBond, Position};

#[cfg(test)]
mod tests {
    #[test]
    fn test_module_compiles() {
        // This test exists only to ensure the module compiles
    }
}
