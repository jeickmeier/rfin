//! Rho calculator for equity options.
//!
//! Computes rho using analytical Black–Scholes formula.
//!
//! Units & sign:
//! - Rho is exposed per +1bp (converted from analytical per‑percent greek)
//! - Rho = PV(rate + 1bp) − PV(base)
//! - Positive Rho means the instrument gains value when rates go up
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
//! Result from the analytical formula is per 1% (0.01) move; we convert to per 1bp.
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
        // Convert analytical per‑percent rho to per‑bp
        Ok(option.rho(&context.curves, context.as_of)? / 100.0)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}
