//! CDS tranche instruments with base correlation and Gaussian copula.
//!
//! CDS tranches (synthetic CDO tranches) provide leveraged credit exposure
//! to specific slices of the default distribution. Pricing uses the one-factor
//! Gaussian copula model with base correlation skew.
//!
//! # Tranche Structure
//!
//! A tranche covers losses between attachment and detachment points:
//! - **Attachment**: Lower loss threshold (e.g., 3%)
//! - **Detachment**: Upper loss threshold (e.g., 7%)
//! - **Tranche notional**: Detachment - Attachment
//!
//! Example tranches on CDX.IG:
//! - **Equity**: 0-3% (first loss)
//! - **Mezzanine**: 3-7%, 7-10%, 10-15%
//! - **Senior**: 15-30%
//! - **Super senior**: 30-100%
//!
//! # Pricing Model: One-Factor Gaussian Copula
//!
//! Default correlation modeled via single systematic factor:
//!
//! ```text
//! Asset value = √ρ · Z + √(1-ρ) · εᵢ
//! ```
//!
//! where ρ is base correlation, Z is common factor, εᵢ is idiosyncratic.
//!
//! **Base correlation**: Implied from market tranche quotes, varies by
//! detachment point (correlation skew).
//!
//! # References
//!
//! - Li, D. X. (2000). "On Default Correlation: A Copula Function Approach."
//!   *Journal of Fixed Income*, 9(4), 43-54.
//!
//! - Laurent, J.-P., & Gregory, J. (2005). "Basket Default Swaps, CDOs and
//!   Factor Copulas." *Journal of Risk*, 7(4), 103-122.
//!
//! - O'Kane, D. (2008). *Modelling Single-name and Multi-name Credit Derivatives*.
//!   Wiley Finance. Chapter 9: Synthetic CDO Valuation.
//!
//! # See Also
//!
//! - [`CdsTranche`] for instrument struct
//! - [`TrancheSide`] for buyer vs seller
//! - Base correlation calibration in [`calibration::methods`](crate::calibration::methods)

pub mod metrics;
pub mod parameters;
pub mod pricer;
mod types;

pub use types::{CdsTranche, TrancheSide};
