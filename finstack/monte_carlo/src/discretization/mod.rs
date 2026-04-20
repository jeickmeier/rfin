//! Time-stepping schemes for stochastic differential equations.
//!
//! Start with [`exact`] whenever an analytical transition is available because
//! it avoids discretization bias. Under the `mc` feature this module adds
//! general-purpose Euler / Milstein schemes and model-specific QE / jump
//! schemes for Heston, CIR, and jump-diffusion dynamics.
//!
//! Each discretization module documents the assumptions it makes about the
//! process state, convergence behavior, and positivity / stability guarantees.

pub mod exact;
pub mod exact_gbm_dividends;

#[cfg(feature = "mc")]
pub mod exact_hw1f;

#[cfg(feature = "mc")]
pub mod euler;

#[cfg(feature = "mc")]
pub mod milstein;

#[cfg(feature = "mc")]
pub(crate) mod qe_common;

#[cfg(feature = "mc")]
pub mod qe_heston;

#[cfg(feature = "mc")]
pub mod qe_cir;

#[cfg(feature = "mc")]
pub mod jump_euler;

#[cfg(feature = "mc")]
pub mod schwartz_smith;

#[cfg(feature = "mc")]
pub mod lmm_predictor_corrector;

#[cfg(feature = "mc")]
pub mod rough_bergomi;

#[cfg(feature = "mc")]
pub mod rough_heston;

#[cfg(feature = "mc")]
pub mod cheyette_rough;

pub use exact::{ExactGbm, ExactMultiGbm, ExactMultiGbmCorrelated};
pub use exact_gbm_dividends::ExactGbmWithDividends;

#[cfg(feature = "mc")]
pub use exact_hw1f::ExactHullWhite1F;

#[cfg(feature = "mc")]
pub use euler::{EulerMaruyama, LogEuler};

#[cfg(feature = "mc")]
pub use milstein::{LogMilstein, Milstein};

#[cfg(feature = "mc")]
pub use qe_cir::QeCir;

#[cfg(feature = "mc")]
pub use qe_heston::QeHeston;

#[cfg(feature = "mc")]
pub use jump_euler::JumpEuler;

#[cfg(feature = "mc")]
pub use schwartz_smith::ExactSchwartzSmith;

#[cfg(feature = "mc")]
pub use rough_bergomi::RoughBergomiEuler;

#[cfg(feature = "mc")]
pub use rough_heston::RoughHestonHybrid;

#[cfg(feature = "mc")]
pub use cheyette_rough::CheyetteRoughEuler;
