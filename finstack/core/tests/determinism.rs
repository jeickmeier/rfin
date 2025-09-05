use finstack_core::dates::Date;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::traits::Discount;
use finstack_core::math::{kahan_sum, pairwise_sum};
use time::Month;

#[test]
fn df_batch_matches_serial() {
    let yc = DiscountCurve::builder("USD-OIS")
        .base_date(Date::from_calendar_date(2025, Month::January, 1).unwrap())
        .knots([(0.0, 1.0), (1.0, 0.98), (2.0, 0.95)])
        .set_interp(finstack_core::market_data::interp::InterpStyle::LogLinear)
        .build()
        .unwrap();

    // Dense grid to exercise batch path
    let times: Vec<f64> = (0..=200).map(|i| i as f64 * 0.01).collect();
    let batch = yc.df_batch(&times);
    let serial: Vec<f64> = times.iter().copied().map(|t| yc.df(t)).collect();

    assert_eq!(batch.len(), serial.len());
    for (a, b) in batch.iter().zip(serial.iter()) {
        assert!((a - b).abs() < 1e-12);
    }
}

#[test]
fn sum_helpers_reasonable_accuracy() {
    // Construct a mix of positive/negative values with varying magnitudes
    let xs: Vec<f64> = (1..5000)
        .map(|i| {
            let x = i as f64 * 0.001;
            (x.sin() * 1e3) + (x.cos() * 1e-3) - (x.tan() * 1e-6)
        })
        .collect();

    let naive: f64 = xs.iter().copied().sum();
    let pairwise = pairwise_sum(&xs);
    let kahan = kahan_sum(xs.iter().copied());

    // All methods should be close; kahan/pairwise typically improve accuracy.
    let tol = 1e-9f64.max(naive.abs() * 1e-12);
    assert!((naive - pairwise).abs() < tol * 10.0);
    assert!((naive - kahan).abs() < tol * 10.0);
}
