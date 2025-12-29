//! Pricing tests for inflation caps/floors.

use crate::inflation_swap::fixtures::{flat_discount, flat_inflation_curve, simple_index};
use finstack_core::currency::Currency;
use finstack_core::dates::{
    BusinessDayConvention, Date, DayCount, DayCountCtx, StubKind, Tenor, TenorUnit,
};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::scalars::InflationLag;
use finstack_core::money::Money;
use finstack_core::types::CurveId;
use finstack_valuations::instruments::common::traits::Attributes;
use finstack_valuations::instruments::inflation_cap_floor::{
    InflationCapFloorBuilder, InflationCapFloorType,
};
use finstack_valuations::instruments::PricingOverrides;
use finstack_valuations::pricer::ModelKey;
use finstack_valuations::test_utils::flat_vol_surface;
use time::{Duration, Month};

#[test]
fn test_caplet_intrinsic_after_fixing() {
    let as_of = Date::from_calendar_date(2025, Month::April, 15).unwrap();
    let start = as_of - Duration::days(60);
    let end = as_of + Duration::days(30);

    let notional = Money::new(1_000_000.0, Currency::USD);
    let disc = flat_discount("USD-OIS", as_of, 0.02).unwrap();
    let infl_curve = flat_inflation_curve("US-CPI-U", 300.0, 0.02).unwrap();
    let index = simple_index(
        "US-CPI-U",
        as_of,
        300.0,
        Currency::USD,
        InflationLag::Months(3),
    );
    let vol_surface = flat_vol_surface("US-CPI-VOL", &[0.25], &[0.02], 0.20);

    let ctx = MarketContext::new()
        .insert_discount(disc)
        .insert_inflation(infl_curve)
        .insert_inflation_index("US-CPI-U", index)
        .insert_surface(vol_surface);

    let caplet = InflationCapFloorBuilder::new()
        .id("INF-CAPLET".into())
        .option_type(InflationCapFloorType::Caplet)
        .notional(notional)
        .strike_rate(0.02)
        .start_date(start)
        .end_date(end)
        .frequency(Tenor::new(3, TenorUnit::Months))
        .day_count(DayCount::Act365F)
        .stub_kind(StubKind::None)
        .bdc(BusinessDayConvention::Following)
        .calendar_id_opt(None)
        .inflation_index_id(CurveId::new("US-CPI-U"))
        .discount_curve_id(CurveId::new("USD-OIS"))
        .vol_surface_id(CurveId::new("US-CPI-VOL"))
        .pricing_overrides(PricingOverrides::default())
        .lag_override_opt(None)
        .attributes(Attributes::new())
        .build()
        .unwrap();

    let idx = ctx.inflation_index_ref("US-CPI-U").unwrap();
    let cpi_start = idx.value_on(start).unwrap();
    let cpi_end = idx.value_on(end).unwrap();
    let accrual = DayCount::Act365F
        .year_fraction(start, end, DayCountCtx::default())
        .unwrap();
    let forward_rate = (cpi_end / cpi_start - 1.0) / accrual;
    let payoff_rate = (forward_rate - 0.02).max(0.0);

    let disc_curve = ctx.get_discount_ref("USD-OIS").unwrap();
    let t_pay = disc_curve
        .day_count()
        .year_fraction(as_of, end, DayCountCtx::default())
        .unwrap();
    let df = disc_curve.df(t_pay);
    let expected = payoff_rate * accrual * notional.amount() * df;

    let pv = caplet
        .npv_with_model(&ctx, as_of, ModelKey::Normal)
        .unwrap();
    assert!((pv.amount() - expected).abs() < 1e-6 * notional.amount());
}

#[test]
fn test_floor_value_with_negative_forward_normal_model() {
    let as_of = Date::from_calendar_date(2025, Month::January, 2).unwrap();
    let start = as_of;
    let end = Date::from_calendar_date(2026, Month::January, 2).unwrap();

    let notional = Money::new(5_000_000.0, Currency::USD);
    let disc = flat_discount("USD-OIS", as_of, 0.01).unwrap();
    let infl_curve = flat_inflation_curve("US-CPI-U", 300.0, -0.01).unwrap();
    let vol_surface = flat_vol_surface("US-CPI-VOL", &[1.0], &[0.0], 0.01);

    let ctx = MarketContext::new()
        .insert_discount(disc)
        .insert_inflation(infl_curve)
        .insert_surface(vol_surface);

    let floorlet = InflationCapFloorBuilder::new()
        .id("INF-FLOOR".into())
        .option_type(InflationCapFloorType::Floorlet)
        .notional(notional)
        .strike_rate(0.0)
        .start_date(start)
        .end_date(end)
        .frequency(Tenor::new(1, TenorUnit::Years))
        .day_count(DayCount::Act365F)
        .stub_kind(StubKind::None)
        .bdc(BusinessDayConvention::Following)
        .calendar_id_opt(None)
        .inflation_index_id(CurveId::new("US-CPI-U"))
        .discount_curve_id(CurveId::new("USD-OIS"))
        .vol_surface_id(CurveId::new("US-CPI-VOL"))
        .pricing_overrides(PricingOverrides::default())
        .lag_override_opt(None)
        .attributes(Attributes::new())
        .build()
        .unwrap();

    let caplet = InflationCapFloorBuilder::new()
        .id("INF-CAP".into())
        .option_type(InflationCapFloorType::Caplet)
        .notional(notional)
        .strike_rate(0.0)
        .start_date(start)
        .end_date(end)
        .frequency(Tenor::new(1, TenorUnit::Years))
        .day_count(DayCount::Act365F)
        .stub_kind(StubKind::None)
        .bdc(BusinessDayConvention::Following)
        .calendar_id_opt(None)
        .inflation_index_id(CurveId::new("US-CPI-U"))
        .discount_curve_id(CurveId::new("USD-OIS"))
        .vol_surface_id(CurveId::new("US-CPI-VOL"))
        .pricing_overrides(PricingOverrides::default())
        .lag_override_opt(None)
        .attributes(Attributes::new())
        .build()
        .unwrap();

    let floor_pv = floorlet
        .npv_with_model(&ctx, as_of, ModelKey::Normal)
        .unwrap();
    let cap_pv = caplet
        .npv_with_model(&ctx, as_of, ModelKey::Normal)
        .unwrap();

    assert!(floor_pv.amount() > cap_pv.amount());
    assert!(floor_pv.amount() > 0.0);
}
