//! Theta calculator for equity options using analytical Black-Scholes formula.
//!
//! # Theta Convention: Trading Days (252/year)
//!
//! **Market Standards Review (Week 5):**
//!
//! Equity option theta follows market convention by returning theta **per trading day**
//! using 252 trading days per year. This differs from fixed income instruments which
//! typically use calendar days (365/year).
//!
//! ## Units
//!
//! - **Returns:** Theta per **trading day** (252 days/year)
//! - **To convert to calendar days:** `theta_calendar = theta_trading × 252 / 365`
//! - **Typical range:** -$0.01 to -$5.00 per day for ATM options
//!
//! ## Why Trading Days?
//!
//! Equity markets are only open for trading 252 days per year (excluding weekends
//! and holidays). Time decay occurs only on trading days, so theta is conventionally
//! quoted per trading day for equity derivatives.
//!
//! ## Comparison with Other Instruments
//!
//! | Instrument Class | Theta Convention | Days/Year | Rationale |
//! |-----------------|------------------|-----------|-----------|
//! | Equity Options | Trading days | 252 | Markets closed on weekends |
//! | FX Options | Trading days | 252 | FX markets trade 24/5 |
//! | Interest Rate Options | Trading days | 252 | Rate markets follow trading calendar |
//! | Bonds | Calendar days | 365 | Interest accrues every day |
//! | Swaps | Calendar days | 365 | Coupons accrue continuously |
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
//! **Result is annualized theta divided by 252 trading days to get per-trading-day theta.**

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
