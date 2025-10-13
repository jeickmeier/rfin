//! Equity Option DV01 metric calculator.
//!
//! Provides DV01 calculation for Equity Option instruments.
//!
//! # Market Standard Formula
//!
//! For options, DV01 represents sensitivity to interest rate changes.
//! In Black-Scholes, this is captured by Rho (sensitivity per 1% rate change).
//!
//! DV01 = Rho / 100
//!
//! Where Rho for a call option is:
//! ρ = K × T × e^(-rT) × N(d₂) / 100  (per 1% rate)
//!
//! And for a put option is:
//! ρ = -K × T × e^(-rT) × N(-d₂) / 100  (per 1% rate)
//!
//! # Note
//!
//! DV01 for equity options is typically small compared to delta and vega risks.
//! Rho is the more commonly reported metric in practice.

use crate::instruments::equity_option::pricer;
use crate::instruments::equity_option::EquityOption;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// DV01 calculator for Equity Option instruments using analytical formula.
pub struct EquityOptionDv01Calculator;

impl MetricCalculator for EquityOptionDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &EquityOption = context.instrument_as()?;
        let as_of = context.as_of;

        if as_of >= option.expiry {
            return Ok(0.0);
        }

        // Use analytical rho from Black-Scholes, convert to per-bp from per-percent
        let greeks = pricer::compute_greeks(option, &context.curves, as_of)?;
        let dv01 = greeks.rho / 100.0; // Rho is per 1%, DV01 is per 1bp

        Ok(dv01)
    }
}
