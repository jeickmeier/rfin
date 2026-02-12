//! Dividend risk calculator for equity TRS.
//!
//! Computes dividend risk (dividend yield sensitivity) using finite differences.
//! Dividend risk measures the change in PV for a 1bp (0.0001) change in dividend yield.

use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::equity::equity_trs::EquityTotalReturnSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::market_data::scalars::MarketScalar;
use finstack_core::money::Money;
use finstack_core::Result;

/// Standard dividend yield bump: 1bp (0.0001)
const DIVIDEND_BUMP_BP: f64 = 0.0001;

/// Dividend risk (Dividend01) calculator for equity TRS.
///
/// Measures the sensitivity of TRS value to changes in dividend yield.
/// For equity TRS, dividend yield affects the forward price of the underlying equity,
/// which impacts the total return leg value.
pub struct Dividend01Calculator;

impl MetricCalculator for Dividend01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let trs: &EquityTotalReturnSwap = context.instrument_as()?;
        let as_of = context.as_of;

        // If no dividend yield ID, risk is zero
        let div_yield_id = match &trs.underlying.div_yield_id {
            Some(id) => id.clone(),
            None => return Ok(0.0),
        };

        // Get current dividend yield
        let current_scalar = match context.curves.price(&div_yield_id) {
            Ok(scalar) => scalar,
            Err(_) => return Ok(0.0), // Default to 0 if not found
        };

        // Extract numeric baseline for robust bump-width handling (clamped at 0 on the downside).
        let q0 = match current_scalar {
            MarketScalar::Unitless(v) => *v,
            MarketScalar::Price(m) => m.amount(),
        };
        let q_up_val = q0 + DIVIDEND_BUMP_BP;
        let q_down_val = (q0 - DIVIDEND_BUMP_BP).max(0.0);
        let actual_width = q_up_val - q_down_val;

        // Bump dividend yield up
        let mut curves_up = context.curves.as_ref().clone();
        let new_value_up = match current_scalar {
            MarketScalar::Unitless(_) => MarketScalar::Unitless(q_up_val),
            MarketScalar::Price(m) => MarketScalar::Price(Money::new(q_up_val, m.currency())),
        };
        curves_up = curves_up.insert_price(div_yield_id.as_str(), new_value_up);
        let pv_up = trs.value(&curves_up, as_of)?.amount();

        // Bump dividend yield down
        let mut curves_down = context.curves.as_ref().clone();
        let new_value_down = match current_scalar {
            MarketScalar::Unitless(_) => MarketScalar::Unitless(q_down_val),
            MarketScalar::Price(m) => MarketScalar::Price(Money::new(q_down_val, m.currency())),
        };
        curves_down = curves_down.insert_price(div_yield_id.as_str(), new_value_down);
        let pv_down = trs.value(&curves_down, as_of)?.amount();

        // MetricId contract: Dividend01 is $/bp (dPV for a 1bp absolute q move).
        // Use actual bump width since the downside bump is clamped at 0.
        let dividend01 = if actual_width > 0.0 {
            (pv_up - pv_down) / actual_width * DIVIDEND_BUMP_BP
        } else {
            0.0
        };

        Ok(dividend01)
    }
}
