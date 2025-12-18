use finstack_core::dates::{Date, DayCount, Tenor};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::scalars::ScalarTimeSeries;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::math::interp::InterpStyle;
use finstack_core::prelude::{DateExt, Money};
use finstack_core::types::Currency;
use finstack_valuations::calibration::pricing::CalibrationPricer;
use finstack_valuations::calibration::quotes::{InstrumentConventions, RatesQuote};
use time::Month;

#[test]
fn test_calibration_pricer_enables_compounding_for_sofr_forward_mode() {
    let base = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    // We are calibrating a 3M forward curve from SOFR swaps (compounded)
    let pricer = CalibrationPricer::for_forward_curve(base, "USD-SOFR-3M", "USD-OIS", 0.25);

    let quote = RatesQuote::Swap {
        maturity: base.add_months(12),
        rate: 0.05,
        is_ois: false, // In forward-curve mode, it's not a single-curve OIS
        conventions: InstrumentConventions::default(),
        fixed_leg_conventions: InstrumentConventions::default()
            .with_payment_frequency(Tenor::annual())
            .with_day_count(DayCount::Thirty360),
        float_leg_conventions: InstrumentConventions::default()
            .with_payment_frequency(Tenor::quarterly())
            .with_day_count(DayCount::Act360)
            .with_index("USD-SOFR"), // RFR index
    };

    let irs = pricer
        .create_ois_swap(
            &quote,
            Money::new(1_000_000.0, Currency::USD),
            Currency::USD,
        )
        .expect("should build swap");

    // Verification: compounding must be enabled for SOFR index even in forward-curve mode
    use finstack_valuations::instruments::irs::FloatingLegCompounding;
    assert!(
        matches!(
            irs.float.compounding,
            FloatingLegCompounding::CompoundedInArrears { .. }
        ),
        "SOFR swap must use compounding for forward calibration"
    );
    assert_eq!(
        irs.float.forward_curve_id.as_str(),
        "USD-SOFR-3M",
        "Forward curve ID must match pricer target"
    );
    assert_eq!(
        irs.float.discount_curve_id.as_str(),
        "USD-OIS",
        "Discount curve ID must match pricer default"
    );
}

#[test]
fn test_compounded_swap_par_npv_invariant() {
    let base = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let disc = DiscountCurve::builder("USD-OIS")
        .base_date(base)
        .knots([(0.0, 1.0), (10.0, 0.6)]) // 5% flat approx
        .set_interp(InterpStyle::LogLinear)
        .build()
        .unwrap();

    // Provide minimal fixings for the lookback window in case the swap starts at/near `base`
    // and observation dates fall before as_of under the chosen compounding convention.
    let fixings: Vec<(Date, f64)> = (1..=10).map(|i| (base.add_weekdays(-i), 0.05)).collect();
    let ctx = MarketContext::new()
        .insert_discount(disc)
        .insert_series(ScalarTimeSeries::new("FIXING:USD-OIS", fixings, None).unwrap());
    let pricer = CalibrationPricer::new(base, "USD-OIS");

    let quote = RatesQuote::Swap {
        maturity: base.add_months(60), // 5Y
        rate: 0.048,                   // Arbitrary starting rate
        is_ois: true,
        // Use spot start (T+2) so SOFR lookback does not require historical fixings at as_of.
        conventions: InstrumentConventions::default()
            .with_calendar_id("usny")
            .with_settlement_days(2),
        fixed_leg_conventions: InstrumentConventions::default()
            .with_payment_frequency(Tenor::annual())
            .with_day_count(DayCount::Act360),
        float_leg_conventions: InstrumentConventions::default()
            .with_payment_frequency(Tenor::annual())
            .with_day_count(DayCount::Act360)
            .with_index("USD-SOFR"),
    };

    let mut irs = pricer
        .create_ois_swap(
            &quote,
            Money::new(10_000_000.0, Currency::USD),
            Currency::USD,
        )
        .expect("should build swap");

    // 1. Calculate par rate using our new metric
    use finstack_valuations::instruments::common::traits::Instrument;
    use finstack_valuations::metrics::MetricId;
    let results = irs
        .price_with_metrics(&ctx, base, &[MetricId::ParRate])
        .expect("should price metrics");

    let par_rate = results
        .measures
        .get(MetricId::ParRate.as_str())
        .copied()
        .expect("should have par rate metric");

    // 2. Set the fixed rate to the computed par rate
    irs.fixed.rate = par_rate;

    // 3. NPV must be zero (within tight tolerance of $1 per $10MM)
    let npv = irs.value(&ctx, base).expect("should calculate NPV");

    assert!(
        npv.amount().abs() < 1.0,
        "Compounded swap NPV at par {} must be < $1",
        npv.amount()
    );
}
