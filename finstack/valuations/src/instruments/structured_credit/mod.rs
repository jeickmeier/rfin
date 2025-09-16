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
//! // Build tranches
//! let equity = AbsTranche::new(
//!     "EQUITY",
//!     0.0,
//!     10.0,
//!     TrancheSeniority::Equity,
//!     Money::new(100_000_000.0, Currency::USD),
//!     TrancheCoupon::Fixed { rate: 0.15 },
//!     finstack_core::dates::Date::from_calendar_date(2030, time::Month::January, 1).unwrap(),
//! )?;
//! let senior = AbsTranche::new(
//!     "SENIOR_A",
//!     10.0,
//!     100.0,
//!     TrancheSeniority::Senior,
//!     Money::new(900_000_000.0, Currency::USD),
//!     TrancheCoupon::Floating { index: "SOFR-3M".to_string(), spread_bp: 150.0, floor: None, cap: None },
//!     finstack_core::dates::Date::from_calendar_date(2030, time::Month::January, 1).unwrap(),
//! )?;
//! let tranches = TrancheStructure::new(vec![equity, senior])?;
//! let waterfall = WaterfallBuilder::standard_clo(&tranches).build();
//! let clo = StructuredCredit::new(
//!     "CLO-2025-1",
//!     DealType::CLO,
//!     pool,
//!     tranches,
//!     waterfall,
//!     finstack_core::dates::Date::from_calendar_date(2030, time::Month::January, 1).unwrap(),
//!     "USD-OIS",
//! );
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
