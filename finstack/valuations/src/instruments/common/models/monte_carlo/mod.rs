//! Monte Carlo pricing module (pricing-specific components).
//!
//! This module contains payoffs, pricers, variance reduction, Greeks,
//! and the pricing engine that build on top of the generic MC
//! infrastructure under `instruments::common::mc`.

pub mod analytical;
pub mod barriers;
pub mod greeks;
pub mod payoff;
pub mod pricer;
pub mod variance_reduction;
pub mod discretization;
pub mod engine;
#[cfg(feature = "mc")]
pub mod seed;
pub mod results;
pub mod traits;
pub mod process;

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
    pub use super::payoff::vanilla::{Digital, EuropeanCall, EuropeanPut, Forward};
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

    pub use super::pricer::european::{EuropeanPricer, EuropeanPricerConfig};
    #[cfg(feature = "mc")]
    pub use super::pricer::path_dependent::{PathDependentPricer, PathDependentPricerConfig};
    #[cfg(feature = "mc")]
    pub use super::pricer::lsmc::{
        AmericanCall, AmericanPut, LaguerreBasis, LsmcConfig, LsmcPricer, PolynomialBasis,
    };

    // Variance reduction helpers
    pub use super::variance_reduction::antithetic::AntitheticConfig;
    pub use super::variance_reduction::control_variate::{black_scholes_call, black_scholes_put};

    // Useful generic MC items
    pub use crate::instruments::common::mc::path_data::{
        PathDataset, PathPoint, PathSamplingMethod, ProcessParams, SimulatedPath,
    };
    pub use crate::instruments::common::mc::results::Estimate;
    pub use crate::instruments::common::mc::stats::OnlineStats;
    pub use crate::instruments::common::mc::time_grid::TimeGrid;
    pub use crate::instruments::common::mc::traits::{
        Discretization, PathState, RandomStream, StochasticProcess,
    };
}
