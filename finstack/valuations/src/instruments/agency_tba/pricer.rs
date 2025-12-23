//! Agency TBA pricing.
//!
//! TBA pricing uses assumed pool characteristics to project cashflows
//! and discount them to present value, then compares to the trade price.

use super::AgencyTba;
use crate::cashflow::builder::specs::PrepaymentModelSpec;
use crate::instruments::agency_mbs_passthrough::{
    pricer::price_mbs, AgencyMbsPassthrough, AgencyProgram, PoolType,
};
use crate::pricer::{InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingResult};
use crate::results::ValuationResult;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::InstrumentId;
use finstack_core::Result;

/// Create assumed pool for TBA valuation.
///
/// Uses standard assumptions for generic pool characteristics based on
/// the TBA's agency, coupon, and term.
pub fn create_assumed_pool(tba: &AgencyTba, _as_of: Date) -> Result<AgencyMbsPassthrough> {
    let settlement_date = tba.get_settlement_date()?;
    let term_months = tba.term.months();

    // Assume pool is newly issued (factor = 1.0)
    let maturity_date = settlement_date
        .checked_add(time::Duration::days((term_months as i64) * 30))
        .ok_or_else(|| finstack_core::Error::Validation("Invalid maturity date".to_string()))?;

    // Standard servicing and g-fee assumptions
    let servicing_fee = 0.0025; // 25 bps
    let guarantee_fee = match tba.agency {
        AgencyProgram::Gnma => 0.0006, // 6 bps for GNMA
        _ => 0.0025,                   // 25 bps for FNMA/FHLMC
    };

    // WAC = pass-through + fees
    let wac = tba.coupon + servicing_fee + guarantee_fee;

    AgencyMbsPassthrough::builder()
        .id(InstrumentId::new(format!("{}-ASSUMED", tba.id.as_str())))
        .pool_id(format!("{}-POOL", tba.id.as_str()))
        .agency(tba.agency)
        .pool_type(PoolType::Generic)
        .original_face(tba.notional)
        .current_face(tba.notional)
        .current_factor(1.0)
        .wac(wac)
        .pass_through_rate(tba.coupon)
        .servicing_fee_rate(servicing_fee)
        .guarantee_fee_rate(guarantee_fee)
        .wam(term_months)
        .issue_date(settlement_date)
        .maturity_date(maturity_date)
        .prepayment_model(PrepaymentModelSpec::psa(1.0))
        .discount_curve_id(tba.discount_curve_id.clone())
        .day_count(DayCount::Thirty360)
        .build()
}

/// Price a TBA forward.
///
/// Calculates the value as the difference between the forward price
/// of the assumed pool and the trade price, discounted to valuation date.
///
/// # Arguments
///
/// * `tba` - TBA forward instrument
/// * `market` - Market context with discount curves
/// * `as_of` - Valuation date
pub fn price_tba(tba: &AgencyTba, market: &MarketContext, as_of: Date) -> Result<Money> {
    let settlement_date = tba.get_settlement_date()?;

    // If settlement has passed, value is the realized P&L
    if settlement_date <= as_of {
        return Ok(Money::new(0.0, tba.notional.currency()));
    }

    // Get or create assumed pool
    let assumed_pool = if let Some(ref pool) = tba.assumed_pool {
        pool.as_ref().clone()
    } else {
        create_assumed_pool(tba, as_of)?
    };

    // Price the assumed pool
    let pool_pv = price_mbs(&assumed_pool, market, as_of)?;

    // Calculate forward price (at settlement)
    let discount_curve = market.get_discount_ref(&tba.discount_curve_id)?;
    let years_to_settle = (settlement_date - as_of).whole_days() as f64 / 365.0;
    let df_to_settle = discount_curve.df(years_to_settle);

    // Trade value at settlement = notional × trade_price / 100
    let trade_value_at_settle = tba.notional.amount() * tba.trade_price / 100.0;

    // PV of trade value
    let trade_pv = trade_value_at_settle * df_to_settle;

    // TBA value = Pool PV - Trade PV
    // Positive if pool is worth more than we're paying
    let value = pool_pv.amount() - trade_pv;

    Ok(Money::new(value, tba.notional.currency()))
}

/// Agency TBA discounting pricer.
#[derive(Clone, Debug, Default)]
pub struct AgencyTbaDiscountingPricer;

impl Pricer for AgencyTbaDiscountingPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::AgencyTba, ModelKey::Discounting)
    }

    fn price_dyn(
        &self,
        instrument: &dyn crate::instruments::common::traits::Instrument,
        market: &MarketContext,
        as_of: Date,
    ) -> PricingResult<ValuationResult> {
        let tba = crate::pricer::expect_inst::<AgencyTba>(instrument, InstrumentType::AgencyTba)?;

        let pv =
            price_tba(tba, market, as_of).map_err(|e| PricingError::ModelFailure(e.to_string()))?;

        Ok(ValuationResult::stamped(tba.id.as_str(), as_of, pv))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    use finstack_core::math::interp::InterpStyle;
    use time::Month;

    fn create_test_market(as_of: Date) -> MarketContext {
        let disc = DiscountCurve::builder("USD-OIS")
            .base_date(as_of)
            .knots([
                (0.0, 1.0),
                (0.25, 0.99),
                (1.0, 0.96),
                (5.0, 0.80),
                (30.0, 0.30),
            ])
            .set_interp(InterpStyle::Linear)
            .build()
            .expect("valid curve");

        MarketContext::new().insert_discount(disc)
    }

    #[test]
    fn test_create_assumed_pool() {
        let tba = AgencyTba::example();
        let as_of = Date::from_calendar_date(2024, Month::January, 15).expect("valid");

        let pool = create_assumed_pool(&tba, as_of).expect("should create pool");

        assert_eq!(pool.agency, tba.agency);
        assert!((pool.pass_through_rate - tba.coupon).abs() < 1e-10);
        assert!((pool.current_factor - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_price_tba() {
        let tba = AgencyTba::example();
        let as_of = Date::from_calendar_date(2024, Month::January, 15).expect("valid");
        let market = create_test_market(as_of);

        let pv = price_tba(&tba, &market, as_of).expect("should price");

        // PV should be reasonable
        assert!(pv.amount().abs() < tba.notional.amount());
    }

    #[test]
    fn test_tba_expired() {
        let mut tba = AgencyTba::example();
        tba.settlement_year = 2023;
        tba.settlement_month = 1;

        let as_of = Date::from_calendar_date(2024, Month::January, 15).expect("valid");
        let market = create_test_market(as_of);

        let pv = price_tba(&tba, &market, as_of).expect("should price");

        // Expired TBA should have zero value
        assert!((pv.amount()).abs() < 1e-10);
    }
}
