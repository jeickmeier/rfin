//! Interest rate derivatives and money market instruments.

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
pub use cap_floor::RateOptionType;
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
