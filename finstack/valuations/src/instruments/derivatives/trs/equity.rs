//! Equity Total Return Swap implementation.

use super::types::{FinancingLegSpec, TotalReturnLegParams, TrsEngine, TrsScheduleSpec, TrsSide};
use crate::instruments::traits::{Attributable, InstrumentLike};
use crate::{
    cashflow::{
        builder::schedule_utils::build_dates,
        traits::{CashflowProvider, DatedFlows},
    },
    instruments::{
        common::parameter_groups::{
            validate_currency_consistency, DateRange, EquityUnderlyingParams,
            InstrumentScheduleParams,
        },
        traits::{Attributes, Priceable},
    },
    metrics::MetricId,
    results::ValuationResult,
};
use finstack_core::{
    dates::{Date, DayCount},
    market_data::MarketContext,
    money::Money,
    types::InstrumentId,
    Error, Result, F,
};
use std::any::Any;

/// Equity Total Return Swap instrument
#[derive(Clone, Debug)]
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
    /// Create a new builder
    pub fn builder() -> EquityTrsBuilder {
        EquityTrsBuilder::new()
    }

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
                let disc = context.discount_ref(self.financing.disc_id.as_str())?;
                
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
        let npv = self.value(context, as_of)?;

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

impl InstrumentLike for EquityTotalReturnSwap {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn instrument_type(&self) -> &'static str {
        "EquityTotalReturnSwap"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn InstrumentLike> {
        Box::new(self.clone())
    }
}

impl CashflowProvider for EquityTotalReturnSwap {
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
            // In practice, the amount would be determined at fixing
            flows.push((*date, Money::new(0.0, self.notional.currency())));
        }

        Ok(flows)
    }
}

/// Builder for EquityTotalReturnSwap
pub struct EquityTrsBuilder {
    id: Option<InstrumentId>,
    notional: Option<Money>,
    underlying: Option<EquityUnderlyingParams>,
    financing: Option<FinancingLegSpec>,
    dates: Option<DateRange>,
    schedule_params: Option<InstrumentScheduleParams>,
    side: TrsSide,
    initial_level: Option<F>,
}

impl EquityTrsBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            id: None,
            notional: None,
            underlying: None,
            financing: None,
            dates: None,
            schedule_params: None,
            side: TrsSide::ReceiveTotalReturn,
            initial_level: None,
        }
    }

    /// Set instrument ID
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(InstrumentId::new(id));
        self
    }

    /// Set notional amount
    pub fn notional(mut self, notional: Money) -> Self {
        self.notional = Some(notional);
        self
    }

    /// Set underlying equity parameters
    pub fn underlying(mut self, underlying: EquityUnderlyingParams) -> Self {
        self.underlying = Some(underlying);
        self
    }

    /// Set financing leg parameters
    pub fn financing(
        mut self,
        disc_id: impl Into<String>,
        fwd_id: impl Into<String>,
        spread_bp: F,
        day_count: DayCount,
    ) -> Self {
        self.financing = Some(FinancingLegSpec::new(disc_id, fwd_id, spread_bp, day_count));
        self
    }

    /// Set date range
    pub fn dates(mut self, start: Date, end: Date) -> Self {
        self.dates = Some(DateRange::new(start, end));
        self
    }

    /// Set date range from tenor
    pub fn tenor(mut self, start: Date, tenor_years: F) -> Self {
        self.dates = Some(DateRange::from_tenor(start, tenor_years));
        self
    }

    /// Set schedule parameters
    pub fn schedule_params(mut self, params: InstrumentScheduleParams) -> Self {
        self.schedule_params = Some(params);
        self
    }

    /// Set to receive total return (pay financing)
    pub fn receive_total_return(mut self) -> Self {
        self.side = TrsSide::ReceiveTotalReturn;
        self
    }

    /// Set to pay total return (receive financing)
    pub fn pay_total_return(mut self) -> Self {
        self.side = TrsSide::PayTotalReturn;
        self
    }

    /// Set initial index level
    pub fn with_initial_level(mut self, level: F) -> Self {
        self.initial_level = Some(level);
        self
    }

    /// Build the TRS
    pub fn build(self) -> Result<EquityTotalReturnSwap> {
        let id = self
            .id
            .ok_or(Error::Input(finstack_core::error::InputError::Invalid))?;
        let notional = self
            .notional
            .ok_or(Error::Input(finstack_core::error::InputError::Invalid))?;
        let underlying = self
            .underlying
            .ok_or(Error::Input(finstack_core::error::InputError::Invalid))?;
        let financing = self
            .financing
            .ok_or(Error::Input(finstack_core::error::InputError::Invalid))?;
        let dates = self
            .dates
            .ok_or(Error::Input(finstack_core::error::InputError::Invalid))?;
        let schedule_params = self
            .schedule_params
            .unwrap_or_else(InstrumentScheduleParams::quarterly_act360);

        let schedule = TrsScheduleSpec::from_params(dates, schedule_params);

        Ok(EquityTotalReturnSwap {
            id,
            notional,
            underlying,
            financing,
            schedule,
            side: self.side,
            initial_level: self.initial_level,
            attributes: Attributes::new(),
        })
    }
}

impl Default for EquityTrsBuilder {
    fn default() -> Self {
        Self::new()
    }
}
