//! Benchmark-relative metrics: tracking error, information ratio, beta, greeks.
//!
//! Delegates to `math::stats` for core statistics (correlation, covariance,
//! variance, OnlineCovariance).

use crate::dates::Date;
use crate::math::stats::{correlation, covariance, mean, variance, OnlineCovariance, OnlineStats};

/// Align a benchmark return series to the target date grid via date lookup.
///
/// For each date in `target_dates`, binary-searches `bench_dates` for an
/// exact match and returns the corresponding benchmark return. Dates present
/// in `target_dates` but absent from `bench_dates` are filled with `0.0`
/// (treated as no benchmark return on that day).
///
/// # Arguments
///
/// * `bench_returns` - Benchmark return series. Length is truncated to
///   `min(bench_returns.len(), bench_dates.len())`.
/// * `bench_dates` - Sorted slice of dates for the benchmark series.
///   **Must be sorted ascending** — binary search is used.
/// * `target_dates` - The date grid to align to.
///
/// # Returns
///
/// A `Vec<f64>` of length `target_dates.len()` with aligned benchmark returns.
///
/// # Examples
///
/// ```rust
/// use finstack_core::analytics::benchmark::align_benchmark;
/// use time::{Date, Month};
///
/// let bd = vec![
///     Date::from_calendar_date(2025, Month::January, 1).unwrap(),
///     Date::from_calendar_date(2025, Month::January, 2).unwrap(),
///     Date::from_calendar_date(2025, Month::January, 3).unwrap(),
/// ];
/// let br = vec![0.01, 0.02, 0.03];
/// let td = vec![
///     Date::from_calendar_date(2025, Month::January, 1).unwrap(),
///     Date::from_calendar_date(2025, Month::January, 3).unwrap(),
///     Date::from_calendar_date(2025, Month::January, 5).unwrap(), // missing → 0.0
/// ];
/// let aligned = align_benchmark(&br, &bd, &td);
/// assert_eq!(aligned, vec![0.01, 0.03, 0.0]);
/// ```
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

/// Tracking error: annualized volatility of active (excess) returns.
///
/// Measures how consistently a portfolio follows its benchmark:
///
/// ```text
/// TE = σ(r_portfolio − r_benchmark) × sqrt(ann_factor)   [if annualized]
/// ```
///
/// A lower tracking error indicates tighter benchmark replication.
///
/// # Arguments
///
/// * `returns` - Portfolio return series.
/// * `benchmark` - Benchmark return series. Lengths are matched to the
///   shorter of the two.
/// * `annualize` - Whether to scale by `sqrt(ann_factor)`.
/// * `ann_factor` - Number of periods per year.
///
/// # Returns
///
/// Tracking error (non-negative). Returns `0.0` for empty or mismatched series.
///
/// # Examples
///
/// ```rust
/// use finstack_core::analytics::benchmark::tracking_error;
///
/// // Identical series → zero tracking error.
/// let r = [0.01, 0.02, -0.01, 0.03];
/// assert!(tracking_error(&r, &r, false, 252.0).abs() < 1e-12);
/// ```
///
/// # References
///
/// - Grinold & Kahn (1999): see docs/REFERENCES.md#grinoldKahn1999ActivePortfolio
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

/// Information ratio: annualized active return divided by tracking error.
///
/// Quantifies the consistency of alpha generation per unit of active risk:
///
/// ```text
/// IR = (mean active return × ann_factor) / (σ active return × sqrt(ann_factor))
///    = mean active return × sqrt(ann_factor) / σ active return
/// ```
///
/// A higher IR indicates more reliable outperformance relative to the
/// benchmark. The IR is related to the Sharpe ratio but uses active
/// (excess) returns rather than returns in excess of the risk-free rate.
///
/// # Arguments
///
/// * `returns`    - Portfolio return series.
/// * `benchmark`  - Benchmark return series.
/// * `annualize`  - Whether to annualize numerator and denominator.
/// * `ann_factor` - Number of periods per year.
///
/// # Returns
///
/// The Information Ratio. Returns `0.0` if tracking error is zero or the
/// series are empty.
///
/// # Examples
///
/// ```rust
/// use finstack_core::analytics::benchmark::information_ratio;
///
/// let r = [0.02, 0.03, 0.01, 0.04];
/// let b = [0.01, 0.01, 0.01, 0.01];
/// let ir = information_ratio(&r, &b, false, 252.0);
/// assert!(ir > 0.0);
/// ```
///
/// # References
///
/// - Grinold & Kahn (1999): see docs/REFERENCES.md#grinoldKahn1999ActivePortfolio
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

/// R-squared: proportion of portfolio variance explained by the benchmark.
///
/// Computed as the square of the Pearson correlation coefficient:
///
/// ```text
/// R² = corr(r_portfolio, r_benchmark)²
/// ```
///
/// A value of 1.0 means the portfolio moves perfectly in line with the
/// benchmark; 0.0 means the two are uncorrelated.
///
/// # Arguments
///
/// * `returns`   - Portfolio return series.
/// * `benchmark` - Benchmark return series.
///
/// # Returns
///
/// R-squared in `[0, 1]`. Returns `0.0` for empty or zero-variance series.
///
/// # Examples
///
/// ```rust
/// use finstack_core::analytics::benchmark::r_squared;
///
/// // Perfect linear relationship → R² = 1.
/// let r = [1.0, 2.0, 3.0, 4.0];
/// let b = [2.0, 4.0, 6.0, 8.0];
/// assert!((r_squared(&r, &b) - 1.0).abs() < 1e-10);
/// ```
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

/// OLS beta of portfolio vs benchmark, with standard error and 95% CI.
///
/// Estimates the slope of the single-factor linear regression
/// `r_portfolio = α + β × r_benchmark + ε` via:
///
/// ```text
/// β = Cov(r_portfolio, r_benchmark) / Var(r_benchmark)
/// ```
///
/// Standard error uses the OLS formula with `(n - 2)` degrees of freedom.
/// Confidence interval: `β ± 1.96 × SE(β)` (asymptotic 95% CI).
///
/// Requires at least 3 observations; returns `NaN` for standard error and
/// CI bounds when `n < 3`.
///
/// # Arguments
///
/// * `portfolio`  - Portfolio return series.
/// * `benchmark`  - Benchmark return series.
///
/// # Returns
///
/// A [`BetaResult`] with `beta`, `std_err`, `ci_lower`, and `ci_upper`.
/// All fields are `0.0` / `NAN` when the series are too short.
///
/// # Examples
///
/// ```rust
/// use finstack_core::analytics::benchmark::calc_beta;
///
/// // Portfolio returns are approximately 2× the benchmark with noise.
/// let port  = [0.020, 0.042, 0.058, 0.081, 0.099];
/// let bench = [0.010, 0.020, 0.030, 0.040, 0.050];
/// let result = calc_beta(&port, &bench);
/// assert!((result.beta - 2.0).abs() < 0.1);
/// assert!(result.ci_lower <= result.ci_upper);
/// assert!(result.std_err.is_finite());
/// ```
pub fn calc_beta(portfolio: &[f64], benchmark: &[f64]) -> BetaResult {
    let n = portfolio.len().min(benchmark.len());
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
        oc.update(portfolio[i], benchmark[i]);
    }
    let beta = oc.optimal_beta();

    // oc.mean_x() = mean(portfolio), oc.mean_y() = mean(benchmark)
    let mean_port = oc.mean_x();
    let mean_bench = oc.mean_y();
    let alpha = mean_port - beta * mean_bench;

    let mut residual_stats = OnlineStats::new();
    for i in 0..n {
        let residual = portfolio[i] - alpha - beta * benchmark[i];
        residual_stats.update(residual);
    }

    let var_bench = oc.variance_y();
    let resid_var = residual_stats.variance();
    let se = if var_bench > 0.0 {
        (resid_var / ((n - 2) as f64 * var_bench)).sqrt()
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
///
/// Runs a simple OLS regression `r_portfolio = α + β × r_benchmark` and
/// returns the annualized alpha, beta, and R² from that fit.
///
/// Unlike [`calc_beta`], this function does not compute standard errors
/// and is lighter-weight for scenarios where the point estimates are
/// sufficient (e.g., rolling window computations).
///
/// # Arguments
///
/// * `returns`    - Portfolio return series.
/// * `benchmark`  - Benchmark return series.
/// * `ann_factor` - Number of periods per year. Used to annualize alpha.
///
/// # Returns
///
/// A [`GreeksResult`] with `alpha` (annualized), `beta`, and `r_squared`.
/// Returns zeros for empty or zero-variance benchmark series.
///
/// # Examples
///
/// ```rust
/// use finstack_core::analytics::benchmark::greeks;
///
/// let r = [0.01, 0.02, 0.03, 0.04, 0.05];
/// let b = [0.005, 0.01, 0.015, 0.02, 0.025];
/// let g = greeks(&r, &b, 252.0);
/// assert!((g.beta - 2.0).abs() < 1e-10);
/// assert!((g.r_squared - 1.0).abs() < 1e-10);
/// ```
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

/// Rolling single-factor greeks (alpha, beta) over a sliding window.
///
/// Computes [`greeks`] independently for each `window`-length sub-slice,
/// advancing one period at a time. Produces `n - window + 1` values where
/// `n = min(returns.len(), benchmark.len(), dates.len())`.
///
/// # Arguments
///
/// * `returns` - Portfolio return series.
/// * `benchmark` - Benchmark return series.
/// * `dates` - Date vector aligned with `returns`. Used to label window
///   end dates in the output.
/// * `window` - Look-back window length in periods.
/// * `ann_factor` - Number of periods per year for alpha annualization.
///
/// # Returns
///
/// A [`RollingGreeks`] with `dates`, `alphas`, and `betas` of equal length.
/// Returns empty vectors if `window` is zero or exceeds the series length.
///
/// # Examples
///
/// ```rust
/// use finstack_core::analytics::benchmark::rolling_greeks;
/// use time::{Date, Month};
///
/// let r: Vec<f64> = (0..20).map(|i| (i as f64 + 1.0) * 0.001).collect();
/// let b: Vec<f64> = (0..20).map(|i| i as f64 * 0.0005).collect();
/// let dates: Vec<Date> = (1..=20)
///     .map(|d| Date::from_calendar_date(2025, Month::January, d).unwrap())
///     .collect();
/// let rg = rolling_greeks(&r, &b, &dates, 5, 252.0);
/// assert_eq!(rg.betas.len(), 16); // 20 − 5 + 1
/// ```
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
