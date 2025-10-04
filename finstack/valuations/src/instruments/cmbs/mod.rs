//! Commercial Mortgage-Backed Security (CMBS) instrument module.
//!
//! Wraps the shared structured credit engine to model CMBS transactions with
//! commercial mortgage pools and tranche waterfalls.

mod impl_waterfall;
mod types;

pub use types::Cmbs;

// Auto-register CMBS discounting pricer
inventory::submit! {
    crate::pricer::PricerRegistration {
        ctor: || Box::new(crate::instruments::common::GenericDiscountingPricer::<Cmbs>::new()),
    }
}
