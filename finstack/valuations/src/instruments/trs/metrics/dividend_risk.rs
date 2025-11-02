//! Dividend risk calculator for equity TRS.
//!
//! Computes dividend risk (dividend yield sensitivity) using finite differences.
//! Dividend risk measures the change in PV for a 1bp (0.0001) change in dividend yield.
//!
//! # Note
//! For TRS, dividend yield affects the forward price of the underlying equity,
//! which impacts the total return leg value.

use crate::instruments::trs::EquityTotalReturnSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Standard dividend yield bump: 1bp (0.0001)
const DIVIDEND_BUMP_BP: f64 = 0.0001;

/// Dividend risk calculator for equity TRS.
pub struct DividendRiskCalculator;

impl MetricCalculator for DividendRiskCalculator {
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

        // Bump dividend yield up and down
        use finstack_core::types::CurveId;

        // Bump up
        let mut curves_up = context.curves.as_ref().clone();
        let new_value_up = match current_scalar {
            finstack_core::market_data::scalars::MarketScalar::Unitless(v) => {
                finstack_core::market_data::scalars::MarketScalar::Unitless(v + DIVIDEND_BUMP_BP)
            }
            finstack_core::market_data::scalars::MarketScalar::Price(m) => {
                finstack_core::market_data::scalars::MarketScalar::Price(
                    finstack_core::money::Money::new(m.amount() + DIVIDEND_BUMP_BP, m.currency()),
                )
            }
        };
        curves_up
            .prices
            .insert(CurveId::from(div_yield_id.clone()), new_value_up);
        let pv_up = trs.npv(&curves_up, as_of)?.amount();

        // Bump down
        let mut curves_down = context.curves.as_ref().clone();
        let div_down_value = match current_scalar {
            finstack_core::market_data::scalars::MarketScalar::Unitless(v) => {
                (v - DIVIDEND_BUMP_BP).max(0.0)
            }
            finstack_core::market_data::scalars::MarketScalar::Price(m) => {
                (m.amount() - DIVIDEND_BUMP_BP).max(0.0)
            }
        };
        let new_value_down = match current_scalar {
            finstack_core::market_data::scalars::MarketScalar::Unitless(_) => {
                finstack_core::market_data::scalars::MarketScalar::Unitless(div_down_value)
            }
            finstack_core::market_data::scalars::MarketScalar::Price(m) => {
                finstack_core::market_data::scalars::MarketScalar::Price(
                    finstack_core::money::Money::new(div_down_value, m.currency()),
                )
            }
        };
        curves_down
            .prices
            .insert(CurveId::from(div_yield_id), new_value_down);
        let pv_down = trs.npv(&curves_down, as_of)?.amount();

        // Dividend01 = (PV_up - PV_down) / (2 * bump_size)
        let dividend01 = (pv_up - pv_down) / (2.0 * DIVIDEND_BUMP_BP);

        Ok(dividend01)
    }
}
