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

pub mod analytical;
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
pub mod xva;

#[cfg(feature = "mc")]
pub mod mlmc;

/// Prelude for convenient imports
pub mod prelude {
    // Engine and configuration
    pub use super::engine::{McEngine, McEngineBuilder, McEngineConfig};
    pub use super::results::{Estimate, MoneyEstimate};
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
    pub use super::payoff::barrier::{BarrierCall, BarrierType};
    #[cfg(feature = "mc")]
    pub use super::payoff::basket::{
        margrabe_exchange_option, BasketCall, BasketPut, BasketType, ExchangeOption,
    };
    #[cfg(feature = "mc")]
    pub use super::payoff::lookback::{FloatingStrikeLookbackCall, LookbackCall, LookbackPut};
    #[cfg(feature = "mc")]
    pub use super::payoff::rates::{cap_floor_parity_swap_value, CapPayoff, FloorPayoff};
    #[cfg(feature = "mc")]
    pub use super::payoff::swaption::{BermudanSwaptionPayoff, SwapSchedule, SwaptionType};
    #[cfg(feature = "mc")]
    pub use super::payoff::quanto::{QuantoCallPayoff, QuantoPutPayoff};
    #[cfg(feature = "mc")]
    pub use super::payoff::autocallable::{AutocallablePayoff, FinalPayoffType};
    #[cfg(feature = "mc")]
    pub use super::payoff::cms::{CmsCapPayoff, CmsFloorPayoff};
    #[cfg(feature = "mc")]
    pub use super::payoff::cliquet::CliquetCallPayoff;
    #[cfg(feature = "mc")]
    pub use super::payoff::range_accrual::RangeAccrualPayoff;
    #[cfg(feature = "mc")]
    pub use super::payoff::fx_barrier::FxBarrierCall;
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
    pub use super::pricer::swaption_lsmc::SwaptionLsmcPricer;
    #[cfg(feature = "mc")]
    pub use super::pricer::swap_rate_utils::{ForwardSwapRate, HullWhiteBondPrice};

    // Variance reduction
    pub use super::variance_reduction::antithetic::AntitheticConfig;
    pub use super::variance_reduction::control_variate::{black_scholes_call, black_scholes_put};

    // MLMC
    #[cfg(feature = "mc")]
    pub use super::mlmc::{optimal_allocation, MlmcConfig, MlmcEngine, MlmcEstimate, MlmcLevel};
}
