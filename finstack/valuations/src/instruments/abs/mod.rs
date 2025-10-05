//! Asset-Backed Security (ABS) instrument module.
//!
//! Built on the shared structured credit components for pools, tranches, coverage tests,
//! and waterfall logic, providing a reusable ABS instrument representation.

pub mod metrics;
mod types;

pub use types::Abs;

// Auto-register ABS discounting pricer
inventory::submit! {
    crate::pricer::PricerRegistration {
        ctor: || Box::new(crate::instruments::common::GenericDiscountingPricer::<Abs>::new()),
    }
}
