//! Fixed Income Index Total Return Swap implementation.

use super::types::{FinancingLegSpec, TotalReturnLegParams, TrsEngine, TrsScheduleSpec, TrsSide};
use crate::instruments::traits::{Attributable, Instrument};
use crate::{
    cashflow::{
        builder::schedule_utils::build_dates,
        traits::{CashflowProvider, DatedFlows},
    },
    instruments::{
        common::parameter_groups::{
            validate_currency_consistency, IndexUnderlyingParams,
        },
        traits::{Attributes, Priceable},
    },
    metrics::MetricId,
    results::ValuationResult,
};
use finstack_core::{
    dates::{Date, DayCountCtx},
    market_data::MarketContext,
    money::Money,
    types::InstrumentId,
    Error, Result, F,
};
use std::any::Any;

/// Fixed Income Index Total Return Swap instrument
#[derive(Clone, Debug, finstack_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct FIIndexTotalReturnSwap {
    /// Unique instrument identifier
    pub id: InstrumentId,
    /// Notional amount
    pub notional: Money,
    /// Underlying index parameters
    pub underlying: IndexUnderlyingParams,
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

impl FIIndexTotalReturnSwap {

    /// Extract underlying data for return calculation
    fn extract_underlying_data(&self, context: &MarketContext) -> Result<(F, F)> {
        // Get index yield if available (for carry calculation)
        let index_yield = self
            .underlying
            .yield_id
            .as_ref()
            .and_then(|id| {
                context.price(id.as_str()).ok().map(|s| match s {
                    finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
                    finstack_core::market_data::scalars::MarketScalar::Price(p) => p.amount(),
                })
            })
            .unwrap_or(0.0);

        // Get duration if available (for roll-down approximation)
        let duration = self
            .underlying
            .duration_id
            .as_ref()
            .and_then(|id| {
                context.price(id.as_str()).ok().map(|s| match s {
                    finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
                    finstack_core::market_data::scalars::MarketScalar::Price(p) => p.amount(),
                })
            })
            .unwrap_or(0.0);

        Ok((index_yield, duration))
    }

    /// Calculate PV of the total return leg using carry-only approximation
    pub(super) fn pv_total_return_leg(
        &self,
        context: &MarketContext,
        as_of: Date,
    ) -> Result<Money> {
        // Extract underlying data
        let (index_yield, duration) = self.extract_underlying_data(context)?;

        // Use shared implementation with fixed income-specific return calculation
        let params = TotalReturnLegParams {
            schedule: &self.schedule,
            notional: self.notional,
            disc_id: self.financing.disc_id.as_str(),
            contract_size: self.underlying.contract_size,
            initial_level: self.initial_level,
        };

        TrsEngine::pv_total_return_leg_common(
            params,
            context,
            as_of,
            |period_start, period_end, _t_start, _t_end, _initial_level, _context| {
                // Calculate year fraction for the period
                let ctx = DayCountCtx::default();
                let yf = self
                    .schedule
                    .params
                    .day_count
                    .year_fraction(period_start, period_end, ctx)?;

                // Carry component: yield * time
                let carry_return = index_yield * yf;

                // Optional roll-down component (simplified)
                // In a more sophisticated model, we'd look at the forward curve slope
                let roll_return = if duration > 0.0 {
                    // Approximate roll-down as duration * yield change
                    // This is a placeholder - real implementation would use forward curve
                    let yield_change_estimate = -0.0001 * yf; // -1bp per year estimate
                    duration * yield_change_estimate
                } else {
                    0.0
                };

                // Total return for the period
                let total_return = carry_return + roll_return;

                Ok(total_return)
            },
        )
    }
}

impl Priceable for FIIndexTotalReturnSwap {
    fn value(&self, context: &MarketContext, as_of: Date) -> Result<Money> {
        // Validate currency consistency
        validate_currency_consistency(&[self.notional])?;

        // Ensure notional currency matches index base currency
        if self.notional.currency() != self.underlying.base_currency {
            return Err(Error::CurrencyMismatch {
                expected: self.underlying.base_currency,
                actual: self.notional.currency(),
            });
        }

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

impl Attributable for FIIndexTotalReturnSwap {
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
        unimplemented!("Mutable attributes not yet implemented for FIIndexTotalReturnSwap")
    }
}

impl Instrument for FIIndexTotalReturnSwap {
    fn id(&self) -> &str { self.id.as_str() }
    fn instrument_type(&self) -> &'static str { "FIIndexTotalReturnSwap" }
    fn as_any(&self) -> &dyn Any { self }
    fn attributes(&self) -> &crate::instruments::traits::Attributes { <Self as Attributable>::attributes(self) }
    fn attributes_mut(&mut self) -> &mut crate::instruments::traits::Attributes { <Self as Attributable>::attributes_mut(self) }
    fn clone_box(&self) -> Box<dyn Instrument> { Box::new(self.clone()) }
}

// Do not add explicit Instrument impl; provided by blanket impl.

impl CashflowProvider for FIIndexTotalReturnSwap {
    fn build_schedule(&self, _context: &MarketContext, _as_of: Date) -> Result<DatedFlows> {
        // For TRS, we'll return the expected payment dates
        // Actual amounts depend on realized returns
        let period_schedule = build_dates(
            self.schedule.dates.start,
            self.schedule.dates.end,
            self.schedule.params.frequency,
            self.schedule.params.stub,
            self.schedule.params.bdc,
            self.schedule.params.calendar_id,
        );

        let mut flows = Vec::new();
        for date in period_schedule.dates.iter().skip(1) {
            // Add a placeholder flow for each payment date
            flows.push((*date, Money::new(0.0, self.notional.currency())));
        }

        Ok(flows)
    }
}

// Manual builder removed; derive-based builder is used.
