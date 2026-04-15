//! Monte Carlo sensitivity estimators.
//!
//! This module groups the three Greek estimators used in the crate:
//! pathwise differentiation, likelihood-ratio / score-function methods, and
//! finite differences with common random numbers.
//!
//! Start with `pathwise` for smooth payoffs, use `lrm` for discontinuous
//! payoffs such as digitals or barriers, and use `finite_diff` when you need a
//! general bump-and-reprice fallback. All three submodules are available
//! behind the `mc` feature.

#[cfg(feature = "mc")]
pub mod pathwise;

#[cfg(feature = "mc")]
pub mod lrm;

#[cfg(feature = "mc")]
pub mod finite_diff;
