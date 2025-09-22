#![cfg(test)]

use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::term_structures::hazard_curve::HazardCurve;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::F;
use finstack_valuations::instruments::cds::pricing::engine::CDSPricer;
use finstack_valuations::instruments::cds::CreditParams;
use finstack_valuations::instruments::cds_index::parameters::{CDSIndexConstituentParam, CDSIndexConstructionParams, CDSIndexParams};
use finstack_valuations::instruments::cds_index::CDSIndex;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::metrics::{standard_registry, MetricContext, MetricId};
use std::sync::Arc;
use time::Month;

fn flat_discount(id: &'static str, base: Date) -> DiscountCurve {
    DiscountCurve::builder(id)
        .base_date(base)
        .knots(vec![(0.0, 1.0), (10.0, 1.0)])
        .build()
        .unwrap()
}

fn flat_hazard(id: &'static str, base: Date, rec: F, hz: F) -> HazardCurve {
    HazardCurve::builder(id)
        .base_date(base)
        .recovery_rate(rec)
        .knots(vec![(1.0, hz), (10.0, hz)])
        .par_spreads(vec![(1.0, hz * 10000.0 * (1.0 - rec))])
        .build()
        .unwrap()
}

#[test]
fn imm_20th_schedule_for_index_synthetic() {
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let start = as_of;
    let end = Date::from_calendar_date(2028, Month::January, 1).unwrap();

    let idx = CDSIndex::new_standard(
        "CDX-IMM",
        &CDSIndexParams::cdx_na_ig(42, 1, 100.0),
        &CDSIndexConstructionParams::buy_protection(Money::new(1_000_000.0, Currency::USD)),
        start,
        end,
        &CreditParams::senior_unsecured("INDEX", "HZ-IDX"),
        "USD-OIS",
        "HZ-IDX",
    );
    let cds = idx.to_synthetic_cds();
    let pricer = CDSPricer::new();
    let schedule = pricer.generate_isda_schedule(&cds).unwrap();
    // Internal coupon dates (excluding first and last) should be on the 20th
    for d in schedule.iter().skip(1).take(schedule.len().saturating_sub(2)) {
        assert_eq!(d.day(), 20);
    }
}

#[test]
fn index_factor_scales_pv() {
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let start = as_of;
    let end = Date::from_calendar_date(2028, Month::January, 1).unwrap();

    let disc = flat_discount("USD-OIS", as_of);
    let rec = finstack_valuations::instruments::cds::parameters::RECOVERY_SENIOR_UNSECURED;
    let hz = 0.02;
    let names = [("N1", "HZ1"), ("N2", "HZ2"), ("N3", "HZ3"), ("N4", "HZ4"), ("N5", "HZ5")];
    let mut ctx = MarketContext::new().insert_discount(disc);
    for (_, hid) in &names { ctx = ctx.insert_hazard(flat_hazard(hid, as_of, rec, hz)); }

    let cons: Vec<CDSIndexConstituentParam> = names
        .iter()
        .map(|(n, hid)| CDSIndexConstituentParam { credit: CreditParams::senior_unsecured(*n, hid), weight: 1.0 / 5.0 })
        .collect();

    let p_base = CDSIndexParams::cdx_na_ig(42, 1, 100.0).with_constituents(cons.clone());
    let p_scaled = CDSIndexParams::cdx_na_ig(42, 1, 100.0).with_index_factor(0.8).with_constituents(cons);

    let idx_base = CDSIndex::new_standard(
        "CDX-BASE",
        &p_base,
        &CDSIndexConstructionParams::buy_protection(Money::new(10_000_000.0, Currency::USD)),
        start,
        end,
        &CreditParams::senior_unsecured("INDEX", "HZ-IDX"),
        "USD-OIS",
        "HZ-IDX",
    );

    let idx_scaled = CDSIndex::new_standard(
        "CDX-SCALED",
        &p_scaled,
        &CDSIndexConstructionParams::buy_protection(Money::new(10_000_000.0, Currency::USD)),
        start,
        end,
        &CreditParams::senior_unsecured("INDEX", "HZ-IDX"),
        "USD-OIS",
        "HZ-IDX",
    );

    let pv_base = idx_base.value(&ctx, as_of).unwrap().amount();
    let pv_scaled = idx_scaled.value(&ctx, as_of).unwrap().amount();
    // PV should scale approximately with index_factor. Allow small numerical tolerance.
    let ratio = pv_scaled / (pv_base * 0.8);
    assert!((ratio - 1.0).abs() < 5e-6);
}

#[test]
fn hazard_cs01_matches_bump_difference() {
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let start = as_of;
    let end = Date::from_calendar_date(2028, Month::January, 1).unwrap();

    let disc = flat_discount("USD-OIS", as_of);
    let rec = finstack_valuations::instruments::cds::parameters::RECOVERY_SENIOR_UNSECURED;
    let hz = 0.02;
    let names = [("N1", "HZ1"), ("N2", "HZ2"), ("N3", "HZ3"), ("N4", "HZ4"), ("N5", "HZ5")];
    let mut ctx = MarketContext::new().insert_discount(disc);
    for (_, hid) in &names { ctx = ctx.insert_hazard(flat_hazard(hid, as_of, rec, hz)); }

    let cons: Vec<CDSIndexConstituentParam> = names
        .iter()
        .map(|(n, hid)| CDSIndexConstituentParam { credit: CreditParams::senior_unsecured(*n, hid), weight: 1.0 / 5.0 })
        .collect();
    let params = CDSIndexParams::cdx_na_ig(42, 1, 100.0).with_constituents(cons);

    let idx = CDSIndex::new_standard(
        "CDX-CS01",
        &params,
        &CDSIndexConstructionParams::buy_protection(Money::new(10_000_000.0, Currency::USD)),
        start,
        end,
        &CreditParams::senior_unsecured("INDEX", "HZ-IDX"),
        "USD-OIS",
        "HZ-IDX",
    );

    // Metric via registry
    let pv = idx.value(&ctx, as_of).unwrap();
    let mut mctx = MetricContext::new(Arc::new(idx.clone()), Arc::new(ctx.clone()), as_of, pv);
    let reg = standard_registry();
    let res = reg.compute(&[MetricId::HazardCs01], &mut mctx).unwrap();
    let cs01 = *res.get(&MetricId::HazardCs01).unwrap();

    // Manual bump using MarketContext::bump
    let mut bumps = hashbrown::HashMap::new();
    for cid in ctx.curve_ids() {
        if ctx.get_ref::<HazardCurve>(cid.as_str()).is_ok() {
            bumps.insert(cid.clone(), finstack_core::market_data::bumps::BumpSpec::parallel_bp(1.0));
        }
    }
    let bumped = ctx.bump(bumps).unwrap();
    let dv = (idx.value(&bumped, as_of).unwrap().amount() - pv.amount()).abs();

    assert!((cs01 - dv).abs() < 1e-6);
}


