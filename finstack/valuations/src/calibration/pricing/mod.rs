/// Convention resolution for pricing (turns quote conventions + market defaults into effective inputs).
pub(crate) mod convention_resolution;
mod convexity;
mod pricer;

pub use convexity::{
    calculate_convexity_adjustment, default_convexity_params, estimate_rate_volatility,
    ho_lee_convexity, ConvexityParameters, VolatilitySource,
};
pub use pricer::{CalibrationPricer, RatesQuoteUseCase};
