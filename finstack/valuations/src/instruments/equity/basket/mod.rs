//! Generic basket instrument for ETFs and multi-asset baskets.
//!
//! This module provides a unified basket instrument that can handle various asset types
//! including equities, bonds, ETFs, and other instruments. It leverages existing
//! pricing infrastructure to avoid code duplication and ensure consistency.
//!
//! # Features
//!
//! - **Multi-asset support**: Equity, bond, ETF, cash, and derivative constituents
//! - **Flexible weighting**: Weight-based or unit-based position sizing
//! - **Existing pricing**: Uses existing Bond and Equity instrument pricing
//! - **NAV calculation**: Real-time net asset value calculation
//! - **Tracking error**: Analysis vs benchmark indices
//! - **Creation/redemption**: Modeling of ETF creation and redemption mechanics
//!
//! # Examples
//!
//! ## Equity ETF (like SPY)
//! ```no_run
//! use finstack_valuations::instruments::equity::basket::*;
//! use finstack_core::prelude::*;
//!
//! let spy = BasketBuilder::equity_etf("SPY", "SPY", "SPDR S&P 500 ETF")
//!     .add_market_data("AAPL", "AAPL", AssetType::Equity, 0.5, Some(150000.0))
//!     .add_market_data("MSFT", "MSFT", AssetType::Equity, 0.5, Some(120000.0))
//!     .build()
//!     .unwrap();
//! ```
//!
//! ## Bond ETF (like LQD)
//! ```no_run
//! use finstack_valuations::instruments::equity::basket::*;
//! use finstack_valuations::instruments::fixed_income::bond::Bond;
//! use finstack_core::prelude::*;
//! use finstack_core::dates::Date;
//! use time::Month;
//!
//! let issue_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
//! let maturity_date = Date::from_calendar_date(2030, Month::January, 1).unwrap();
//! let bond = Bond::fixed_semiannual("AAPL_BOND", Money::new(1000.0, Currency::USD), 
//!                                   0.025, issue_date, maturity_date, "USD-OIS");
//!
//! let lqd = BasketBuilder::bond_etf("LQD", "LQD", "iShares iBoxx $ IG Corporate Bond ETF")
//!     .add_bond("AAPL_BOND", bond, 1.0, Some(15000.0))
//!     .build()
//!     .unwrap();
//! ```

pub mod builder;
pub mod metrics;
pub mod types;

// Re-export main types for convenience
pub use builder::BasketBuilder;
pub use metrics::register_basket_metrics;
pub use types::{AssetType, Basket, BasketConstituent, ConstituentReference, ReplicationMethod};
