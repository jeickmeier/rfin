//! Integration tests for OAS stability with amortizing and PIK cashflows.

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount, Frequency};
use finstack_core::market_data::context::MarketContext;
use finstack_core::math::interp::InterpStyle;
use finstack_core::money::Money;
use finstack_core::F;
use finstack_valuations::cashflow::builder::{cf, CouponType, FixedCouponSpec, ScheduleParams};
use finstack_core::cashflow::primitives::AmortizationSpec;
use finstack_valuations::instruments::bond::pricing::tree_pricer::TreePricer;
use finstack_valuations::instruments::bond::Bond;
use finstack_valuations::instruments::PricingOverrides;
use time::Month;

fn create_test_curve() -> DiscountCurve {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots([
            (0.0, 1.0),
            (0.5, 0.975),
            (1.0, 0.950),
            (2.0, 0.905),
            (5.0, 0.80),
            (10.0, 0.60),
        ])
        .set_interp(InterpStyle::LogLinear)
        .build()
        .unwrap()
}

fn create_market_context() -> MarketContext {
    MarketContext::new().insert_discount(create_test_curve())
}

#[test]
fn test_oas_stability_amortizing_bond() {
    let issue = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let maturity = Date::from_calendar_date(2030, Month::January, 1).unwrap();
    let curves = create_market_context();

    let bond = Bond::builder()
        .id("AMORT_BOND".into())
        .notional(Money::new(1_000.0, Currency::USD))
        .coupon(0.06)
        .freq(Frequency::semi_annual())
        .dc(DayCount::Act365F)
        .issue(issue)
        .maturity(maturity)
        .disc_id("USD-OIS".into())
        .pricing_overrides(PricingOverrides::default().with_clean_price(99.0))
        .call_put_opt(None)
        .amortization_opt(Some(AmortizationSpec::LinearTo {
            final_notional: Money::new(400.0, Currency::USD),
        }))
        .custom_cashflows_opt(None)
        .attributes(finstack_valuations::instruments::common::traits::Attributes::new())
        .build()
        .unwrap();

    let as_ofs = [
        Date::from_calendar_date(2025, Month::January, 1).unwrap(),
        Date::from_calendar_date(2026, Month::January, 1).unwrap(),
    ];
    let steps = [25, 50, 100];

    for as_of in &as_ofs {
        let mut values: Vec<F> = Vec::new();
        for &n in &steps {
            let calc = TreePricer::with_config(finstack_valuations::instruments::bond::pricing::tree_pricer::TreePricerConfig {
                tree_steps: n,
                volatility: 0.01,
                tolerance: 1e-6,
                max_iterations: 50,
                ..Default::default()
            });
            let oas = calc.calculate_oas(&bond, &curves, *as_of, 99.0).unwrap();
            assert!(oas.is_finite());
            values.push(oas);
        }
        // Stability across tree steps: max-min within a reasonable band
        let min = values
            .iter()
            .copied()
            .fold(F::INFINITY, |a, b| a.min(b));
        let max = values
            .iter()
            .copied()
            .fold(F::NEG_INFINITY, |a, b| a.max(b));
        assert!(
            (max - min).abs() < 250.0,
            "Amortizing OAS unstable across steps: min={:.2} bp max={:.2} bp as_of={:?}",
            min,
            max,
            as_of
        );
    }
}

#[test]
fn test_oas_stability_pik_bond() {
    let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2027, Month::January, 15).unwrap();
    let curves = create_market_context();

    // Build custom cashflows: step-up coupons with a PIK window in year 1
    let sched = ScheduleParams {
        freq: Frequency::quarterly(),
        dc: DayCount::Act365F,
        bdc: finstack_core::dates::BusinessDayConvention::Following,
        calendar_id: None,
        stub: finstack_core::dates::StubKind::None,
    };

    let start_pik = Date::from_calendar_date(2025, Month::July, 15).unwrap();
    let end_pik = Date::from_calendar_date(2026, Month::July, 15).unwrap();

    let mut builder = cf();
    builder
        .principal(Money::new(1_000.0, Currency::USD), issue, maturity)
        .fixed_stepup(&[(maturity, 0.08)], sched, CouponType::Cash)
        .add_payment_window(start_pik, end_pik, CouponType::PIK);
    let schedule = builder.build().expect("schedule");

    // Create bond from custom cashflows with quoted clean price
    let bond = Bond::from_cashflows("PIK_BOND", schedule, "USD-OIS", Some(98.0)).unwrap();

    let as_of = issue;
    let steps = [25, 50, 100];
    let mut oas_values: Vec<F> = Vec::new();
    for &n in &steps {
        let calc = TreePricer::with_config(finstack_valuations::instruments::bond::pricing::tree_pricer::TreePricerConfig {
            tree_steps: n,
            volatility: 0.01,
            tolerance: 1e-6,
            max_iterations: 50,
            ..Default::default()
        });
        let oas = calc.calculate_oas(&bond, &curves, as_of, 98.0).unwrap();
        assert!(oas.is_finite());
        oas_values.push(oas);
    }

    // Stability across tree steps
    let min = oas_values.iter().copied().fold(F::INFINITY, |a, b| a.min(b));
    let max = oas_values
        .iter()
        .copied()
        .fold(F::NEG_INFINITY, |a, b| a.max(b));
    assert!(
        (max - min).abs() < 300.0,
        "PIK OAS unstable across steps: min={:.2} bp max={:.2} bp",
        min,
        max
    );
}

