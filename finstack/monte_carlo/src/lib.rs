//! Monte Carlo simulation engine for derivative pricing.
//!
//! This module provides a complete Monte Carlo framework including both
//! simulation primitives (RNG, stochastic processes, discretization schemes)
//! and pricing infrastructure (payoffs, pricers, Greeks, variance reduction).
//!
//! # Features
//!
//! - **RNG**: Philox counter-based RNG, Sobol quasi-random sequences, Brownian bridge
//! - **Stochastic Processes**: GBM, Heston, Hull-White, CIR, Bates, jump diffusion
//! - **Discretization**: Exact, Euler-Maruyama, Milstein, QE schemes
//! - **Payoffs**: Vanilla, Asian, barrier, basket, lookback, autocallable, cliquet
//! - **Early Exercise**: Longstaff-Schwartz LSM algorithm
//! - **Variance Reduction**: Antithetic variates, control variates, importance sampling
//! - **Greeks**: Pathwise, finite-difference, and likelihood ratio sensitivities
//! - **Deterministic Results**: Seedable RNG for reproducibility
//!
//! # Supported Models
//!
//! | Process | Dynamics | Discretization |
//! |---------|----------|----------------|
//! | GBM | dS = μS dt + σS dW | Exact, Euler, Milstein |
//! | Heston | dS = μS dt + √v S dW₁, dv = κ(θ-v)dt + ξ√v dW₂ | Andersen QE |
//! | Hull-White | dr = (θ(t) - ar)dt + σ dW | Exact |
//! | CIR | dr = κ(θ-r)dt + σ√r dW | QE |
//! | Bates | GBM + Heston + Merton jumps | QE + Jump-Euler |
//!
//! # Quick Example
//!
//! ```rust,ignore
//! use finstack_monte_carlo::prelude::*;
//!
//! let engine = McEngine::builder()
//!     .num_paths(100_000)
//!     .seed(42)
//!     .uniform_grid(1.0, 252)
//!     .build()
//!     .expect("valid config");
//!
//! let gbm = GbmProcess::with_params(0.03, 0.00, 0.20);
//! let payoff = EuropeanCall::new(100.0, 1.0, 252);
//! let rng = PhiloxRng::new(42);
//! let disc = ExactGbm::new();
//! let result = engine.price(&rng, &gbm, &disc, &[100.0], &payoff, Currency::USD, 1.0)?;
//! ```
//!
//! # Academic References
//!
//! - Glasserman, P. (2003). *Monte Carlo Methods in Financial Engineering*. Springer.
//! - Longstaff, F. A., & Schwartz, E. S. (2001). "Valuing American Options by Simulation."
//! - Andersen, L. (2008). "Simple and Efficient Simulation of the Heston Model."
//! - Salmon, J. K. et al. (2011). "Parallel Random Numbers: As Easy as 1, 2, 3."

// --- Simulation primitives ---
mod captured_path_stats;
pub mod discretization;
pub mod estimate;
mod indexed_spot_table;
pub mod online_stats;
pub mod paths;
pub mod process;
pub mod rng;
pub mod time_grid;
pub mod traits;

// --- Pricing infrastructure ---
pub mod barriers;
pub mod engine;
pub mod greeks;
pub mod payoff;
pub mod pricer;
pub mod results;
#[cfg(feature = "mc")]
pub mod seed;
pub mod variance_reduction;

#[cfg(all(test, feature = "mc"))]
mod mc_process_params_serialization;

pub use traits::{
    state_keys, Discretization, PathState, Payoff, RandomStream, StateKey, StateVariables,
    StochasticProcess,
};

/// Prelude for convenient imports of the full Monte Carlo framework.
pub mod prelude {
    // --- Core traits and infrastructure ---
    pub use super::estimate::Estimate;
    pub use super::online_stats::{required_samples, OnlineCovariance, OnlineStats};
    pub use super::paths::{
        CashflowType, PathDataset, PathPoint, PathSamplingMethod, ProcessParams, SimulatedPath,
    };
    pub use super::time_grid::TimeGrid;
    pub use super::traits::{Discretization, PathState, RandomStream, StochasticProcess};

    // --- RNG ---
    pub use super::rng::philox::PhiloxRng;
    #[cfg(feature = "mc")]
    pub use super::rng::sobol::SobolRng;

    // --- Processes ---
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

    // --- Discretization schemes ---
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

    // --- Engine and configuration ---
    pub use super::engine::{
        McEngine, McEngineBuilder, McEngineConfig, PathCaptureConfig, PathCaptureMode,
    };

    // --- Pricing results ---
    pub use super::results::{MoneyEstimate, MonteCarloResult};
    pub use super::traits::Payoff;

    // --- Payoffs ---
    #[cfg(feature = "mc")]
    pub use super::payoff::asian::{
        geometric_asian_call_closed_form, AsianCall, AsianPut, AveragingMethod,
    };
    #[cfg(feature = "mc")]
    pub use super::payoff::barrier::{BarrierOptionPayoff, BarrierType};
    #[cfg(feature = "mc")]
    pub use super::payoff::basket::{
        margrabe_exchange_option, BasketCall, BasketPut, BasketType, ExchangeOption,
    };
    pub use super::payoff::vanilla::{Digital, EuropeanCall, EuropeanPut, Forward};

    // --- Pricers ---
    #[cfg(feature = "mc")]
    pub use super::pricer::basis::{LaguerreBasis, PolynomialBasis};
    pub use super::pricer::european::{EuropeanPricer, EuropeanPricerConfig};
    #[cfg(feature = "mc")]
    pub use super::pricer::lsmc::{AmericanCall, AmericanPut, LsmcConfig, LsmcPricer};
    #[cfg(feature = "mc")]
    pub use super::pricer::path_dependent::{PathDependentPricer, PathDependentPricerConfig};

    // --- Variance reduction ---
    pub use super::variance_reduction::antithetic::AntitheticConfig;
    pub use super::variance_reduction::control_variate::{black_scholes_call, black_scholes_put};
}
