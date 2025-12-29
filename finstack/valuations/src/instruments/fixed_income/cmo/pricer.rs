//! Agency CMO pricing.
//!
//! CMO pricing projects collateral cashflows and distributes them
//! through the waterfall to calculate the PV of the reference tranche.

use super::types::{AgencyCmo, CmoTrancheType};
use super::waterfall::{allocate_io_cashflow, execute_waterfall};
use crate::cashflow::builder::specs::PrepaymentModelSpec;
use crate::instruments::agency_mbs_passthrough::pricer::generate_cashflows;
use crate::instruments::agency_mbs_passthrough::{AgencyMbsPassthrough, PoolType};
use crate::pricer::{InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingResult};
use crate::results::ValuationResult;
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount, DayCountCtx};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::InstrumentId;
use finstack_core::Result;

/// Tranche cashflow for a single period.
#[derive(Clone, Debug)]
pub struct TrancheCashflow {
    /// Payment date
    pub payment_date: Date,
    /// Principal payment
    pub principal: f64,
    /// Interest payment
    pub interest: f64,
    /// Total payment
    pub total: f64,
    /// Ending balance after this period
    pub ending_balance: f64,
}

/// Generate cashflows for the reference tranche.
///
/// Projects collateral cashflows and runs them through the waterfall
/// to determine the reference tranche's cashflows.
pub fn generate_tranche_cashflows(
    cmo: &AgencyCmo,
    as_of: Date,
    max_periods: Option<u32>,
) -> Result<Vec<TrancheCashflow>> {
    // Create or use collateral pool
    let collateral = if let Some(ref pool) = cmo.collateral {
        pool.as_ref().clone()
    } else {
        create_assumed_collateral(cmo)?
    };

    // Generate collateral cashflows
    let collateral_cfs = generate_cashflows(&collateral, as_of, max_periods)?;

    // Create a working copy of the waterfall
    let mut waterfall = cmo.waterfall.clone();

    let mut tranche_cfs = Vec::new();
    let ref_id = &cmo.reference_tranche_id;

    let ref_tranche = waterfall
        .get_tranche(ref_id)
        .ok_or_else(|| finstack_core::Error::Validation(format!("Tranche {} not found", ref_id)))?;

    let is_io = ref_tranche.tranche_type == CmoTrancheType::InterestOnly;

    // Track collateral factor for IO strips
    let original_collateral = collateral.current_face.amount();

    for cf in &collateral_cfs {
        // Run waterfall for this period
        let total_principal = cf.scheduled_principal + cf.prepayment;
        let total_interest = cf.interest;

        if is_io {
            // IO gets interest based on collateral factor
            let factor = cf.ending_balance / original_collateral;
            // We validated ref_id exists at function start, so this should always succeed
            if let Some(io_tranche) = waterfall.get_tranche(ref_id) {
                let io_payment = allocate_io_cashflow(io_tranche, factor);

                tranche_cfs.push(TrancheCashflow {
                    payment_date: cf.payment_date,
                    principal: 0.0,
                    interest: io_payment,
                    total: io_payment,
                    ending_balance: io_tranche.current_face.amount() * factor,
                });
            }
        } else {
            // Regular waterfall execution
            let result = execute_waterfall(&mut waterfall, total_principal, total_interest);

            // Find allocation for reference tranche
            if let Some(alloc) = result.allocations.iter().find(|a| a.tranche_id == *ref_id) {
                tranche_cfs.push(TrancheCashflow {
                    payment_date: cf.payment_date,
                    principal: alloc.principal,
                    interest: alloc.interest,
                    total: alloc.principal + alloc.interest,
                    ending_balance: alloc.ending_balance,
                });
            }
        }
    }

    Ok(tranche_cfs)
}

/// Create assumed collateral for CMO valuation.
fn create_assumed_collateral(cmo: &AgencyCmo) -> Result<AgencyMbsPassthrough> {
    let total_face = cmo.waterfall.total_current_face();
    let wac = cmo.collateral_wac.unwrap_or(0.045);
    let wam = cmo.collateral_wam.unwrap_or(360);

    // Standard fee assumptions
    let servicing_fee = 0.0025;
    let guarantee_fee = 0.0025;
    let pass_through = wac - servicing_fee - guarantee_fee;

    let maturity_date = cmo
        .issue_date
        .checked_add(time::Duration::days((wam as i64) * 30))
        .ok_or_else(|| finstack_core::Error::Validation("Invalid maturity".to_string()))?;

    AgencyMbsPassthrough::builder()
        .id(InstrumentId::new(format!("{}-COLLATERAL", cmo.id.as_str())))
        .pool_id(format!("{}-POOL", cmo.deal_name))
        .agency(cmo.agency)
        .pool_type(PoolType::Generic)
        .original_face(total_face)
        .current_face(total_face)
        .current_factor(1.0)
        .wac(wac)
        .pass_through_rate(pass_through)
        .servicing_fee_rate(servicing_fee)
        .guarantee_fee_rate(guarantee_fee)
        .wam(wam)
        .issue_date(cmo.issue_date)
        .maturity_date(maturity_date)
        .prepayment_model(PrepaymentModelSpec::psa(1.0))
        .discount_curve_id(cmo.discount_curve_id.clone())
        .day_count(DayCount::Thirty360)
        .build()
}

/// Price a CMO reference tranche.
///
/// Generates tranche cashflows and discounts them to present value.
pub fn price_cmo(cmo: &AgencyCmo, market: &MarketContext, as_of: Date) -> Result<Money> {
    let tranche_cfs = generate_tranche_cashflows(cmo, as_of, None)?;

    if tranche_cfs.is_empty() {
        return Ok(Money::new(0.0, Currency::USD));
    }

    let discount_curve = market.get_discount_ref(&cmo.discount_curve_id)?;
    let day_count = DayCount::Thirty360;

    let mut pv = 0.0;
    for cf in &tranche_cfs {
        let years = day_count.year_fraction(as_of, cf.payment_date, DayCountCtx::default())?;
        let df = discount_curve.df(years);
        pv += cf.total * df;
    }

    Ok(Money::new(pv, Currency::USD))
}

/// Agency CMO discounting pricer.
#[derive(Clone, Debug, Default)]
pub struct AgencyCmoDiscountingPricer;

impl Pricer for AgencyCmoDiscountingPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::AgencyCmo, ModelKey::Discounting)
    }

    fn price_dyn(
        &self,
        instrument: &dyn crate::instruments::common::traits::Instrument,
        market: &MarketContext,
        as_of: Date,
    ) -> PricingResult<ValuationResult> {
        let cmo = crate::pricer::expect_inst::<AgencyCmo>(instrument, InstrumentType::AgencyCmo)?;

        let pv = price_cmo(cmo, market, as_of)
            .map_err(|e| PricingError::model_failure(e.to_string()))?;

        Ok(ValuationResult::stamped(cmo.id.as_str(), as_of, pv))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
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
    fn test_generate_tranche_cashflows() {
        let cmo = AgencyCmo::example();
        let as_of = Date::from_calendar_date(2024, Month::January, 15).expect("valid");

        let cfs = generate_tranche_cashflows(&cmo, as_of, Some(12)).expect("should generate");

        assert!(!cfs.is_empty());

        // Sequential A tranche should get principal first
        for cf in &cfs {
            // Should have some cashflow
            assert!(cf.total >= 0.0);
        }
    }

    #[test]
    fn test_price_cmo() {
        let cmo = AgencyCmo::example();
        let as_of = Date::from_calendar_date(2024, Month::January, 15).expect("valid");
        let market = create_test_market(as_of);

        let pv = price_cmo(&cmo, &market, as_of).expect("should price");

        // PV should be positive
        assert!(pv.amount() > 0.0);
    }

    #[test]
    fn test_price_io_strip() {
        let cmo = AgencyCmo::example_io_po();
        let as_of = Date::from_calendar_date(2024, Month::January, 15).expect("valid");
        let market = create_test_market(as_of);

        // Change reference to IO
        let mut io_cmo = cmo.clone();
        io_cmo.reference_tranche_id = "IO".to_string();

        let pv = price_cmo(&io_cmo, &market, as_of).expect("should price");

        // IO should have positive value
        assert!(pv.amount() > 0.0);
    }

    #[test]
    fn test_price_po_strip() {
        let cmo = AgencyCmo::example_io_po();
        let as_of = Date::from_calendar_date(2024, Month::January, 15).expect("valid");
        let market = create_test_market(as_of);

        // Change reference to PO
        let mut po_cmo = cmo.clone();
        po_cmo.reference_tranche_id = "PO".to_string();

        let pv = price_cmo(&po_cmo, &market, as_of).expect("should price");

        // PO should have positive value
        assert!(pv.amount() > 0.0);
    }
}
