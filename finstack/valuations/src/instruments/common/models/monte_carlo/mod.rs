//! Monte Carlo simulation engine for derivative pricing.
//!
//! This module provides a complete Monte Carlo pricing framework for path-dependent
//! and exotic options. It includes stochastic process simulation, payoff evaluation,
//! variance reduction techniques, and American option pricing via LSM.
//!
//! # Features
//!
//! - **Stochastic Processes**: GBM, Heston, Hull-White
//! - **Payoffs**: Vanilla, Asian, barrier, basket, lookback
//! - **Early Exercise**: Longstaff-Schwartz LSM algorithm
//! - **Variance Reduction**: Antithetic variates, control variates
//! - **Greeks**: Pathwise and finite-difference sensitivities
//! - **Deterministic Results**: Seedable RNG for reproducibility
//!
//! # Supported Models
//!
//! | Process | Dynamics | Discretization |
//! |---------|----------|----------------|
//! | GBM | dS = μS dt + σS dW | Euler, Milstein |
//! | Heston | dS = μS dt + √v S dW₁, dv = κ(θ-v)dt + ξ√v dW₂ | Andersen QE |
//! | Hull-White | dr = (θ(t) - ar)dt + σ dW | Euler |
//!
//! # Variance Reduction
//!
//! Monte Carlo variance can be reduced via:
//! - **Antithetic variates**: Pair paths with Z and -Z
//! - **Control variates**: Use closed-form delta as control
//! - **Importance sampling**: Tilt drift toward exercise region
//!
//! # Quick Example
//!
//! ```rust,no_run
//! use finstack_valuations::instruments::common::models::monte_carlo::prelude::*;
//!
//! // Configure Monte Carlo engine
//! let config = McEngineConfig {
//!     num_paths: 100_000,
//!     seed: Some(42),  // Deterministic
//!     ..Default::default()
//! };
//!
//! // Price European call
//! let payoff = EuropeanCall { strike: 100.0 };
//! // let result = engine.price(&payoff, &process, &market, expiry)?;
//! ```
//!
//! # LSM for American Options
//!
//! Longstaff-Schwartz least squares Monte Carlo for early exercise:
//!
//! ```rust,no_run
//! # #[cfg(feature = "mc")]
//! # fn example() {
//! use finstack_valuations::instruments::common::models::monte_carlo::prelude::*;
//!
//! // American put with Laguerre basis
//! let config = LsmcConfig {
//!     num_paths: 50_000,
//!     basis: LaguerreBasis::new(3),
//!     seed: Some(42),
//!     ..Default::default()
//! };
//! # }
//! ```
//!
//! # Academic References
//!
//! - Glasserman, P. (2003). *Monte Carlo Methods in Financial Engineering*. Springer.
//! - Longstaff, F. A., & Schwartz, E. S. (2001). "Valuing American Options by Simulation."
//! - Andersen, L. (2008). "Simple and Efficient Simulation of the Heston Model."
//!
//! # See Also
//!
//! - [`engine::McEngine`] for the main simulation engine
//! - [`pricer::lsmc`] for American option pricing
//! - [`variance_reduction`] for variance reduction techniques
//! - [`crate::instruments::common::mc`] for low-level MC infrastructure

pub mod barriers;
pub mod discretization;
pub mod engine;
pub mod greeks;
pub mod payoff;
pub mod pricer;
pub mod process;
pub mod results;
#[cfg(feature = "mc")]
pub mod seed;
pub mod traits;
pub mod variance_reduction;

/// Prelude for pricing-side convenient imports
pub mod prelude {
    // Engine and configuration
    pub use super::engine::{
        McEngine, McEngineBuilder, McEngineConfig, PathCaptureConfig, PathCaptureMode,
    };

    // Pricing results
    pub use super::results::{MoneyEstimate, MonteCarloResult};
    pub use super::traits::Payoff;

    // Re-export commonly used payoffs and pricers
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

    #[cfg(feature = "mc")]
    pub use super::pricer::basis::{LaguerreBasis, PolynomialBasis};
    pub use super::pricer::european::{EuropeanPricer, EuropeanPricerConfig};
    #[cfg(feature = "mc")]
    pub use super::pricer::lsmc::{AmericanCall, AmericanPut, LsmcConfig, LsmcPricer};
    #[cfg(feature = "mc")]
    pub use super::pricer::path_dependent::{PathDependentPricer, PathDependentPricerConfig};

    // Variance reduction helpers
    pub use super::variance_reduction::antithetic::AntitheticConfig;
    pub use super::variance_reduction::control_variate::{black_scholes_call, black_scholes_put};

    // Useful generic MC items
    pub use crate::instruments::common::mc::estimate::Estimate;
    pub use crate::instruments::common::mc::online_stats::OnlineStats;
    pub use crate::instruments::common::mc::paths::{
        PathDataset, PathPoint, PathSamplingMethod, ProcessParams, SimulatedPath,
    };
    pub use crate::instruments::common::mc::time_grid::TimeGrid;
    pub use crate::instruments::common::mc::traits::{
        Discretization, PathState, RandomStream, StochasticProcess,
    };
}
