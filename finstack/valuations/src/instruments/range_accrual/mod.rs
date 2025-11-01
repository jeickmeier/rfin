//! Range accrual notes with path-dependent coupon accrual.
//!
//! Range accrual notes pay coupons that accrue only when a reference rate
//! stays within a specified range. Popular for investors with views on
//! range-bound markets or mean reversion.
//!
//! # Structure
//!
//! - **Reference rate**: Typically LIBOR, SOFR, or other floating index
//! - **Range**: [Lower bound, Upper bound]
//! - **Observation frequency**: Daily, weekly, or business days
//! - **Accrual formula**: Coupon × (Days in range / Total days)
//!
//! # Pricing Method
//!
//! Range accruals require:
//! - **Monte Carlo**: For complex rate dynamics and barriers
//! - **Analytical approximations**: For single-factor cases
//! - **Tree methods**: For short rate models
//!
//! # Market Usage
//!
//! Common structures:
//! - **Digital range accruals**: Full coupon or zero (no proportional accrual)
//! - **Corridor notes**: Range defined by strike levels
//! - **Target redemption notes**: Early redemption when cumulative coupons hit target
//!
//! # See Also
//!
//! - [`RangeAccrual`] for instrument struct
//! - Monte Carlo pricer for rate simulation

pub mod metrics;
pub mod pricer;
pub mod traits;
pub mod types;

pub use types::RangeAccrual;
