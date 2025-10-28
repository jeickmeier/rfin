//! Monte Carlo pricing engine for path-dependent derivatives.
//!
//! This module provides a production-grade Monte Carlo simulation framework
//! with the following features:
//!
//! - **Deterministic & Reproducible**: Counter-based RNG (Philox) with splittable streams
//! - **High Performance**: Rayon parallelism, SoA layouts, optional SIMD
//! - **QMC Support**: Sobol sequences with Owen scrambling, Brownian bridge ordering
//! - **Variance Reduction**: Antithetic variates, control variates, moment matching, IS
//! - **Advanced Models**: GBM, Heston, Hull-White, with exact and approximate schemes
//! - **Early Exercise**: LSMC (Longstaff-Schwartz) for American/Bermudan options
//! - **Greeks**: Pathwise, LRM, and CRN finite differences
//! - **Currency Safety**: All payoffs use `Money` types
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use finstack_valuations::instruments::common::mc::prelude::*;
//!
//! // Create MC engine
//! let engine = McEngine::builder()
//!     .num_paths(100_000)
//!     .num_steps(252)
//!     .seed(42)
//!     .variance_reduction(vec![VR::Antithetic, VR::ControlBS])
//!     .build();
//!
//! // Define GBM process
//! let gbm = GbmProcess::new(r, q, sigma);
//!
//! // Define European call payoff
//! let payoff = EuropeanCall::new(strike, notional);
//!
//! // Price
//! let result = engine.price(&gbm, &payoff, market_context)?;
//! println!("Price: {} ± {}", result.mean, result.stderr);
//! ```
//!
//! # Architecture
//!
//! The MC engine is built around several key traits:
//!
//! - `RandomStream`: RNG abstraction (Philox, Sobol, etc.)
//! - `StochasticProcess`: SDE specification (GBM, Heston, etc.)
//! - `Discretization`: Time-stepping schemes (Exact, Euler, QE, etc.)
//! - `Payoff`: Payoff computation with `Money` types
//!
//! These traits enable composability and testability while maintaining performance.

// Allow some clippy lints for MC module (many parameters are necessary for flexibility)
#![allow(clippy::too_many_arguments)]

pub mod barriers;
pub mod discretization;
pub mod engine;
pub mod greeks;
pub mod payoff;
pub mod pricer;
pub mod process;
pub mod results;
pub mod rng;
pub mod stats;
pub mod time_grid;
pub mod traits;
pub mod variance_reduction;

/// Prelude for convenient imports
pub mod prelude {
    // Engine and configuration
    pub use super::engine::{McEngine, McEngineBuilder, McEngineConfig};
    pub use super::results::{Estimate, MoneyEstimate};
    pub use super::stats::OnlineStats;
    pub use super::time_grid::TimeGrid;
    pub use super::traits::{Payoff, PathState, RandomStream, StochasticProcess, Discretization};

    // RNG
    pub use super::rng::philox::PhiloxRng;
    #[cfg(feature = "mc")]
    pub use super::rng::sobol::SobolRng;

    // Processes
    pub use super::process::gbm::{GbmParams, GbmProcess, MultiGbmProcess};
    #[cfg(feature = "mc")]
    pub use super::process::heston::{HestonParams, HestonProcess};
    #[cfg(feature = "mc")]
    pub use super::process::ou::{HullWhite1FParams, HullWhite1FProcess, VasicekProcess};
    #[cfg(feature = "mc")]
    pub use super::process::cir::{CirParams, CirProcess, CirPlusPlusProcess};
    #[cfg(feature = "mc")]
    pub use super::process::jump_diffusion::{MertonJumpParams, MertonJumpProcess};
    #[cfg(feature = "mc")]
    pub use super::process::bates::{BatesParams, BatesProcess};
    pub use super::process::correlation::{cholesky_decomposition, apply_correlation};

    // Discretization
    pub use super::discretization::exact::{ExactGbm, ExactMultiGbm};
    #[cfg(feature = "mc")]
    pub use super::discretization::exact_hw1f::ExactHullWhite1F;
    #[cfg(feature = "mc")]
    pub use super::discretization::qe_heston::QeHeston;
    #[cfg(feature = "mc")]
    pub use super::discretization::qe_cir::QeCir;
    #[cfg(feature = "mc")]
    pub use super::discretization::euler::{EulerMaruyama, LogEuler};
    #[cfg(feature = "mc")]
    pub use super::discretization::milstein::{Milstein, LogMilstein};
    #[cfg(feature = "mc")]
    pub use super::discretization::jump_euler::JumpEuler;

    // Payoffs
    pub use super::payoff::vanilla::{EuropeanCall, EuropeanPut, Digital, Forward};
    #[cfg(feature = "mc")]
    pub use super::payoff::asian::{AsianCall, AsianPut, AveragingMethod, geometric_asian_call_closed_form};
    #[cfg(feature = "mc")]
    pub use super::payoff::barrier::{BarrierCall, BarrierType};
    #[cfg(feature = "mc")]
    pub use super::payoff::lookback::{LookbackCall, LookbackPut, FloatingStrikeLookbackCall};
    #[cfg(feature = "mc")]
    pub use super::payoff::basket::{BasketCall, BasketPut, BasketType, ExchangeOption, margrabe_exchange_option};
    #[cfg(feature = "mc")]
    pub use super::payoff::rates::{CapPayoff, FloorPayoff, cap_floor_parity_swap_value};

    // Pricers
    pub use super::pricer::european::{EuropeanPricer, EuropeanPricerConfig};
    #[cfg(feature = "mc")]
    pub use super::pricer::path_dependent::{PathDependentPricer, PathDependentPricerConfig};
    #[cfg(feature = "mc")]
    pub use super::pricer::lsmc::{LsmcPricer, LsmcConfig, AmericanPut, AmericanCall, PolynomialBasis, LaguerreBasis};

    // Variance reduction
    pub use super::variance_reduction::antithetic::AntitheticConfig;
    pub use super::variance_reduction::control_variate::{black_scholes_call, black_scholes_put};
}

