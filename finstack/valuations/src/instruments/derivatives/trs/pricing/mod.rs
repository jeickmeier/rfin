//! TRS pricing entrypoints and pricers.

pub mod engine;
pub mod equity;
pub mod fixed_income_index;

use crate::instruments::derivatives::trs::{EquityTotalReturnSwap, FIIndexTotalReturnSwap, TrsSide};
use crate::instruments::derivatives::trs::helpers::validate_trs_currencies;
use crate::instruments::traits::Priceable;
use crate::metrics::MetricId;
use crate::results::ValuationResult;
use finstack_core::dates::Date;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;

impl Priceable for EquityTotalReturnSwap {
    fn value(&self, context: &MarketContext, as_of: Date) -> Result<Money> {
        // Currency safety
        validate_trs_currencies(self.notional, None)?;
        // Total return leg PV
        let tr_pv = equity::pv_total_return_leg(self, context, as_of)?;
        // Financing leg PV
        let fin_pv = engine::TrsEngine::pv_financing_leg(
            &self.financing,
            &self.schedule,
            self.notional,
            context,
            as_of,
        )?;

        // Net PV based on side
        let net_pv = match self.side {
            TrsSide::ReceiveTotalReturn => tr_pv - fin_pv,
            TrsSide::PayTotalReturn => fin_pv - tr_pv,
        }?;

        Ok(net_pv)
    }

    fn price_with_metrics(
        &self,
        context: &MarketContext,
        as_of: Date,
        _metrics: &[MetricId],
    ) -> Result<ValuationResult> {
        let npv = <Self as Priceable>::value(self, context, as_of)?;
        Ok(ValuationResult::stamped(self.id.as_str(), as_of, npv))
    }
}

impl Priceable for FIIndexTotalReturnSwap {
    fn value(&self, context: &MarketContext, as_of: Date) -> Result<Money> {
        // Currency safety (ensure notional matches index base currency)
        validate_trs_currencies(self.notional, Some(self.underlying.base_currency))?;
        // Total return leg PV
        let tr_pv = fixed_income_index::pv_total_return_leg(self, context, as_of)?;
        // Financing leg PV
        let fin_pv = engine::TrsEngine::pv_financing_leg(
            &self.financing,
            &self.schedule,
            self.notional,
            context,
            as_of,
        )?;

        // Net PV based on side
        let net_pv = match self.side {
            TrsSide::ReceiveTotalReturn => tr_pv - fin_pv,
            TrsSide::PayTotalReturn => fin_pv - tr_pv,
        }?;

        Ok(net_pv)
    }

    fn price_with_metrics(
        &self,
        context: &MarketContext,
        as_of: Date,
        _metrics: &[MetricId],
    ) -> Result<ValuationResult> {
        let npv = <Self as Priceable>::value(self, context, as_of)?;
        Ok(ValuationResult::stamped(self.id.as_str(), as_of, npv))
    }
}


