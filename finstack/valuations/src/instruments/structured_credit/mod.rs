//! Structured credit instruments: CLO, ABS, and related securities.
//!
//! This module provides comprehensive modeling for collateralized loan obligations (CLOs),
//! asset-backed securities (ABS), and other structured credit products.
//!
//! ## Key Features
//!
//! - **Asset Pool Management**: Leverage existing loan/bond instruments as pool assets
//! - **Tranche Subordination**: Full attachment/detachment point modeling with triggers
//! - **Waterfall Logic**: Interest and principal distribution with coverage tests
//! - **Pool Behavior**: Prepayment/default/recovery modeling using existing simulation
//! - **Risk Metrics**: Comprehensive analytics including DV01, duration, and expected loss
//!
//! ## Example
//!
//! ```rust
//! use finstack_valuations::instruments::structured_credit::*;
//! use finstack_core::money::Money;
//! use finstack_core::currency::Currency;
//! 
//! // Create a simple CLO with equity and senior tranches
//! # fn example() -> finstack_core::Result<()> {
//! # let pool = AssetPool::new("TEST_POOL", DealType::CLO, Currency::USD);
//! let clo = StructuredCredit::builder("CLO-2025-1", DealType::CLO)
//!     .pool(pool)
//!     .add_equity_tranche(0.0, 10.0, Money::new(100_000_000.0, Currency::USD), 0.15)
//!     .add_senior_tranche(10.0, 100.0, Money::new(900_000_000.0, Currency::USD), 150.0)
//!     .legal_maturity(finstack_core::dates::Date::from_calendar_date(2030, time::Month::January, 1).unwrap())
//!     .disc_id("USD-OIS")
//!     .build()?;
//! # Ok(())
//! # }
//! ```

pub mod coverage_tests;
pub mod pool;
pub mod tranches;
pub mod types;
pub mod waterfall;

// Re-export main types for convenience
pub use types::*;
pub use pool::*;
pub use tranches::*;
pub use waterfall::*;
pub use coverage_tests::*;
