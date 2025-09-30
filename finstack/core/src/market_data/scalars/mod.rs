//! Scalar market data and time series helpers.
//!
//! The submodules expose lightweight types that complement the full
//! term-structure framework.  They cover single-value quotes (`MarketScalar`),
//! generic time series (`ScalarTimeSeries`), and inflation-specific indices.
//!
//! # Examples
//! ```rust
//! use finstack_core::market_data::scalars::{MarketScalar, ScalarTimeSeries, SeriesInterpolation};
//! use finstack_core::dates::Date;
//! use finstack_core::currency::Currency;
//! use time::Month;
//!
//! // 1. Store a spot quote as a scalar
//! let spot = MarketScalar::Price(finstack_core::money::Money::new(101.5, Currency::USD));
//! if let MarketScalar::Price(m) = &spot {
//!     assert_eq!(m.currency(), Currency::USD);
//! }
//!
//! // 2. Build a small time series with linear interpolation
//! let ts = ScalarTimeSeries::new(
//!     "US CPI",
//!     vec![
//!         (Date::from_calendar_date(2024, Month::January, 31).unwrap(), 100.0),
//!         (Date::from_calendar_date(2024, Month::February, 29).unwrap(), 101.0),
//!     ],
//!     None,
//! )
//! .unwrap()
//! .with_interpolation(SeriesInterpolation::Linear);
//! let mid = Date::from_calendar_date(2024, Month::February, 14).unwrap();
//! let value = ts.value_on(mid).unwrap();
//! assert!(value > 100.0 && value < 101.0);
//! ```

/// Lightweight storage for time series data.
mod storage;

/// Generic market primitives: scalars and ad-hoc time series.
pub mod primitives;

/// Inflation index data (CPI/RPI) with lagging and seasonality support.
pub mod inflation_index;

// Re-export for ergonomic access
pub use inflation_index::*;
pub use primitives::*;
