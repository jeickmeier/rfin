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
//! ```rust,no_run
//! use finstack_valuations::instruments::fixed_income::bond_future::{
//!     BondFuture, DeliverableBond, Position,
//! };
//! use finstack_core::money::Money;
//! use finstack_core::currency::Currency;
//! use finstack_core::types::{InstrumentId, CurveId};
//! use time::macros::date;
//!
//! let future = BondFuture::ust_10y(
//!     InstrumentId::new("TYH5"),
//!     Money::new(1_000_000.0, Currency::USD),
//!     date!(2025-03-20),
//!     date!(2025-03-21),
//!     date!(2025-03-31),
//!     125.50,
//!     Position::Long,
//!     vec![DeliverableBond {
//!         bond_id: InstrumentId::new("US912828XG33"),
//!         conversion_factor: 0.8234,
//!     }],
//!     InstrumentId::new("US912828XG33"),
//!     CurveId::new("USD-TREASURY"),
//! ).expect("Valid bond future");
//! ```

pub(crate) mod metrics;
pub(crate) mod pricer;
pub(crate) mod types;

// Re-export commonly used types
pub use pricer::BondFuturePricer;
pub use types::{BondFuture, BondFutureBuilder, BondFutureSpecs, DeliverableBond, Position};

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    #[test]
    fn test_module_compiles() {
        // This test exists only to ensure the module compiles
    }
}
