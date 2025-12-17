use criterion::{criterion_group, criterion_main, Criterion};
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::context::MarketContext;
use finstack_core::math::interp::{ExtrapolationPolicy, InterpStyle};
use finstack_core::types::{Currency, CurveId};
use finstack_valuations::calibration::adapters::discount::{
    DiscountCurveTarget, DiscountCurveTargetParams,
};
use finstack_valuations::calibration::domain::pricing::CalibrationPricer;
use finstack_valuations::calibration::domain::quotes::{InstrumentConventions, RatesQuote};
use finstack_valuations::calibration::domain::solver::{BootstrapTarget, GlobalSolveTarget};
use finstack_valuations::calibration::CalibrationConfig;
use time::Month;

fn bench_discount_v2_residuals(c: &mut Criterion) {
    let base_date = Date::from_calendar_date(2025, Month::January, 2).unwrap();

    let quotes = vec![
        RatesQuote::Deposit {
            maturity: base_date + time::Duration::days(30),
            rate: 0.045,
            conventions: InstrumentConventions::default().with_day_count(DayCount::Act360),
        },
        RatesQuote::Deposit {
            maturity: base_date + time::Duration::days(90),
            rate: 0.046,
            conventions: InstrumentConventions::default().with_day_count(DayCount::Act360),
        },
        RatesQuote::Swap {
            maturity: base_date + time::Duration::days(365),
            rate: 0.0475,
            is_ois: true,
            conventions: InstrumentConventions::default(),
            fixed_leg_conventions: InstrumentConventions::default()
                .with_day_count(DayCount::Thirty360),
            float_leg_conventions: InstrumentConventions::default()
                .with_day_count(DayCount::Act360)
                .with_index("USD-OIS"),
        },
    ];

    let pricer = CalibrationPricer::new(base_date, CurveId::from("USD-OIS"));
    let target = DiscountCurveTarget::new(DiscountCurveTargetParams {
        base_date,
        currency: Currency::USD,
        curve_id: CurveId::from("USD-OIS"),
        discount_curve_id: CurveId::from("USD-OIS"),
        forward_curve_id: CurveId::from("USD-OIS"),
        solve_interp: InterpStyle::Linear,
        extrapolation: ExtrapolationPolicy::FlatForward,
        config: CalibrationConfig::default(),
        pricer,
        curve_day_count: DayCount::Act365F,
        spot_knot: None,
        settlement_date: base_date,
        base_context: MarketContext::new(),
    });

    let knots = vec![(0.0, 1.0), (0.5, 0.98), (2.0, 0.9)];
    let curve = target.build_curve_for_solver(&knots).expect("temp curve");
    let mut residuals = vec![0.0; quotes.len()];

    c.bench_function("discount_v2_residuals_seq", |b| {
        b.iter(|| {
            target
                .calculate_residuals(&curve, &quotes, &mut residuals)
                .expect("residual calculation");
        });
    });
}

criterion_group!(benches, bench_discount_v2_residuals);
criterion_main!(benches);
