//! Option pricing models and numerical methods with academic foundations.
//!
//! Provides reusable pricing models for options and derivatives, including
//! tree-based methods, volatility models, and Black-Scholes variants. All
//! implementations cite their academic sources for correctness verification.
//!
//! # Pricing Models
//!
//! ## Tree Methods for American/Bermudan Options
//!
//! - **Binomial trees**: Cox-Ross-Rubinstein (1979), Jarrow-Rudd, Leisen-Reimer (1996)
//! - **Trinomial trees**: Boyle (1986), Hull-White for short rates
//! - **Multi-factor trees**: Two-factor equity-rates, rates-credit models
//!
//! ## Volatility Models
//!
//! - **SABR**: Hagan et al. (2002) stochastic volatility model
//! - **Black-Scholes**: Helper functions for d1, d2 parameters
//! - **Short rate models**: Hull-White, Black-Karasinski trees
//!
//! # Binomial Tree Methods
//!
//! ## Cox-Ross-Rubinstein (CRR, 1979)
//!
//! Standard binomial tree with:
//! ```text
//! u = e^(σ√Δt)
//! d = 1/u
//! p = (e^(rΔt) - d) / (u - d)
//! ```
//!
//! ## Jarrow-Rudd (Equal Probability)
//!
//! Uses p = 0.5 for simpler analysis:
//! ```text
//! u = e^((r - σ²/2)Δt + σ√Δt)
//! d = e^((r - σ²/2)Δt - σ√Δt)
//! p = 0.5
//! ```
//!
//! ## Leisen-Reimer (1996)
//!
//! Improved convergence using Peizer-Pratt inversion for matching
//! strike and spot probabilities exactly at odd number of steps.
//!
//! # SABR Model (Hagan et al. 2002)
//!
//! Stochastic Alpha Beta Rho model for volatility smiles:
//!
//! ```text
//! dF = α·F^β dW₁
//! dα = ν·α dW₂
//! dW₁·dW₂ = ρ dt
//! ```
//!
//! Parameters:
//! - **α**: Initial volatility level
//! - **β**: Backbone parameter (0=normal, 1=lognormal)
//! - **ρ**: Correlation between rate and volatility
//! - **ν**: Volatility of volatility
//!
//! # Academic References
//!
//! ## Binomial Trees
//!
//! - Cox, J. C., Ross, S. A., & Rubinstein, M. (1979). "Option Pricing: A
//!   Simplified Approach." *Journal of Financial Economics*, 7(3), 229-263.
//!
//! - Jarrow, R., & Rudd, A. (1983). *Option Pricing*. Irwin.
//!
//! - Leisen, D. P. J., & Reimer, M. (1996). "Binomial Models for Option
//!   Valuation - Examining and Improving Convergence." *Applied Mathematical
//!   Finance*, 3(4), 319-346.
//!
//! ## Trinomial Trees
//!
//! - Boyle, P. P. (1986). "Option Valuation Using a Three-Jump Process."
//!   *International Options Journal*, 3, 7-12.
//!
//! - Hull, J., & White, A. (1994). "Numerical Procedures for Implementing
//!   Term Structure Models I: Single-Factor Models." *Journal of Derivatives*,
//!   2(1), 7-16.
//!
//! ## SABR Model
//!
//! - Hagan, P. S., Kumar, D., Lesniewski, A. S., & Woodward, D. E. (2002).
//!   "Managing Smile Risk." *Wilmott Magazine*, September, 84-108.
//!
//! - West, G. (2005). "Calibration of the SABR Model in Illiquid Markets."
//!   *Applied Mathematical Finance*, 12(4), 371-385.
//!
//! # See Also
//!
//! - [`binomial_tree`] for binomial tree implementations
//! - [`trinomial_tree`] for trinomial tree methods
//! - [`sabr`] for SABR volatility model
//! - [`tree_framework`] for generic tree pricing framework
//! - [`short_rate_tree`] for interest rate tree models

pub mod binomial_tree;
pub mod black;
pub mod sabr;
pub mod short_rate_tree;
pub mod tree_framework;
pub mod trinomial_tree;
// Multi-factor tree scaffold is implemented in multi_factor_tree.rs
pub mod multi_factor_tree;
pub mod two_factor_binomial;
pub mod two_factor_rates_credit;

pub use binomial_tree::{BinomialTree, TreeType};
pub use black::{d1, d1_black76, d2, d2_black76};
pub use finstack_core::math::{norm_cdf, norm_pdf};
pub use sabr::{SABRCalibrator, SABRModel, SABRParameters, SABRSmile};
pub use short_rate_tree::{short_rate_keys, ShortRateModel, ShortRateTree, ShortRateTreeConfig};
pub use tree_framework::{
    single_factor_equity_state, state_keys, two_factor_equity_rates_state, BarrierSpec,
    BarrierStyle, EvolutionParams, NodeState, StateVariables, TreeBranching, TreeGreeks, TreeModel,
    TreeParameters, TreeValuator,
};
pub use trinomial_tree::{TrinomialTree, TrinomialTreeType};
