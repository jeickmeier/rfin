//! Stochastic recovery models for CDS tranche pricing.
//!
//! This module re-exports the shared recovery model infrastructure from
//! [`crate::instruments::common::models::correlation::recovery`].
//!
//! Recovery rates empirically decrease in stressed markets (negative correlation
//! with default intensity). This is critical for senior tranches which are
//! sensitive to realized recovery at the time of defaults.
//!
//! # Constant vs Stochastic Recovery
//!
//! - **Constant**: Simplest model, uses fixed recovery (e.g., 40%)
//! - **Stochastic**: Recovery varies with market conditions
//!
//! # Stochastic Recovery Models
//!
//! ## Market-Correlated (Andersen-Sidenius)
//!
//! Recovery negatively correlated with the systematic factor:
//! ```text
//! R(Z) = μ_R + ρ_R · σ_R · Z
//! ```
//! where ρ_R < 0 (typically -0.3 to -0.5).
//!
//! This captures the "double hit" effect: defaults cluster AND recovery
//! falls simultaneously in stressed environments.
//!
//! ## Beta-Distributed
//!
//! Recovery bounded to [0, 1] with specified mean and variance:
//! ```text
//! R ~ Beta(α, β) with E[R] = μ, Var(R) = σ²
//! ```
//!
//! ## Frye Model (LGD-Default Correlation)
//!
//! LGD as function of portfolio default rate:
//! ```text
//! LGD(DR) = α + β · DR
//! ```
//!
//! # References
//!
//! - Altman, E., et al. (2005). "The Link between Default and Recovery Rates."
//!   *Journal of Business*, 78(6).
//! - Andersen, L., & Sidenius, J. (2005). "Extensions to the Gaussian Copula."
//! - Amraoui, S., & Hitier, S. (2008). "Optimal Stochastic Recovery for Base
//!   Correlation."
//! - Frye, J. (2000). "Depressing Recoveries." *Risk*, November 2000.

// Re-export everything from the shared correlation module
pub use crate::instruments::common_impl::models::correlation::recovery::*;
