use finstack_core::math::{correlation, covariance, mean, mean_var, variance};
use finstack_core::math::stats::{log_returns, realized_variance, realized_variance_ohlc, RealizedVarMethod};

#[test]
fn mean_and_variance_basic() {
    let xs = [1.0, 2.0, 3.0, 4.0];
    let m = mean(&xs);
    let v = variance(&xs);
    let (m2, v2) = mean_var(&xs);
    assert!((m - 2.5).abs() < 1e-12);
    // Population variance of 1..4 is 1.25
    assert!((v - 1.25).abs() < 1e-12);
    assert!((m - m2).abs() < 1e-12);
    assert!((v - v2).abs() < 1e-12);
}

#[test]
fn covariance_and_correlation() {
    let x = [1.0, 2.0, 3.0, 4.0];
    let y = [2.0, 4.0, 6.0, 8.0];
    let cov = covariance(&x, &y);
    let corr = correlation(&x, &y);
    // Perfect linear relationship ⇒ correlation 1
    assert!((corr - 1.0).abs() < 1e-12);
    // Covariance should be positive and consistent with scaling
    assert!(cov > 0.0);
}

#[test]
fn log_returns_and_realized_variance_close_to_close() {
    let prices = [100.0, 102.0, 101.0, 105.0];
    let returns = log_returns(&prices);
    assert_eq!(returns.len(), prices.len() - 1);

    let rv = realized_variance(&prices, RealizedVarMethod::CloseToClose, 252.0);
    assert!(rv.is_finite() && rv >= 0.0);

    let rv_alt = realized_variance(&prices, RealizedVarMethod::Parkinson, 252.0);
    assert!(rv_alt.is_finite());
}

#[test]
fn realized_variance_ohlc_estimators_behave() {
    let open = [100.0, 101.0, 102.0];
    let high = [102.0, 103.0, 104.0];
    let low = [99.0, 100.0, 101.0];
    let close = [101.0, 102.0, 103.0];

    for method in [
        RealizedVarMethod::CloseToClose,
        RealizedVarMethod::Parkinson,
        RealizedVarMethod::GarmanKlass,
        RealizedVarMethod::RogersSatchell,
        RealizedVarMethod::YangZhang,
    ] {
        let value = realized_variance_ohlc(&open, &high, &low, &close, method, 252.0);
        assert!(value.is_finite() && value >= 0.0);
    }
}
