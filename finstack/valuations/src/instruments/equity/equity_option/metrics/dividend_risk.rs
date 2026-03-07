//! Dividend risk calculator for equity options.
//!
//! Computes dividend risk (dividend yield sensitivity) using finite differences.
//! Dividend risk measures the change in PV for a 1bp (0.0001) change in dividend yield.
//!
//! # Formula
//! ```text
//! Dividend01 = (PV(q + dq) - PV(q - dq)) / (q_up - q_down) * dq
//! ```
//! Where `dq` is the bump size (e.g., 0.0001 for 1bp).
//!
//! # Note
//! For options, dividend yield affects the forward price: F = S * exp((r - q) * T).
//! Higher dividend yield reduces the forward, making calls less valuable and puts more valuable.

use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::equity::equity_option::EquityOption;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Standard dividend yield bump: 1bp (0.0001)
const DIVIDEND_BUMP_BP: f64 = 0.0001;

/// Dividend risk calculator for equity options.
pub struct DividendRiskCalculator;

impl MetricCalculator for DividendRiskCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &EquityOption = context.instrument_as()?;
        let as_of = context.as_of;

        // Check if expired
        let t = option.day_count.year_fraction(
            as_of,
            option.expiry,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        if t <= 0.0 {
            return Ok(0.0);
        }

        // If no dividend yield ID, risk is zero
        let div_yield_id = match &option.div_yield_id {
            Some(id) => id.clone(),
            None => return Ok(0.0),
        };

        // Get current dividend yield
        let _current_div = match context.curves.get_price(&div_yield_id) {
            Ok(scalar) => match scalar {
                finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
                finstack_core::market_data::scalars::MarketScalar::Price(_) => 0.0,
            },
            Err(_) => 0.0, // Default to 0 if not found
        };

        // Get current scalar to clone its structure
        let current_scalar = context.curves.get_price(&div_yield_id)?;

        // Extract numeric baseline for robust bump-width handling (clamped at 0 on the downside).
        let q0 = match current_scalar {
            finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
            finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
        };
        let q_up_val = q0 + DIVIDEND_BUMP_BP;
        let q_down_val = (q0 - DIVIDEND_BUMP_BP).max(0.0);
        let actual_width = q_up_val - q_down_val;

        // Bump up
        let mut curves_up = context.curves.as_ref().clone();
        let new_value_up = match current_scalar {
            finstack_core::market_data::scalars::MarketScalar::Unitless(_) => {
                finstack_core::market_data::scalars::MarketScalar::Unitless(q_up_val)
            }
            finstack_core::market_data::scalars::MarketScalar::Price(m) => {
                finstack_core::market_data::scalars::MarketScalar::Price(
                    finstack_core::money::Money::new(q_up_val, m.currency()),
                )
            }
        };
        curves_up = curves_up.insert_price(div_yield_id.as_str(), new_value_up);
        let pv_up = option.value(&curves_up, as_of)?.amount();

        // Bump down
        let mut curves_down = context.curves.as_ref().clone();
        let new_value_down = match current_scalar {
            finstack_core::market_data::scalars::MarketScalar::Unitless(_) => {
                finstack_core::market_data::scalars::MarketScalar::Unitless(q_down_val)
            }
            finstack_core::market_data::scalars::MarketScalar::Price(m) => {
                finstack_core::market_data::scalars::MarketScalar::Price(
                    finstack_core::money::Money::new(q_down_val, m.currency()),
                )
            }
        };
        curves_down = curves_down.insert_price(div_yield_id.as_str(), new_value_down);
        let pv_down = option.value(&curves_down, as_of)?.amount();

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
