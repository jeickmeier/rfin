//! Benchmark-relative metrics: tracking error, information ratio, beta, greeks.
//!
//! Delegates to `math::stats` for core statistics (correlation, covariance,
//! variance, OnlineCovariance).

use crate::dates::Date;
use crate::math::stats::{correlation, covariance, mean, variance, OnlineCovariance, OnlineStats};

/// Align a benchmark return series to the target date grid via date lookup.
///
/// For each `target_date`, finds the matching benchmark return. Missing dates
/// are filled with 0.0 (no benchmark return).
pub fn align_benchmark(
    bench_returns: &[f64],
    bench_dates: &[Date],
    target_dates: &[Date],
) -> Vec<f64> {
    let n_bench = bench_returns.len().min(bench_dates.len());
    target_dates
        .iter()
        .map(|td| {
            bench_dates[..n_bench]
                .binary_search(td)
                .ok()
                .map(|i| bench_returns[i])
                .unwrap_or(0.0)
        })
        .collect()
}

/// Tracking error: annualized volatility of excess returns (portfolio − benchmark).
pub fn tracking_error(returns: &[f64], benchmark: &[f64], annualize: bool, ann_factor: f64) -> f64 {
    let n = returns.len().min(benchmark.len());
    if n == 0 {
        return 0.0;
    }
    let excess: Vec<f64> = (0..n).map(|i| returns[i] - benchmark[i]).collect();
    let te = variance(&excess).sqrt();
    if annualize {
        te * ann_factor.sqrt()
    } else {
        te
    }
}

/// Information ratio: annualized excess return / tracking error.
pub fn information_ratio(
    returns: &[f64],
    benchmark: &[f64],
    annualize: bool,
    ann_factor: f64,
) -> f64 {
    let n = returns.len().min(benchmark.len());
    if n == 0 {
        return 0.0;
    }
    let excess: Vec<f64> = (0..n).map(|i| returns[i] - benchmark[i]).collect();
    let er = mean(&excess);
    let te = variance(&excess).sqrt();
    if te == 0.0 {
        return 0.0;
    }
    if annualize {
        (er * ann_factor) / (te * ann_factor.sqrt())
    } else {
        er / te
    }
}

/// R-squared: `correlation(returns, benchmark)^2`.
pub fn r_squared(returns: &[f64], benchmark: &[f64]) -> f64 {
    let c = correlation(returns, benchmark);
    c * c
}

/// OLS beta result with optional standard error and confidence interval.
#[derive(Debug, Clone)]
pub struct BetaResult {
    /// Estimated beta coefficient.
    pub beta: f64,
    /// Standard error of the beta estimate.
    pub std_err: f64,
    /// Lower bound of the 95% confidence interval.
    pub ci_lower: f64,
    /// Upper bound of the 95% confidence interval.
    pub ci_upper: f64,
}

/// Compute beta = Cov(y, x) / Var(x) with standard error and 95% CI.
///
/// Uses `OnlineCovariance::optimal_beta()` for the core computation.
pub fn calc_beta(y: &[f64], x: &[f64]) -> BetaResult {
    let n = y.len().min(x.len());
    if n < 3 {
        return BetaResult {
            beta: 0.0,
            std_err: f64::NAN,
            ci_lower: f64::NAN,
            ci_upper: f64::NAN,
        };
    }
    let mut oc = OnlineCovariance::new();
    for i in 0..n {
        oc.update(y[i], x[i]);
    }
    let beta = oc.optimal_beta();

    let mut residual_stats = OnlineStats::new();
    let mean_x = oc.mean_y();
    let mean_y = oc.mean_x();
    let alpha = mean_y - beta * mean_x;
    for i in 0..n {
        let residual = y[i] - alpha - beta * x[i];
        residual_stats.update(residual);
    }

    let var_x = oc.variance_y();
    let resid_var = residual_stats.variance();
    let se = if var_x > 0.0 {
        (resid_var / ((n - 2) as f64 * var_x)).sqrt()
    } else {
        f64::NAN
    };

    BetaResult {
        beta,
        std_err: se,
        ci_lower: beta - 1.96 * se,
        ci_upper: beta + 1.96 * se,
    }
}

/// Greeks (alpha, beta, r_squared) from a single-factor regression.
#[derive(Debug, Clone)]
pub struct GreeksResult {
    /// Annualized alpha (intercept).
    pub alpha: f64,
    /// Beta (slope) of portfolio vs benchmark.
    pub beta: f64,
    /// R-squared of the regression.
    pub r_squared: f64,
}

/// Single-factor greeks for portfolio vs benchmark.
pub fn greeks(returns: &[f64], benchmark: &[f64], ann_factor: f64) -> GreeksResult {
    let n = returns.len().min(benchmark.len());
    if n == 0 {
        return GreeksResult {
            alpha: 0.0,
            beta: 0.0,
            r_squared: 0.0,
        };
    }
    let cov = covariance(&returns[..n], &benchmark[..n]);
    let var_b = variance(&benchmark[..n]);
    let beta = if var_b > 0.0 { cov / var_b } else { 0.0 };
    let alpha = (mean(&returns[..n]) - beta * mean(&benchmark[..n])) * ann_factor;
    let r2 = r_squared(&returns[..n], &benchmark[..n]);
    GreeksResult {
        alpha,
        beta,
        r_squared: r2,
    }
}

/// Rolling greeks output.
#[derive(Debug, Clone)]
pub struct RollingGreeks {
    /// End dates for each rolling window.
    pub dates: Vec<Date>,
    /// Rolling alpha values.
    pub alphas: Vec<f64>,
    /// Rolling beta values.
    pub betas: Vec<f64>,
}

/// Rolling single-factor greeks over a sliding window.
///
/// **FIX DEFECT #3**: implements correctly using rolling OnlineCovariance
/// rather than the buggy Python version with undefined variable `col`.
pub fn rolling_greeks(
    returns: &[f64],
    benchmark: &[f64],
    dates: &[Date],
    window: usize,
    ann_factor: f64,
) -> RollingGreeks {
    let n = returns.len().min(benchmark.len()).min(dates.len());
    if n < window || window == 0 {
        return RollingGreeks {
            dates: vec![],
            alphas: vec![],
            betas: vec![],
        };
    }
    let mut out_dates = Vec::with_capacity(n - window + 1);
    let mut alphas = Vec::with_capacity(n - window + 1);
    let mut betas = Vec::with_capacity(n - window + 1);

    for i in window..=n {
        let r_slice = &returns[i - window..i];
        let b_slice = &benchmark[i - window..i];
        let g = greeks(r_slice, b_slice, ann_factor);
        out_dates.push(dates[i - 1]);
        alphas.push(g.alpha);
        betas.push(g.beta);
    }

    RollingGreeks {
        dates: out_dates,
        alphas,
        betas,
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use time::Month;

    fn jan(day: u8) -> Date {
        Date::from_calendar_date(2025, Month::January, day).expect("valid date")
    }

    #[test]
    fn tracking_error_zero_when_identical() {
        let r = [0.01, 0.02, -0.01, 0.03];
        let te = tracking_error(&r, &r, false, 252.0);
        assert!(te.abs() < 1e-12);
    }

    #[test]
    fn information_ratio_basic() {
        let r = [0.02, 0.03, 0.01, 0.04];
        let b = [0.01, 0.01, 0.01, 0.01];
        let ir = information_ratio(&r, &b, false, 252.0);
        assert!(ir > 0.0);
    }

    #[test]
    fn r_squared_perfect_correlation() {
        let r = [1.0, 2.0, 3.0, 4.0];
        let b = [2.0, 4.0, 6.0, 8.0];
        let r2 = r_squared(&r, &b);
        assert!((r2 - 1.0).abs() < 1e-10);
    }

    #[test]
    fn calc_beta_basic() {
        let y = [0.02, 0.04, 0.06, 0.08, 0.10];
        let x = [0.01, 0.02, 0.03, 0.04, 0.05];
        let result = calc_beta(&y, &x);
        assert!((result.beta - 2.0).abs() < 1e-10);
    }

    #[test]
    fn greeks_basic() {
        let r = [0.01, 0.02, 0.03, 0.04, 0.05];
        let b = [0.005, 0.01, 0.015, 0.02, 0.025];
        let g = greeks(&r, &b, 252.0);
        assert!((g.beta - 2.0).abs() < 1e-10);
    }

    #[test]
    fn rolling_greeks_basic() {
        let r: Vec<f64> = (0..20).map(|i| (i as f64 + 1.0) * 0.001).collect();
        let b: Vec<f64> = (0..20).map(|i| i as f64 * 0.0005).collect();
        let dates: Vec<Date> = (1..=20).map(jan).collect();
        let rg = rolling_greeks(&r, &b, &dates, 5, 252.0);
        assert_eq!(rg.betas.len(), 16);
    }

    #[test]
    fn align_benchmark_basic() {
        let bd = vec![jan(1), jan(2), jan(3)];
        let br = vec![0.01, 0.02, 0.03];
        let td = vec![jan(1), jan(3), jan(5)];
        let aligned = align_benchmark(&br, &bd, &td);
        assert_eq!(aligned, vec![0.01, 0.03, 0.0]);
    }
}
