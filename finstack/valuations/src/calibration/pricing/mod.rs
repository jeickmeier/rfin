//! Shared pricing infrastructure for curve calibration.
//!
//! This module provides the [`CalibrationPricer`] used by all calibration methods
//! to price instruments against candidate curves during optimization. It also
//! includes convexity adjustment calculations for futures pricing.
//!
//! # Architecture
//!
//! The pricing module sits between quote schemas and calibration methods:
//!
//! ```text
//! quotes/          →  pricing/           →  methods/
//! (pure schemas)      (pricer + convexity)  (discount, forward, hazard, ...)
//! ```
//!
//! # Key Types
//!
//! - [`CalibrationPricer`]: Instrument pricer with settlement and curve configuration
//! - [`RatesQuoteUseCase`]: Validation mode (discount vs forward curve)
//! - [`ConvexityParameters`]: Futures convexity adjustment configuration

mod convexity;
mod pricer;

pub use convexity::{
    calculate_convexity_adjustment, default_convexity_params, estimate_rate_volatility,
    ho_lee_convexity, ConvexityParameters, VolatilitySource,
};
pub use pricer::{CalibrationPricer, RatesQuoteUseCase};

