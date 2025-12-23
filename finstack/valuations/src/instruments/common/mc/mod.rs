//! Monte Carlo simulation engine for path-dependent derivatives with academic rigor.
//!
//! This module provides a production-grade Monte Carlo framework implementing
//! state-of-the-art algorithms from financial engineering research. All methods
//! are cited to their original academic sources for correctness and traceability.
//!
//! # Key Features
//!
//! ## Determinism & Reproducibility
//! - **Philox counter-based RNG**: Salmon et al. (2011) for parallel reproducibility
//! - **Splittable streams**: Independent sub-streams per path without correlation
//! - **Seed control**: Identical results across runs and thread counts
//!
//! ## Quasi-Monte Carlo (QMC)
//! - **Sobol sequences**: Sobol (1967), Joe & Kuo (2008) direction numbers
//! - **Owen scrambling**: Owen (1995, 1997) for improved uniformity
//! - **Brownian bridge ordering**: Caflisch et al. (1997) for path-dependent options
//!
//! ## Variance Reduction
//! - **Antithetic variates**: Hammersley & Handscomb (1964)
//! - **Control variates**: Black-Scholes for European payoff components
//! - **Importance sampling**: Glasserman & Li (2005)
//! - **Moment matching**: Post-simulation correction
//!
//! ## Stochastic Process Discretization
//! - **Geometric Brownian Motion**: Exact simulation (no bias)
//! - **Heston model**: QE scheme (Andersen 2008) with moment matching
//! - **Hull-White**: Exact discretization for short rate trees
//! - **Euler-Maruyama**: General-purpose first-order scheme
//!
//! ## Early Exercise (American/Bermudan)
//! - **Longstaff-Schwartz LSM**: Longstaff & Schwartz (2001)
//! - **Laguerre polynomials**: Basis functions for continuation value
//! - **Regression on ITM paths**: As per original paper
//!
//! ## Greeks Computation
//! - **Pathwise differentiation**: Broadie & Glasserman (1996)
//! - **Likelihood ratio method (LRM)**: Broadie & Glasserman (1996)
//! - **Finite differences with CRN**: Common random numbers for stability
//!
//! # Mathematical Foundation
//!
//! ## Standard Monte Carlo Estimator
//!
//! For a derivative with payoff h(S_T), the fair value under risk-neutral measure:
//!
//! ```text
//! V = e^(-rT) 𝔼^ℚ[h(S_T)]
//!   ≈ e^(-rT) (1/N) Σᵢ h(Sᵢ_T)
//! ```
//!
//! Standard error: σ̂ / √N where σ̂ is sample standard deviation.
//!
//! ## Variance Reduction Factor
//!
//! Antithetic variates achieve variance reduction factor typically 1.5-2.5×:
//! ```text
//! Var[(f(Z) + f(-Z))/2] ≤ Var[f(Z)]
//! ```
//!
//! Control variates using Black-Scholes can achieve 5-10× reduction for
//! options close to vanilla payoffs.
//!
//! # Academic References
//!
//! ## Foundational Texts
//!
//! - Glasserman, P. (2003). *Monte Carlo Methods in Financial Engineering*.
//!   Springer. (Comprehensive reference for all MC techniques)
//!
//! - Jäckel, P. (2002). *Monte Carlo Methods in Finance*. Wiley.
//!   (Practical implementation guide)
//!
//! ## Random Number Generation
//!
//! - Salmon, J. K., Moraes, M. A., Dror, R. O., & Shaw, D. E. (2011).
//!   "Parallel Random Numbers: As Easy as 1, 2, 3." *SC '11: Proceedings of 2011
//!   International Conference for High Performance Computing, Networking, Storage
//!   and Analysis*, 1-12. (Philox counter-based RNG)
//!
//! - L'Ecuyer, P. (1999). "Good Parameters and Implementations for Combined
//!   Multiple Recursive Random Number Generators." *Operations Research*, 47(1), 159-164.
//!
//! ## Quasi-Monte Carlo
//!
//! - Sobol, I. M. (1967). "Distribution of Points in a Cube and Approximate
//!   Evaluation of Integrals." *USSR Computational Mathematics and Mathematical
//!   Physics*, 7(4), 86-112.
//!
//! - Joe, S., & Kuo, F. Y. (2008). "Constructing Sobol Sequences with Better
//!   Two-Dimensional Projections." *SIAM Journal on Scientific Computing*, 30(5), 2635-2654.
//!
//! - Owen, A. B. (1995). "Randomly Permuted (t,m,s)-Nets and (t,s)-Sequences."
//!   *Monte Carlo and Quasi-Monte Carlo Methods in Scientific Computing*, 299-317.
//!
//! - Caflisch, R. E., Morokoff, W., & Owen, A. B. (1997). "Valuation of Mortgage
//!   Backed Securities Using Brownian Bridges to Reduce Effective Dimension."
//!   *Journal of Computational Finance*, 1(1), 27-46.
//!
//! ## Variance Reduction
//!
//! - Hammersley, J. M., & Handscomb, D. C. (1964). *Monte Carlo Methods*.
//!   Methuen, London. (Antithetic variates)
//!
//! - Glasserman, P., Heidelberger, P., & Shahabuddin, P. (2000). "Variance
//!   Reduction Techniques for Estimating Value-at-Risk." *Management Science*, 46(10), 1349-1364.
//!
//! - Glasserman, P., & Li, J. (2005). "Importance Sampling for Portfolio Credit
//!   Risk." *Management Science*, 51(11), 1643-1656.
//!
//! ## Discretization Schemes
//!
//! - Kloeden, P. E., & Platen, E. (1992). *Numerical Solution of Stochastic
//!   Differential Equations*. Springer. (Euler-Maruyama and higher-order schemes)
//!
//! - Andersen, L. (2008). "Simple and Efficient Simulation of the Heston
//!   Stochastic Volatility Model." *Journal of Computational Finance*, 11(3), 1-42.
//!   (QE scheme for Heston)
//!
//! - Lord, R., Koekkoek, R., & Van Dijk, D. (2010). "A Comparison of Biased
//!   Simulation Schemes for Stochastic Volatility Models." *Quantitative Finance*, 10(2), 177-194.
//!
//! ## American/Bermudan Options
//!
//! - Longstaff, F. A., & Schwartz, E. S. (2001). "Valuing American Options by
//!   Simulation: A Simple Least-Squares Approach." *Review of Financial Studies*,
//!   14(1), 113-147. (LSM algorithm)
//!
//! - Clément, E., Lamberton, D., & Protter, P. (2002). "An Analysis of a Least
//!   Squares Regression Method for American Option Pricing." *Finance and Stochastics*,
//!   6(4), 449-471. (Convergence theory)
//!
//! ## Greeks Computation
//!
//! - Broadie, M., & Glasserman, P. (1996). "Estimating Security Price Derivatives
//!   Using Simulation." *Management Science*, 42(2), 269-285.
//!   (Pathwise and likelihood ratio methods)
//!
//! - Glasserman, P., & Yao, D. D. (1992). "Some Guidelines and Guarantees for
//!   Common Random Numbers." *Management Science*, 38(6), 884-908.
//!   (Common random numbers for finite differences)
//!
//! # Architecture
//!
//! The framework is built around composable traits:
//!
//! - [`RandomStream`](traits::RandomStream): RNG abstraction (Philox, Sobol, etc.)
//! - [`StochasticProcess`](traits::StochasticProcess): SDE specification (GBM, Heston, etc.)
//! - [`Discretization`](traits::Discretization): Time-stepping schemes
//! - [`Payoff`](traits::Payoff): Payoff functions with currency safety
//!
//! # Performance Characteristics
//!
//! - **Standard MC**: Error ∝ 1/√N (halving error requires 4× paths)
//! - **QMC (Sobol)**: Error ∝ (log N)^d / N for smooth integrands (d = effective dimension)
//! - **Antithetic variates**: Typically 1.5-2.5× variance reduction
//! - **Control variates**: 5-10× reduction for near-vanilla payoffs
//! - **Parallelism**: Near-linear scaling with Rayon (tested to 32 cores)
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use finstack_core::currency::Currency;
//! use finstack_valuations::instruments::common::mc::prelude::{ExactGbm, GbmProcess, PhiloxRng};
//! use finstack_valuations::instruments::common::models::monte_carlo::prelude::{EuropeanCall, McEngine};
//!
//! # fn main() -> finstack_core::Result<()> {
//! // Create MC engine with deterministic seed
//! let engine = McEngine::builder()
//!     .num_paths(10_000)
//!     .uniform_grid(1.0, 252) // 1y horizon, daily-ish steps
//!     .seed(42) // Reproducible results
//!     .build()?;
//!
//! // Define geometric Brownian motion
//! let r = 0.03;
//! let q = 0.00;
//! let sigma = 0.20;
//! let gbm = GbmProcess::with_params(r, q, sigma);
//!
//! // Define European call payoff
//! let strike = 100.0;
//! let notional = 1.0;
//! let maturity_step = 251;
//! let payoff = EuropeanCall::new(strike, notional, maturity_step);
//!
//! // Price with standard error
//! let rng = PhiloxRng::new(42);
//! let disc = ExactGbm::new();
//! let initial_state = [100.0];
//! let result = engine.price(&rng, &gbm, &disc, &initial_state, &payoff, Currency::USD, 1.0)?;
//! println!("Price: {} ± {} (95% CI)", result.mean, 1.96 * result.stderr);
//! # Ok(())
//! # }
//! ```
//!
//! ## American Option Example
//!
//! ```rust,no_run
//! use finstack_core::currency::Currency;
//! use finstack_valuations::instruments::common::mc::prelude::GbmProcess;
//! use finstack_valuations::instruments::common::models::monte_carlo::prelude::{
//!     AmericanPut, LaguerreBasis, LsmcConfig, LsmcPricer,
//! };
//!
//! # fn main() -> finstack_core::Result<()> {
//! let strike = 100.0;
//! let process = GbmProcess::with_params(0.03, 0.00, 0.20);
//!
//! let num_steps = 50;
//! let exercise_dates = (1..num_steps).collect::<Vec<_>>(); // allow exercise at any step except t=0
//! let config = LsmcConfig::new(10_000, exercise_dates).with_seed(42);
//! let pricer = LsmcPricer::new(config);
//! let exercise = AmericanPut { strike };
//! let basis = LaguerreBasis::new(3, strike);
//!
//! let result = pricer.price(
//!     &process,
//!     100.0,
//!     1.0,
//!     num_steps,
//!     &exercise,
//!     &basis,
//!     Currency::USD,
//!     0.03,
//! )?;
//! # let _ = result;
//! # Ok(())
//! # }
//! ```
//!
//! # See Also
//!
//! - [`engine`] for MC engine configuration
//! - [`process`] for stochastic process implementations
//! - [`variance_reduction`] for variance reduction techniques
//! - [`greeks`] for Greeks computation methods
//! - [`xva`] for XVA (CVA/DVA/FVA) calculations

// Allow some clippy lints for MC module (many parameters are necessary for flexibility)
#![allow(clippy::too_many_arguments)]

pub mod discretization;
pub mod estimate;
pub mod online_stats;
pub mod paths;
pub mod process;
pub mod rng;
pub mod time_grid;
pub mod traits;

/// Prelude for convenient imports
pub mod prelude {
    // Generic path data
    pub use super::estimate::Estimate;
    pub use super::online_stats::{required_samples, OnlineCovariance, OnlineStats};
    pub use super::paths::{
        CashflowType, PathDataset, PathPoint, PathSamplingMethod, ProcessParams, SimulatedPath,
    };
    pub use super::time_grid::TimeGrid;
    pub use super::traits::{Discretization, PathState, RandomStream, StochasticProcess};

    // RNG
    pub use super::rng::philox::PhiloxRng;
    #[cfg(feature = "mc")]
    pub use super::rng::sobol::SobolRng;

    // Processes
    #[cfg(feature = "mc")]
    pub use super::process::bates::{BatesParams, BatesProcess};
    pub use super::process::brownian::{BrownianParams, BrownianProcess, MultiBrownianProcess};
    #[cfg(feature = "mc")]
    pub use super::process::cir::{CirParams, CirPlusPlusProcess, CirProcess};
    pub use super::process::correlation::{apply_correlation, cholesky_decomposition};
    pub use super::process::gbm::{GbmParams, GbmProcess, MultiGbmProcess};
    #[cfg(feature = "mc")]
    pub use super::process::heston::{HestonParams, HestonProcess};
    #[cfg(feature = "mc")]
    pub use super::process::jump_diffusion::{MertonJumpParams, MertonJumpProcess};
    pub use super::process::multi_ou::{MultiOuParams, MultiOuProcess};
    #[cfg(feature = "mc")]
    pub use super::process::ou::{HullWhite1FParams, HullWhite1FProcess, VasicekProcess};
    #[cfg(feature = "mc")]
    pub use super::process::schwartz_smith::{SchwartzSmithParams, SchwartzSmithProcess};

    // Discretization
    #[cfg(feature = "mc")]
    pub use super::discretization::euler::{EulerMaruyama, LogEuler};
    pub use super::discretization::exact::{ExactGbm, ExactMultiGbm, ExactMultiGbmCorrelated};
    #[cfg(feature = "mc")]
    pub use super::discretization::exact_hw1f::ExactHullWhite1F;
    #[cfg(feature = "mc")]
    pub use super::discretization::jump_euler::JumpEuler;
    #[cfg(feature = "mc")]
    pub use super::discretization::milstein::{LogMilstein, Milstein};
    #[cfg(feature = "mc")]
    pub use super::discretization::qe_cir::QeCir;
    #[cfg(feature = "mc")]
    pub use super::discretization::qe_heston::QeHeston;
    #[cfg(feature = "mc")]
    pub use super::discretization::schwartz_smith::ExactSchwartzSmith;

    // Pricing-specific exports moved under models::monte_carlo
}
