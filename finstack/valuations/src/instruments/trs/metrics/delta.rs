use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::{Error, Result, F};
use crate::instruments::trs::{EquityTotalReturnSwap, FIIndexTotalReturnSwap, TrsSide};

/// Calculates delta to the underlying index level for a TRS.
///
/// Delta measures the sensitivity of the TRS value to changes in the underlying index level.
/// For equity TRS, this is approximately notional/spot. For fixed income index TRS,
/// this uses duration as a proxy for sensitivity.
pub struct IndexDeltaCalculator;

impl MetricCalculator for IndexDeltaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        if let Some(equity_trs) = context.instrument.as_any().downcast_ref::<EquityTotalReturnSwap>() {
            let spot = match context.curves.price(&equity_trs.underlying.spot_id)? {
                finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
                finstack_core::market_data::scalars::MarketScalar::Price(p) => p.amount(),
            };
            if spot.abs() < 1e-10 {
                return Err(Error::Validation("Spot price too small for delta calculation".into()));
            }
            let delta = equity_trs.notional.amount() * equity_trs.underlying.contract_size / spot;
            Ok(match equity_trs.side { TrsSide::ReceiveTotalReturn => delta, TrsSide::PayTotalReturn => -delta })
        } else if let Some(fi_trs) = context.instrument.as_any().downcast_ref::<FIIndexTotalReturnSwap>() {
            let duration = fi_trs
                .underlying
                .duration_id
                .as_ref()
                .and_then(|id| {
                    context.curves.price(id.as_str()).ok().map(|s| match s {
                        finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
                        finstack_core::market_data::scalars::MarketScalar::Price(p) => p.amount(),
                    })
                })
                .unwrap_or(5.0);
            let delta = fi_trs.notional.amount() * duration * 0.0001;
            Ok(match fi_trs.side { TrsSide::ReceiveTotalReturn => delta, TrsSide::PayTotalReturn => -delta })
        } else {
            Err(Error::Input(finstack_core::error::InputError::Invalid))
        }
    }
}


