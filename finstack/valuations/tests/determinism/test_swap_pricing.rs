//! Determinism tests for interest rate swap pricing.
//!
//! Verifies that IRS valuation produces bitwise-identical results across
//! multiple runs with the same inputs.

use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::{DiscountCurve, ForwardCurve};
use finstack_core::math::interp::InterpStyle;
use finstack_core::money::Money;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::instruments::irs::{InterestRateSwap, PayReceive};
use finstack_valuations::metrics::MetricId;
use time::Month;

fn create_test_swap() -> InterestRateSwap {
    let start = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let end = Date::from_calendar_date(2030, Month::January, 15).unwrap();

    InterestRateSwap::new(
        "IRS-DETERMINISM-TEST".into(),
        Money::new(10_000_000.0, Currency::USD),
        0.04, // 4% fixed rate
        start,
        end,
        PayReceive::PayFixed,
    )
}

fn create_test_market(base_date: Date) -> MarketContext {
    let disc = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots([
            (0.0, 1.0),
            (1.0, 0.98),
            (2.0, 0.96),
            (5.0, 0.88),
            (10.0, 0.70),
        ])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();

    let fwd = ForwardCurve::builder("USD-SOFR-3M", 0.25)
        .base_date(base_date)
        .knots([
            (0.0, 0.035),
            (1.0, 0.038),
            (2.0, 0.040),
            (5.0, 0.045),
            (10.0, 0.050),
        ])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();

    MarketContext::new()
        .insert_discount(disc)
        .insert_forward(fwd)
}

#[test]
fn test_swap_pv_determinism() {
    let swap = create_test_swap();
    let as_of = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let market = create_test_market(as_of);

    // Price the swap 100 times
    let prices: Vec<f64> = (0..100)
        .map(|_| swap.value(&market, as_of).unwrap().amount())
        .collect();

    // All prices must be bitwise identical
    for i in 1..prices.len() {
        assert_eq!(
            prices[i], prices[0],
            "Swap PV at iteration {} = {:.15} differs from iteration 0 = {:.15}",
            i, prices[i], prices[0]
        );
    }
}

#[test]
fn test_swap_dv01_determinism() {
    let swap = create_test_swap();
    let as_of = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let market = create_test_market(as_of);

    // Calculate DV01 50 times
    let dv01s: Vec<f64> = (0..50)
        .map(|_| {
            let result = swap
                .price_with_metrics(&market, as_of, &[MetricId::Dv01])
                .unwrap();
            result.measures[MetricId::Dv01.as_str()]
        })
        .collect();

    // All DV01s must be identical
    for i in 1..dv01s.len() {
        assert_eq!(
            dv01s[i], dv01s[0],
            "DV01 differs at iteration {}: {:.15} vs {:.15}",
            i, dv01s[i], dv01s[0]
        );
    }
}

#[test]
fn test_swap_annuity_determinism() {
    let swap = create_test_swap();
    let as_of = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let market = create_test_market(as_of);

    // Calculate annuity 50 times
    let annuities: Vec<f64> = (0..50)
        .map(|_| {
            let result = swap
                .price_with_metrics(&market, as_of, &[MetricId::Annuity])
                .unwrap();
            result.measures[MetricId::Annuity.as_str()]
        })
        .collect();

    // All annuities must be identical
    for i in 1..annuities.len() {
        assert_eq!(
            annuities[i], annuities[0],
            "Annuity differs at iteration {}: {:.15} vs {:.15}",
            i, annuities[i], annuities[0]
        );
    }
}

#[test]
fn test_swap_par_rate_determinism() {
    let swap = create_test_swap();
    let as_of = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let market = create_test_market(as_of);

    // Calculate par rate 50 times
    let par_rates: Vec<f64> = (0..50)
        .map(|_| {
            let result = swap
                .price_with_metrics(&market, as_of, &[MetricId::ParRate])
                .unwrap();
            result.measures[MetricId::ParRate.as_str()]
        })
        .collect();

    // All par rates must be identical
    for i in 1..par_rates.len() {
        assert_eq!(
            par_rates[i], par_rates[0],
            "Par rate differs at iteration {}: {:.15} vs {:.15}",
            i, par_rates[i], par_rates[0]
        );
    }
}

#[test]
fn test_swap_multiple_metrics_determinism() {
    let swap = create_test_swap();
    let as_of = Date::from_calendar_date(2025, Month::February, 1).unwrap();
    let market = create_test_market(as_of);

    let metrics = vec![
        MetricId::Dv01,
        MetricId::Annuity,
        MetricId::ParRate,
        MetricId::PvFixed,
        MetricId::PvFloat,
    ];

    // Calculate all metrics 30 times
    let results: Vec<_> = (0..30)
        .map(|_| swap.price_with_metrics(&market, as_of, &metrics).unwrap())
        .collect();

    // Verify each metric is deterministic
    for metric in &metrics {
        let values: Vec<f64> = results
            .iter()
            .map(|r| r.measures[metric.as_str()])
            .collect();

        for i in 1..values.len() {
            assert_eq!(
                values[i],
                values[0],
                "{} differs at iteration {}: {:.15} vs {:.15}",
                metric.as_str(),
                i,
                values[i],
                values[0]
            );
        }
    }
}

#[test]
fn test_swap_pay_vs_receive_determinism() {
    let as_of = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let market = create_test_market(as_of);

    // Create both pay-fixed and receive-fixed swaps
    let start = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let end = Date::from_calendar_date(2030, Month::January, 15).unwrap();

    let swap_pay = InterestRateSwap::new(
        "PAY-FIXED".into(),
        Money::new(10_000_000.0, Currency::USD),
        0.04,
        start,
        end,
        PayReceive::PayFixed,
    );

    let swap_rec = InterestRateSwap::new(
        "RECEIVE-FIXED".into(),
        Money::new(10_000_000.0, Currency::USD),
        0.04,
        start,
        end,
        PayReceive::ReceiveFixed,
    );

    // Price each swap 30 times
    let pay_prices: Vec<f64> = (0..30)
        .map(|_| swap_pay.value(&market, as_of).unwrap().amount())
        .collect();

    let rec_prices: Vec<f64> = (0..30)
        .map(|_| swap_rec.value(&market, as_of).unwrap().amount())
        .collect();

    // Verify determinism for each side
    for i in 1..pay_prices.len() {
        assert_eq!(pay_prices[i], pay_prices[0]);
        assert_eq!(rec_prices[i], rec_prices[0]);
    }

    // Verify symmetry: pay + receive ≈ 0 (and this relationship is deterministic)
    let sym_sum: Vec<f64> = pay_prices
        .iter()
        .zip(rec_prices.iter())
        .map(|(p, r)| p + r)
        .collect();

    for i in 1..sym_sum.len() {
        assert_eq!(
            sym_sum[i], sym_sum[0],
            "Symmetry sum differs at iteration {}: {:.15} vs {:.15}",
            i, sym_sum[i], sym_sum[0]
        );
    }
}
