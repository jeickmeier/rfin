//! Time discretization schemes for SDEs.
//!
//! Implements exact, Euler, Milstein, and specialized schemes.

pub mod exact;
pub mod exact_gbm_dividends;

#[cfg(feature = "mc")]
pub mod exact_hw1f;

#[cfg(feature = "mc")]
pub mod euler;

#[cfg(feature = "mc")]
pub mod milstein;

#[cfg(feature = "mc")]
pub mod qe_heston;

#[cfg(feature = "mc")]
pub mod qe_cir;

#[cfg(feature = "mc")]
pub mod jump_euler;

#[cfg(feature = "mc")]
pub mod schwartz_smith;

pub mod revolving_credit;

pub use exact::{ExactGbm, ExactMultiGbm, ExactMultiGbmCorrelated};
pub use exact_gbm_dividends::ExactGbmWithDividends;

// Backwards-compatible alias
pub use exact_gbm_dividends as exact_gbm_div;

#[cfg(feature = "mc")]
pub use exact_hw1f::ExactHullWhite1F;

#[cfg(feature = "mc")]
pub use euler::{EulerMaruyama, LogEuler};

#[cfg(feature = "mc")]
pub use milstein::{LogMilstein, Milstein};

#[cfg(feature = "mc")]
pub use qe_cir::QeCir;

#[cfg(feature = "mc")]
pub use jump_euler::JumpEuler;

#[cfg(feature = "mc")]
pub use schwartz_smith::ExactSchwartzSmith;
