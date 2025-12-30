//! CollateralPrice01 calculator for Repo.
//!
//! Computes CollateralPrice01 (collateral price sensitivity) using finite differences.
//! CollateralPrice01 measures the change in PV for a 1% change in collateral price.
//!
//! # Formula
//! ```text
//! CollateralPrice01 = (PV(collateral_price * 1.01) - PV(collateral_price * 0.99)) / (2 * bump_size)
//! ```
//! Where bump_size is 1% (0.01).
//!
//! # Note
//! Collateral price is accessed via `collateral.market_value_id` from MarketContext.
//! This metric bumps the collateral price in MarketContext and reprices the repo.
//! Changes in collateral price may affect margin requirements and collateral coverage.

use crate::instruments::common::traits::Instrument;
use crate::instruments::repo::Repo;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::market_data::scalars::MarketScalar;
use finstack_core::Result;

/// Standard collateral price bump: 1% (0.01)
const COLLATERAL_PRICE_BUMP_PCT: f64 = 0.01;

/// CollateralPrice01 calculator for Repo.
pub struct CollateralPrice01Calculator;

impl MetricCalculator for CollateralPrice01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let repo: &Repo = context.instrument_as()?;
        let as_of = context.as_of;

        // Get current collateral price
        let market_value_id = &repo.collateral.market_value_id;
        let current_scalar = context.curves.price(market_value_id)?;
        let current_price = match current_scalar {
            MarketScalar::Price(money) => money.amount(),
            MarketScalar::Unitless(v) => *v,
        };

        // Bump collateral price up by 1%
        let bumped_price_up = current_price * (1.0 + COLLATERAL_PRICE_BUMP_PCT);
        let mut ctx_up = context.curves.as_ref().clone();
        let new_scalar_up = match current_scalar {
            MarketScalar::Price(m) => MarketScalar::Price(finstack_core::money::Money::new(
                bumped_price_up,
                m.currency(),
            )),
            MarketScalar::Unitless(_) => MarketScalar::Unitless(bumped_price_up),
        };
        ctx_up = ctx_up.insert_price(market_value_id.as_str(), new_scalar_up);
        let pv_up = repo.value(&ctx_up, as_of)?.amount();

        // Bump collateral price down by 1%
        let bumped_price_down = current_price * (1.0 - COLLATERAL_PRICE_BUMP_PCT);
        let mut ctx_down = context.curves.as_ref().clone();
        let new_scalar_down = match current_scalar {
            MarketScalar::Price(m) => MarketScalar::Price(finstack_core::money::Money::new(
                bumped_price_down,
                m.currency(),
            )),
            MarketScalar::Unitless(_) => MarketScalar::Unitless(bumped_price_down),
        };
        ctx_down = ctx_down.insert_price(market_value_id.as_str(), new_scalar_down);
        let pv_down = repo.value(&ctx_down, as_of)?.amount();

        // CollateralPrice01 = (PV_up - PV_down) / (2 * bump_size)
        // Result is per 1% change in collateral price
        let bump_size = current_price * COLLATERAL_PRICE_BUMP_PCT;
        let collateral_price01 = if bump_size.abs() > 1e-10 {
            (pv_up - pv_down) / (2.0 * bump_size) * current_price // Scale to per 1% of price
        } else {
            0.0
        };

        Ok(collateral_price01)
    }
}
