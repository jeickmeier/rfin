//! Fixed Income Index Total Return Swap implementation.

use super::types::{FinancingLegSpec, TrsEngine, TrsScheduleSpec, TrsSide};
use crate::instruments::traits::{Attributable, InstrumentLike};
use crate::{
    cashflow::{
        builder::schedule_utils::build_dates,
        traits::{CashflowProvider, DatedFlows},
    },
    instruments::{
        common::parameter_groups::{
            validate_currency_consistency, DateRange, IndexUnderlyingParams,
            InstrumentScheduleParams,
        },
        traits::{Attributes, Priceable},
    },
    metrics::MetricId,
    results::ValuationResult,
};
use finstack_core::{
    dates::{Date, DayCount, DayCountCtx},
    market_data::MarketContext,
    money::Money,
    types::InstrumentId,
    Error, Result, F,
};
use std::any::Any;

/// Fixed Income Index Total Return Swap instrument
#[derive(Clone, Debug)]
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
    /// Create a new builder
    pub fn builder() -> FIIndexTrsBuilder {
        FIIndexTrsBuilder::new()
    }

    /// Calculate PV of the total return leg using carry-only approximation
    pub(super) fn pv_total_return_leg(
        &self,
        context: &MarketContext,
        as_of: Date,
    ) -> Result<Money> {
        // Get discount curve
        let disc_curve_id = self.financing.disc_id.as_str();
        let disc = context.disc(disc_curve_id)?;

        // Get index yield if available (for carry calculation)
        let index_yield = self
            .underlying
            .yield_id
            .as_ref()
            .and_then(|id| {
                context.price(id.as_str()).ok().map(|s| match s {
                    finstack_core::market_data::primitives::MarketScalar::Unitless(v) => *v,
                    finstack_core::market_data::primitives::MarketScalar::Price(p) => p.amount(),
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
                    finstack_core::market_data::primitives::MarketScalar::Unitless(v) => *v,
                    finstack_core::market_data::primitives::MarketScalar::Price(p) => p.amount(),
                })
            })
            .unwrap_or(0.0);

        // Build schedule
        let period_schedule = build_dates(
            self.schedule.dates.start,
            self.schedule.dates.end,
            self.schedule.params.frequency,
            self.schedule.params.stub,
            self.schedule.params.bdc,
            self.schedule.params.calendar_id,
        );

        let mut total_pv = 0.0;
        let currency = self.notional.currency();
        let ctx = DayCountCtx::default();

        // Iterate through periods
        for i in 1..period_schedule.dates.len() {
            let period_start = period_schedule.dates[i - 1];
            let period_end = period_schedule.dates[i];

            // Year fraction for the period
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

            // Payment amount
            let payment = self.notional.amount() * total_return * self.underlying.contract_size;

            // Discount to present
            let t_pay = self
                .schedule
                .params
                .day_count
                .year_fraction(as_of, period_end, ctx)?;
            let df = disc.df(t_pay);
            total_pv += payment * df;
        }

        Ok(Money::new(total_pv, currency))
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
        let npv = self.value(context, as_of)?;

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

impl InstrumentLike for FIIndexTotalReturnSwap {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn instrument_type(&self) -> &'static str {
        "FIIndexTotalReturnSwap"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn InstrumentLike> {
        Box::new(self.clone())
    }
}

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

/// Builder for FIIndexTotalReturnSwap
pub struct FIIndexTrsBuilder {
    id: Option<InstrumentId>,
    notional: Option<Money>,
    underlying: Option<IndexUnderlyingParams>,
    financing: Option<FinancingLegSpec>,
    dates: Option<DateRange>,
    schedule_params: Option<InstrumentScheduleParams>,
    side: TrsSide,
    initial_level: Option<F>,
}

impl FIIndexTrsBuilder {
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

    /// Set underlying index parameters
    pub fn underlying(mut self, underlying: IndexUnderlyingParams) -> Self {
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
    pub fn build(self) -> Result<FIIndexTotalReturnSwap> {
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

        // Validate currency consistency
        if notional.currency() != underlying.base_currency {
            return Err(Error::CurrencyMismatch {
                expected: underlying.base_currency,
                actual: notional.currency(),
            });
        }

        let schedule = TrsScheduleSpec::from_params(dates, schedule_params);

        Ok(FIIndexTotalReturnSwap {
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

impl Default for FIIndexTrsBuilder {
    fn default() -> Self {
        Self::new()
    }
}
