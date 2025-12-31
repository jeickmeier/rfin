//! Interest rate derivatives and money market instruments.
//!
//! This module provides interest rate instruments from simple money market
//! products to complex volatility derivatives. All instruments support
//! multi-curve pricing with separate discount and projection curves.
//!
//! # Features
//!
//! - **Swaps**: Vanilla IRS, basis swaps, cross-currency swaps
//! - **Options**: Caps, floors, swaptions, CMS options
//! - **Money Market**: Deposits, FRAs, repos
//! - **Futures**: SOFR futures, Eurodollar futures
//! - **Inflation**: Zero-coupon swaps, YoY swaps, inflation caps/floors
//! - **Exotics**: Range accruals, Bermudan swaptions
//!
//! # Pricing Framework
//!
//! Post-2008 multi-curve framework:
//! - **Discount curve**: OIS curve for collateralized discounting
//! - **Projection curves**: Term SOFR, EURIBOR, etc. for floating legs
//! - **Volatility surfaces**: Normal or lognormal vol for options
//!
//! # Quick Example
//!
//! ```rust,no_run
//! use finstack_valuations::instruments::rates::InterestRateSwap;
//! use finstack_core::currency::Currency;
//! use finstack_core::money::Money;
//! use time::macros::date;
//!
//! // Create a 5-year payer swap (pay fixed, receive floating)
//! let swap = InterestRateSwap::payer(
//!     "IRS-5Y-USD",
//!     Money::new(10_000_000.0, Currency::USD),
//!     date!(2025-01-15),
//!     date!(2030-01-15),
//!     0.04,  // 4% fixed rate
//!     "USD-OIS",
//!     "USD-SOFR-3M",
//! );
//! ```
//!
//! # Risk Metrics
//!
//! All rate instruments support:
//! - **DV01**: Dollar value of 1bp parallel curve shift
//! - **Bucketed DV01**: Sensitivity by tenor bucket
//! - **Convexity**: Second-order rate sensitivity
//! - **Theta**: Time decay
//!
//! # References
//!
//! - ISDA 2006 Definitions for swap conventions
//! - Black (1976) for cap/floor and swaption pricing
//! - Hull-White (1990) for short rate models
//!
//! # See Also
//!
//! - [`InterestRateSwap`] for vanilla IRS
//! - [`Swaption`] for European swaptions
//! - [`InterestRateOption`] for caps and floors
//! - [`crate::calibration`] for curve calibration

/// Basis swap module - Floating vs floating swaps.
pub mod basis_swap;
/// Cap/floor module - Interest rate caps and floors.
pub mod cap_floor;
/// CMS option module - Constant maturity swap options.
pub mod cms_option;
/// Deposit module - Money market deposits.
pub mod deposit;
/// FRA module - Forward rate agreements.
pub mod fra;
/// Inflation cap/floor module.
pub mod inflation_cap_floor;
/// Inflation swap module.
pub mod inflation_swap;
/// IR future module - Interest rate futures.
pub mod ir_future;
/// IRS module - Interest rate swaps.
pub mod irs;
/// Range accrual module.
pub mod range_accrual;
/// Repo module - Repurchase agreements.
pub mod repo;
/// Swaption module - Options on interest rate swaps.
pub mod swaption;
/// Cross-currency swap module.
pub mod xccy_swap;

// Re-export primary types
pub use basis_swap::BasisSwap;
pub use cap_floor::{InterestRateOption, RateOptionType};
pub use cms_option::CmsOption;
pub use deposit::Deposit;
pub use fra::ForwardRateAgreement;
pub use inflation_cap_floor::{InflationCapFloor, InflationCapFloorType};
pub use inflation_swap::{InflationSwap, YoYInflationSwap};
pub use ir_future::InterestRateFuture;
pub use irs::InterestRateSwap;
pub use range_accrual::RangeAccrual;
pub use repo::{CollateralSpec, CollateralType, Repo, RepoType};
pub use swaption::Swaption;
pub use xccy_swap::XccySwap;
