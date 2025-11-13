//! Constituent delta calculator for baskets.
//!
//! Computes delta (price sensitivity) for each constituent using finite differences.
//! For each constituent, bumps its price by 1% and measures the impact on basket NAV.
//!
//! # Formula
//! ```text
//! ConstituentDelta_i = (PV(basket with bumped constituent_i) - PV_base) / bump_size
//! ```
//! Where bump_size is 1% (0.01) of the constituent price.
//!
//! Results are stored as a series with labels derived from constituent IDs or tickers.
//!
//! # Known Limitations
//!
//! Currently, `constituent_delta` only fully supports `ConstituentReference::MarketData`.
//! For `ConstituentReference::Instrument`, returns 0.0 as a placeholder.
//!
//! **Workaround**: Convert instrument references to synthetic market data prices
//! or use the instrument's own greeks directly if available.
//!
//! **Future Enhancement**: Will add `price_override` field to `BasketConstituent`
//! for full support of instrument-based constituents.

use crate::instruments::basket::{Basket, BasketConstituent};
use crate::instruments::common::traits::Instrument;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Standard price bump: 1% (0.01)
const PRICE_BUMP_PCT: f64 = 0.01;

/// Constituent delta calculator for baskets.
pub struct ConstituentDeltaCalculator;

impl MetricCalculator for ConstituentDeltaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let basket: &Basket = context.instrument_as()?;
        let as_of = context.as_of;
        let base_pv = context.base_value.amount();

        let mut series: Vec<(String, f64)> = Vec::new();
        let mut total_delta = 0.0;

        // For each constituent, bump its price and measure impact
        for constituent in &basket.constituents {
            let label = constituent
                .ticker
                .clone()
                .unwrap_or_else(|| constituent.id.clone());

            // Get current constituent price
            let current_price = get_constituent_price_value(basket, constituent, context)?;

            // Bump price up by 1%
            let bumped_price = current_price * (1.0 + PRICE_BUMP_PCT);
            let delta =
                bump_and_measure_delta(basket, constituent, bumped_price, context, as_of, base_pv)?;

            series.push((label, delta));
            total_delta += delta;
        }

        // Store as bucketed series
        context.store_bucketed_series(
            crate::metrics::MetricId::custom("constituent_delta"),
            series,
        );

        Ok(total_delta)
    }
}

/// Helper to get the price value (amount) of a constituent.
fn get_constituent_price_value(
    _basket: &Basket,
    constituent: &BasketConstituent,
    context: &MetricContext,
) -> Result<f64> {
    match &constituent.reference {
        #[cfg(feature = "serde")]
        crate::instruments::basket::types::ConstituentReference::Instrument(instr_json) => {
            // Convert InstrumentJson to boxed instrument and price it
            let boxed = instr_json.as_ref().clone().into_boxed()?;
            let price = boxed.value(context.curves.as_ref(), context.as_of)?;
            Ok(price.amount())
        }
        crate::instruments::basket::types::ConstituentReference::MarketData {
            price_id, ..
        } => {
            let scalar = context.curves.price(price_id.as_ref())?;
            match scalar {
                finstack_core::market_data::scalars::MarketScalar::Price(money) => {
                    Ok(money.amount())
                }
                finstack_core::market_data::scalars::MarketScalar::Unitless(v) => Ok(*v),
            }
        }
    }
}

/// Bump a constituent's price and measure the impact on basket NAV.
fn bump_and_measure_delta(
    basket: &Basket,
    constituent: &BasketConstituent,
    bumped_price: f64,
    context: &MetricContext,
    as_of: finstack_core::dates::Date,
    base_pv: f64,
) -> Result<f64> {
    use finstack_core::types::CurveId;

    // Create bumped market context
    let mut bumped_ctx = context.curves.as_ref().clone();

    match &constituent.reference {
        crate::instruments::basket::types::ConstituentReference::Instrument(_) => {
            // For instruments, we can't easily bump their internal prices
            // For now, return 0 - this would require instrument cloning and price override
            // which is complex. In practice, constituent_delta for instrument references
            // might need special handling or the instrument itself should expose delta.
            tracing::warn!(
                constituent_id = %constituent.id,
                "Basket ConstituentDelta: Instrument-based constituents not yet supported, returning 0.0"
            );
            return Ok(0.0);
        }
        crate::instruments::basket::types::ConstituentReference::MarketData {
            price_id, ..
        } => {
            // Bump the price scalar
            let current_scalar = bumped_ctx.price(price_id.as_ref())?;
            let new_scalar = match current_scalar {
                finstack_core::market_data::scalars::MarketScalar::Price(m) => {
                    finstack_core::market_data::scalars::MarketScalar::Price(
                        finstack_core::money::Money::new(bumped_price, m.currency()),
                    )
                }
                finstack_core::market_data::scalars::MarketScalar::Unitless(_) => {
                    finstack_core::market_data::scalars::MarketScalar::Unitless(bumped_price)
                }
            };
            bumped_ctx
                .prices
                .insert(CurveId::from(price_id.as_ref()), new_scalar);
        }
    }

    // Reprice basket with bumped constituent
    let pv_bumped = basket.value(&bumped_ctx, as_of)?.amount();

    // Delta = (PV_bumped - PV_base) / bump_size
    // bump_size is the absolute change in price, so we divide by (bumped_price - current_price)
    let current_price = get_constituent_price_value(basket, constituent, context)?;
    let bump_size = bumped_price - current_price;
    let delta = if bump_size.abs() > 1e-10 {
        (pv_bumped - base_pv) / bump_size * current_price // Scale to per 1% of price
    } else {
        0.0
    };

    Ok(delta)
}
