#![cfg(any())]

mod cms_vanna_test {
    use finstack_core::dates::{DayCount, Frequency};
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::market_data::{
        surfaces::vol_surface::VolSurface,
        term_structures::{discount_curve::DiscountCurve, forward_curve::ForwardCurve},
    };
    use finstack_core::money::Money;
    use finstack_core::types::{Currency, CurveId, InstrumentId};
    use finstack_valuations::instruments::cms_option::types::CmsOption;
    use finstack_valuations::instruments::common::traits::Instrument;
    use finstack_valuations::metrics::MetricId;
    use time::macros::date;

    #[test]
    fn test_cms_option_vanna() -> finstack_core::Result<()> {
        let as_of = date!(2024 - 01 - 01);

        // 1. Create CMS Option
        // 5Y CMS Cap, 10Y tenor, Strike 3%
        let expiry = date!(2029 - 01 - 01);
        let payment_dates = vec![expiry]; // Simplified: single period
        let fixing_dates = vec![date!(2028 - 12 - 29)]; // 2 days before
        let accrual_fractions = vec![1.0];

        let cms = CmsOption {
            id: InstrumentId::new("CMS-Cap"),
            option_type: finstack_valuations::instruments::OptionType::Call,
            notional: Money::new(1_000_000.0, Currency::USD),
            strike_rate: 0.03,
            payment_dates,
            fixing_dates,
            accrual_fractions,
            cms_tenor: 10.0,
            swap_fixed_freq: Frequency::semi_annual(),
            swap_float_freq: Frequency::quarterly(),
            swap_day_count: DayCount::Thirty360,
            day_count: DayCount::Thirty360,
            discount_curve_id: CurveId::new("USD-OIS"),
            forward_curve_id: Some(CurveId::new("USD-LIBOR-3M")),
            vol_surface_id: Some(CurveId::new("USD-SWPN-VOL")),
            pricing_overrides: Default::default(),
            attributes: Default::default(),
        };

        // 2. Create Market
        // Flat 3% rates
        let discount_curve = DiscountCurve::builder("USD-OIS")
            .base_date(as_of)
            .day_count(DayCount::Act365F)
            .knots(vec![(0.0, 1.0), (100.0, (-0.03 * 100.0f64).exp())])
            .build()?;

        let forward_curve = ForwardCurve::builder("USD-LIBOR-3M", 0.25)
            .base_date(as_of)
            .day_count(DayCount::Act365F)
            .knots(vec![(0.0, 0.03), (100.0, 0.03)])
            .build()?;

        // Flat 20% volatility
        let vol_surface = VolSurface::builder("USD-SWPN-VOL")
            .expiries(&[0.0, 100.0])
            .strikes(&[0.0, 1.0])
            .row(&[0.20, 0.20])
            .row(&[0.20, 0.20])
            .build()?;

        let market = MarketContext::new()
            .insert_discount(discount_curve)
            .insert_forward(forward_curve)
            .insert_surface(vol_surface);

        // 3. Calculate Vanna
        // We expect Vanna to be non-zero
        let result = cms.price_with_metrics(&market, as_of, &[MetricId::Vanna])?;

        let vanna = result
            .measures
            .get(MetricId::Vanna.as_str())
            .copied()
            .unwrap_or(0.0);
        println!("Calculated Vanna: {}", vanna);

        // Vanna should be non-zero for OTM/ATM option
        assert!(vanna.abs() > 1e-6, "Vanna should be non-zero");

        Ok(())
    }
}
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency, StubKind};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::money::Money;
use finstack_core::types::{Currency, CurveId, InstrumentId};
use finstack_valuations::instruments::cms_option::types::CmsOption;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::metrics::MetricId;
use std::sync::Arc;
use time::macros::date;

#[test]
fn test_cms_option_vanna() -> finstack_core::Result<()> {
    let as_of = date!(2024 - 01 - 01);

    // 1. Create CMS Option
    // 5Y CMS Cap, 10Y tenor, Strike 3%
    let expiry = date!(2029 - 01 - 01);
    let payment_dates = vec![expiry]; // Simplified: single period
    let fixing_dates = vec![date!(2028 - 12 - 29)]; // 2 days before
    let accrual_fractions = vec![1.0];

    let cms = CmsOption {
        id: InstrumentId::new("CMS-Cap"),
        option_type: finstack_valuations::instruments::OptionType::Call,
        notional: Money::new(1_000_000.0, Currency::USD),
        strike_rate: 0.03,
        payment_dates,
        fixing_dates,
        accrual_fractions,
        cms_tenor: 10.0,
        swap_fixed_freq: Frequency::semi_annual(),
        swap_float_freq: Frequency::quarterly(),
        swap_day_count: DayCount::Thirty360,
        day_count: DayCount::Thirty360,
        discount_curve_id: CurveId::new("USD-OIS"),
        forward_curve_id: Some(CurveId::new("USD-LIBOR-3M")),
        vol_surface_id: Some(CurveId::new("USD-SWPN-VOL")),
        pricing_overrides: Default::default(),
        attributes: Default::default(),
    };

    // 2. Create Market
    // Flat 3% rates
    let discount_curve = DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .day_count(DayCount::Act365F)
        .knots(vec![(0.0, 1.0), (100.0, (-0.03 * 100.0f64).exp())])
        .build()?;

    use finstack_core::market_data::term_structures::forward_curve::ForwardCurve;
    let forward_curve = ForwardCurve::builder("USD-LIBOR-3M", 0.25)
        .base_date(as_of)
        .day_count(DayCount::Act365F)
        .knots(vec![(0.0, 0.03), (100.0, 0.03)])
        .build()?;

    // Flat 20% volatility
    use finstack_core::market_data::surfaces::vol_surface::VolSurface;
    let vol_surface = VolSurface::builder("USD-SWPN-VOL")
        .expiries(&[0.0, 100.0])
        .strikes(&[0.0, 1.0])
        .row(&[0.20, 0.20])
        .row(&[0.20, 0.20])
        .build()?;

    let market = MarketContext::new()
        .insert_discount(discount_curve)
        .insert_forward(forward_curve)
        .insert_surface(vol_surface);

    // 3. Calculate Vanna
    // We expect Vanna to be non-zero
    let result = cms.price_with_metrics(&market, as_of, &[MetricId::Vanna])?;

    let vanna = result
        .measures
        .get(MetricId::Vanna.as_str())
        .copied()
        .unwrap_or(0.0);
    println!("Calculated Vanna: {}", vanna);

    // Vanna should be non-zero for OTM/ATM option
    // Forward swap rate approx 3% (flat curve)
    // Strike 3% -> ATM
    // Vanna for ATM is usually small or zero?
    // Vanna = - exp(-rT) * N'(d1) * d2 / sigma
    // ATM: d1 = 0.5 * sigma * sqrt(T), d2 = -0.5 * sigma * sqrt(T)
    // Vanna ~ - exp(-rT) * N'(0) * (-0.5*sigma*sqrt(T)) / sigma
    // ~ exp(-rT) * N'(0) * 0.5 * sqrt(T) > 0
    // So it should be positive.

    assert!(vanna.abs() > 1e-6, "Vanna should be non-zero");

    Ok(())
}
