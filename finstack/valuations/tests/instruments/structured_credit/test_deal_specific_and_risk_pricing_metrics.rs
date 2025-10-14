use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::money::Money;
use finstack_valuations::instruments::structured_credit::components::enums::{DealType, TrancheSeniority};
use finstack_valuations::instruments::structured_credit::components::pool::{AssetPool, PoolAsset};
use finstack_valuations::instruments::structured_credit::components::tranches::{Tranche, TrancheCoupon, TrancheStructure};
use finstack_valuations::instruments::structured_credit::{StructuredCredit, CreditFactors, BehaviorOverrides};
use finstack_valuations::metrics::MetricId;
use time::Month;

fn d(y: i32, m: u8, day: u8) -> Date {
    Date::from_calendar_date(y, Month::try_from(m).unwrap(), day).unwrap()
}

fn flat_disc(rate: f64, base: Date, id: &str) -> DiscountCurve {
    DiscountCurve::builder(id)
        .base_date(base)
        .knots(vec![(0.0, 1.0), (1.0, (-rate).exp()), (5.0, (-rate * 5.0).exp())])
        .build()
        .unwrap()
}

fn make_sc_abs(as_of: Date) -> (StructuredCredit, MarketContext) {
    let mut pool = AssetPool::new("ABS_POOL", DealType::ABS, Currency::USD);
    pool.assets.push(PoolAsset::fixed_rate_bond(
        "A1",
        Money::new(5_000_000.0, Currency::USD),
        0.06,
        d(2029, 1, 1),
    ));
    pool.assets.push(PoolAsset::fixed_rate_bond(
        "A2",
        Money::new(3_000_000.0, Currency::USD),
        0.05,
        d(2028, 1, 1),
    ));

    let senior = Tranche::new(
        "SENIOR",
        0.0,
        100.0,
        TrancheSeniority::Senior,
        Money::new(8_000_000.0, Currency::USD),
        TrancheCoupon::Fixed { rate: 0.03 },
        d(2030, 1, 1),
    )
    .unwrap();
    let tranches = TrancheStructure::new(vec![senior]).unwrap();

    let sc = StructuredCredit {
        id: "ABS-TEST".into(),
        deal_type: DealType::ABS,
        pool,
        tranches,
        behavior_overrides: StructuredCreditOverrides { abs_speed: Some(0.02), ..Default::default() },
        credit_factors: StructuredCreditCreditFactors::default(),
        attributes: Default::default(),
        currency: Currency::USD,
        discount_curve_id: "USD-OIS".into(),
    };
    let ctx = MarketContext::new().insert_discount(flat_disc(0.04, as_of, "USD-OIS"));
    (sc, ctx)
}

fn make_sc_cmbs(as_of: Date) -> (StructuredCredit, MarketContext) {
    let mut pool = AssetPool::new("CMBS_POOL", DealType::CMBS, Currency::USD);
    pool.assets.push(PoolAsset::fixed_rate_bond(
        "C1",
        Money::new(10_000_000.0, Currency::USD),
        0.055,
        d(2032, 1, 1),
    ));
    let senior = Tranche::new(
        "SENIOR",
        0.0,
        100.0,
        TrancheSeniority::Senior,
        Money::new(10_000_000.0, Currency::USD),
        TrancheCoupon::Fixed { rate: 0.04 },
        d(2035, 1, 1),
    )
    .unwrap();
    let tranches = TrancheStructure::new(vec![senior]).unwrap();
    let sc = StructuredCredit {
        id: "CMBS-TEST".into(),
        deal_type: DealType::CMBS,
        pool,
        tranches,
        behavior_overrides: StructuredCreditOverrides::default(),
        credit_factors: StructuredCreditCreditFactors { ltv: Some(0.60), ..Default::default() },
        attributes: Default::default(),
        currency: Currency::USD,
        discount_curve_id: "USD-OIS".into(),
    };
    let ctx = MarketContext::new().insert_discount(flat_disc(0.04, as_of, "USD-OIS"));
    (sc, ctx)
}

fn make_sc_rmbs(as_of: Date) -> (StructuredCredit, MarketContext) {
    let mut pool = AssetPool::new("RMBS_POOL", DealType::RMBS, Currency::USD);
    pool.assets.push(PoolAsset::fixed_rate_bond(
        "R1",
        Money::new(6_000_000.0, Currency::USD),
        0.045,
        d(2035, 1, 1),
    ));
    let senior = Tranche::new(
        "SENIOR",
        0.0,
        100.0,
        TrancheSeniority::Senior,
        Money::new(6_000_000.0, Currency::USD),
        TrancheCoupon::Fixed { rate: 0.035 },
        d(2037, 1, 1),
    )
    .unwrap();
    let tranches = TrancheStructure::new(vec![senior]).unwrap();
    let sc = StructuredCredit {
        id: "RMBS-TEST".into(),
        deal_type: DealType::RMBS,
        pool,
        tranches,
        behavior_overrides: StructuredCreditOverrides { psa_speed_multiplier: Some(1.0), ..Default::default() },
        credit_factors: StructuredCreditCreditFactors { ltv: Some(0.70), credit_score: Some(740), ..Default::default() },
        attributes: Default::default(),
        currency: Currency::USD,
        discount_curve_id: "USD-OIS".into(),
    };
    let ctx = MarketContext::new().insert_discount(flat_disc(0.04, as_of, "USD-OIS"));
    (sc, ctx)
}

#[test]
fn test_abs_deal_specific_metrics() {
    let as_of = d(2025, 1, 1);
    let (sc, ctx) = make_sc_abs(as_of);

    let result = sc
        .price_with_metrics(
            &ctx,
            as_of,
            &[
                MetricId::AbsSpeed,
                MetricId::AbsDelinquency,
                MetricId::AbsChargeOff,
                MetricId::AbsExcessSpread,
                MetricId::AbsCreditEnhancement,
            ],
        )
        .unwrap();

    assert!(result.measures["abs_speed"] > 0.0);
    assert!(result.measures["abs_delinquency"] >= 0.0);
    assert!(result.measures["abs_charge_off"] >= 0.0);
    assert!(result.measures["abs_excess_spread"].is_finite());
    assert!(result.measures["abs_credit_enhancement"] >= 0.0);
}

#[test]
fn test_cmbs_deal_specific_metrics() {
    let as_of = d(2025, 1, 1);
    let (sc, ctx) = make_sc_cmbs(as_of);

    let result = sc
        .price_with_metrics(&ctx, as_of, &[MetricId::CmbsLtv, MetricId::CmbsDscr])
        .unwrap();

    assert!(result.measures["cmbs_ltv"] > 0.0);
    assert!(result.measures["cmbs_dscr"] > 0.0);
}

#[test]
fn test_rmbs_deal_specific_metrics_and_wal() {
    let as_of = d(2025, 1, 1);
    let (sc, ctx) = make_sc_rmbs(as_of);

    let result = sc
        .price_with_metrics(
            &ctx,
            as_of,
            &[MetricId::RmbsLtv, MetricId::RmbsFico, MetricId::RmbsWal],
        )
        .unwrap();

    assert!(result.measures["rmbs_ltv"] > 0.0);
    assert!(result.measures["rmbs_fico"] >= 300.0);
    assert!(result.measures["rmbs_wal"] > 0.0);
}

#[test]
fn test_risk_pricing_metrics_chain() {
    let as_of = d(2025, 1, 1);
    let (sc, ctx) = make_sc_abs(as_of);

    // Request full chain: Accrued → Dirty → Clean → WAL → Macaulay/Modified → Z → CS01 → SpreadDuration → YTM
    let result = sc
        .price_with_metrics(
            &ctx,
            as_of,
            &[
                MetricId::Accrued,
                MetricId::DirtyPrice,
                MetricId::CleanPrice,
                MetricId::Wal,
                MetricId::MacaulayDuration,
                MetricId::ModifiedDuration,
                MetricId::ZSpread,
                MetricId::Cs01,
                MetricId::SpreadDuration,
                MetricId::Ytm,
            ],
        )
        .unwrap();

    // Basic sanity checks and relationships
    let accrued = result.measures["accrued"];
    let dirty = result.measures["dirty_price"];
    let clean = result.measures["clean_price"];
    assert!(dirty >= clean - 1e-9);
    assert!(accrued >= 0.0);

    let wal = result.measures["wal"];
    assert!(wal > 0.0);

    let mac_dur = result.measures["macaulay_duration"];
    let mod_dur = result.measures["modified_duration"];
    assert!(mac_dur >= 0.0 && mod_dur >= 0.0);

    let z = result.measures["z_spread"];
    let cs01 = result.measures["cs01"];
    let spr_dur = result.measures["spread_duration"];
    assert!(z.is_finite());
    assert!(cs01.is_finite());
    assert!(spr_dur.is_finite());
}


