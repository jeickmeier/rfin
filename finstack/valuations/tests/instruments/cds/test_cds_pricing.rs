//! CDS pricing engine tests.
//!
//! Tests core pricing logic including protection leg, premium leg,
//! NPV calculations, par spreads, and schedule generation.

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::{DiscountCurve, HazardCurve};
use finstack_core::money::Money;
use finstack_valuations::instruments::cds::pricer::{
    CDSPricer, CDSPricerConfig,
};
use finstack_valuations::instruments::cds::CreditDefaultSwap;
use finstack_valuations::instruments::common::traits::Instrument;
use time::macros::date;

/// Build flat discount curve for testing
fn build_discount_curve(rate: f64, base_date: Date, id: &str) -> DiscountCurve {
    DiscountCurve::builder(id)
        .base_date(base_date)
        .day_count(DayCount::Act360)
        .knots([
            (0.0, 1.0),
            (1.0, (-rate).exp() as f64),
            (5.0, (-rate * 5.0).exp() as f64),
            (10.0, (-rate * 10.0).exp() as f64),
        ])
        .build()
        .unwrap()
}

/// Build flat hazard curve for testing
fn build_hazard_curve(hazard_rate: f64, recovery: f64, base_date: Date, id: &str) -> HazardCurve {
    HazardCurve::builder(id)
        .base_date(base_date)
        .recovery_rate(recovery)
        .knots([
            (0.0, hazard_rate),
            (1.0, hazard_rate),
            (5.0, hazard_rate),
            (10.0, hazard_rate),
        ])
        .build()
        .unwrap()
}

#[test]
fn test_protection_leg_positive_pv() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);

    let disc = build_discount_curve(0.05, as_of, "USD_OIS");
    let hazard = build_hazard_curve(0.02, 0.40, as_of, "CORP");

    let cds = CreditDefaultSwap::buy_protection(
        "PROT_LEG_TEST",
        Money::new(10_000_000.0, Currency::USD),
        100.0,
        as_of,
        end,
        "USD_OIS",
        "CORP",
    );

    let market = MarketContext::new()
        .insert_discount(disc)
        .insert_hazard(hazard);

    let protection_pv = cds
        .pv_protection_leg(
            market.get_discount_ref("USD_OIS").unwrap(),
            market.get_hazard_ref("CORP").unwrap(),
            as_of,
        )
        .unwrap();

    assert!(
        protection_pv.amount() > 0.0,
        "Protection leg PV should be positive"
    );
}

#[test]
fn test_premium_leg_positive_pv() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);

    let disc = build_discount_curve(0.05, as_of, "USD_OIS");
    let hazard = build_hazard_curve(0.02, 0.40, as_of, "CORP");

    let cds = CreditDefaultSwap::buy_protection(
        "PREM_LEG_TEST",
        Money::new(10_000_000.0, Currency::USD),
        100.0,
        as_of,
        end,
        "USD_OIS",
        "CORP",
    );

    let market = MarketContext::new()
        .insert_discount(disc)
        .insert_hazard(hazard);

    let premium_pv = cds
        .pv_premium_leg(
            market.get_discount_ref("USD_OIS").unwrap(),
            market.get_hazard_ref("CORP").unwrap(),
            as_of,
        )
        .unwrap();

    assert!(
        premium_pv.amount() > 0.0,
        "Premium leg PV should be positive for positive spread"
    );
}

#[test]
fn test_npv_calculation_buyer() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);

    let disc = build_discount_curve(0.05, as_of, "USD_OIS");
    let hazard = build_hazard_curve(0.02, 0.40, as_of, "CORP");

    let market = MarketContext::new()
        .insert_discount(disc)
        .insert_hazard(hazard);

    let cds = CreditDefaultSwap::buy_protection(
        "NPV_BUYER",
        Money::new(10_000_000.0, Currency::USD),
        100.0,
        as_of,
        end,
        "USD_OIS",
        "CORP",
    );

    let npv = cds.value(&market, as_of).unwrap();

    // NPV should be finite
    assert!(npv.amount().is_finite());
}

#[test]
fn test_npv_buyer_seller_opposite() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);

    let disc = build_discount_curve(0.05, as_of, "USD_OIS");
    let hazard = build_hazard_curve(0.015, 0.40, as_of, "CORP");

    let market = MarketContext::new()
        .insert_discount(disc)
        .insert_hazard(hazard);

    let buyer = CreditDefaultSwap::buy_protection(
        "BUYER",
        Money::new(10_000_000.0, Currency::USD),
        100.0,
        as_of,
        end,
        "USD_OIS",
        "CORP",
    );

    let seller = CreditDefaultSwap::sell_protection(
        "SELLER",
        Money::new(10_000_000.0, Currency::USD),
        100.0,
        as_of,
        end,
        "USD_OIS",
        "CORP",
    );

    let npv_buyer = buyer.value(&market, as_of).unwrap();
    let npv_seller = seller.value(&market, as_of).unwrap();

    // Should be opposite signs
    assert!(npv_buyer.amount() * npv_seller.amount() < 0.0);

    // Should sum to approximately zero (within numerical tolerance)
    let sum = npv_buyer.amount() + npv_seller.amount();
    assert!(
        sum.abs() < 1000.0,
        "Buyer + Seller NPV sum should be near zero, got {}",
        sum
    );
}

#[test]
fn test_par_spread_positive() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);

    let disc = build_discount_curve(0.05, as_of, "USD_OIS");
    let hazard = build_hazard_curve(0.015, 0.40, as_of, "CORP");

    let market = MarketContext::new()
        .insert_discount(disc)
        .insert_hazard(hazard);

    let cds = CreditDefaultSwap::buy_protection(
        "PAR_SPREAD_TEST",
        Money::new(10_000_000.0, Currency::USD),
        100.0,
        as_of,
        end,
        "USD_OIS",
        "CORP",
    );

    let par_spread = cds
        .par_spread(
            market.get_discount_ref("USD_OIS").unwrap(),
            market.get_hazard_ref("CORP").unwrap(),
            as_of,
        )
        .unwrap();

    assert!(par_spread > 0.0, "Par spread should be positive");
    assert!(
        par_spread < 1000.0,
        "Par spread should be reasonable (<1000 bps)"
    );
}

#[test]
fn test_par_spread_gives_zero_npv() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);

    let disc = build_discount_curve(0.05, as_of, "USD_OIS");
    let hazard = build_hazard_curve(0.01, 0.40, as_of, "CORP");

    let market = MarketContext::new()
        .insert_discount(disc)
        .insert_hazard(hazard);

    let mut cds = CreditDefaultSwap::buy_protection(
        "PAR_NPV_TEST",
        Money::new(10_000_000.0, Currency::USD),
        100.0,
        as_of,
        end,
        "USD_OIS",
        "CORP",
    );

    // Get par spread
    let par_spread = cds
        .par_spread(
            market.get_discount_ref("USD_OIS").unwrap(),
            market.get_hazard_ref("CORP").unwrap(),
            as_of,
        )
        .unwrap();

    // Set spread to par
    cds.premium.spread_bp = par_spread;

    // NPV should be near zero
    let npv = cds.value(&market, as_of).unwrap();
    assert!(
        npv.amount().abs() < 10000.0,
        "NPV at par spread should be near zero, got {}",
        npv.amount()
    );
}

#[test]
fn test_risky_annuity_positive() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);

    let disc = build_discount_curve(0.05, as_of, "USD_OIS");
    let hazard = build_hazard_curve(0.01, 0.40, as_of, "CORP");

    let market = MarketContext::new()
        .insert_discount(disc)
        .insert_hazard(hazard);

    let cds = CreditDefaultSwap::buy_protection(
        "RISKY_ANN_TEST",
        Money::new(10_000_000.0, Currency::USD),
        100.0,
        as_of,
        end,
        "USD_OIS",
        "CORP",
    );

    let risky_annuity = cds
        .risky_annuity(
            market.get_discount_ref("USD_OIS").unwrap(),
            market.get_hazard_ref("CORP").unwrap(),
            as_of,
        )
        .unwrap();

    assert!(risky_annuity > 0.0);
    assert!(risky_annuity < 10.0, "Risky annuity for 5Y should be < 10");
}

#[test]
fn test_risky_pv01_scales_with_notional() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);

    let disc = build_discount_curve(0.05, as_of, "USD_OIS");
    let hazard = build_hazard_curve(0.01, 0.40, as_of, "CORP");

    let market = MarketContext::new()
        .insert_discount(disc)
        .insert_hazard(hazard);

    let cds1 = CreditDefaultSwap::buy_protection(
        "PV01_1MM",
        Money::new(1_000_000.0, Currency::USD),
        100.0,
        as_of,
        end,
        "USD_OIS",
        "CORP",
    );

    let cds10 = CreditDefaultSwap::buy_protection(
        "PV01_10MM",
        Money::new(10_000_000.0, Currency::USD),
        100.0,
        as_of,
        end,
        "USD_OIS",
        "CORP",
    );

    let pv01_1 = cds1
        .risky_pv01(
            market.get_discount_ref("USD_OIS").unwrap(),
            market.get_hazard_ref("CORP").unwrap(),
            as_of,
        )
        .unwrap();

    let pv01_10 = cds10
        .risky_pv01(
            market.get_discount_ref("USD_OIS").unwrap(),
            market.get_hazard_ref("CORP").unwrap(),
            as_of,
        )
        .unwrap();

    // Should scale linearly with notional
    let ratio = pv01_10 / pv01_1;
    assert!(
        (ratio - 10.0).abs() < 0.01,
        "PV01 should scale 10x, got ratio {}",
        ratio
    );
}

#[test]
fn test_schedule_generation_isda() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);

    let cds = CreditDefaultSwap::buy_protection(
        "SCHEDULE_TEST",
        Money::new(10_000_000.0, Currency::USD),
        100.0,
        as_of,
        end,
        "USD_OIS",
        "CORP",
    );

    let pricer = CDSPricer::new();
    let schedule = pricer.generate_isda_schedule(&cds).unwrap();

    assert!(
        schedule.len() >= 2,
        "Schedule should have at least start and end"
    );
    assert_eq!(schedule[0], cds.premium.start);
    assert_eq!(schedule[schedule.len() - 1], cds.premium.end);

    // Interior dates should be on the 20th (ISDA standard)
    for &date in schedule
        .iter()
        .skip(1)
        .take(schedule.len().saturating_sub(2))
    {
        assert_eq!(date.day(), 20, "ISDA coupon dates should be on 20th");
    }
}

#[test]
fn test_higher_hazard_increases_protection_value() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);

    let mut protection_pvs = Vec::new();

    for hazard_rate in [0.005, 0.010, 0.020, 0.030] {
        let disc_local = build_discount_curve(0.05, as_of, "USD_OIS");
        let hazard = build_hazard_curve(hazard_rate, 0.40, as_of, "CORP");
        let market = MarketContext::new()
            .insert_discount(disc_local)
            .insert_hazard(hazard);

        let cds = CreditDefaultSwap::buy_protection(
            "HAZARD_SENS",
            Money::new(10_000_000.0, Currency::USD),
            100.0,
            as_of,
            end,
            "USD_OIS",
            "CORP",
        );

        let protection_pv = cds
            .pv_protection_leg(
                market.get_discount_ref("USD_OIS").unwrap(),
                market.get_hazard_ref("CORP").unwrap(),
                as_of,
            )
            .unwrap();

        protection_pvs.push((hazard_rate, protection_pv.amount()));
    }

    // Protection value should increase with hazard rate
    for i in 1..protection_pvs.len() {
        assert!(
            protection_pvs[i].1 > protection_pvs[i - 1].1,
            "Protection PV should increase with hazard rate"
        );
    }
}

#[test]
fn test_higher_recovery_decreases_protection_value() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);

    let mut protection_pvs = Vec::new();

    for recovery in [0.20, 0.40, 0.60] {
        let disc_local = build_discount_curve(0.05, as_of, "USD_OIS");
        let hazard = build_hazard_curve(0.015, recovery, as_of, "CORP");
        let market = MarketContext::new()
            .insert_discount(disc_local)
            .insert_hazard(hazard);

        let mut cds = CreditDefaultSwap::buy_protection(
            "RECOVERY_SENS",
            Money::new(10_000_000.0, Currency::USD),
            100.0,
            as_of,
            end,
            "USD_OIS",
            "CORP",
        );
        cds.protection.recovery_rate = recovery;

        let protection_pv = cds
            .pv_protection_leg(
                market.get_discount_ref("USD_OIS").unwrap(),
                market.get_hazard_ref("CORP").unwrap(),
                as_of,
            )
            .unwrap();

        protection_pvs.push((recovery, protection_pv.amount()));
    }

    // Protection value should decrease with recovery rate
    for i in 1..protection_pvs.len() {
        assert!(
            protection_pvs[i].1 < protection_pvs[i - 1].1,
            "Protection PV should decrease with recovery rate"
        );
    }
}

#[test]
fn test_zero_spread_gives_negative_npv_for_buyer() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);

    let disc = build_discount_curve(0.05, as_of, "USD_OIS");
    let hazard = build_hazard_curve(0.015, 0.40, as_of, "CORP");

    let market = MarketContext::new()
        .insert_discount(disc)
        .insert_hazard(hazard);

    let cds = CreditDefaultSwap::buy_protection(
        "ZERO_SPREAD",
        Money::new(10_000_000.0, Currency::USD),
        0.0, // Zero spread
        as_of,
        end,
        "USD_OIS",
        "CORP",
    );

    let npv = cds.value(&market, as_of).unwrap();

    // With zero spread, buyer pays nothing but receives protection
    // NPV should be positive for buyer
    assert!(
        npv.amount() > 0.0,
        "Zero spread CDS should have positive NPV for buyer"
    );
}

#[test]
fn test_accrual_on_default_increases_premium() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);

    let disc = build_discount_curve(0.05, as_of, "USD_OIS");
    let hazard = build_hazard_curve(0.02, 0.40, as_of, "CORP");

    let market = MarketContext::new()
        .insert_discount(disc)
        .insert_hazard(hazard);

    let cds = CreditDefaultSwap::buy_protection(
        "ACCRUAL_TEST",
        Money::new(10_000_000.0, Currency::USD),
        100.0,
        as_of,
        end,
        "USD_OIS",
        "CORP",
    );

    let pricer_with_accrual = CDSPricer::new(); // Default includes accrual
    let pricer_without_accrual = CDSPricer::with_config(CDSPricerConfig {
        include_accrual: false,
        ..Default::default()
    });

    let pv_with = pricer_with_accrual
        .pv_premium_leg(
            &cds,
            market.get_discount_ref("USD_OIS").unwrap(),
            market.get_hazard_ref("CORP").unwrap(),
            as_of,
        )
        .unwrap()
        .amount();

    let pv_without = pricer_without_accrual
        .pv_premium_leg(
            &cds,
            market.get_discount_ref("USD_OIS").unwrap(),
            market.get_hazard_ref("CORP").unwrap(),
            as_of,
        )
        .unwrap()
        .amount();

    assert!(
        pv_with > pv_without,
        "Accrual on default should increase premium PV"
    );
}

#[test]
fn test_settlement_delay_reduces_protection_pv() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);

    let disc = build_discount_curve(0.05, as_of, "USD_OIS");
    let hazard = build_hazard_curve(0.02, 0.40, as_of, "CORP");

    let market = MarketContext::new()
        .insert_discount(disc)
        .insert_hazard(hazard);

    let mut cds_no_delay = CreditDefaultSwap::buy_protection(
        "NO_DELAY",
        Money::new(10_000_000.0, Currency::USD),
        100.0,
        as_of,
        end,
        "USD_OIS",
        "CORP",
    );
    cds_no_delay.protection.settlement_delay = 0;

    let mut cds_with_delay = cds_no_delay.clone();
    cds_with_delay.protection.settlement_delay = 20;

    let pricer = CDSPricer::new();

    let pv_no_delay = pricer
        .pv_protection_leg(
            &cds_no_delay,
            market.get_discount_ref("USD_OIS").unwrap(),
            market.get_hazard_ref("CORP").unwrap(),
            as_of,
        )
        .unwrap()
        .amount();

    let pv_with_delay = pricer
        .pv_protection_leg(
            &cds_with_delay,
            market.get_discount_ref("USD_OIS").unwrap(),
            market.get_hazard_ref("CORP").unwrap(),
            as_of,
        )
        .unwrap()
        .amount();

    assert!(
        pv_with_delay < pv_no_delay,
        "Settlement delay should reduce protection PV"
    );
}
