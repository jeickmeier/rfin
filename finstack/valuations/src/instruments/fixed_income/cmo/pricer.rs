//! Agency CMO pricing.
//!
//! CMO pricing projects collateral cashflows and distributes them
//! through the waterfall to calculate the PV of the reference tranche.

use super::types::{AgencyCmo, CmoTrancheType};
use super::waterfall::{allocate_io_cashflow, execute_waterfall_with_principal_breakdown};
use crate::cashflow::builder::specs::PrepaymentModelSpec;
use crate::cashflow::builder::{CashFlowMeta, CashFlowSchedule};
use crate::cashflow::primitives::{CFKind, CashFlow};
use crate::instruments::fixed_income::mbs_passthrough::pricer::generate_cashflows;
use crate::instruments::fixed_income::mbs_passthrough::{AgencyMbsPassthrough, PoolType};
use crate::pricer::{
    InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingErrorContext, PricingResult,
};
use crate::results::ValuationResult;
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount, DayCountContext};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::InstrumentId;
use finstack_core::Result;

/// Tranche cashflow for a single period.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct TrancheCashflow {
    /// Payment date
    pub(crate) payment_date: Date,
    /// Principal payment
    pub(crate) principal: f64,
    /// Scheduled principal payment
    pub(crate) scheduled_principal: f64,
    /// Prepayment principal payment
    pub(crate) prepayment_principal: f64,
    /// Interest payment
    pub(crate) interest: f64,
    /// Total payment
    pub(crate) total: f64,
    /// Ending balance after this period
    pub(crate) ending_balance: f64,
}

/// Resolve the collateral pool used as the canonical source for tranche projection.
pub(crate) fn resolve_collateral(cmo: &AgencyCmo) -> Result<AgencyMbsPassthrough> {
    if let Some(ref pool) = cmo.collateral {
        Ok(pool.as_ref().clone())
    } else {
        create_assumed_collateral(cmo)
    }
}

/// Generate cashflows for the reference tranche.
///
/// Projects collateral cashflows and runs them through the waterfall
/// to determine the reference tranche's cashflows.
pub(crate) fn generate_tranche_cashflows(
    cmo: &AgencyCmo,
    as_of: Date,
    max_periods: Option<u32>,
) -> Result<Vec<TrancheCashflow>> {
    let collateral = resolve_collateral(cmo)?;

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
        let total_interest = cf.interest;

        if is_io {
            // IO gets interest based on collateral factor.
            // Use beginning_balance (not ending_balance) because interest accrues
            // on the balance at the start of the period, before principal payments.
            let factor = cf.beginning_balance / original_collateral;
            // We validated ref_id exists at function start, so this should always succeed
            if let Some(io_tranche) = waterfall.get_tranche(ref_id) {
                let io_payment = allocate_io_cashflow(io_tranche, factor);

                tranche_cfs.push(TrancheCashflow {
                    payment_date: cf.payment_date,
                    principal: 0.0,
                    scheduled_principal: 0.0,
                    prepayment_principal: 0.0,
                    interest: io_payment,
                    total: io_payment,
                    ending_balance: io_tranche.current_face.amount() * factor,
                });
            }
        } else {
            // Regular waterfall execution
            let result = execute_waterfall_with_principal_breakdown(
                &mut waterfall,
                cf.scheduled_principal,
                cf.prepayment,
                total_interest,
                None,
            );

            // Find allocation for reference tranche
            if let Some(alloc) = result.allocations.iter().find(|a| a.tranche_id == *ref_id) {
                tranche_cfs.push(TrancheCashflow {
                    payment_date: cf.payment_date,
                    principal: alloc.principal,
                    scheduled_principal: alloc.scheduled_principal,
                    prepayment_principal: alloc.prepayment_principal,
                    interest: alloc.interest,
                    total: alloc.principal + alloc.interest,
                    ending_balance: alloc.ending_balance,
                });
            }
        }
    }

    Ok(tranche_cfs)
}

/// Build the canonical reference-tranche schedule used by pricing and providers.
///
pub(crate) fn build_reference_tranche_schedule(
    cmo: &AgencyCmo,
    as_of: Date,
    max_periods: Option<u32>,
) -> Result<CashFlowSchedule> {
    let tranche = cmo.reference_tranche().ok_or_else(|| {
        finstack_core::Error::Validation(format!("Tranche {} not found", cmo.reference_tranche_id))
    })?;
    let tranche_cashflows = generate_tranche_cashflows(cmo, as_of, max_periods)?;
    let mut flows = Vec::with_capacity(tranche_cashflows.len() * 2);

    for cf in tranche_cashflows {
        if cf.interest.abs() > f64::EPSILON {
            flows.push(CashFlow {
                date: cf.payment_date,
                reset_date: None,
                amount: Money::new(cf.interest, tranche.current_face.currency()),
                kind: CFKind::Fixed,
                accrual_factor: 0.0,
                rate: Some(tranche.coupon),
            });
        }
        if cf.scheduled_principal.abs() > f64::EPSILON {
            flows.push(CashFlow {
                date: cf.payment_date,
                reset_date: None,
                amount: Money::new(cf.scheduled_principal, tranche.current_face.currency()),
                kind: CFKind::Amortization,
                accrual_factor: 0.0,
                rate: None,
            });
        }
        if cf.prepayment_principal.abs() > f64::EPSILON {
            flows.push(CashFlow {
                date: cf.payment_date,
                reset_date: None,
                amount: Money::new(cf.prepayment_principal, tranche.current_face.currency()),
                kind: CFKind::PrePayment,
                accrual_factor: 0.0,
                rate: None,
            });
        }
    }

    Ok(crate::cashflow::traits::schedule_from_classified_flows(
        flows,
        DayCount::Thirty360,
        crate::cashflow::traits::ScheduleBuildOpts {
            notional_hint: Some(tranche.current_face),
            meta: Some(CashFlowMeta {
                representation: crate::cashflow::builder::CashflowRepresentation::Projected,
                calendar_ids: Vec::new(),
                facility_limit: None,
                issue_date: Some(cmo.issue_date),
            }),
            ..Default::default()
        },
    ))
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

    let maturity = cmo
        .issue_date
        .checked_add(time::Duration::days((wam as i64) * 30))
        .ok_or_else(|| finstack_core::Error::Validation("Invalid maturity".to_string()))?;

    AgencyMbsPassthrough::builder()
        .id(InstrumentId::new(format!("{}-COLLATERAL", cmo.id.as_str())))
        .pool_id(format!("{}-POOL", cmo.deal_name).into())
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
        .maturity(maturity)
        .prepayment_model(PrepaymentModelSpec::psa(1.0))
        .discount_curve_id(cmo.discount_curve_id.clone())
        .day_count(DayCount::Thirty360)
        .build()
}

/// Price a CMO reference tranche.
///
/// Generates tranche cashflows and discounts them to present value.
pub(crate) fn price_cmo(cmo: &AgencyCmo, market: &MarketContext, as_of: Date) -> Result<Money> {
    let schedule = build_reference_tranche_schedule(cmo, as_of, None)?;
    let currency = cmo
        .reference_tranche()
        .map(|tranche| tranche.current_face.currency())
        .unwrap_or(Currency::USD);

    if schedule.flows.is_empty() {
        return Ok(Money::new(0.0, currency));
    }

    let discount_curve = market.get_discount(&cmo.discount_curve_id)?;
    let dc = discount_curve.day_count();

    let mut pv = 0.0;
    for cf in &schedule.flows {
        let years = dc.year_fraction(as_of, cf.date, DayCountContext::default())?;
        let df = discount_curve.df(years);
        pv += cf.amount.amount() * df;
    }

    Ok(Money::new(pv, currency))
}

/// Agency CMO discounting pricer.
#[derive(Debug, Clone, Default)]
pub(crate) struct AgencyCmoDiscountingPricer;

impl Pricer for AgencyCmoDiscountingPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::AgencyCmo, ModelKey::Discounting)
    }

    fn price_dyn(
        &self,
        instrument: &dyn crate::instruments::common_impl::traits::Instrument,
        market: &MarketContext,
        as_of: Date,
    ) -> PricingResult<ValuationResult> {
        let cmo = crate::pricer::expect_inst::<AgencyCmo>(instrument, InstrumentType::AgencyCmo)?;

        let pv = price_cmo(cmo, market, as_of).map_err(|e| {
            PricingError::model_failure_with_context(e.to_string(), PricingErrorContext::default())
        })?;

        Ok(ValuationResult::stamped(cmo.id.as_str(), as_of, pv))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::cashflow::primitives::CFKind;
    use finstack_core::market_data::term_structures::DiscountCurve;
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
            .interp(InterpStyle::Linear)
            .build()
            .expect("valid curve");

        MarketContext::new().insert(disc)
    }

    #[test]
    fn test_generate_tranche_cashflows() {
        let cmo = AgencyCmo::example().expect("AgencyCmo example is valid");
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
    fn test_reference_tranche_schedule_preserves_scheduled_and_prepay_rows() {
        let cmo = AgencyCmo::example().expect("AgencyCmo example is valid");
        let as_of = Date::from_calendar_date(2024, Month::January, 15).expect("valid");
        let schedule = build_reference_tranche_schedule(&cmo, as_of, Some(6))
            .expect("reference tranche schedule should build");

        assert!(!schedule.flows.is_empty());
        assert!(schedule.flows.iter().any(|cf| cf.kind == CFKind::Fixed));
        assert!(schedule
            .flows
            .iter()
            .any(|cf| cf.kind == CFKind::Amortization));
        assert!(schedule
            .flows
            .iter()
            .any(|cf| cf.kind == CFKind::PrePayment));
    }

    #[test]
    fn test_pac_support_reference_schedule_preserves_prepayment_rows() {
        let cmo = AgencyCmo::example_pac_support().expect("PAC/support example is valid");
        let as_of = Date::from_calendar_date(2024, Month::January, 15).expect("valid");
        let schedule = build_reference_tranche_schedule(&cmo, as_of, Some(12))
            .expect("PAC/support schedule should build");

        assert!(schedule
            .flows
            .iter()
            .any(|cf| cf.kind == CFKind::Amortization));
        assert!(schedule
            .flows
            .iter()
            .any(|cf| cf.kind == CFKind::PrePayment));
    }

    #[test]
    fn test_price_cmo() {
        let cmo = AgencyCmo::example().expect("AgencyCmo example is valid");
        let as_of = Date::from_calendar_date(2024, Month::January, 15).expect("valid");
        let market = create_test_market(as_of);

        let pv = price_cmo(&cmo, &market, as_of).expect("should price");

        // PV should be positive
        assert!(pv.amount() > 0.0);
    }

    #[test]
    fn test_price_io_strip() {
        let cmo = AgencyCmo::example_io_po().expect("AgencyCmo IO/PO example is valid");
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
        let cmo = AgencyCmo::example_io_po().expect("AgencyCmo IO/PO example is valid");
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
