use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::money::Money;
use finstack_valuations::instruments::structured_credit::components::enums::TrancheSeniority;
use finstack_valuations::instruments::structured_credit::components::pool::{AssetPool, PoolAsset};
use finstack_valuations::instruments::structured_credit::components::tranches::{Tranche, TrancheCoupon, TrancheStructure};
use finstack_valuations::instruments::structured_credit::{DealType, StructuredCredit, CreditFactors, BehaviorOverrides};
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

#[test]
fn structured_credit_pricer_value_and_prices() {
    let as_of = d(2025, 1, 1);

    // Build a small ABS-like structure to exercise pricer paths
    let mut pool = AssetPool::new("POOL", DealType::ABS, Currency::USD);
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
        TrancheCoupon::Fixed { rate: 0.035 },
        d(2030, 1, 1),
    )
    .unwrap();
    let tranches = TrancheStructure::new(vec![senior]).unwrap();

    let sc = StructuredCredit {
        id: "SC-PRICER".into(),
        deal_type: DealType::ABS,
        pool,
        tranches,
        behavior_overrides: StructuredCreditOverrides::default(),
        credit_factors: StructuredCreditCreditFactors::default(),
        attributes: Default::default(),
        currency: Currency::USD,
        discount_curve_id: "USD-OIS".into(),
    };

    let ctx = MarketContext::new().insert_discount(flat_disc(0.04, as_of, "USD-OIS"));

    // Request key pricing, risk, and pool metrics to exercise metric calculators end-to-end
    let res = sc
        .price_with_metrics(
            &ctx,
            as_of,
            &[
                // Pricing
                MetricId::Accrued,
                MetricId::DirtyPrice,
                MetricId::CleanPrice,
                MetricId::WAL,
                // Risk
                MetricId::DurationMac,
                MetricId::DurationMod,
                MetricId::ZSpread,
                MetricId::Cs01,
                MetricId::SpreadDuration,
                // Pool metrics
                MetricId::WAM,
                MetricId::CPR,
                MetricId::CDR,
                // Additional
                MetricId::Dv01,
                MetricId::Theta,
            ],
        )
        .unwrap();

    // Basic sanity
    let dirty = res.measures["dirty_price"];
    let clean = res.measures["clean_price"];
    assert!(dirty >= clean - 1e-9);
    for key in [
        "accrued",
        "wal",
        "duration_mac",
        "duration_mod",
        "z_spread",
        "cs01",
        "spread_duration",
        "wam",
        "cpr",
        "cdr",
        "dv01",
        "theta",
    ] {
        assert!(res.measures.contains_key(key), "missing metric {key}");
        assert!(res.measures[key].is_finite(), "metric {key} not finite");
    }
}


