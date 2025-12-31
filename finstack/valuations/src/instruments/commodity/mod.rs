//! Commodity derivatives: forwards, swaps, and options.
//!
//! This module provides commodity derivative instruments for energy, metals,
//! and agricultural products. Pricing uses forward curves with cost-of-carry
//! adjustments for physical settlement considerations.
//!
//! # Features
//!
//! - **Forwards/Futures**: Physical and cash-settled commodity forwards
//! - **Swaps**: Fixed-for-floating commodity price swaps
//! - **Options**: European options on commodity forwards
//!
//! # Commodity Markets
//!
//! Supports major commodity classes:
//! - **Energy**: Crude oil (WTI, Brent), natural gas, refined products
//! - **Metals**: Gold, silver, copper, aluminum
//! - **Agriculture**: Corn, wheat, soybeans
//!
//! # Pricing Framework
//!
//! Commodity instruments are priced using:
//! - **Forward curves**: Term structure of forward prices
//! - **Convenience yield**: Storage cost and availability premium
//! - **Black (1976)**: Standard model for commodity options
//!
//! # Quick Example
//!
//! ```rust,no_run
//! use finstack_valuations::instruments::commodity::CommodityForward;
//! use finstack_core::currency::Currency;
//! use finstack_core::money::Money;
//! use time::macros::date;
//!
//! let forward = CommodityForward::new(
//!     "WTI-DEC25",
//!     "CL",  // WTI Crude
//!     date!(2025-12-15),
//!     75.50,  // Forward price
//!     1000.0, // Quantity (barrels)
//!     Money::new(1_000_000.0, Currency::USD),
//!     "USD-OIS",
//!     "CL-FORWARD",
//! );
//! ```
//!
//! # References
//!
//! - Black, F. (1976). "The Pricing of Commodity Contracts."
//! - Schwartz, E. S. (1997). "The Stochastic Behavior of Commodity Prices."
//!
//! # See Also
//!
//! - [`CommodityForward`] for forwards and futures
//! - [`CommoditySwap`] for fixed-float swaps
//! - [`CommodityOption`] for commodity options

/// Commodity forward module.
pub mod commodity_forward;
/// Commodity option module.
pub mod commodity_option;
/// Commodity swap module.
pub mod commodity_swap;

// Re-export primary types
pub use commodity_forward::CommodityForward;
pub use commodity_option::CommodityOption;
pub use commodity_swap::CommoditySwap;
