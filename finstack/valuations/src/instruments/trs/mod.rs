//! Total return swap (TRS) instruments for synthetic exposure.
//!
//! Total return swaps exchange the total return (price appreciation + dividends/coupons)
//! of an underlying index for a financing rate. They provide synthetic exposure
//! without owning the underlying assets.
//!
//! # TRS Structure
//!
//! Two legs:
//! - **Total return leg**: Pay/receive index return + income
//! - **Financing leg**: Pay/receive floating rate + spread
//!
//! # Total Return Calculation
//!
//! ```text
//! Total return = (Index_end - Index_start)/Index_start + Dividends/Index_start
//! ```
//!
//! For equity TRS:
//! ```text
//! Total return = (S_T - S_0)/S_0 + Dividend_yield × T
//! ```
//!
//! For fixed income TRS:
//! ```text
//! Total return = (Price_T - Price_0)/Price_0 + Coupon_accrual
//! ```
//!
//! # Pricing
//!
//! Present value is the difference between total return and financing:
//!
//! ```text
//! PV_TRS = PV(Total return leg) - PV(Financing leg)
//! ```
//!
//! # Market Usage
//!
//! - **Synthetic long**: Gain index exposure without buying assets
//! - **Leverage**: Minimize upfront capital requirements
//! - **Regulatory capital**: May have different capital treatment
//! - **Short exposure**: Easier than borrowing securities
//!
//! # Types
//!
//! - **Equity TRS**: On equity indices (S&P 500, Euro Stoxx, etc.)
//! - **Fixed income TRS**: On bond indices
//! - **Single-name TRS**: On individual stocks or bonds
//!
//! # Key Metrics
//!
//! - **Delta**: Sensitivity to underlying index
//! - **Dividend risk**: Sensitivity to dividend changes
//! - **DV01**: Sensitivity to financing rate
//!
//! # See Also
//!
//! - [`EquityTotalReturnSwap`] for equity TRS
//! - [`FIIndexTotalReturnSwap`] for fixed income TRS
//! - [`TrsEngine`] for pricing calculations

mod equity;
mod fixed_income_index;
pub mod metrics;
pub mod pricing;
mod types;

// Re-export main types
pub use equity::EquityTotalReturnSwap;
pub use fixed_income_index::FIIndexTotalReturnSwap;
pub use pricing::engine::TrsEngine;
pub use types::{
    FinancingLegSpec, IndexUnderlyingParams, TotalReturnLegSpec, TrsScheduleSpec, TrsSide,
};

// Note: TRS helpers module removed - was empty
