//! Fixed Income Index Total Return Swap (TRS) for synthetic bond exposure.
//!
//! This module provides the [`FIIndexTotalReturnSwap`] instrument for exchanging
//! fixed income index total return (carry + roll) for a financing rate.
//!
//! # Overview
//!
//! Total return swaps exchange the total return of an underlying index for a financing
//! rate. For fixed income index TRS:
//!
//! ```text
//! Total return = (Price_T - Price_0) / Price_0 + Coupon_accrual
//! ```
//!
//! # Use Cases
//!
//! - **Synthetic bond exposure**: Gain bond index exposure without buying bonds
//! - **Duration management**: Adjust portfolio duration synthetically
//! - **ETF replication**: Replicate bond ETF returns synthetically
//! - **Credit exposure**: Access corporate bond index returns
//!
//! # Key Metrics
//!
//! - **DurationDelta**: Sensitivity to underlying index (duration-weighted)
//! - **DV01**: Sensitivity to financing rate
//! - **BucketedDV01**: Key-rate sensitivities
//! - **ParSpread**: Spread that makes NPV = 0
//!
//! # Example
//!
//! ```
//! use finstack_valuations::instruments::fi_trs::FIIndexTotalReturnSwap;
//!
//! let trs = FIIndexTotalReturnSwap::example();
//! // let pv = trs.npv(&market_context, as_of_date)?;
//! ```
//!
//! # See Also
//!
//! - [`crate::instruments::equity_trs`] for equity TRS
//! - [`TrsEngine`](crate::instruments::common::pricing::TrsEngine) for shared pricing logic

mod types;
pub mod metrics;
pub(crate) mod pricer;

pub use types::FIIndexTotalReturnSwap;

// Re-export common TRS types for convenience
pub use crate::instruments::common::parameters::trs_common::{TrsScheduleSpec, TrsSide};

