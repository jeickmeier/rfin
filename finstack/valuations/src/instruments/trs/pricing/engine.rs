//! Core TRS pricing engine and shared helpers.

use finstack_core::dates::{Date, DayCountCtx};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;

use super::super::types::{FinancingLegSpec, TrsScheduleSpec};

/// Common TRS pricing engine for shared calculations.
///
/// Provides utility functions for calculating present values of TRS legs
/// and other common pricing operations.
pub struct TrsEngine;

/// Parameters for total return leg calculation.
#[derive(Debug, Clone)]
pub struct TotalReturnLegParams<'a> {
    /// Schedule specification for payment periods.
    pub schedule: &'a TrsScheduleSpec,
    /// Notional amount for the leg.
    pub notional: Money,
    /// Discount curve identifier.
    pub disc_id: &'a str,
    /// Contract size multiplier for the underlying.
    pub contract_size: f64,
    /// Initial level of the underlying (if known).
    pub initial_level: Option<f64>,
}

/// Trait for underlying-specific total return models.
///
/// Implementations of this trait provide the logic for calculating
/// total returns over a period for different underlying types.
pub trait TrsReturnModel {
    /// Computes total return over a period given times from as_of and initial level.
    ///
    /// # Arguments
    /// * `period_start` — Start date of the period
    /// * `period_end` — End date of the period
    /// * `t_start` — Time from as_of to period start
    /// * `t_end` — Time from as_of to period end
    /// * `initial_level` — Initial level of the underlying
    /// * `context` — Market context for data access
    ///
    /// # Returns
    /// Total return as a decimal (e.g., 0.05 for 5% return).
    fn period_return(
        &self,
        period_start: Date,
        period_end: Date,
        t_start: f64,
        t_end: f64,
        initial_level: f64,
        context: &MarketContext,
    ) -> Result<f64>;
}

impl TrsEngine {
    /// Calculates the present value of a total return leg using shared logic.
    ///
    /// This method contains the common period iteration and discounting logic,
    /// while delegating underlying-specific return calculations to the model.
    ///
    /// # Arguments
    /// * `params` — Parameters for the total return leg calculation
    /// * `context` — Market context containing curves and market data
    /// * `as_of` — Valuation date
    /// * `model` — Model implementing TrsReturnModel for underlying-specific logic
    ///
    /// # Returns
    /// Present value of the total return leg in the instrument's currency.
    #[allow(clippy::too_many_arguments)]
    pub fn pv_total_return_leg_with_model(
        params: TotalReturnLegParams,
        context: &MarketContext,
        as_of: Date,
        model: &impl TrsReturnModel,
    ) -> Result<Money> {
        // Get discount curve
        let disc = context.get_discount_ref(params.disc_id)?;

        // Build schedule
        let period_schedule = params.schedule.period_schedule();

        let mut total_pv = 0.0;
        let currency = params.notional.currency();
        let ctx = DayCountCtx::default();

        // Iterate through periods
        for i in 1..period_schedule.dates.len() {
            let period_start = period_schedule.dates[i - 1];
            let period_end = period_schedule.dates[i];

            // Time fractions
            let t_start = params
                .schedule
                .params
                .dc
                .year_fraction(as_of, period_start, ctx)?;
            let t_end = params
                .schedule
                .params
                .dc
                .year_fraction(as_of, period_end, ctx)?;

            // Calculate underlying return for this period (delegated to underlying-specific logic)
            let total_return = model.period_return(
                period_start,
                period_end,
                t_start,
                t_end,
                params.initial_level.unwrap_or(1.0),
                context,
            )?;

            // Payment amount
            let payment = params.notional.amount() * total_return * params.contract_size;

            // Discount to present
            let df = disc.df(t_end);
            total_pv += payment * df;
        }

        Ok(Money::new(total_pv, currency))
    }

    /// Calculates the present value of the financing leg.
    ///
    /// # Arguments
    /// * `financing` — Financing leg specification
    /// * `schedule` — Schedule specification for payment periods
    /// * `notional` — Notional amount for the leg
    /// * `context` — Market context containing curves and market data
    /// * `as_of` — Valuation date
    ///
    /// # Returns
    /// Present value of the financing leg in the instrument's currency.
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

        let disc = context.get_discount_ref(disc_curve_id)?;
        let fwd = context.get_forward_ref(fwd_curve_id)?;

        // Build schedule
        let period_schedule = schedule.period_schedule();

        let mut total_pv = 0.0;
        let currency = notional.currency();
        let ctx = DayCountCtx::default();

        // Iterate through periods
        for i in 1..period_schedule.dates.len() {
            let period_start = period_schedule.dates[i - 1];
            let period_end = period_schedule.dates[i];

            // Year fraction for accrual
            let yf = schedule
                .params
                .dc
                .year_fraction(period_start, period_end, ctx)?;

            // Forward rate for the period
            let t_start = schedule.params.dc.year_fraction(as_of, period_start, ctx)?;
            let t_end = schedule.params.dc.year_fraction(as_of, period_end, ctx)?;
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

    /// Calculates the financing annuity for par spread calculation.
    ///
    /// # Arguments
    /// * `financing` — Financing leg specification
    /// * `schedule` — Schedule specification for payment periods
    /// * `notional` — Notional amount for the leg
    /// * `context` — Market context containing curves and market data
    /// * `as_of` — Valuation date
    ///
    /// # Returns
    /// Financing annuity (sum of discounted year fractions × notional).
    pub fn financing_annuity(
        financing: &FinancingLegSpec,
        schedule: &TrsScheduleSpec,
        notional: Money,
        context: &MarketContext,
        as_of: Date,
    ) -> Result<f64> {
        // Get discount curve
        let disc_curve_id = financing.disc_id.as_str();
        let disc = context.get_discount_ref(disc_curve_id)?;

        // Build schedule
        let period_schedule = schedule.period_schedule();

        let mut annuity = 0.0;
        let ctx = DayCountCtx::default();

        // Iterate through periods
        for i in 1..period_schedule.dates.len() {
            let period_start = period_schedule.dates[i - 1];
            let period_end = period_schedule.dates[i];

            // Year fraction for accrual
            let yf = schedule
                .params
                .dc
                .year_fraction(period_start, period_end, ctx)?;

            // Discount factor to payment date
            let t_pay = schedule.params.dc.year_fraction(as_of, period_end, ctx)?;
            let df = disc.df(t_pay);

            annuity += df * yf;
        }

        Ok(annuity * notional.amount())
    }
}
