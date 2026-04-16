//! Orchestrators: single-model backtest, rolling VaR forecasts, and
//! multi-model comparison.

use super::tests::{christoffersen_test, classify_breaches, kupiec_test, traffic_light};
use super::types::{BacktestResult, MultiModelComparison, VarBacktestConfig, VarMethod};

/// Run a complete VaR backtest on a single forecast series.
///
/// Given paired VaR forecasts and realized P&L, classifies breaches
/// and runs all statistical tests (Kupiec, Christoffersen, Basel
/// traffic light).
///
/// # Arguments
///
/// * `var_forecasts` - Daily VaR forecasts (negative loss thresholds).
/// * `realized_pnl` - Daily realized P&L values.
/// * `config` - Backtest configuration (confidence, window size, significance).
///
/// # Returns
///
/// `BacktestResult` aggregating all test outcomes. Returns a result with
/// `NaN` statistics if inputs are empty or mismatched.
///
/// # Examples
///
/// ```rust
/// use finstack_analytics::backtesting::{run_backtest, VarBacktestConfig, TrafficLightZone};
///
/// // 250 days: VaR at -2% daily, realized P&L with 3 breaches
/// let var_forecasts = vec![-0.02; 250];
/// let mut realized = vec![-0.01; 250];  // mostly within VaR
/// realized[50] = -0.03;   // breach
/// realized[120] = -0.025; // breach
/// realized[200] = -0.04;  // breach
///
/// let config = VarBacktestConfig::new().with_confidence(0.99);
/// let result = run_backtest(&var_forecasts, &realized, &config);
///
/// assert_eq!(result.traffic_light.zone, TrafficLightZone::Green);
/// ```
#[must_use]
pub fn run_backtest(
    var_forecasts: &[f64],
    realized_pnl: &[f64],
    config: &VarBacktestConfig,
) -> BacktestResult {
    let breaches = classify_breaches(var_forecasts, realized_pnl);

    let kupiec = kupiec_test(&breaches, config.confidence);
    let christoffersen = christoffersen_test(&breaches, config.confidence);
    let tl = traffic_light(&breaches, config.confidence, config.window_size);

    BacktestResult {
        kupiec,
        christoffersen,
        traffic_light: tl,
        breaches,
        confidence: config.confidence,
    }
}

/// Rolling-window VaR backtester.
///
/// Computes VaR at each time step using a trailing window of returns,
/// then evaluates the VaR forecast against the next day's realized P&L.
/// This produces the paired (forecast, realized) series that feeds into
/// `run_backtest()`.
///
/// # Arguments
///
/// * `returns` - Full return series.
/// * `lookback` - Number of periods for VaR estimation window.
/// * `confidence` - VaR confidence level.
/// * `var_fn` - VaR estimation function: `fn(&[f64], f64) -> f64`.
///
/// # Returns
///
/// Tuple of (var_forecasts, realized_pnl) aligned so that
/// `var_forecasts[i]` is the VaR computed from `returns[i-lookback..i]`
/// and `realized_pnl[i]` is `returns[i]`.
#[must_use]
pub fn rolling_var_forecasts(
    returns: &[f64],
    lookback: usize,
    confidence: f64,
    var_fn: fn(&[f64], f64) -> f64,
) -> (Vec<f64>, Vec<f64>) {
    if returns.len() <= lookback {
        return (Vec::new(), Vec::new());
    }

    let n = returns.len() - lookback;
    let mut forecasts = Vec::with_capacity(n);
    let mut realized = Vec::with_capacity(n);

    for i in lookback..returns.len() {
        let window = &returns[i - lookback..i];
        forecasts.push(var_fn(window, confidence));
        realized.push(returns[i]);
    }

    (forecasts, realized)
}

/// Run the same realized P&L through multiple VaR model forecasts.
///
/// Useful for comparing historical, parametric, and Cornish-Fisher VaR
/// accuracy side-by-side.
///
/// # Arguments
///
/// * `models` - Slice of (method label, VaR forecast series) pairs.
/// * `realized_pnl` - Realized P&L (same for all models).
/// * `config` - Backtest configuration.
///
/// # Returns
///
/// `MultiModelComparison` containing one `BacktestResult` per model.
#[must_use]
pub fn compare_models(
    models: &[(VarMethod, &[f64])],
    realized_pnl: &[f64],
    config: &VarBacktestConfig,
) -> MultiModelComparison {
    let results = models
        .iter()
        .map(|(method, forecasts)| (*method, run_backtest(forecasts, realized_pnl, config)))
        .collect();
    MultiModelComparison { results }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod unit_tests {
    use super::*;
    use crate::backtesting::types::{Breach, TrafficLightZone};

    #[test]
    fn run_backtest_basic() {
        let var_forecasts = vec![-0.02; 250];
        let mut realized = vec![-0.01; 250];
        realized[50] = -0.03;
        realized[120] = -0.025;
        realized[200] = -0.04;

        let config = VarBacktestConfig::new().with_confidence(0.99);
        let result = run_backtest(&var_forecasts, &realized, &config);

        assert_eq!(result.kupiec.breach_count, 3);
        assert_eq!(result.traffic_light.zone, TrafficLightZone::Green);
        assert_eq!(result.breaches.len(), 250);
        assert!((result.confidence - 0.99).abs() < 1e-10);
    }

    #[test]
    fn run_backtest_empty() {
        let config = VarBacktestConfig::new();
        let result = run_backtest(&[], &[], &config);
        assert!(result.breaches.is_empty());
        assert!(result.kupiec.lr_statistic.is_nan());
    }

    #[test]
    fn run_backtest_mismatched_lengths() {
        let config = VarBacktestConfig::new();
        let result = run_backtest(&[-0.02; 10], &[-0.01; 5], &config);
        assert!(result.breaches.is_empty());
    }

    #[test]
    fn rolling_var_forecasts_basic() {
        // Simple VaR function: returns the minimum of the window
        fn simple_var(returns: &[f64], _confidence: f64) -> f64 {
            returns.iter().cloned().fold(f64::INFINITY, f64::min)
        }

        let returns = vec![
            -0.01, 0.02, -0.03, 0.01, -0.02, 0.03, -0.01, 0.02, -0.04, 0.01,
        ];
        let (forecasts, realized) = rolling_var_forecasts(&returns, 5, 0.99, simple_var);

        assert_eq!(forecasts.len(), 5);
        assert_eq!(realized.len(), 5);
        // First forecast uses returns[0..5], realized is returns[5]
        assert!((realized[0] - returns[5]).abs() < 1e-10);
    }

    #[test]
    fn rolling_var_forecasts_insufficient_data() {
        let returns = vec![-0.01; 5];
        let (forecasts, realized) = rolling_var_forecasts(&returns, 10, 0.99, |_, _| 0.0);
        assert!(forecasts.is_empty());
        assert!(realized.is_empty());
    }

    #[test]
    fn rolling_var_forecasts_exact_lookback() {
        let returns = vec![-0.01; 10];
        let (forecasts, realized) = rolling_var_forecasts(&returns, 10, 0.99, |_, _| 0.0);
        assert!(forecasts.is_empty());
        assert!(realized.is_empty());
    }

    #[test]
    fn rolling_var_with_historical_var() {
        use crate::risk_metrics::value_at_risk;

        // Wrapper matching the expected fn signature
        fn hist_var(returns: &[f64], confidence: f64) -> f64 {
            value_at_risk(returns, confidence, None)
        }

        // Generate a simple return series
        let n = 300;
        let returns: Vec<f64> = (0..n).map(|i| ((i as f64 * 7.3).sin()) * 0.02).collect();
        let lookback = 250;

        let (forecasts, realized) = rolling_var_forecasts(&returns, lookback, 0.99, hist_var);
        assert_eq!(forecasts.len(), n - lookback);
        assert_eq!(realized.len(), n - lookback);

        // All forecasts should be negative (VaR is a loss threshold)
        for f in &forecasts {
            assert!(*f < 0.0 || *f == 0.0, "VaR forecast should be non-positive");
        }
    }

    #[test]
    fn compare_models_basic() {
        let var_hist = vec![-0.02; 100];
        let var_param = vec![-0.015; 100];
        let mut realized = vec![-0.01; 100];
        realized[10] = -0.03;
        realized[50] = -0.025;

        let config = VarBacktestConfig::new().with_confidence(0.99).with_window_size(100);

        let models: Vec<(VarMethod, &[f64])> = vec![
            (VarMethod::Historical, &var_hist),
            (VarMethod::Parametric, &var_param),
        ];

        let comparison = compare_models(&models, &realized, &config);
        assert_eq!(comparison.results.len(), 2);
        assert_eq!(comparison.results[0].0, VarMethod::Historical);
        assert_eq!(comparison.results[1].0, VarMethod::Parametric);

        // Parametric VaR at -0.015 is tighter, so it should have more breaches
        let hist_breaches = comparison.results[0].1.kupiec.breach_count;
        let param_breaches = comparison.results[1].1.kupiec.breach_count;
        assert!(
            param_breaches >= hist_breaches,
            "Tighter VaR should have >= breaches"
        );
    }

    #[test]
    fn compare_models_three_methods() {
        let var_a = vec![-0.02; 50];
        let var_b = vec![-0.02; 50];
        let var_c = vec![-0.02; 50];
        let realized = vec![-0.01; 50];

        let config = VarBacktestConfig::new().with_window_size(50);

        let models: Vec<(VarMethod, &[f64])> = vec![
            (VarMethod::Historical, &var_a),
            (VarMethod::Parametric, &var_b),
            (VarMethod::CornishFisher, &var_c),
        ];

        let comparison = compare_models(&models, &realized, &config);
        assert_eq!(comparison.results.len(), 3);
    }

    #[test]
    fn run_backtest_red_zone() {
        // 250 observations with 15 breaches => Red zone
        let var_forecasts = vec![-0.02; 250];
        let mut realized = vec![-0.01; 250];
        for i in 0..15 {
            realized[i * 16] = -0.03;
        }

        let config = VarBacktestConfig::new().with_confidence(0.99);
        let result = run_backtest(&var_forecasts, &realized, &config);

        assert_eq!(result.kupiec.breach_count, 15);
        assert_eq!(result.traffic_light.zone, TrafficLightZone::Red);
        assert!((result.traffic_light.capital_multiplier - 4.0).abs() < 1e-10);
    }

    #[test]
    fn run_backtest_breach_vector_matches() {
        let var_forecasts = vec![-0.02; 10];
        let mut realized = vec![-0.01; 10];
        realized[3] = -0.03;
        realized[7] = -0.05;

        let config = VarBacktestConfig::new();
        let result = run_backtest(&var_forecasts, &realized, &config);

        assert_eq!(result.breaches.len(), 10);
        assert_eq!(result.breaches[3], Breach::Hit);
        assert_eq!(result.breaches[7], Breach::Hit);
        assert_eq!(result.breaches[0], Breach::Miss);
    }
}
