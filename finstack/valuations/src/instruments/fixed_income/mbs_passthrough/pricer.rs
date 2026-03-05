//! Agency MBS passthrough pricing.
//!
//! This module provides discounting-based pricing for agency MBS passthroughs,
//! generating projected cashflows with prepayment and payment delay adjustments.
//!
//! # SIFMA Settlement
//!
//! TBA-eligible agency MBS settle on SIFMA Good Delivery dates (third Wednesday
//! of each month). The [`sifma_settlement_for_period`] helper derives the SIFMA
//! settlement date for a given accrual period, useful for aligning TBA trade
//! settlement and pool allocation.

use super::delay::actual_payment_date;
use super::AgencyMbsPassthrough;
use crate::pricer::{
    InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingErrorContext, PricingResult,
};
use crate::results::ValuationResult;
use finstack_core::dates::{Date, DayCountCtx};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::money::Money;
use finstack_core::Result;

/// Generated MBS cashflow for a single period.
#[derive(Debug, Clone)]
pub struct MbsCashflow {
    /// Accrual period start date
    pub period_start: Date,
    /// Accrual period end date
    pub period_end: Date,
    /// Actual payment date (after delay)
    pub payment_date: Date,
    /// SIFMA Good Delivery settlement date for this period.
    pub sifma_date: Date,
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

/// Derive the SIFMA Good Delivery settlement date for a given accrual period.
pub fn sifma_settlement_for_period(period_end: Date) -> Date {
    finstack_core::dates::sifma_settlement_date(period_end.month(), period_end.year())
}

/// Generate projected cashflows for an agency MBS.
pub fn generate_cashflows(
    mbs: &AgencyMbsPassthrough,
    as_of: Date,
    max_periods: Option<u32>,
) -> Result<Vec<MbsCashflow>> {
    use time::Duration;

    let mut cashflows = Vec::new();
    let mut balance = mbs.current_face.amount();

    if mbs.wam == 0 {
        return Err(finstack_core::Error::Validation(
            "WAM must be positive".to_string(),
        ));
    }

    // Start from the first of next month after as_of, but never before issue_date.
    // For forward-settling instruments (e.g. TBAs), issue_date is in the future
    // and no cashflows should accrue before it.
    let effective_start = as_of.max(mbs.issue_date);
    let mut period_start =
        Date::from_calendar_date(effective_start.year(), effective_start.month(), 1)
            .map_err(|e| finstack_core::Error::Validation(e.to_string()))?;
    if period_start <= effective_start {
        period_start = period_start
            .checked_add(Duration::days(32))
            .and_then(|d| Date::from_calendar_date(d.year(), d.month(), 1).ok())
            .unwrap_or(period_start);
    }

    let payment_delay = mbs.effective_payment_delay();
    let max_periods = max_periods.unwrap_or(mbs.wam);
    let monthly_rate = mbs.pass_through_rate / 12.0;
    let monthly_mortgage_rate = mbs.wac / 12.0;

    let mut projected_count: u32 = 0;
    loop {
        if balance < 0.01 || projected_count >= max_periods {
            break;
        }

        let period_end = end_of_month(period_start)?;
        let payment_date = actual_payment_date(period_end, payment_delay, false)?;

        if payment_date <= as_of {
            period_start = next_month_start(period_start)?;
            continue;
        }

        projected_count += 1;

        let seasoning = mbs.seasoning_months(period_end);
        let smm = mbs.prepayment_model.smm(seasoning)?.min(0.9999);

        let remaining_months = mbs.wam.saturating_sub(seasoning);
        let remaining_months = if remaining_months == 0 {
            1
        } else {
            remaining_months
        };
        let scheduled_principal = if remaining_months == 1 {
            balance
        } else if monthly_mortgage_rate > 1e-12 {
            let factor = (1.0 + monthly_mortgage_rate).powi(remaining_months as i32);
            let payment = balance * monthly_mortgage_rate * factor / (factor - 1.0);
            let interest_component = balance * monthly_mortgage_rate;
            (payment - interest_component).max(0.0).min(balance)
        } else {
            balance / remaining_months as f64
        };

        let prepayment = balance * smm;
        let interest = balance * monthly_rate;
        let total_principal = scheduled_principal + prepayment;
        let ending_balance = (balance - total_principal).max(0.0);

        let sifma_date = sifma_settlement_for_period(period_end);

        cashflows.push(MbsCashflow {
            period_start,
            period_end,
            payment_date,
            sifma_date,
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

        if period_end >= mbs.maturity {
            break;
        }
    }

    Ok(cashflows)
}

fn end_of_month(date: Date) -> Result<Date> {
    let year = date.year();
    let month = date.month();
    let days_in_month = month.length(year);
    Date::from_calendar_date(year, month, days_in_month)
        .map_err(|e| finstack_core::Error::Validation(e.to_string()))
}

fn next_month_start(date: Date) -> Result<Date> {
    use time::Duration;
    let end = end_of_month(date)?;
    let next = end + Duration::days(1);
    Ok(next)
}

/// Discount a set of MBS cashflows to present value.
///
/// Uses the curve's own day count for time calculation, applying an optional
/// spread adjustment: `DF_spread = DF_base × exp(-spread × t)`.
fn discount_cashflows(
    cashflows: &[MbsCashflow],
    curve: &DiscountCurve,
    as_of: Date,
    spread: f64,
) -> Result<f64> {
    let dc = curve.day_count();
    let mut pv = 0.0;
    for cf in cashflows {
        let years = dc.year_fraction(as_of, cf.payment_date, DayCountCtx::default())?;
        let base_df = curve.df(years);
        let df = if spread.abs() > f64::EPSILON {
            base_df * (-spread * years).exp()
        } else {
            base_df
        };
        pv += cf.total * df;
    }
    Ok(pv)
}

/// Price an agency MBS using discounting.
///
/// Uses the discount curve's own day count convention for computing
/// year fractions, ensuring consistency with the curve's interpolation.
pub fn price_mbs(mbs: &AgencyMbsPassthrough, market: &MarketContext, as_of: Date) -> Result<Money> {
    let cashflows = generate_cashflows(mbs, as_of, Some(mbs.wam + 12))?;

    if cashflows.is_empty() {
        return Ok(Money::new(0.0, mbs.current_face.currency()));
    }

    let discount_curve = market.get_discount(&mbs.discount_curve_id)?;
    let pv = discount_cashflows(&cashflows, &discount_curve, as_of, 0.0)?;

    Ok(Money::new(pv, mbs.current_face.currency()))
}

/// Price an agency MBS with a spread adjustment.
///
/// Adds a spread (in decimal) to the discount rate when computing present value.
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
    discount_cashflows(&cashflows, &discount_curve, as_of, spread)
}

/// Agency MBS discounting pricer.
#[derive(Debug, Clone, Default)]
pub struct AgencyMbsDiscountingPricer;

impl Pricer for AgencyMbsDiscountingPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::AgencyMbsPassthrough, ModelKey::Discounting)
    }

    fn price_dyn(
        &self,
        instrument: &dyn crate::instruments::common_impl::traits::Instrument,
        market: &MarketContext,
        as_of: Date,
    ) -> PricingResult<ValuationResult> {
        let mbs = crate::pricer::expect_inst::<AgencyMbsPassthrough>(
            instrument,
            InstrumentType::AgencyMbsPassthrough,
        )?;

        let pv = price_mbs(mbs, market, as_of).map_err(|e| {
            PricingError::model_failure_with_context(e.to_string(), PricingErrorContext::default())
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
            .pool_id("TEST-POOL".into())
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
            .maturity(Date::from_calendar_date(2054, Month::January, 1).expect("valid"))
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
            .interp(InterpStyle::Linear)
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
        assert!((cashflows[0].beginning_balance - 1_000_000.0).abs() < 1.0);

        for cf in &cashflows {
            assert!(cf.interest > 0.0);
            assert!(cf.total > 0.0);
        }

        for i in 1..cashflows.len() {
            assert!(cashflows[i].beginning_balance <= cashflows[i - 1].beginning_balance);
        }
    }

    #[test]
    fn test_forward_start_no_cashflows_before_issue() {
        let as_of = Date::from_calendar_date(2024, Month::January, 15).expect("valid");
        let future_issue = Date::from_calendar_date(2024, Month::March, 20).expect("valid");

        let mbs = AgencyMbsPassthrough::builder()
            .id(InstrumentId::new("FORWARD-POOL"))
            .pool_id("FWD-POOL".into())
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
            .issue_date(future_issue)
            .maturity(Date::from_calendar_date(2054, Month::March, 20).expect("valid"))
            .prepayment_model(PrepaymentModelSpec::psa(1.0))
            .discount_curve_id(CurveId::new("USD-OIS"))
            .day_count(DayCount::Thirty360)
            .build()
            .expect("valid mbs");

        let cashflows = generate_cashflows(&mbs, as_of, Some(12)).expect("should generate");

        for cf in &cashflows {
            assert!(
                cf.period_start >= Date::from_calendar_date(2024, Month::April, 1).expect("valid"),
                "Forward-start pool should not generate cashflows before issue_date; got {}",
                cf.period_start
            );
        }
    }

    #[test]
    fn test_payment_delay_in_cashflows() {
        let mbs = create_test_mbs();
        let as_of = Date::from_calendar_date(2024, Month::January, 15).expect("valid");

        let cashflows = generate_cashflows(&mbs, as_of, Some(3)).expect("should generate");

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

        assert!(pv.amount() > 0.0);
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

        assert!(pv_spread < pv_base);
    }

    #[test]
    fn test_prepayment_impact() {
        let as_of = Date::from_calendar_date(2024, Month::January, 15).expect("valid");
        let market = create_test_market(as_of);

        let mut mbs_slow = create_test_mbs();
        mbs_slow.prepayment_model = PrepaymentModelSpec::psa(0.5);

        let mut mbs_fast = create_test_mbs();
        mbs_fast.prepayment_model = PrepaymentModelSpec::psa(2.0);

        let pv_slow = price_mbs(&mbs_slow, &market, as_of).expect("should price");
        let pv_fast = price_mbs(&mbs_fast, &market, as_of).expect("should price");

        assert!((pv_slow.amount() - pv_fast.amount()).abs() > 1.0);
    }
}
