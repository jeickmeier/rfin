//! Equity Total Return Swap implementation.

use super::types::{FinancingLegSpec, TotalReturnLegParams, TrsEngine, TrsScheduleSpec, TrsSide};
use crate::instruments::traits::{Attributable, Instrument};
use crate::{
    cashflow::{
        builder::schedule_utils::build_dates,
        traits::{CashflowProvider, DatedFlows},
    },
    instruments::{
        common::parameter_groups::{
            validate_currency_consistency, EquityUnderlyingParams,
        },
        traits::{Attributes, Priceable},
    },
    metrics::MetricId,
    results::ValuationResult,
};
use finstack_core::{
    dates::Date,
    market_data::MarketContext,
    money::Money,
    types::InstrumentId,
    Result, F,
};
use std::any::Any;

/// Equity Total Return Swap instrument
#[derive(Clone, Debug, finstack_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct EquityTotalReturnSwap {
    /// Unique instrument identifier
    pub id: InstrumentId,
    /// Notional amount
    pub notional: Money,
    /// Underlying equity parameters
    pub underlying: EquityUnderlyingParams,
    /// Financing leg specification
    pub financing: FinancingLegSpec,
    /// Schedule specification
    pub schedule: TrsScheduleSpec,
    /// Trade side (receive/pay total return)
    pub side: TrsSide,
    /// Initial index level (if known)
    pub initial_level: Option<F>,
    /// Attributes for scenario selection
    pub attributes: Attributes,
}

impl EquityTotalReturnSwap {

    /// Extract underlying data for return calculation
    fn extract_underlying_data(&self, context: &MarketContext) -> Result<(F, F)> {
        // Get spot price
        let spot = match context.price(&self.underlying.spot_id)? {
            finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
            finstack_core::market_data::scalars::MarketScalar::Price(p) => p.amount(),
        };

        // Get dividend yield if available
        let div_yield = self
            .underlying
            .dividend_yield_id
            .as_ref()
            .and_then(|id| {
                context.price(id.as_str()).ok().map(|s| match s {
                    finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
                    finstack_core::market_data::scalars::MarketScalar::Price(p) => p.amount(),
                })
            })
            .unwrap_or(0.0);

        Ok((spot, div_yield))
    }

    /// Calculate PV of the total return leg
    pub(super) fn pv_total_return_leg(
        &self,
        context: &MarketContext,
        as_of: Date,
    ) -> Result<Money> {
        // Extract underlying data
        let (spot, div_yield) = self.extract_underlying_data(context)?;
        let initial = self.initial_level.unwrap_or(spot);

        // Use shared implementation with equity-specific return calculation
        let params = TotalReturnLegParams {
            schedule: &self.schedule,
            notional: self.notional,
            disc_id: self.financing.disc_id.as_str(),
            contract_size: self.underlying.contract_size,
            initial_level: Some(initial),
        };

        TrsEngine::pv_total_return_leg_common(
            params,
            context,
            as_of,
            |_period_start, _period_end, t_start, t_end, initial_level, context| {
                // Get discount curve for forward calculation
                let disc = context
                    .get_ref::<finstack_core::market_data::term_structures::discount_curve::DiscountCurve>(
                        self.financing.disc_id.as_str(),
                    )?;
                
                // Forward levels using cost-of-carry model
                // F(t) = S0 * exp((r - q) * t)
                // where r is implied from discount curve, q is dividend yield
                let df_start = disc.df(t_start);
                let df_end = disc.df(t_end);

                // Implied forward levels
                let fwd_start = initial_level * df_start.recip() * (-div_yield * t_start).exp();
                let fwd_end = initial_level * df_end.recip() * (-div_yield * t_end).exp();

                // Total return for the period
                let total_return = (fwd_end - fwd_start) / fwd_start;

                Ok(total_return)
            },
        )
    }
}

impl Priceable for EquityTotalReturnSwap {
    fn value(&self, context: &MarketContext, as_of: Date) -> Result<Money> {
        // Validate currency consistency
        validate_currency_consistency(&[self.notional])?;

        // Calculate leg PVs
        let tr_pv = self.pv_total_return_leg(context, as_of)?;
        let fin_pv = TrsEngine::pv_financing_leg(
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

        let result = ValuationResult::stamped(self.id.as_str(), as_of, npv);

        // TODO: Add metrics if requested
        // This would require access to the MetricRegistry to calculate metrics

        Ok(result)
    }
}

impl Attributable for EquityTotalReturnSwap {
    fn attributes(&self) -> &crate::instruments::traits::Attributes {
        // For now, return a static empty attributes
        // In a real implementation, this would be a field in the struct
        static EMPTY: once_cell::sync::Lazy<crate::instruments::traits::Attributes> =
            once_cell::sync::Lazy::new(crate::instruments::traits::Attributes::default);
        &EMPTY
    }

    fn attributes_mut(&mut self) -> &mut crate::instruments::traits::Attributes {
        // This would normally return a mutable reference to an attributes field
        // For now, we'll panic as this is not properly implemented
        unimplemented!("Mutable attributes not yet implemented for EquityTotalReturnSwap")
    }
}

impl Instrument for EquityTotalReturnSwap {
    fn id(&self) -> &str { self.id.as_str() }
    fn instrument_type(&self) -> &'static str { "EquityTotalReturnSwap" }
    fn as_any(&self) -> &dyn Any { self }
    fn attributes(&self) -> &crate::instruments::traits::Attributes { <Self as Attributable>::attributes(self) }
    fn attributes_mut(&mut self) -> &mut crate::instruments::traits::Attributes { <Self as Attributable>::attributes_mut(self) }
    fn clone_box(&self) -> Box<dyn Instrument> { Box::new(self.clone()) }
}

// Do not add explicit Instrument impl; provided by blanket impl.

impl CashflowProvider for EquityTotalReturnSwap {
    fn build_schedule(&self, _context: &MarketContext, _as_of: Date) -> Result<DatedFlows> {
        // For TRS, we'll return the expected payment dates
        // Actual amounts depend on realized returns
        let period_schedule = build_dates(
            self.schedule.start,
            self.schedule.end,
            self.schedule.params.frequency,
            self.schedule.params.stub,
            self.schedule.params.bdc,
            self.schedule.params.calendar_id,
        );

        let mut flows = Vec::new();
        for date in period_schedule.dates.iter().skip(1) {
            // Add a placeholder flow for each payment date
            // In practice, the amount would be determined at fixing
            flows.push((*date, Money::new(0.0, self.notional.currency())));
        }

        Ok(flows)
    }
}

// Manual builder removed; derive-based builder is used.
