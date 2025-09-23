#![cfg(test)]

use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::term_structures::hazard_curve::HazardCurve;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::F;
use finstack_valuations::instruments::cds::parameters::RECOVERY_SENIOR_UNSECURED;
use finstack_valuations::instruments::cds::CreditParams;
use finstack_valuations::instruments::cds_index::parameters::{CDSIndexConstituentParam, CDSIndexConstructionParams, CDSIndexParams};
use finstack_valuations::instruments::cds_index::{CDSIndex, IndexPricing};
use finstack_valuations::instruments::common::traits::Priceable;
use finstack_valuations::metrics::{standard_registry, MetricId, MetricContext};
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
fn cds_index_constituents_pricing_and_metrics() {
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let start = as_of;
    let end = Date::from_calendar_date(2028, Month::January, 1).unwrap();

    // Market curves: one discount, five hazard (equal hazard for simplicity)
    let disc = flat_discount("USD-OIS", as_of);
    let rec = RECOVERY_SENIOR_UNSECURED;
    let hz = 0.02; // 2% hazard

    let names = [
        ("N1", "HZ1"),
        ("N2", "HZ2"),
        ("N3", "HZ3"),
        ("N4", "HZ4"),
        ("N5", "HZ5"),
    ];
    let mut ctx = MarketContext::new().insert_discount(disc);
    for (_, hid) in &names {
        let hc = flat_hazard(hid, as_of, rec, hz);
        ctx = ctx.insert_hazard(hc);
    }

    // Build params with equal-weight constituents
    let cons: Vec<CDSIndexConstituentParam> = names
        .iter()
        .map(|(n, hid)| CDSIndexConstituentParam {
            credit: CreditParams::senior_unsecured(*n, hid),
            weight: 1.0 / 5.0,
        })
        .collect();
    let params = CDSIndexParams::cdx_na_ig(42, 1, 100.0).with_constituents(cons);

    // Construct index with constituents path
    let idx = CDSIndex::new_standard(
        "CDX-TEST",
        &params,
        &CDSIndexConstructionParams::buy_protection(Money::new(10_000_000.0, Currency::USD)),
        start,
        end,
        &CreditParams::senior_unsecured("INDEX", "HZ-IDX"),
        "USD-OIS",
        "HZ-IDX", // unused in constituents path
    );

    // Ensure pricing mode and weights
    assert!(matches!(idx.pricing, IndexPricing::Constituents));
    let wsum: F = idx.constituents.iter().map(|c| c.weight).sum();
    assert!((wsum - 1.0).abs() < 1e-12);

    // Value and metrics
    let curves = Arc::new(ctx);
    let pv = idx.value(&curves, as_of).unwrap();
    assert!(pv.amount().is_finite());

    let base_value = pv;
    let mut mctx = MetricContext::new(Arc::new(idx.clone()), curves.clone(), as_of, base_value);
    let reg = standard_registry();
    let metrics = reg
        .compute(
            &[
                MetricId::ParSpread,
                MetricId::RiskyPv01,
                MetricId::ProtectionLegPv,
                MetricId::PremiumLegPv,
                MetricId::Cs01,
            ],
            &mut mctx,
        )
        .unwrap();

    // Basic sanity checks
    assert!(metrics.get(&MetricId::ParSpread).copied().unwrap_or(0.0) >= 0.0);
    assert!(metrics.get(&MetricId::RiskyPv01).copied().unwrap_or(0.0) > 0.0);
    let prot = metrics.get(&MetricId::ProtectionLegPv).copied().unwrap_or(0.0);
    let prem = metrics.get(&MetricId::PremiumLegPv).copied().unwrap_or(0.0);
    assert!(prot >= 0.0 && prem >= 0.0);
}


