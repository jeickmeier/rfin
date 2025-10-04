//! Collateralized Loan Obligation (CLO) instrument module.
//!
//! Builds on the shared structured credit components (pool, tranches, coverage tests,
//! waterfall engine) to provide a fully-fledged CLO instrument.
//!
//! ```rust
//! use finstack_valuations::instruments::clo::Clo;
//! use finstack_valuations::instruments::common::structured_credit::{
//!     AssetPool, DealType, TrancheStructure, WaterfallBuilder,
//! };
//! use finstack_core::currency::Currency;
//! use finstack_core::money::Money;
//! use time::Month;
//!
//! # fn example() -> finstack_core::Result<()> {
//! let mut pool = AssetPool::new("TEST_POOL", DealType::CLO, Currency::USD);
//! // ... populate pool assets ...
//! let tranches = TrancheStructure::new(Vec::new())?;
//! let waterfall = WaterfallBuilder::standard_clo(&tranches).build();
//! let clo = Clo::new(
//!     "CLO-2025-1",
//!     pool,
//!     tranches,
//!     waterfall,
//!     finstack_core::dates::Date::from_calendar_date(2030, Month::January, 1).unwrap(),
//!     "USD-OIS",
//! );
//! # Ok(())
//! # }
//! ```

pub mod metrics;
mod types;

pub use types::Clo;

// Auto-register CLO discounting pricer
inventory::submit! {
    crate::pricer::PricerRegistration {
        ctor: || Box::new(crate::instruments::common::GenericDiscountingPricer::<Clo>::new()),
    }
}
