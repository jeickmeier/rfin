//! Agency MBS passthrough pricing.
//!
//! This module provides discounting-based pricing for agency MBS passthroughs,
//! generating projected cashflows with prepayment and payment delay adjustments.

use super::delay::actual_payment_date;
use super::AgencyMbsPassthrough;
use crate::pricer::{
    InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingErrorContext, PricingResult,
};
use crate::results::ValuationResult;
use finstack_core::dates::{Date, DayCountCtx};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;

/// Generated MBS cashflow for a single period.
#[derive(Clone, Debug)]
pub struct MbsCashflow {
    /// Accrual period start date
    pub period_start: Date,
    /// Accrual period end date
    pub period_end: Date,
    /// Actual payment date (after delay)
    pub payment_date: Date,
    /// Scheduled principal payment
    pub scheduled_principal: f64,
    /// Prepayment (unscheduled principal)
    pub prepayment: f64,
    /// Interest payment
    pub interest: f64,
    /// Total cashflow
    pub total: f64,
    /// Beginning balance for this period
    pub beginning_balance: f64,
    /// Ending balance after this period
    pub ending_balance: f64,
    /// SMM used for this period
    pub smm: f64,
}

/// Generate projected cashflows for an agency MBS.
///
/// This function projects the cashflow schedule based on the prepayment model,
/// applying servicing fees and payment delays.
///
/// # Arguments
///
/// * `mbs` - Agency MBS passthrough instrument
/// * `as_of` - Valuation date
/// * `max_periods` - Maximum number of periods to project (typically WAM)
///
/// # Returns
///
/// Vector of projected cashflows
pub fn generate_cashflows(
    mbs: &AgencyMbsPassthrough,
    as_of: Date,
    max_periods: Option<u32>,
) -> Result<Vec<MbsCashflow>> {
    use time::Duration;

    let mut cashflows = Vec::new();
    let mut balance = mbs.current_face.amount();

    // Start from the first of next month after as_of
    let mut period_start = Date::from_calendar_date(as_of.year(), as_of.month(), 1)
        .map_err(|e| finstack_core::Error::Validation(e.to_string()))?;
    if period_start <= as_of {
        period_start = period_start
            .checked_add(Duration::days(32))
            .and_then(|d| Date::from_calendar_date(d.year(), d.month(), 1).ok())
            .unwrap_or(period_start);
    }

    let payment_delay = mbs.effective_payment_delay();
    let max_periods = max_periods.unwrap_or(mbs.wam);
    let pass_through_rate = mbs.pass_through_rate;

    // Calculate monthly rate
    let monthly_rate = pass_through_rate / 12.0;

    // Original loan rate for amortization (approximate from WAC)
    let mortgage_rate = mbs.wac;
    let monthly_mortgage_rate = mortgage_rate / 12.0;

    for period_num in 0..max_periods {
        if balance < 0.01 {
            break;
        }

        // Calculate period end (last day of month)
        let period_end = end_of_month(period_start)?;

        // Calculate payment date with delay
        let payment_date = actual_payment_date(period_end, payment_delay, false)?;

        // Skip if payment date is before valuation date
        if payment_date <= as_of {
            period_start = next_month_start(period_start)?;
            continue;
        }

        // Get SMM for this period
        let seasoning = mbs.seasoning_months(period_end);
        let smm = mbs.prepayment_model.smm(seasoning);

        // Calculate scheduled principal payment (amortization)
        let remaining_months = mbs.wam.saturating_sub(seasoning);
        let scheduled_principal = if remaining_months > 0 && monthly_mortgage_rate > 0.0 {
            // Standard mortgage amortization formula
            let factor = (1.0 + monthly_mortgage_rate).powi(remaining_months as i32);
            let payment = balance * monthly_mortgage_rate * factor / (factor - 1.0);
            let interest_component = balance * monthly_mortgage_rate;
            (payment - interest_component).max(0.0).min(balance)
        } else if remaining_months == 1 {
            balance
        } else {
            // Simple straight-line if no rate
            balance / remaining_months.max(1) as f64
        };

        // Calculate prepayment (SMM applied to remaining balance after scheduled)
        let balance_after_scheduled = balance - scheduled_principal;
        let prepayment = balance_after_scheduled * smm;

        // Calculate interest (pass-through rate on beginning balance)
        let interest = balance * monthly_rate;

        // Total principal
        let total_principal = scheduled_principal + prepayment;

        // Update ending balance
        let ending_balance = (balance - total_principal).max(0.0);

        cashflows.push(MbsCashflow {
            period_start,
            period_end,
            payment_date,
            scheduled_principal,
            prepayment,
            interest,
            total: total_principal + interest,
            beginning_balance: balance,
            ending_balance,
            smm,
        });

        balance = ending_balance;
        period_start = next_month_start(period_start)?;

        // Early termination if past maturity
        if period_end >= mbs.maturity_date {
            break;
        }

        // Limit periods
        if period_num >= max_periods {
            break;
        }
    }

    Ok(cashflows)
}

/// Calculate end of month for a given date.
fn end_of_month(date: Date) -> Result<Date> {
    let year = date.year();
    let month = date.month();

    // Get the last day of the month
    let days_in_month = month.length(year);
    Date::from_calendar_date(year, month, days_in_month)
        .map_err(|e| finstack_core::Error::Validation(e.to_string()))
}

/// Get first day of next month.
fn next_month_start(date: Date) -> Result<Date> {
    use time::Duration;
    let end = end_of_month(date)?;
    let next = end + Duration::days(1);
    Ok(next)
}

/// Price an agency MBS using discounting.
///
/// This is the main pricing function that:
/// 1. Generates projected cashflows with prepayment
/// 2. Discounts each cashflow to present value
/// 3. Returns the sum as the MBS price
///
/// # Arguments
///
/// * `mbs` - Agency MBS passthrough instrument
/// * `market` - Market context with discount curves
/// * `as_of` - Valuation date
///
/// # Returns
///
/// Present value of the MBS
pub fn price_mbs(mbs: &AgencyMbsPassthrough, market: &MarketContext, as_of: Date) -> Result<Money> {
    // Generate projected cashflows
    let cashflows = generate_cashflows(mbs, as_of, Some(mbs.wam + 12))?;

    if cashflows.is_empty() {
        return Ok(Money::new(0.0, mbs.current_face.currency()));
    }

    // Get discount curve
    let discount_curve = market.get_discount(&mbs.discount_curve_id)?;

    // Discount each cashflow
    let mut pv = 0.0;
    for cf in &cashflows {
        let years = mbs
            .day_count
            .year_fraction(as_of, cf.payment_date, DayCountCtx::default())?;
        let df = discount_curve.df(years);
        pv += cf.total * df;
    }

    Ok(Money::new(pv, mbs.current_face.currency()))
}

/// Price an agency MBS with a spread adjustment.
///
/// Adds a spread (in decimal) to the discount rate when computing present value.
///
/// # Arguments
///
/// * `mbs` - Agency MBS passthrough instrument
/// * `market` - Market context with discount curves
/// * `as_of` - Valuation date
/// * `spread` - Spread to add (decimal, e.g., 0.01 for 100 bps)
///
/// # Returns
///
/// Present value with spread-adjusted discounting
pub fn price_with_spread(
    mbs: &AgencyMbsPassthrough,
    market: &MarketContext,
    as_of: Date,
    spread: f64,
) -> Result<f64> {
    let cashflows = generate_cashflows(mbs, as_of, Some(mbs.wam + 12))?;

    if cashflows.is_empty() {
        return Ok(0.0);
    }

    let discount_curve = market.get_discount(&mbs.discount_curve_id)?;

    let mut pv = 0.0;
    for cf in &cashflows {
        let years = mbs
            .day_count
            .year_fraction(as_of, cf.payment_date, DayCountCtx::default())?;
        let base_df = discount_curve.df(years);
        // Apply spread adjustment: DF_spread = DF_base * exp(-spread * t)
        let spread_adj = (-spread * years).exp();
        let df = base_df * spread_adj;
        pv += cf.total * df;
    }

    Ok(pv)
}

/// Agency MBS discounting pricer.
///
/// Implements the `Pricer` trait for registry-based dispatch.
#[derive(Clone, Debug, Default)]
pub struct AgencyMbsDiscountingPricer;

impl Pricer for AgencyMbsDiscountingPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::AgencyMbsPassthrough, ModelKey::Discounting)
    }

    fn price_dyn(
        &self,
        instrument: &dyn crate::instruments::common::traits::Instrument,
        market: &MarketContext,
        as_of: Date,
    ) -> PricingResult<ValuationResult> {
        let mbs = crate::pricer::expect_inst::<AgencyMbsPassthrough>(
            instrument,
            InstrumentType::AgencyMbsPassthrough,
        )?;

        let pv = price_mbs(mbs, market, as_of).map_err(|e| {
            PricingError::model_failure_ctx(e.to_string(), PricingErrorContext::default())
        })?;

        Ok(ValuationResult::stamped(mbs.id.as_str(), as_of, pv))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::cashflow::builder::specs::PrepaymentModelSpec;
    use finstack_core::currency::Currency;
    use finstack_core::dates::DayCount;
    use finstack_core::market_data::term_structures::DiscountCurve;
    use finstack_core::math::interp::InterpStyle;
    use finstack_core::types::{CurveId, InstrumentId};
    use time::Month;

    fn create_test_mbs() -> AgencyMbsPassthrough {
        AgencyMbsPassthrough::builder()
            .id(InstrumentId::new("TEST-MBS"))
            .pool_id("TEST-POOL".to_string())
            .agency(super::super::AgencyProgram::Fnma)
            .pool_type(super::super::PoolType::Generic)
            .original_face(Money::new(1_000_000.0, Currency::USD))
            .current_face(Money::new(1_000_000.0, Currency::USD))
            .current_factor(1.0)
            .wac(0.045)
            .pass_through_rate(0.04)
            .servicing_fee_rate(0.0025)
            .guarantee_fee_rate(0.0025)
            .wam(360)
            .issue_date(Date::from_calendar_date(2024, Month::January, 1).expect("valid"))
            .maturity_date(Date::from_calendar_date(2054, Month::January, 1).expect("valid"))
            .prepayment_model(PrepaymentModelSpec::psa(1.0))
            .discount_curve_id(CurveId::new("USD-OIS"))
            .day_count(DayCount::Thirty360)
            .build()
            .expect("valid mbs")
    }

    fn create_test_market(as_of: Date) -> MarketContext {
        let disc = DiscountCurve::builder("USD-OIS")
            .base_date(as_of)
            .knots([
                (0.0, 1.0),
                (1.0, 0.96),
                (5.0, 0.80),
                (10.0, 0.60),
                (30.0, 0.30),
            ])
            .set_interp(InterpStyle::Linear)
            .build()
            .expect("valid curve");

        MarketContext::new().insert_discount(disc)
    }

    #[test]
    fn test_generate_cashflows() {
        let mbs = create_test_mbs();
        let as_of = Date::from_calendar_date(2024, Month::January, 15).expect("valid");

        let cashflows = generate_cashflows(&mbs, as_of, Some(12)).expect("should generate");

        assert!(!cashflows.is_empty());
        assert!(cashflows.len() <= 12);

        // First cashflow should have beginning balance equal to current face
        assert!((cashflows[0].beginning_balance - 1_000_000.0).abs() < 1.0);

        // Each cashflow should have positive interest
        for cf in &cashflows {
            assert!(cf.interest > 0.0);
            assert!(cf.total > 0.0);
        }

        // Balance should decrease over time
        for i in 1..cashflows.len() {
            assert!(cashflows[i].beginning_balance <= cashflows[i - 1].beginning_balance);
        }
    }

    #[test]
    fn test_payment_delay_in_cashflows() {
        let mbs = create_test_mbs();
        let as_of = Date::from_calendar_date(2024, Month::January, 15).expect("valid");

        let cashflows = generate_cashflows(&mbs, as_of, Some(3)).expect("should generate");

        // Check that payment dates are delayed from period ends
        for cf in &cashflows {
            let days_diff = (cf.payment_date - cf.period_end).whole_days();
            assert_eq!(days_diff as u32, mbs.effective_payment_delay());
        }
    }

    #[test]
    fn test_price_mbs() {
        let mbs = create_test_mbs();
        let as_of = Date::from_calendar_date(2024, Month::January, 15).expect("valid");
        let market = create_test_market(as_of);

        let pv = price_mbs(&mbs, &market, as_of).expect("should price");

        // PV should be positive and less than face value for typical discount rates
        assert!(pv.amount() > 0.0);
        // With positive yields, MBS should trade near par for on-the-run pools
        // Allow wide range due to discounting
        assert!(pv.amount() > 500_000.0);
        assert!(pv.amount() < 1_500_000.0);
    }

    #[test]
    fn test_price_with_spread() {
        let mbs = create_test_mbs();
        let as_of = Date::from_calendar_date(2024, Month::January, 15).expect("valid");
        let market = create_test_market(as_of);

        let pv_base = price_with_spread(&mbs, &market, as_of, 0.0).expect("should price");
        let pv_spread = price_with_spread(&mbs, &market, as_of, 0.01).expect("should price");

        // Higher spread should reduce PV
        assert!(pv_spread < pv_base);
    }

    #[test]
    fn test_prepayment_impact() {
        let as_of = Date::from_calendar_date(2024, Month::January, 15).expect("valid");
        let market = create_test_market(as_of);

        // Create two MBS with different prepayment speeds
        let mut mbs_slow = create_test_mbs();
        mbs_slow.prepayment_model = PrepaymentModelSpec::psa(0.5); // 50% PSA

        let mut mbs_fast = create_test_mbs();
        mbs_fast.prepayment_model = PrepaymentModelSpec::psa(2.0); // 200% PSA

        let pv_slow = price_mbs(&mbs_slow, &market, as_of).expect("should price");
        let pv_fast = price_mbs(&mbs_fast, &market, as_of).expect("should price");

        // With positive discount rates (rates > coupon), faster prepayment
        // reduces duration and can change PV
        // The direction depends on whether we're at premium or discount
        assert!((pv_slow.amount() - pv_fast.amount()).abs() > 1.0);
    }
}
