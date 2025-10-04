//! Residential Mortgage-Backed Security (RMBS) instrument module.
//!
//! Uses the shared structured credit components to represent RMBS structures with
//! mortgage-specific pool behavior and waterfall logic.

mod impl_waterfall;
mod types;

pub use types::Rmbs;

// Auto-register RMBS discounting pricer
inventory::submit! {
    crate::pricer::PricerRegistration {
        ctor: || Box::new(crate::instruments::common::GenericDiscountingPricer::<Rmbs>::new()),
    }
}
