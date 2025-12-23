//! Monte Carlo pricing module (pricing-specific components).
//!
//! This module contains payoffs, pricers, variance reduction, Greeks,
//! and the pricing engine that build on top of the generic MC
//! infrastructure under `instruments::common::mc`.

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
