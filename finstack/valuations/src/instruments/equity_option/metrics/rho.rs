//! Rho calculator for equity options.
//!
//! Computes rho using analytical Black–Scholes formula.
//!
//! # Market Standard Formula
//!
//! **Call Rho:** ρ = (K × T × e^(-rT) × N(d₂)) / 100
//!
//! **Put Rho:** ρ = -(K × T × e^(-rT) × N(-d₂)) / 100
//!
//! Where:
//! - K = strike price
//! - T = time to expiration (years)
//! - r = risk-free interest rate
//! - N(·) = cumulative standard normal distribution
//! - d₂ = d₁ - σ√T
//!
//! Result is per 1% (0.01) interest rate move and scaled by contract size.
//!
//! # Note
//!
//! Rho represents the rate of change of option value with respect to
//! the risk-free interest rate. It is generally small for equity options
//! compared to delta and vega, but becomes more significant for longer-dated
//! options and higher strike prices.

use crate::instruments::equity_option::EquityOption;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::Result;

pub struct RhoCalculator;

impl MetricCalculator for RhoCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &EquityOption = context.instrument_as()?;
        option.rho(&context.curves, context.as_of)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}
