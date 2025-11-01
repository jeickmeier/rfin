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
//! ```rust,ignore
//! use finstack_valuations::instruments::common::mc::prelude::*;
//!
//! // Create MC engine with deterministic seed
//! let engine = McEngine::builder()
//!     .num_paths(100_000)
//!     .num_steps(252)       // Daily steps
//!     .seed(42)             // Reproducible results
//!     .variance_reduction(vec![VR::Antithetic, VR::ControlBS])
//!     .build();
//!
//! // Define geometric Brownian motion
//! let gbm = GbmProcess::new(r, q, sigma);
//!
//! // Define European call payoff
//! let payoff = EuropeanCall::new(strike, notional);
//!
//! // Price with standard error
//! let result = engine.price(&gbm, &payoff, market_context)?;
//! println!("Price: {} ± {} (95% CI)", result.mean, 1.96 * result.stderr);
//! ```
//!
//! ## American Option Example
//!
//! ```rust,ignore
//! use finstack_valuations::instruments::common::mc::prelude::*;
//!
//! // Use Longstaff-Schwartz for American put
//! let lsm_config = LsmConfig::new()
//!     .with_basis_degree(3)           // Laguerre polynomials up to degree 3
//!     .with_regression_on_itm(true);  // Regress only on ITM paths
//!
//! let result = engine.price_american(&gbm, &put_payoff, lsm_config, market_context)?;
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

pub mod analytical;
pub mod barriers;
pub mod discretization;
pub mod engine;
pub mod greeks;
pub mod path_data;
pub mod payoff;
pub mod pricer;
pub mod process;
pub mod results;
pub mod rng;
#[cfg(feature = "mc")]
pub mod seed;
pub mod stats;
pub mod time_grid;
pub mod traits;
pub mod variance_reduction;
pub mod xva;

#[cfg(feature = "mc")]
pub mod mlmc;

#[cfg(test)]
mod path_capture_tests;

/// Prelude for convenient imports
pub mod prelude {
    // Engine and configuration
    pub use super::engine::{
        McEngine, McEngineBuilder, McEngineConfig, PathCaptureConfig, PathCaptureMode,
    };
    pub use super::path_data::{
        PathDataset, PathPoint, PathSamplingMethod, ProcessParams, SimulatedPath,
    };
    pub use super::results::{Estimate, MoneyEstimate, MonteCarloResult};
    pub use super::stats::OnlineStats;
    pub use super::time_grid::TimeGrid;
    pub use super::traits::{Discretization, PathState, Payoff, RandomStream, StochasticProcess};

    // RNG
    pub use super::rng::philox::PhiloxRng;
    #[cfg(feature = "mc")]
    pub use super::rng::sobol::SobolRng;

    // Processes
    #[cfg(feature = "mc")]
    pub use super::process::bates::{BatesParams, BatesProcess};
    #[cfg(feature = "mc")]
    pub use super::process::cir::{CirParams, CirPlusPlusProcess, CirProcess};
    pub use super::process::correlation::{apply_correlation, cholesky_decomposition};
    pub use super::process::gbm::{GbmParams, GbmProcess, MultiGbmProcess};
    #[cfg(feature = "mc")]
    pub use super::process::heston::{HestonParams, HestonProcess};
    #[cfg(feature = "mc")]
    pub use super::process::jump_diffusion::{MertonJumpParams, MertonJumpProcess};
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

    // Payoffs
    #[cfg(feature = "mc")]
    pub use super::payoff::asian::{
        geometric_asian_call_closed_form, AsianCall, AsianPut, AveragingMethod,
    };
    #[cfg(feature = "mc")]
    pub use super::payoff::autocallable::{AutocallablePayoff, FinalPayoffType};
    #[cfg(feature = "mc")]
    pub use super::payoff::barrier::{BarrierCall, BarrierType};
    #[cfg(feature = "mc")]
    pub use super::payoff::basket::{
        margrabe_exchange_option, BasketCall, BasketPut, BasketType, ExchangeOption,
    };
    #[cfg(feature = "mc")]
    pub use super::payoff::cliquet::CliquetCallPayoff;
    #[cfg(feature = "mc")]
    pub use super::payoff::cms::{CmsCapPayoff, CmsFloorPayoff};
    #[cfg(feature = "mc")]
    pub use super::payoff::fx_barrier::FxBarrierCall;
    #[cfg(feature = "mc")]
    pub use super::payoff::lookback::{FloatingStrikeLookbackCall, LookbackCall, LookbackPut};
    #[cfg(feature = "mc")]
    pub use super::payoff::quanto::{QuantoCallPayoff, QuantoPutPayoff};
    #[cfg(feature = "mc")]
    pub use super::payoff::range_accrual::RangeAccrualPayoff;
    #[cfg(feature = "mc")]
    pub use super::payoff::rates::{cap_floor_parity_swap_value, CapPayoff, FloorPayoff};
    #[cfg(feature = "mc")]
    pub use super::payoff::swaption::{BermudanSwaptionPayoff, SwapSchedule, SwaptionType};
    pub use super::payoff::vanilla::{Digital, EuropeanCall, EuropeanPut, Forward};

    // Pricers
    pub use super::pricer::european::{EuropeanPricer, EuropeanPricerConfig};
    #[cfg(feature = "mc")]
    pub use super::pricer::lsmc::{
        AmericanCall, AmericanPut, LaguerreBasis, LsmcConfig, LsmcPricer, PolynomialBasis,
    };
    #[cfg(feature = "mc")]
    pub use super::pricer::path_dependent::{PathDependentPricer, PathDependentPricerConfig};
    #[cfg(feature = "mc")]
    pub use super::pricer::swap_rate_utils::{ForwardSwapRate, HullWhiteBondPrice};
    #[cfg(feature = "mc")]
    pub use super::pricer::swaption_lsmc::SwaptionLsmcPricer;

    // Variance reduction
    pub use super::variance_reduction::antithetic::AntitheticConfig;
    pub use super::variance_reduction::control_variate::{black_scholes_call, black_scholes_put};

    // MLMC
    #[cfg(feature = "mc")]
    pub use super::mlmc::{optimal_allocation, MlmcConfig, MlmcEngine, MlmcEstimate, MlmcLevel};
}
