//! CDS Index metrics validation tests against market standards.
//!
//! These tests validate CDS Index pricing and metrics against:
//! - CDX/iTraxx standard methodologies
//! - Single-curve vs constituents pricing consistency
//! - Index factor conventions
//! - Market-standard risk metrics
//!
//! References:
//! - Markit CDX/iTraxx documentation
//! - ISDA CDS Index conventions

use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::MarketContext;
use finstack_core::market_data::term_structures::{DiscountCurve, HazardCurve};
use finstack_core::money::Money;
use finstack_valuations::instruments::cds::parameters::RECOVERY_SENIOR_UNSECURED;
use finstack_valuations::instruments::CreditParams;
use finstack_valuations::instruments::cds_index::parameters::{
    CDSIndexConstituentParam, CDSIndexConstructionParams, CDSIndexParams,
};
use finstack_valuations::instruments::cds_index::{CDSIndex, IndexPricing};
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::metrics::MetricId;
use time::macros::date;

fn flat_discount(id: &str, base: Date) -> DiscountCurve {
    DiscountCurve::builder(id)
        .base_date(base)
        .day_count(finstack_core::dates::DayCount::Act360)
        .knots([(0.0, 1.0), (5.0, 0.80), (10.0, 0.60)])
        .build()
        .unwrap()
}

fn flat_hazard(id: &str, base: Date, rec: f64, hz: f64) -> HazardCurve {
    let par = hz * 10000.0 * (1.0 - rec);
    HazardCurve::builder(id)
        .base_date(base)
        .recovery_rate(rec)
        .knots([(1.0, hz), (5.0, hz), (10.0, hz)])
        .par_spreads([(1.0, par), (5.0, par)])
        .build()
        .unwrap()
}

#[test]
fn test_cds_index_single_vs_constituents_pricing() {
    // Market Standard: Single curve and constituents pricing should be similar
    // when all constituents have same credit quality
    
    let as_of = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    
    let disc = flat_discount("USD-OIS", as_of);
    let rec = RECOVERY_SENIOR_UNSECURED;
    let hz = 0.015; // 1.5% hazard rate for all
    
    // Create 5 equal hazard curves
    let names = [("N1", "HZ1"), ("N2", "HZ2"), ("N3", "HZ3"), ("N4", "HZ4"), ("N5", "HZ5")];
    let mut ctx = MarketContext::new().insert_discount(disc);
    
    for (_, hid) in &names {
        let hc = flat_hazard(hid, as_of, rec, hz);
        ctx = ctx.insert_hazard(hc);
    }
    
    // Single aggregate hazard for index
    let hz_index = flat_hazard("HZ-IDX", as_of, rec, hz);
    ctx = ctx.insert_hazard(hz_index);
    
    // Build constituents with equal weights
    let cons: Vec<CDSIndexConstituentParam> = names
        .iter()
        .map(|(n, hid)| CDSIndexConstituentParam {
            credit: CreditParams::corporate_standard(*n, *hid),
            weight: 0.20, // Equal weight
        })
        .collect();
    
    let params = CDSIndexParams::cdx_na_ig(42, 1, 100.0).with_constituents(cons);
    
    // Index with constituents pricing
    let idx_constituents = CDSIndex::new_standard(
        "CDX-CONSTITUENTS",
        &params,
        &CDSIndexConstructionParams::buy_protection(Money::new(10_000_000.0, Currency::USD)),
        as_of,
        end,
        &CreditParams::corporate_standard("INDEX", "HZ-IDX"),
        "USD-OIS",
        "HZ-IDX",
    );
    
    // Index with single curve pricing
    let mut idx_single = idx_constituents.clone();
    idx_single.id = "CDX-SINGLE".into();
    idx_single.pricing = IndexPricing::SingleCurve;
    
    let pv_constituents = idx_constituents.value(&ctx, as_of).unwrap();
    let pv_single = idx_single.value(&ctx, as_of).unwrap();
    
    // When all constituents have same hazard, single curve should approximate constituents
    // Allow 10% tolerance due to different calculation paths
    let diff_pct = ((pv_constituents.amount() - pv_single.amount()) / pv_constituents.amount().abs()).abs();
    
    assert!(
        diff_pct < 0.10,
        "Single curve NPV={:.0} vs Constituents NPV={:.0} differ by {:.1}% (should be <10%)",
        pv_single.amount(),
        pv_constituents.amount(),
        diff_pct * 100.0
    );
}

#[test]
fn test_cds_index_constituent_weights_sum_to_one() {
    // Market Standard: Constituent weights must sum to 1.0
    
    let as_of = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    
    let names = [("N1", "HZ1"), ("N2", "HZ2"), ("N3", "HZ3")];
    let cons: Vec<CDSIndexConstituentParam> = names
        .iter()
        .map(|(n, hid)| CDSIndexConstituentParam {
            credit: CreditParams::corporate_standard(*n, *hid),
            weight: 1.0 / 3.0,
        })
        .collect();
    
    let params = CDSIndexParams::cdx_na_ig(42, 1, 100.0).with_constituents(cons);
    
    let idx = CDSIndex::new_standard(
        "CDX-WEIGHTS-TEST",
        &params,
        &CDSIndexConstructionParams::buy_protection(Money::new(10_000_000.0, Currency::USD)),
        as_of,
        end,
        &CreditParams::corporate_standard("INDEX", "HZ-IDX"),
        "USD-OIS",
        "HZ-IDX",
    );
    
    let weight_sum: f64 = idx.constituents.iter().map(|c| c.weight).sum();
    
    assert!(
        (weight_sum - 1.0).abs() < 1e-10,
        "Constituent weights sum to {:.6}, expected 1.0",
        weight_sum
    );
}

#[test]
fn test_cds_index_cs01_scales_with_notional() {
    // CS01 should scale linearly with notional
    
    let as_of = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    
    let disc = flat_discount("USD-OIS", as_of);
    let rec = RECOVERY_SENIOR_UNSECURED;
    let hz = 0.015;
    let hz_index = flat_hazard("HZ-IDX", as_of, rec, hz);
    
    let ctx = MarketContext::new()
        .insert_discount(disc)
        .insert_hazard(hz_index);
    
    let params = CDSIndexParams::cdx_na_ig(42, 1, 100.0);
    
    // Test with $10MM notional
    let idx_10mm = CDSIndex::new_standard(
        "CDX-10MM",
        &params,
        &CDSIndexConstructionParams::buy_protection(Money::new(10_000_000.0, Currency::USD)),
        as_of,
        end,
        &CreditParams::corporate_standard("INDEX", "HZ-IDX"),
        "USD-OIS",
        "HZ-IDX",
    );
    
    // Test with $20MM notional
    let idx_20mm = CDSIndex::new_standard(
        "CDX-20MM",
        &params,
        &CDSIndexConstructionParams::buy_protection(Money::new(20_000_000.0, Currency::USD)),
        as_of,
        end,
        &CreditParams::corporate_standard("INDEX", "HZ-IDX"),
        "USD-OIS",
        "HZ-IDX",
    );
    
    let result_10mm = idx_10mm
        .price_with_metrics(&ctx, as_of, &[MetricId::Cs01])
        .unwrap();
    let result_20mm = idx_20mm
        .price_with_metrics(&ctx, as_of, &[MetricId::Cs01])
        .unwrap();
    
    let cs01_10mm = *result_10mm.measures.get("cs01").unwrap();
    let cs01_20mm = *result_20mm.measures.get("cs01").unwrap();
    
    // CS01 should double with notional
    assert!(
        (cs01_20mm - 2.0 * cs01_10mm).abs() < 100.0,
        "CS01 should scale linearly: 2×{:.0} ≠ {:.0}",
        cs01_10mm,
        cs01_20mm
    );
}

#[test]
fn test_cds_index_par_spread_positive() {
    // Par spread should be positive for standard market conditions
    
    let as_of = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    
    let disc = flat_discount("USD-OIS", as_of);
    let hz_index = flat_hazard("HZ-IDX", as_of, 0.40, 0.015);
    
    let ctx = MarketContext::new()
        .insert_discount(disc)
        .insert_hazard(hz_index);
    
    let params = CDSIndexParams::cdx_na_ig(42, 1, 100.0);
    
    let idx = CDSIndex::new_standard(
        "CDX-PAR-TEST",
        &params,
        &CDSIndexConstructionParams::buy_protection(Money::new(10_000_000.0, Currency::USD)),
        as_of,
        end,
        &CreditParams::corporate_standard("INDEX", "HZ-IDX"),
        "USD-OIS",
        "HZ-IDX",
    );
    
    let result = idx
        .price_with_metrics(&ctx, as_of, &[MetricId::ParSpread])
        .unwrap();
    
    let par_spread = *result.measures.get("par_spread").unwrap();
    
    assert!(
        par_spread > 0.0,
        "Par spread={:.2} bps should be positive",
        par_spread
    );
    
    // For 1.5% hazard, 40% recovery: spread ≈ 1.5% × 0.6 = 90 bps
    assert!(
        par_spread > 50.0 && par_spread < 150.0,
        "Par spread={:.2} bps outside expected range 50-150 bps",
        par_spread
    );
}

#[test]
fn test_cds_index_risky_pv01_positive() {
    // Risky PV01 should be positive
    
    let as_of = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    
    let disc = flat_discount("USD-OIS", as_of);
    let hz_index = flat_hazard("HZ-IDX", as_of, 0.40, 0.015);
    
    let ctx = MarketContext::new()
        .insert_discount(disc)
        .insert_hazard(hz_index);
    
    let params = CDSIndexParams::cdx_na_ig(42, 1, 100.0);
    
    let idx = CDSIndex::new_standard(
        "CDX-PV01-TEST",
        &params,
        &CDSIndexConstructionParams::buy_protection(Money::new(10_000_000.0, Currency::USD)),
        as_of,
        end,
        &CreditParams::corporate_standard("INDEX", "HZ-IDX"),
        "USD-OIS",
        "HZ-IDX",
    );
    
    let result = idx
        .price_with_metrics(&ctx, as_of, &[MetricId::RiskyPv01])
        .unwrap();
    
    let risky_pv01 = *result.measures.get("risky_pv01").unwrap();
    
    assert!(
        risky_pv01 > 0.0,
        "Risky PV01={:.2} should be positive",
        risky_pv01
    );
    
    // For $10MM, 5Y index, expect $4,000-$5,000 range
    assert!(
        risky_pv01 > 3_500.0 && risky_pv01 < 5_500.0,
        "Risky PV01={:.0} outside expected range $3,500-$5,500",
        risky_pv01
    );
}

