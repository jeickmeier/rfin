//! Core types and common engine for Total Return Swaps.

use crate::cashflow::builder::schedule_utils::build_dates;
use crate::instruments::common::parameter_groups::{DateRange, InstrumentScheduleParams};
use finstack_core::{
    dates::{BusinessDayConvention, Date, DayCount, DayCountCtx, Frequency, StubKind},
    market_data::MarketContext,
    money::Money,
    types::CurveId,
    Result, F,
};

/// Side of the TRS trade (perspective of the party)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TrsSide {
    /// Receive total return, pay financing
    ReceiveTotalReturn,
    /// Pay total return, receive financing
    PayTotalReturn,
}

impl TrsSide {
    /// Get the sign multiplier for PV calculation
    pub fn sign(&self) -> F {
        match self {
            TrsSide::ReceiveTotalReturn => 1.0,
            TrsSide::PayTotalReturn => -1.0,
        }
    }
}

/// Specification for the financing leg of a TRS
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct FinancingLegSpec {
    /// Discount curve identifier
    pub disc_id: CurveId,
    /// Forward curve identifier (e.g., USD-SOFR-3M)
    pub fwd_id: CurveId,
    /// Spread in basis points over the floating rate
    pub spread_bp: F,
    /// Day count convention for accrual
    pub day_count: DayCount,
}

impl FinancingLegSpec {
    /// Create a new financing leg specification
    pub fn new(
        disc_id: impl Into<String>,
        fwd_id: impl Into<String>,
        spread_bp: F,
        day_count: DayCount,
    ) -> Self {
        Self {
            disc_id: CurveId::new(disc_id),
            fwd_id: CurveId::new(fwd_id),
            spread_bp,
            day_count,
        }
    }
}

/// Specification for the total return leg of a TRS
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct TotalReturnLegSpec {
    /// Reference index or asset identifier
    pub reference_id: String,
    /// Initial price/level (if known, otherwise fetched from market)
    pub initial_level: Option<F>,
    /// Whether to include dividends/distributions
    pub include_distributions: bool,
}

/// Schedule specification for TRS
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct TrsScheduleSpec {
    /// Start date
    pub start: Date,
    /// End date
    pub end: Date,
    /// Payment frequency
    pub frequency: Frequency,
    /// Day count convention
    pub day_count: DayCount,
    /// Business day convention
    pub bdc: BusinessDayConvention,
    /// Optional calendar identifier
    pub calendar_id: Option<String>,
    /// Stub period handling
    pub stub: StubKind,
}

impl TrsScheduleSpec {
    /// Create from DateRange and InstrumentScheduleParams
    pub fn from_params(dates: DateRange, schedule: InstrumentScheduleParams) -> Self {
        Self {
            start: dates.start,
            end: dates.end,
            frequency: schedule.frequency,
            day_count: schedule.day_count,
            bdc: schedule.bdc,
            calendar_id: schedule.calendar_id.map(|s| s.to_string()),
            stub: schedule.stub,
        }
    }
}

/// Common TRS pricing engine
pub struct TrsEngine;

impl TrsEngine {
    /// Calculate PV of financing leg
    pub fn pv_financing_leg(
        financing: &FinancingLegSpec,
        schedule: &TrsScheduleSpec,
        notional: Money,
        context: &MarketContext,
        as_of: Date,
    ) -> Result<Money> {
        // Get curves
        let disc_curve_id = financing.disc_id.as_str();
        let fwd_curve_id = financing.fwd_id.as_str();

        let disc = context.disc(disc_curve_id)?;
        let fwd = context.fwd(fwd_curve_id)?;

        // Build schedule
        let period_schedule = build_dates(
            schedule.start,
            schedule.end,
            schedule.frequency,
            schedule.stub,
            schedule.bdc,
            None, // TODO: Handle calendar_id properly
        );

        let mut total_pv = 0.0;
        let currency = notional.currency();
        let ctx = DayCountCtx::default();

        // Iterate through periods
        for i in 1..period_schedule.dates.len() {
            let period_start = period_schedule.dates[i - 1];
            let period_end = period_schedule.dates[i];

            // Year fraction for accrual
            let yf = schedule
                .day_count
                .year_fraction(period_start, period_end, ctx)?;

            // Forward rate for the period
            let t_start = schedule.day_count.year_fraction(as_of, period_start, ctx)?;
            let t_end = schedule.day_count.year_fraction(as_of, period_end, ctx)?;
            let fwd_rate = fwd.rate_period(t_start, t_end);

            // Add spread
            let total_rate = fwd_rate + financing.spread_bp / 10000.0;

            // Payment amount
            let payment = notional.amount() * total_rate * yf;

            // Discount to present
            let df = disc.df(t_end);
            total_pv += payment * df;
        }

        Ok(Money::new(total_pv, currency))
    }

    /// Calculate financing annuity (for par spread calculation)
    pub fn financing_annuity(
        financing: &FinancingLegSpec,
        schedule: &TrsScheduleSpec,
        notional: Money,
        context: &MarketContext,
        as_of: Date,
    ) -> Result<F> {
        // Get discount curve
        let disc_curve_id = financing.disc_id.as_str();
        let disc = context.disc(disc_curve_id)?;

        // Build schedule
        let period_schedule = build_dates(
            schedule.start,
            schedule.end,
            schedule.frequency,
            schedule.stub,
            schedule.bdc,
            None, // TODO: Handle calendar_id properly
        );

        let mut annuity = 0.0;
        let ctx = DayCountCtx::default();

        // Iterate through periods
        for i in 1..period_schedule.dates.len() {
            let period_start = period_schedule.dates[i - 1];
            let period_end = period_schedule.dates[i];

            // Year fraction for accrual
            let yf = schedule
                .day_count
                .year_fraction(period_start, period_end, ctx)?;

            // Discount factor to payment date
            let t_pay = schedule.day_count.year_fraction(as_of, period_end, ctx)?;
            let df = disc.df(t_pay);

            annuity += df * yf;
        }

        Ok(annuity * notional.amount())
    }
}
