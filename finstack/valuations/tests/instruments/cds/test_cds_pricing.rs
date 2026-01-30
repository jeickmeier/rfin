//! CDS pricing engine tests.
//!
//! Tests core pricing logic including protection leg, premium leg,
//! NPV calculations, par spreads, and schedule generation.

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::{DiscountCurve, HazardCurve};
use finstack_core::money::Money;
use finstack_valuations::instruments::credit_derivatives::cds::{CDSPricer, CDSPricerConfig};
use finstack_valuations::instruments::Instrument;
use finstack_valuations::test_utils;
use rust_decimal::Decimal;
use time::macros::date;

/// Build flat discount curve for testing
fn build_discount_curve(rate: f64, base_date: Date, id: &str) -> DiscountCurve {
    DiscountCurve::builder(id)
        .base_date(base_date)
        .day_count(DayCount::Act360)
        .knots([
            (0.0, 1.0),
            (1.0, (-rate).exp()),
            (5.0, (-rate * 5.0).exp()),
            (10.0, (-rate * 10.0).exp()),
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

    let cds = test_utils::cds_buy_protection(
        "PROT_LEG_TEST",
        Money::new(10_000_000.0, Currency::USD),
        100.0,
        as_of,
        end,
        "USD_OIS",
        "CORP",
    )
    .expect("CDS construction should succeed");

    let market = MarketContext::new()
        .insert_discount(disc)
        .insert_hazard(hazard);

    let protection_pv = cds
        .pv_protection_leg(
            market.get_discount("USD_OIS").unwrap().as_ref(),
            market.get_hazard("CORP").unwrap().as_ref(),
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

    let cds = test_utils::cds_buy_protection(
        "PREM_LEG_TEST",
        Money::new(10_000_000.0, Currency::USD),
        100.0,
        as_of,
        end,
        "USD_OIS",
        "CORP",
    )
    .expect("CDS construction should succeed");

    let market = MarketContext::new()
        .insert_discount(disc)
        .insert_hazard(hazard);

    let premium_pv = cds
        .pv_premium_leg(
            market.get_discount("USD_OIS").unwrap().as_ref(),
            market.get_hazard("CORP").unwrap().as_ref(),
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

    let cds = test_utils::cds_buy_protection(
        "NPV_BUYER",
        Money::new(10_000_000.0, Currency::USD),
        100.0,
        as_of,
        end,
        "USD_OIS",
        "CORP",
    )
    .expect("CDS construction should succeed");

    let npv = cds.value(&market, as_of).unwrap();

    // NPV should be finite
    assert!(npv.amount().is_finite());
}

#[test]
fn test_par_spread_full_premium_branch() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    let disc = build_discount_curve(0.03, as_of, "USD_OIS");
    let hazard = build_hazard_curve(0.04, 0.40, as_of, "CORP");
    let market = MarketContext::new()
        .insert_discount(disc)
        .insert_hazard(hazard);

    let cds = test_utils::cds_buy_protection(
        "FULL_PREM",
        Money::new(10_000_000.0, Currency::USD),
        100.0,
        as_of,
        end,
        "USD_OIS",
        "CORP",
    )
    .expect("CDS construction should succeed");

    let pricer_base = CDSPricer::new();

    let mut cfg_full = CDSPricerConfig::isda_standard();
    cfg_full.par_spread_uses_full_premium = true;
    let pricer_full = CDSPricer::with_config(cfg_full);

    let par_base = pricer_base
        .par_spread(
            &cds,
            market.get_discount("USD_OIS").unwrap().as_ref(),
            market.get_hazard("CORP").unwrap().as_ref(),
            as_of,
        )
        .expect("par spread");

    let par_full = pricer_full
        .par_spread(
            &cds,
            market.get_discount("USD_OIS").unwrap().as_ref(),
            market.get_hazard("CORP").unwrap().as_ref(),
            as_of,
        )
        .expect("par spread full premium");

    // Full premium denominator is larger -> par spread should tighten.
    assert!(
        par_full < par_base,
        "Full premium par spread should be lower; base={par_base}, full={par_full}"
    );
}

#[test]
fn test_par_spread_errors_when_expired() {
    let as_of = date!(2029 - 01 - 01);
    let start = date!(2024 - 01 - 01);
    let end = as_of; // expired on valuation date
    let disc = build_discount_curve(0.03, start, "USD_OIS");
    let hazard = build_hazard_curve(0.02, 0.40, start, "CORP");
    let market = MarketContext::new()
        .insert_discount(disc)
        .insert_hazard(hazard);

    let cds = test_utils::cds_buy_protection(
        "EXPIRED_PAR",
        Money::new(10_000_000.0, Currency::USD),
        100.0,
        start,
        end,
        "USD_OIS",
        "CORP",
    )
    .expect("CDS construction should succeed");

    let pricer = CDSPricer::new();
    let err = pricer
        .par_spread(
            &cds,
            market.get_discount("USD_OIS").unwrap().as_ref(),
            market.get_hazard("CORP").unwrap().as_ref(),
            as_of,
        )
        .expect_err("expired CDS should error");
    let msg = err.to_string();
    assert!(
        msg.contains("denominator") || msg.contains("start time"),
        "expected denominator/start time error, got {msg}"
    );
}

#[test]
fn test_premium_leg_excludes_accrual_when_disabled() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2026 - 01 - 01);
    let disc = build_discount_curve(0.02, as_of, "USD_OIS");
    let hazard = build_hazard_curve(0.05, 0.40, as_of, "CORP");
    let market = MarketContext::new()
        .insert_discount(disc)
        .insert_hazard(hazard);

    let cds = test_utils::cds_buy_protection(
        "AOE_TOGGLE",
        Money::new(10_000_000.0, Currency::USD),
        500.0,
        as_of,
        end,
        "USD_OIS",
        "CORP",
    )
    .expect("CDS construction should succeed");

    let pricer_default = CDSPricer::new(); // include_accrual = true

    let mut cfg_no_aod = CDSPricerConfig::isda_standard();
    cfg_no_aod.include_accrual = false;
    let pricer_no_aod = CDSPricer::with_config(cfg_no_aod);

    let disc_ref = market.get_discount("USD_OIS").unwrap();
    let hazard_ref = market.get_hazard("CORP").unwrap();

    let with_aod = pricer_default
        .premium_leg_pv_per_bp(&cds, disc_ref.as_ref(), hazard_ref.as_ref(), as_of)
        .expect("premium with AoD");
    let without_aod = pricer_no_aod
        .premium_leg_pv_per_bp(&cds, disc_ref.as_ref(), hazard_ref.as_ref(), as_of)
        .expect("premium without AoD");

    assert!(
        with_aod > without_aod,
        "Including accrual-on-default should increase premium PV per bp; with={with_aod}, without={without_aod}"
    );
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

    let buyer = test_utils::cds_buy_protection(
        "BUYER",
        Money::new(10_000_000.0, Currency::USD),
        100.0,
        as_of,
        end,
        "USD_OIS",
        "CORP",
    )
    .expect("CDS construction should succeed");

    let seller = test_utils::cds_sell_protection(
        "SELLER",
        Money::new(10_000_000.0, Currency::USD),
        100.0,
        as_of,
        end,
        "USD_OIS",
        "CORP",
    )
    .expect("CDS construction should succeed");

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

    let cds = test_utils::cds_buy_protection(
        "PAR_SPREAD_TEST",
        Money::new(10_000_000.0, Currency::USD),
        100.0,
        as_of,
        end,
        "USD_OIS",
        "CORP",
    )
    .expect("CDS construction should succeed");

    let par_spread = cds
        .par_spread(
            market.get_discount("USD_OIS").unwrap().as_ref(),
            market.get_hazard("CORP").unwrap().as_ref(),
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

    let mut cds = test_utils::cds_buy_protection(
        "PAR_NPV_TEST",
        Money::new(10_000_000.0, Currency::USD),
        100.0,
        as_of,
        end,
        "USD_OIS",
        "CORP",
    )
    .expect("CDS construction should succeed");

    // Get par spread
    let par_spread = cds
        .par_spread(
            market.get_discount("USD_OIS").unwrap().as_ref(),
            market.get_hazard("CORP").unwrap().as_ref(),
            as_of,
        )
        .unwrap();

    // Set spread to par (convert f64 to Decimal)
    cds.premium.spread_bp = Decimal::try_from(par_spread).expect("valid par_spread");

    // NPV should be near zero (allowing for float->Decimal conversion / rounding).
    let npv = cds.value(&market, as_of).unwrap();
    assert!(
        npv.amount().abs() < 500.0,
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

    let cds = test_utils::cds_buy_protection(
        "RISKY_ANN_TEST",
        Money::new(10_000_000.0, Currency::USD),
        100.0,
        as_of,
        end,
        "USD_OIS",
        "CORP",
    )
    .expect("CDS construction should succeed");

    let risky_annuity = cds
        .risky_annuity(
            market.get_discount("USD_OIS").unwrap().as_ref(),
            market.get_hazard("CORP").unwrap().as_ref(),
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

    let cds1 = test_utils::cds_buy_protection(
        "PV01_1MM",
        Money::new(1_000_000.0, Currency::USD),
        100.0,
        as_of,
        end,
        "USD_OIS",
        "CORP",
    )
    .expect("CDS construction should succeed");

    let cds10 = test_utils::cds_buy_protection(
        "PV01_10MM",
        Money::new(10_000_000.0, Currency::USD),
        100.0,
        as_of,
        end,
        "USD_OIS",
        "CORP",
    )
    .expect("CDS construction should succeed");

    let pv01_1 = cds1
        .risky_pv01(
            market.get_discount("USD_OIS").unwrap().as_ref(),
            market.get_hazard("CORP").unwrap().as_ref(),
            as_of,
        )
        .unwrap();

    let pv01_10 = cds10
        .risky_pv01(
            market.get_discount("USD_OIS").unwrap().as_ref(),
            market.get_hazard("CORP").unwrap().as_ref(),
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

    let cds = test_utils::cds_buy_protection(
        "SCHEDULE_TEST",
        Money::new(10_000_000.0, Currency::USD),
        100.0,
        as_of,
        end,
        "USD_OIS",
        "CORP",
    )
    .expect("CDS construction should succeed");

    let pricer = CDSPricer::new();
    let schedule = pricer.generate_isda_schedule(&cds).unwrap();

    assert!(
        schedule.len() >= 2,
        "Schedule should have at least start and end"
    );
    assert_eq!(schedule[0], cds.premium.start);

    // The maturity date may be adjusted to a business day via Modified Following
    // so we just check it's on or after the original maturity
    let last_date = schedule[schedule.len() - 1];
    assert!(
        last_date >= cds.premium.end.saturating_sub(time::Duration::days(3))
            && last_date <= cds.premium.end.saturating_add(time::Duration::days(3)),
        "Last date should be near maturity date (got {} vs expected ~{})",
        last_date,
        cds.premium.end
    );

    // Interior dates should be near the 20th (ISDA standard)
    // Business day adjustment per ISDA 2014 (Modified Following) may move dates
    // forward if the 20th falls on a weekend or holiday.
    for &date in schedule
        .iter()
        .skip(1)
        .take(schedule.len().saturating_sub(2))
    {
        let day = date.day();
        assert!(
            (18..=23).contains(&day),
            "ISDA coupon dates should be near 20th (got day {})",
            day
        );
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

        let cds = test_utils::cds_buy_protection(
            "HAZARD_SENS",
            Money::new(10_000_000.0, Currency::USD),
            100.0,
            as_of,
            end,
            "USD_OIS",
            "CORP",
        )
        .expect("CDS construction should succeed");

        let protection_pv = cds
            .pv_protection_leg(
                market.get_discount("USD_OIS").unwrap().as_ref(),
                market.get_hazard("CORP").unwrap().as_ref(),
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

        let mut cds = test_utils::cds_buy_protection(
            "RECOVERY_SENS",
            Money::new(10_000_000.0, Currency::USD),
            100.0,
            as_of,
            end,
            "USD_OIS",
            "CORP",
        )
        .expect("CDS construction should succeed");
        cds.protection.recovery_rate = recovery;

        let protection_pv = cds
            .pv_protection_leg(
                market.get_discount("USD_OIS").unwrap().as_ref(),
                market.get_hazard("CORP").unwrap().as_ref(),
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
fn test_zero_spread_gives_positive_npv_for_buyer() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);

    let disc = build_discount_curve(0.05, as_of, "USD_OIS");
    let hazard = build_hazard_curve(0.015, 0.40, as_of, "CORP");

    let market = MarketContext::new()
        .insert_discount(disc)
        .insert_hazard(hazard);

    let cds = test_utils::cds_buy_protection(
        "ZERO_SPREAD",
        Money::new(10_000_000.0, Currency::USD),
        0.0, // Zero spread
        as_of,
        end,
        "USD_OIS",
        "CORP",
    )
    .expect("CDS construction should succeed");

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

    let cds = test_utils::cds_buy_protection(
        "ACCRUAL_TEST",
        Money::new(10_000_000.0, Currency::USD),
        100.0,
        as_of,
        end,
        "USD_OIS",
        "CORP",
    )
    .expect("CDS construction should succeed");

    let pricer_with_accrual = CDSPricer::new(); // Default includes accrual
    let pricer_without_accrual = CDSPricer::with_config(CDSPricerConfig {
        include_accrual: false,
        ..Default::default()
    });

    let pv_with = pricer_with_accrual
        .pv_premium_leg(
            &cds,
            market.get_discount("USD_OIS").unwrap().as_ref(),
            market.get_hazard("CORP").unwrap().as_ref(),
            as_of,
        )
        .unwrap()
        .amount();

    let pv_without = pricer_without_accrual
        .pv_premium_leg(
            &cds,
            market.get_discount("USD_OIS").unwrap().as_ref(),
            market.get_hazard("CORP").unwrap().as_ref(),
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

    let mut cds_no_delay = test_utils::cds_buy_protection(
        "NO_DELAY",
        Money::new(10_000_000.0, Currency::USD),
        100.0,
        as_of,
        end,
        "USD_OIS",
        "CORP",
    )
    .expect("CDS construction should succeed");
    cds_no_delay.protection.settlement_delay = 0;

    let mut cds_with_delay = cds_no_delay.clone();
    cds_with_delay.protection.settlement_delay = 20;

    let pricer = CDSPricer::new();

    let pv_no_delay = pricer
        .pv_protection_leg(
            &cds_no_delay,
            market.get_discount("USD_OIS").unwrap().as_ref(),
            market.get_hazard("CORP").unwrap().as_ref(),
            as_of,
        )
        .unwrap()
        .amount();

    let pv_with_delay = pricer
        .pv_protection_leg(
            &cds_with_delay,
            market.get_discount("USD_OIS").unwrap().as_ref(),
            market.get_hazard("CORP").unwrap().as_ref(),
            as_of,
        )
        .unwrap()
        .amount();

    assert!(
        pv_with_delay < pv_no_delay,
        "Settlement delay should reduce protection PV"
    );
}
