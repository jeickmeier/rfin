//! Dividend risk calculator for convertible bonds.
//!
//! Computes dividend risk (dividend yield sensitivity) using finite differences.
//! Dividend risk measures the change in PV for a 1bp (0.0001) change in dividend yield.
//!
//! # Note
//! For convertibles, dividend yield affects the equity option component.
//! Higher dividend yield reduces the forward price, making conversion less attractive.

use crate::instruments::common::traits::Instrument;
use crate::instruments::fixed_income::convertible::ConvertibleBond;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Standard dividend yield bump: 1bp (0.0001)
const DIVIDEND_BUMP_BP: f64 = 0.0001;

/// Dividend risk calculator for convertible bonds.
pub struct DividendRiskCalculator;

impl MetricCalculator for DividendRiskCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let convertible: &ConvertibleBond = context.instrument_as()?;
        let as_of = context.as_of;
        let _base_pv = context.base_value.amount();

        // Resolve dividend yield ID using same logic as pricer
        let underlying_id = convertible.underlying_equity_id.as_ref().ok_or_else(|| {
            finstack_core::Error::from(finstack_core::InputError::NotFound {
                id: "underlying_equity_id".to_string(),
            })
        })?;

        let mut dividend_candidates: Vec<String> = Vec::new();
        if let Some(id) = convertible.attributes.get_meta("div_yield_id") {
            dividend_candidates.push(id.to_string());
        }
        dividend_candidates.push(format!("{}-DIVYIELD", underlying_id));
        if let Some(stripped) = underlying_id.strip_suffix("-SPOT") {
            dividend_candidates.push(format!("{}-DIVYIELD", stripped));
        }

        // Find the first available dividend yield ID
        let div_yield_id = dividend_candidates
            .iter()
            .find(|id| context.curves.price(id.as_str()).is_ok())
            .cloned();

        let div_yield_id = match div_yield_id {
            Some(id) => id,
            None => return Ok(0.0), // No dividend yield available, risk is zero
        };

        // Get current dividend yield
        let current_scalar = context.curves.price(&div_yield_id)?;

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
        curves_up = curves_up.insert_price(div_yield_id.as_str(), new_value_up);
        let pv_up = convertible.value(&curves_up, as_of)?.amount();

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
        curves_down = curves_down.insert_price(div_yield_id.as_str(), new_value_down);
        let pv_down = convertible.value(&curves_down, as_of)?.amount();

        // Dividend01 = (PV_up - PV_down) / (2 * bump_size)
        let dividend01 = (pv_up - pv_down) / (2.0 * DIVIDEND_BUMP_BP);

        Ok(dividend01)
    }
}
