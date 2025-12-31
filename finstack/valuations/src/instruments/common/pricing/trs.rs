//! Total Return Swap (TRS) pricing engine.
//!
//! This module provides shared pricing infrastructure for equity and fixed income
//! total return swaps. It separates the common period iteration and discounting
//! logic from underlying-specific return calculations.
//!
//! # Architecture
//!
//! The TRS pricing engine uses a trait-based approach:
//! - [`TrsReturnModel`]: Trait for underlying-specific return calculations
//! - [`TrsEngine`]: Shared pricing logic for all TRS types
//!
//! This allows equity TRS and fixed income TRS to share the common infrastructure
//! while implementing their own return calculation logic.

use crate::instruments::common::parameters::legs::FinancingLegSpec;
use crate::instruments::common::parameters::trs_common::TrsScheduleSpec;
use finstack_core::dates::{Date, DayCountCtx};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use rust_decimal::prelude::ToPrimitive;

/// Parameters for total return leg calculation.
#[derive(Debug, Clone)]
pub struct TotalReturnLegParams<'a> {
    /// Schedule specification for payment periods.
    pub schedule: &'a TrsScheduleSpec,
    /// Notional amount for the leg.
    pub notional: Money,
    /// Discount curve identifier.
    pub discount_curve_id: &'a str,
    /// Contract size multiplier for the underlying.
    pub contract_size: f64,
    /// Initial level of the underlying (if known).
    pub initial_level: Option<f64>,
}

/// Trait for underlying-specific total return models.
///
/// Implementations of this trait provide the logic for calculating
/// total returns over a period for different underlying types (equity vs fixed income).
///
/// # Return Value Contract
///
/// Implementations **must** return:
/// - **Finite values**: Returns must be finite (`is_finite() == true`). NaN or Inf values
///   will propagate through PV calculations and break determinism guarantees.
/// - **Reasonable bounds**: While there's no hard limit, returns outside [-1.0, 10.0] per period
///   are unusual and may indicate a bug. Returns below -1.0 imply more than 100% loss.
///
/// # Example Implementation
///
/// ```rust,ignore
/// impl TrsReturnModel for EquityReturn {
///     fn period_return(
///         &self,
///         period_start: Date,
///         period_end: Date,
///         t_start: f64,
///         t_end: f64,
///         initial_level: f64,
///         context: &MarketContext,
///     ) -> finstack_core::Result<f64> {
///         let start_price = context.get_equity_spot(self.ticker, t_start)?;
///         let end_price = context.get_equity_spot(self.ticker, t_end)?;
///
///         // Return as decimal (e.g., 0.05 for 5% return)
///         let ret = (end_price - start_price) / initial_level;
///
///         // Validate return is reasonable
///         if !ret.is_finite() {
///             return Err(Error::Validation("Non-finite return".into()));
///         }
///         Ok(ret)
///     }
/// }
/// ```
pub trait TrsReturnModel {
    /// Computes total return over a period given times from as_of and initial level.
    ///
    /// # Arguments
    /// * `period_start` — Start date of the period
    /// * `period_end` — End date of the period
    /// * `t_start` — Time from as_of to period start (year fraction)
    /// * `t_end` — Time from as_of to period end (year fraction)
    /// * `initial_level` — Initial level of the underlying
    /// * `context` — Market context for data access
    ///
    /// # Returns
    ///
    /// Total return as a decimal (e.g., 0.05 for 5% return).
    ///
    /// # Contract
    ///
    /// - Return value **must** be finite
    /// - Return value **should** be in a reasonable range (typically -1.0 to 10.0 per period)
    /// - Implementations should return an error rather than returning NaN/Inf
    fn period_return(
        &self,
        period_start: Date,
        period_end: Date,
        t_start: f64,
        t_end: f64,
        initial_level: f64,
        context: &MarketContext,
    ) -> finstack_core::Result<f64>;
}

/// Common TRS pricing engine for shared calculations.
///
/// Provides utility functions for calculating present values of TRS legs
/// and other common pricing operations shared between equity and fixed income TRS.
pub struct TrsEngine;

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
    pub fn pv_total_return_leg_with_model(
        params: TotalReturnLegParams,
        context: &MarketContext,
        as_of: Date,
        model: &impl TrsReturnModel,
    ) -> finstack_core::Result<Money> {
        // Get discount curve
        let disc = context.get_discount(params.discount_curve_id)?;

        // Build schedule
        let period_schedule = params.schedule.period_schedule()?;

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

            // Validate return is finite (defensive check on model output)
            if !total_return.is_finite() {
                return Err(finstack_core::Error::Validation(format!(
                    "TRS return model produced non-finite return ({}) for period {} to {}",
                    total_return, period_start, period_end
                )));
            }

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
    /// This is shared by both equity and fixed income TRS.
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
    ) -> finstack_core::Result<Money> {
        // Get curves
        let disc_curve_id = financing.discount_curve_id.as_str();
        let fwd_curve_id = financing.forward_curve_id.as_str();

        let disc = context.get_discount(disc_curve_id)?;
        let fwd = context.get_forward(fwd_curve_id)?;

        // Build schedule
        let period_schedule = schedule.period_schedule()?;

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

            // Add spread (convert Decimal to f64 for calculation)
            let spread_decimal = financing.spread_bp.to_f64().unwrap_or(0.0) / 10000.0;
            let total_rate = fwd_rate + spread_decimal;

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
    ///
    /// # Errors
    ///
    /// Returns an error if the computed annuity is below
    /// [`crate::instruments::common::pricing::swap_legs::ANNUITY_EPSILON`] (1e-12),
    /// which would cause divide-by-zero in downstream par spread calculations.
    /// This typically occurs when:
    /// - All periods have already expired (payment dates before as_of)
    /// - Extreme discounting scenarios with very high rates
    pub fn financing_annuity(
        financing: &FinancingLegSpec,
        schedule: &TrsScheduleSpec,
        notional: Money,
        context: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<f64> {
        // Get discount curve
        let disc_curve_id = financing.discount_curve_id.as_str();
        let disc = context.get_discount(disc_curve_id)?;

        // Build schedule
        let period_schedule = schedule.period_schedule()?;

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

        let result = annuity * notional.amount();

        // Guard against zero/near-zero annuity to prevent divide-by-zero in par spread calculations
        if result.abs() < super::swap_legs::ANNUITY_EPSILON {
            return Err(finstack_core::Error::Validation(format!(
                "Financing annuity ({:.2e}) is below minimum threshold ({:.2e}). \
                 This may indicate all periods have expired or extreme discounting scenarios. \
                 Cannot compute par spread with near-zero annuity.",
                result,
                super::swap_legs::ANNUITY_EPSILON
            )));
        }

        Ok(result)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use crate::instruments::common::pricing::swap_legs;

    #[test]
    fn test_trs_annuity_epsilon_is_reasonable() {
        // Verify the threshold catches near-zero but allows reasonable values
        let eps = swap_legs::ANNUITY_EPSILON;
        assert!(eps > 0.0, "ANNUITY_EPSILON should be positive");
        assert!(eps < 1e-10, "ANNUITY_EPSILON should be small");

        // A typical annuity for a 1-year quarterly swap with $1M notional would be
        // roughly 0.25 * 4 * 1M * 0.95 = 950,000, which is well above epsilon
        let typical_annuity = 950_000.0;
        assert!(
            typical_annuity > eps,
            "Typical annuity should be above threshold"
        );
    }
}
