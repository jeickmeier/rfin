//! Theta calculator for equity options using analytical Black-Scholes formula.
//!
//! # Market Standard Formula
//!
//! For European options under Black-Scholes:
//!
//! **Call Theta:**
//! Θ = -[S × N'(d₁) × σ × e^(-qT)] / (2√T) - r × K × e^(-rT) × N(d₂) + q × S × e^(-qT) × N(d₁)
//!
//! **Put Theta:**
//! Θ = -[S × N'(d₁) × σ × e^(-qT)] / (2√T) + r × K × e^(-rT) × N(-d₂) - q × S × e^(-qT) × N(-d₁)
//!
//! Where:
//! - S = spot price, K = strike, r = risk-free rate, q = dividend yield
//! - σ = volatility, T = time to expiry
//! - N(·) = cumulative normal distribution, N'(·) = normal PDF
//!
//! Result is annualized theta divided by trading days per year (252) to get daily theta.

use crate::instruments::equity_option::pricer;
use crate::instruments::equity_option::EquityOption;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::Result;

pub struct ThetaCalculator;

impl MetricCalculator for ThetaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &EquityOption = context.instrument_as()?;
        
        // Use analytical Black-Scholes theta from pricer (market standard)
        let greeks = pricer::compute_greeks(option, &context.curves, context.as_of)?;
        
        Ok(greeks.theta)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}
