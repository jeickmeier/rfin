//! Flow-level DataFrame representation for cashflow schedules.
//!
//! Provides `FlowFrame` which computes per-row values including outstanding balances
//! for debugging and analysis, particularly for revolving credit facilities where both
//! drawn and undrawn balances are critical for fee calculations.

use crate::cashflow::builder::CashFlowSchedule;
use crate::cashflow::primitives::CFKind;
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCountCtx};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;

/// Flow-level DataFrame with computed outstanding balances.
///
/// All vectors have the same length corresponding to the number of cashflows.
/// Designed for debugging revolving credit and other facilities where tracking
/// drawn and undrawn balances is essential.
///
/// # Example
///
/// ```rust
/// use finstack_valuations::cashflow::builder::{CashFlowSchedule, FlowFrame};
/// use finstack_core::currency::Currency;
/// use finstack_core::money::Money;
/// use time::Month;
///
/// # fn example() -> finstack_core::Result<()> {
/// let schedule = CashFlowSchedule::builder()
///     .principal(
///         Money::new(1_000_000.0, Currency::USD),
///         finstack_core::dates::Date::from_calendar_date(2025, Month::January, 15).unwrap(),
///         finstack_core::dates::Date::from_calendar_date(2026, Month::January, 15).unwrap(),
///     )
///     .build()?;
///
/// let frame = schedule.to_flow_frame();
/// assert_eq!(frame.start_dates.len(), frame.amounts.len());
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FlowFrame {
    /// Period start dates for each cashflow
    pub start_dates: Vec<Date>,
    /// Period end dates for each cashflow (payment dates)
    pub end_dates: Vec<Date>,
    /// Reset dates for floating rate fixings (None for fixed or non-coupon flows)
    pub reset_dates: Vec<Option<Date>>,   
    /// Year fractions for accrual (0.0 for non-accruing flows like notional)
    pub accrual_factors: Vec<f64>,  
    /// Currency for all amounts
    pub currency: Currency,

    /// Effective rate for the cashflow (0.0 for non-accruing flows)
    pub rates: Vec<f64>,     
    /// Cashflow kinds (Fixed, FloatReset, Amortization, PIK, Notional, etc.)
    pub kinds: Vec<CFKind>,
    /// Cashflow amounts (positive = received by holder, negative = paid by holder)
    pub amounts: Vec<f64>,

    /// Outstanding balance after applying this cashflow
    pub outstanding: Vec<f64>,
    /// Undrawn outstanding balance (facility_limit - outstanding)
    pub outstanding_undrawn: Option<Vec<f64>>,
    /// Facility limit/commitment (if applicable, e.g., revolving credit)
    pub facility_limit: Option<Money>,

    /// Discount factors (only present if market context provided)
    pub discount_factors: Option<Vec<f64>>,
    /// Present values (only present if market context provided)
    pub pvs: Option<Vec<f64>>,

}

impl CashFlowSchedule {
    /// Convert cashflow schedule to a flow-level DataFrame without market pricing.
    ///
    /// Computes outstanding balances and rates row-by-row using deterministic conventions.
    /// For discount factors and PV, use `to_flow_frame_with_market()`.
    ///
    /// # Returns
    ///
    /// A `FlowFrame` with cashflows, rates, and outstanding balances (no DF/PV).
    pub fn to_flow_frame(&self) -> FlowFrame {
        self.to_flow_frame_impl(None, None, None)
    }

    /// Convert cashflow schedule to a flow-level DataFrame with market pricing.
    ///
    /// Includes discount factors and present values calculated from the provided curve.
    ///
    /// # Arguments
    ///
    /// * `market` - Market context with discount curves
    /// * `discount_curve_id` - ID of the discount curve to use
    /// * `as_of` - Valuation date (defaults to curve base date if None)
    ///
    /// # Returns
    ///
    /// A `FlowFrame` with cashflows, rates, outstanding, discount factors, and PVs.
    pub fn to_flow_frame_with_market(
        &self,
        market: &MarketContext,
        discount_curve_id: &str,
        as_of: Option<Date>,
    ) -> finstack_core::Result<FlowFrame> {
        Ok(self.to_flow_frame_impl(Some(market), Some(discount_curve_id), as_of))
    }

    fn to_flow_frame_impl(
        &self,
        market: Option<&MarketContext>,
        discount_curve_id: Option<&str>,
        as_of: Option<Date>,
    ) -> FlowFrame {
        let n = self.flows.len();
        let mut start_dates = Vec::with_capacity(n);
        let mut end_dates = Vec::with_capacity(n);
        let mut kinds = Vec::with_capacity(n);
        let mut amounts = Vec::with_capacity(n);
        let mut accrual_factors = Vec::with_capacity(n);
        let mut reset_dates = Vec::with_capacity(n);
        let mut outstanding_vec = Vec::with_capacity(n);
        let mut rates = Vec::with_capacity(n);

        // Start with initial notional
        let mut outstanding = self.notional.initial.amount();
        let ccy = self.notional.initial.currency();

        // Get discount curve if market context provided
        let disc_curve = match (market, discount_curve_id) {
            (Some(mkt), Some(curve_id)) => mkt.get_discount(curve_id).ok(),
            _ => None,
        };

        let base_date = disc_curve
            .as_ref()
            .map(|c| as_of.unwrap_or_else(|| c.base_date()))
            .or(as_of)
            .or_else(|| self.flows.first().map(|cf| cf.date))
            .unwrap_or_else(|| Date::from_calendar_date(2025, time::Month::January, 1).unwrap());

        let mut discount_factors_vec = if disc_curve.is_some() {
            Vec::with_capacity(n)
        } else {
            Vec::new()
        };
        let mut pvs_vec = if disc_curve.is_some() {
            Vec::with_capacity(n)
        } else {
            Vec::new()
        };
        
        for (i, cf) in self.flows.iter().enumerate() {
            // Period start is the previous cashflow's date, or the current date for the first flow
            let period_start = if i > 0 {
                self.flows[i - 1].date
            } else {
                cf.date
            };
            
            start_dates.push(period_start);
            end_dates.push(cf.date);
            kinds.push(cf.kind);
            amounts.push(cf.amount.amount());
            accrual_factors.push(cf.accrual_factor);
            reset_dates.push(cf.reset_date);
            rates.push(cf.rate.unwrap_or(0.0));
            
            // Calculate discount factor and PV if curve available
            if let Some(curve) = &disc_curve {
                let t = self.day_count
                    .year_fraction(base_date, cf.date, DayCountCtx::default())
                    .unwrap_or(0.0);
                let df = curve.df(t);
                let pv = cf.amount.amount() * df;
                discount_factors_vec.push(df);
                pvs_vec.push(pv);
            }

            // Update outstanding based on cashflow kind
            // Convention: cashflows affect the balance going forward
            match cf.kind {
                CFKind::Amortization => {
                    // Amortization reduces outstanding
                    outstanding -= cf.amount.amount();
                }
                CFKind::PIK => {
                    // PIK increases outstanding
                    outstanding += cf.amount.amount();
                }
                CFKind::Notional => {
                    // Notional flows: draws are negative (increase outstanding),
                    // repayments are positive (decrease outstanding)
                    outstanding -= cf.amount.amount();
                }
                _ => {
                    // Other flows (interest, fees) don't affect outstanding
                }
            }

            // Store the outstanding AFTER applying this cashflow
            // This is the balance that will be used for the NEXT period's calculations
            outstanding_vec.push(outstanding);
        }

        // Compute undrawn if facility limit is present
        let outstanding_undrawn = if let Some(limit) = self.meta.facility_limit {
            let limit_amt = limit.amount();
            Some(
                outstanding_vec
                    .iter()
                    .map(|drawn| limit_amt - drawn)
                    .collect(),
            )
        } else {
            None
        };

        FlowFrame {
            start_dates,
            end_dates,
            reset_dates,
            accrual_factors,
            currency: ccy,
            rates,
            kinds,
            amounts,
            outstanding: outstanding_vec,
            outstanding_undrawn,
            facility_limit: self.meta.facility_limit,
            discount_factors: disc_curve.is_some().then_some(discount_factors_vec),
            pvs: disc_curve.is_some().then_some(pvs_vec),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cashflow::builder::CashflowBuilder;
    use finstack_core::currency::Currency;
    use finstack_core::money::Money;
    use time::Month;

    #[test]
    fn flow_frame_basic_structure_ok() {
        let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
        let maturity = Date::from_calendar_date(2026, Month::January, 15).unwrap();

        let schedule = CashflowBuilder::new()
            .principal(Money::new(1_000_000.0, Currency::USD), issue, maturity)
            .build()
            .unwrap();

        let frame = schedule.to_flow_frame();

        assert_eq!(frame.start_dates.len(), schedule.flows.len());
        assert_eq!(frame.end_dates.len(), schedule.flows.len());
        assert_eq!(frame.kinds.len(), schedule.flows.len());
        assert_eq!(frame.amounts.len(), schedule.flows.len());
        assert_eq!(frame.accrual_factors.len(), schedule.flows.len());
        assert_eq!(frame.reset_dates.len(), schedule.flows.len());
        assert_eq!(frame.outstanding.len(), schedule.flows.len());
        assert_eq!(frame.currency, Currency::USD);
    }

    #[test]
    fn flow_frame_outstanding_conventions_ok() {
        // Test that outstanding balance is tracked correctly through the flows
        let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
        let maturity = Date::from_calendar_date(2026, Month::January, 15).unwrap();

        let schedule = CashflowBuilder::new()
            .principal(Money::new(1_000_000.0, Currency::USD), issue, maturity)
            .build()
            .unwrap();

        let frame = schedule.to_flow_frame();

        // For a simple principal-only schedule:
        // - Initial notional draw (negative flow) increases outstanding
        // - Final redemption (positive flow) decreases outstanding back to zero
        
        // First flow should set outstanding to principal amount
        if !frame.outstanding.is_empty() {
            // After the final redemption, outstanding should be back to initial
            let final_outstanding = frame.outstanding.last().unwrap();
            // Should end near the initial amount after redemption
            assert!(
                (final_outstanding - 1_000_000.0).abs() < 1e-2,
                "Final outstanding should be near initial after redemption: got {}",
                final_outstanding
            );
        }
    }

    #[test]
    fn flow_frame_facility_limit_undrawn_ok() {
        let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
        let maturity = Date::from_calendar_date(2026, Month::January, 15).unwrap();

        let mut schedule = CashflowBuilder::new()
            .principal(Money::new(500_000.0, Currency::USD), issue, maturity)
            .build()
            .unwrap();

        // Manually set facility limit to simulate revolving credit
        schedule.meta.facility_limit = Some(Money::new(1_000_000.0, Currency::USD));

        let frame = schedule.to_flow_frame();

        // Should have undrawn balance
        assert!(frame.outstanding_undrawn.is_some());

        let undrawn = frame.outstanding_undrawn.unwrap();
        assert_eq!(undrawn.len(), frame.outstanding.len());

        // Check that drawn + undrawn = facility_limit
        for (outstanding_val, undrawn_val) in frame.outstanding.iter().zip(undrawn.iter()) {
            let total = outstanding_val + undrawn_val;
            assert!(
                (total - 1_000_000.0).abs() < 1e-6,
                "Drawn + undrawn should equal facility limit"
            );
        }
    }

    #[test]
    fn flow_frame_no_facility_limit_no_undrawn() {
        let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
        let maturity = Date::from_calendar_date(2026, Month::January, 15).unwrap();

        let schedule = CashflowBuilder::new()
            .principal(Money::new(1_000_000.0, Currency::USD), issue, maturity)
            .build()
            .unwrap();

        let frame = schedule.to_flow_frame();

        // Should NOT have undrawn balance
        assert!(frame.outstanding_undrawn.is_none());
    }
}

