//! Benchmark-relative metrics: tracking error, information ratio, beta, greeks.
//!
//! Delegates to `math::stats` for core statistics (correlation, covariance,
//! variance, OnlineCovariance).

use crate::dates::Date;
use crate::math::stats::{correlation, mean, OnlineCovariance, OnlineStats};
use finstack_core::math::neumaier_sum;

const ROLLING_GREEKS_RECOMPUTE_INTERVAL: usize = 64;

#[inline]
fn recompute_rolling_greeks_sums(returns: &[f64], benchmark: &[f64]) -> (f64, f64, f64, f64) {
    (
        neumaier_sum(returns.iter().copied()),
        neumaier_sum(benchmark.iter().copied()),
        neumaier_sum(returns.iter().zip(benchmark.iter()).map(|(&r, &b)| r * b)),
        neumaier_sum(benchmark.iter().map(|&b| b * b)),
    )
}

#[inline]
fn compensated_add(sum: &mut f64, compensation: &mut f64, value: f64) {
    let y = value - *compensation;
    let t = *sum + y;
    *compensation = (t - *sum) - y;
    *sum = t;
}

/// Policy for handling benchmark dates that are missing from the target grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BenchmarkAlignmentPolicy {
    /// Preserve legacy behavior by synthesizing a zero return for missing dates.
    ZeroReturnOnMissingDates,
    /// Reject missing benchmark dates explicitly.
    ErrorOnMissingDates,
}

/// Align a benchmark return series to the target date grid via date lookup.
///
/// For each date in `target_dates`, binary-searches `bench_dates` for an
/// exact match and returns the corresponding benchmark return. Dates present
/// in `target_dates` but absent from `bench_dates` are filled with `0.0`.
///
/// The zero-fill is correct because this function operates in **return
/// space**: a missing date means no trading occurred for the benchmark on
/// that day, so the return is 0.0 (no change in value). For price/index
/// data, use fill-forward instead before converting to returns.
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
/// use finstack_analytics::benchmark::align_benchmark;
/// use finstack_core::dates::{Date, Month};
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
    align_benchmark_with_policy(
        bench_returns,
        bench_dates,
        target_dates,
        BenchmarkAlignmentPolicy::ZeroReturnOnMissingDates,
    )
    .unwrap_or_else(|_| vec![0.0; target_dates.len()])
}

/// Align a benchmark return series to the target date grid using an explicit policy.
///
/// This is the fallible variant of [`align_benchmark`]. It performs the same
/// exact-date lookup, but lets the caller decide whether missing benchmark
/// dates should be synthesized as `0.0` returns or rejected as invalid input.
///
/// The function operates in return space, not price space: `0.0` means "no
/// return observed on that target date", not "price unchanged after fill-forward
/// interpolation".
///
/// # Arguments
///
/// * `bench_returns` - Benchmark simple-return series in decimal form. The
///   usable sample is truncated to `min(bench_returns.len(), bench_dates.len())`.
/// * `bench_dates` - Sorted benchmark dates corresponding to `bench_returns`.
///   Dates must be strictly ascending for binary search to behave correctly.
/// * `target_dates` - Sorted target date grid to align onto.
/// * `policy` - Behavior when a target date is absent from the benchmark grid.
///
/// # Returns
///
/// A `Vec<f64>` of length `target_dates.len()` containing benchmark returns
/// aligned to the target grid.
///
/// # Errors
///
/// Returns an error when `policy` is
/// [`BenchmarkAlignmentPolicy::ErrorOnMissingDates`] and any `target_dates`
/// entry is absent from `bench_dates`.
///
/// # Examples
///
/// ```rust
/// use finstack_analytics::benchmark::{
///     align_benchmark_with_policy, BenchmarkAlignmentPolicy,
/// };
/// use finstack_core::dates::{Date, Month};
///
/// let bench_dates = vec![
///     Date::from_calendar_date(2025, Month::January, 1).unwrap(),
///     Date::from_calendar_date(2025, Month::January, 3).unwrap(),
/// ];
/// let bench_returns = vec![0.01, 0.03];
/// let target_dates = vec![
///     Date::from_calendar_date(2025, Month::January, 1).unwrap(),
///     Date::from_calendar_date(2025, Month::January, 2).unwrap(),
/// ];
///
/// let err = align_benchmark_with_policy(
///     &bench_returns,
///     &bench_dates,
///     &target_dates,
///     BenchmarkAlignmentPolicy::ErrorOnMissingDates,
/// );
/// assert!(err.is_err());
/// ```
pub fn align_benchmark_with_policy(
    bench_returns: &[f64],
    bench_dates: &[Date],
    target_dates: &[Date],
    policy: BenchmarkAlignmentPolicy,
) -> crate::Result<Vec<f64>> {
    let n_bench = bench_returns.len().min(bench_dates.len());
    let mut aligned = Vec::with_capacity(target_dates.len());
    for &target_date in target_dates {
        match bench_dates[..n_bench].binary_search(&target_date) {
            Ok(index) => aligned.push(bench_returns[index]),
            Err(_) => match policy {
                BenchmarkAlignmentPolicy::ZeroReturnOnMissingDates => aligned.push(0.0),
                BenchmarkAlignmentPolicy::ErrorOnMissingDates => {
                    return Err(crate::error::InputError::Invalid.into());
                }
            },
        }
    }
    Ok(aligned)
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
/// When `annualize` is `true`, returns [`f64::NAN`] if `ann_factor` is not finite
/// or is `<= 0`.
///
/// # Examples
///
/// ```rust
/// use finstack_analytics::benchmark::tracking_error;
///
/// // Identical series → zero tracking error.
/// let r = [0.01, 0.02, -0.01, 0.03];
/// assert!(tracking_error(&r, &r, false, 252.0).abs() < 1e-12);
/// ```
///
/// # References
///
/// - Grinold & Kahn (1999): see docs/REFERENCES.md#grinoldKahn1999ActivePortfolio
#[must_use]
pub fn tracking_error(returns: &[f64], benchmark: &[f64], annualize: bool, ann_factor: f64) -> f64 {
    let n = returns.len().min(benchmark.len());
    if n == 0 {
        return 0.0;
    }
    if annualize && (!ann_factor.is_finite() || ann_factor <= 0.0) {
        return f64::NAN;
    }
    let mut os = OnlineStats::new();
    for i in 0..n {
        os.update(returns[i] - benchmark[i]);
    }
    let te = os.std_dev();
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
/// series are empty. When `annualize` is `true`, returns [`f64::NAN`] if
/// `ann_factor` is not finite or is `<= 0`.
///
/// # Examples
///
/// ```rust
/// use finstack_analytics::benchmark::information_ratio;
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
#[must_use]
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
    if annualize && (!ann_factor.is_finite() || ann_factor <= 0.0) {
        return f64::NAN;
    }
    let mut os = OnlineStats::new();
    for i in 0..n {
        os.update(returns[i] - benchmark[i]);
    }
    let er = os.mean();
    let te = os.std_dev();
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
/// use finstack_analytics::benchmark::r_squared;
///
/// // Perfect linear relationship → R² = 1.
/// let r = [1.0, 2.0, 3.0, 4.0];
/// let b = [2.0, 4.0, 6.0, 8.0];
/// assert!((r_squared(&r, &b) - 1.0).abs() < 1e-10);
/// ```
#[must_use]
pub fn r_squared(returns: &[f64], benchmark: &[f64]) -> f64 {
    let c = correlation(returns, benchmark);
    c * c
}

/// OLS beta result with optional standard error and confidence interval.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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

// Two-sided 95% critical value: Student's t with `n−2` degrees of freedom for small
// samples, and 1.96 when `n − 2 ≥ 38` (normal approximation).
fn beta_ci_critical_value(sample_size: usize) -> f64 {
    match sample_size.saturating_sub(2) {
        0 => f64::NAN,
        1 => 12.706_204_736_432_095,
        2 => 4.302_652_729_696_142,
        3 => 3.182_446_305_284_263,
        4 => 2.776_445_105_197_798_7,
        5 => 2.570_581_835_636_305,
        6 => 2.446_911_851_144_969_2,
        7 => 2.364_624_251_592_784_4,
        8 => 2.306_004_135_204_166,
        9 => 2.262_157_162_854_099_3,
        10 => 2.228_138_851_964_938_5,
        11 => 2.200_985_160_082_949,
        12 => 2.178_812_829_663_417_7,
        13 => 2.160_368_656_461_013,
        14 => 2.144_786_687_916_927_7,
        15 => 2.131_449_545_559_323,
        16 => 2.119_905_299_221_011_2,
        17 => 2.109_815_577_833_180_6,
        18 => 2.100_922_040_240_96,
        19 => 2.093_024_054_408_263,
        20 => 2.085_963_447_265_837,
        21 => 2.079_613_844_727_662,
        22 => 2.073_873_067_904_015,
        23 => 2.068_657_610_419_041,
        24 => 2.063_898_561_628_021,
        25 => 2.059_538_552_753_294,
        26 => 2.055_529_438_642_872,
        27 => 2.051_830_516_480_283_3,
        28 => 2.048_407_141_795_244,
        29 => 2.045_229_642_132_703,
        30 => 2.042_272_456_301_238,
        31 => 2.039_513_446_396_408_5,
        32 => 2.036_933_343_460_101_6,
        33 => 2.034_515_297_449_338_3,
        34 => 2.032_244_509_317_719,
        35 => 2.030_107_928_250_343,
        36 => 2.028_094_000_980_451,
        37 => 2.026_192_463_029_109_3,
        _ => 1.96,
    }
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
///
/// The **95% two-sided** interval uses `β ± t_{n−2, 0.975} × SE(β)`, where
/// `t_{n−2, 0.975}` is the **Student's t** critical value for `n − 2` degrees
/// of freedom (implemented as a small fixed table for `n − 2 ≤ 37`). For
/// `n − 2 ≥ 38` (i.e. `n ≥ 40`), the **normal** critical value **1.96**
/// (two-sided 95%) is used as the usual asymptotic approximation to the
/// t distribution.
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
/// All fields are `NAN` when the series are too short (`n < 3`).
///
/// # Examples
///
/// ```rust
/// use finstack_analytics::benchmark::calc_beta;
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
            beta: f64::NAN,
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

    let critical_value = beta_ci_critical_value(n);
    BetaResult {
        beta,
        std_err: se,
        ci_lower: beta - critical_value * se,
        ci_upper: beta + critical_value * se,
    }
}

/// Greeks (alpha, beta, r_squared) from a single-factor regression.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
/// use finstack_analytics::benchmark::greeks;
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
    let mut oc = OnlineCovariance::new();
    for i in 0..n {
        oc.update(returns[i], benchmark[i]);
    }
    let beta = oc.optimal_beta();
    let alpha = (oc.mean_x() - beta * oc.mean_y()) * ann_factor;
    let c = oc.correlation();
    GreeksResult {
        alpha,
        beta,
        r_squared: c * c,
    }
}

/// Rolling greeks output.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
/// use finstack_analytics::benchmark::rolling_greeks;
/// use finstack_core::dates::{Date, Month};
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
    let count = n - window + 1;
    let mut out_dates = Vec::with_capacity(count);
    let mut alphas = Vec::with_capacity(count);
    let mut betas = Vec::with_capacity(count);

    // Incremental O(n) sliding-window OLS via running sums.
    let w = window as f64;
    let (mut sr, mut sb, mut srb, mut sb2) =
        recompute_rolling_greeks_sums(&returns[..window], &benchmark[..window]);
    let (mut csr, mut csb, mut csrb, mut csb2) = (0.0, 0.0, 0.0, 0.0);
    let mut steps_since_recompute = 0usize;

    for i in window..=n {
        let denom = w * sb2 - sb * sb;
        let beta = if denom.abs() < 1e-30 {
            0.0
        } else {
            (w * srb - sb * sr) / denom
        };
        let alpha = (sr / w - beta * sb / w) * ann_factor;
        out_dates.push(dates[i - 1]);
        alphas.push(alpha);
        betas.push(beta);

        if i < n {
            let old_r = returns[i - window];
            let old_b = benchmark[i - window];
            let new_r = returns[i];
            let new_b = benchmark[i];
            compensated_add(&mut sr, &mut csr, new_r - old_r);
            compensated_add(&mut sb, &mut csb, new_b - old_b);
            compensated_add(&mut srb, &mut csrb, new_r * new_b - old_r * old_b);
            compensated_add(&mut sb2, &mut csb2, new_b * new_b - old_b * old_b);
            steps_since_recompute += 1;
            if steps_since_recompute >= ROLLING_GREEKS_RECOMPUTE_INTERVAL {
                let start = i + 1 - window;
                (sr, sb, srb, sb2) =
                    recompute_rolling_greeks_sums(&returns[start..=i], &benchmark[start..=i]);
                (csr, csb, csrb, csb2) = (0.0, 0.0, 0.0, 0.0);
                steps_since_recompute = 0;
            }
        }
    }

    RollingGreeks {
        dates: out_dates,
        alphas,
        betas,
    }
}

/// Up-market capture ratio: portfolio performance during benchmark up-periods.
///
/// Computes the ratio of the portfolio's compounded return to the benchmark's
/// compounded return over periods where the benchmark return is non-negative.
/// A value > 1.0 means the portfolio amplifies benchmark gains.
///
/// # Arguments
///
/// * `returns`   - Portfolio return series.
/// * `benchmark` - Benchmark return series.
///
/// # Returns
///
/// Up capture ratio. Returns `0.0` if there are no up-benchmark periods
/// or the benchmark's compounded up-period return is negligible.
///
/// # Examples
///
/// ```rust
/// use finstack_analytics::benchmark::up_capture;
///
/// // Portfolio doubles the benchmark in up periods.
/// let r = [0.04, -0.01, 0.06];
/// let b = [0.02, -0.03, 0.03];
/// let uc = up_capture(&r, &b);
/// assert!(uc > 1.0);
/// ```
#[must_use]
pub fn up_capture(returns: &[f64], benchmark: &[f64]) -> f64 {
    let n = returns.len().min(benchmark.len());
    if n == 0 {
        return 0.0;
    }
    let mut port_prod = 1.0_f64;
    let mut bench_prod = 1.0_f64;
    let mut has_up = false;
    for i in 0..n {
        if benchmark[i] >= 0.0 {
            port_prod *= 1.0 + returns[i];
            bench_prod *= 1.0 + benchmark[i];
            has_up = true;
        }
    }
    if !has_up {
        return 0.0;
    }
    let bench_ret = bench_prod - 1.0;
    if bench_ret.abs() < 1e-18 {
        return 0.0;
    }
    (port_prod - 1.0) / bench_ret
}

/// Down-market capture ratio: portfolio performance during benchmark down-periods.
///
/// Computes the ratio of the portfolio's compounded return to the benchmark's
/// compounded return over periods where the benchmark return is negative.
/// A value < 1.0 means the portfolio loses less than the benchmark during
/// downturns (desirable).
///
/// # Arguments
///
/// * `returns`   - Portfolio return series.
/// * `benchmark` - Benchmark return series.
///
/// # Returns
///
/// Down capture ratio. Returns `0.0` if there are no down-benchmark periods
/// or the benchmark's compounded down-period return is negligible.
///
/// # Examples
///
/// ```rust
/// use finstack_analytics::benchmark::down_capture;
///
/// // Portfolio loses less than benchmark in down periods (defensive).
/// let r = [0.04, -0.01, 0.06];
/// let b = [0.02, -0.03, 0.03];
/// let dc = down_capture(&r, &b);
/// assert!(dc < 1.0);
/// ```
#[must_use]
pub fn down_capture(returns: &[f64], benchmark: &[f64]) -> f64 {
    let n = returns.len().min(benchmark.len());
    if n == 0 {
        return 0.0;
    }
    let mut port_prod = 1.0_f64;
    let mut bench_prod = 1.0_f64;
    let mut has_down = false;
    for i in 0..n {
        if benchmark[i] < 0.0 {
            port_prod *= 1.0 + returns[i];
            bench_prod *= 1.0 + benchmark[i];
            has_down = true;
        }
    }
    if !has_down {
        return 0.0;
    }
    let bench_ret = bench_prod - 1.0;
    if bench_ret.abs() < 1e-18 {
        return 0.0;
    }
    (port_prod - 1.0) / bench_ret
}

/// Capture ratio = up capture / down capture.
///
/// A value > 1.0 indicates the portfolio captures more upside than downside
/// relative to the benchmark -- the hallmark of a skillful active manager.
///
/// # Arguments
///
/// * `returns`   - Portfolio return series.
/// * `benchmark` - Benchmark return series.
///
/// # Returns
///
/// The capture ratio. Returns `0.0` if either capture component is zero.
///
/// # Examples
///
/// ```rust
/// use finstack_analytics::benchmark::capture_ratio;
///
/// let r = [0.04, -0.01, 0.06];
/// let b = [0.02, -0.03, 0.03];
/// let cr = capture_ratio(&r, &b);
/// assert!(cr > 1.0);
/// ```
#[must_use]
pub fn capture_ratio(returns: &[f64], benchmark: &[f64]) -> f64 {
    let dc = down_capture(returns, benchmark);
    if dc == 0.0 {
        return 0.0;
    }
    up_capture(returns, benchmark) / dc
}

/// Batting average: fraction of periods where portfolio outperforms benchmark.
///
/// ```text
/// BA = count(r_portfolio > r_benchmark) / n
/// ```
///
/// A value above 0.5 indicates the portfolio beats the benchmark more often
/// than not, though it says nothing about the magnitude of wins vs losses.
///
/// # Arguments
///
/// * `returns`   - Portfolio return series.
/// * `benchmark` - Benchmark return series.
///
/// # Returns
///
/// Fraction in `[0, 1]`. Returns `0.0` for empty series.
///
/// # Examples
///
/// ```rust
/// use finstack_analytics::benchmark::batting_average;
///
/// let r = [0.02, 0.01, 0.03, -0.01];
/// let b = [0.01, 0.02, 0.01, 0.00];
/// let ba = batting_average(&r, &b);
/// // Beats benchmark in periods 0, 2 → 2/4 = 0.5
/// // Period 3: -0.01 < 0.00 → loss
/// assert!((ba - 0.5).abs() < 1e-12);
/// ```
#[must_use]
pub fn batting_average(returns: &[f64], benchmark: &[f64]) -> f64 {
    let n = returns.len().min(benchmark.len());
    if n == 0 {
        return 0.0;
    }
    let wins = (0..n).filter(|&i| returns[i] > benchmark[i]).count();
    wins as f64 / n as f64
}

/// Result of a multi-factor regression.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MultiFactorResult {
    /// Annualized intercept (alpha).
    pub alpha: f64,
    /// Regression coefficients, one per factor.
    pub betas: Vec<f64>,
    /// R-squared: fraction of variance explained by the factors.
    pub r_squared: f64,
    /// Adjusted R-squared: penalizes additional regressors.
    ///
    /// ```text
    /// adj_R² = 1 − (1 − R²) × (n − 1) / (n − k − 1)
    /// ```
    pub adjusted_r_squared: f64,
    /// Annualized residual volatility.
    pub residual_vol: f64,
}

fn dot(lhs: &[f64], rhs: &[f64]) -> f64 {
    lhs.iter().zip(rhs.iter()).map(|(l, r)| l * r).sum()
}

fn qr_least_squares(columns: &[Vec<f64>], y: &[f64]) -> crate::Result<Vec<f64>> {
    let p = columns.len();
    let n = y.len();
    let max_col_norm = columns
        .iter()
        .map(|column| dot(column, column).sqrt())
        .fold(0.0_f64, f64::max);
    let tolerance = 1.0e-10 * max_col_norm.max(1.0);

    let mut q_columns = vec![vec![0.0_f64; n]; p];
    let mut r = vec![0.0_f64; p * p];

    for j in 0..p {
        let mut v = columns[j].clone();
        for i in 0..j {
            let projection = dot(&q_columns[i], &v);
            r[i * p + j] = projection;
            for (value, basis) in v.iter_mut().zip(q_columns[i].iter()) {
                *value -= projection * basis;
            }
        }

        let norm = dot(&v, &v).sqrt();
        if !norm.is_finite() || norm <= tolerance {
            return Err(crate::error::InputError::Invalid.into());
        }

        r[j * p + j] = norm;
        for (target, value) in q_columns[j].iter_mut().zip(v.iter()) {
            *target = *value / norm;
        }
    }

    let mut q_transpose_y = vec![0.0_f64; p];
    for j in 0..p {
        q_transpose_y[j] = dot(&q_columns[j], y);
    }

    let mut beta = vec![0.0_f64; p];
    for i in (0..p).rev() {
        let mut sum = q_transpose_y[i];
        for j in (i + 1)..p {
            sum -= r[i * p + j] * beta[j];
        }
        let diag = r[i * p + i];
        if !diag.is_finite() || diag.abs() <= tolerance {
            return Err(crate::error::InputError::Invalid.into());
        }
        beta[i] = sum / diag;
    }

    Ok(beta)
}

/// Multi-factor OLS regression of portfolio returns on factor returns.
///
/// Estimates the linear model
///
/// ```text
/// r_portfolio = α + β₁f₁ + β₂f₂ + ... + βₖfₖ + ε
/// ```
///
/// by solving the least-squares system with a QR decomposition of the design
/// matrix. Using QR avoids explicitly forming the normal equations and is more
/// numerically stable when factors are correlated.
///
/// # Arguments
///
/// * `returns`    - Portfolio simple-return series in decimal form (for example,
///   `0.01` for 1%).
/// * `factors`    - Slice of factor return series (each inner slice is one
///   factor's return series, all the same length as `returns`). Factor returns
///   use the same decimal convention as `returns`.
/// * `ann_factor` - Number of observation periods per year used to annualize
///   the intercept and residual volatility. For example, use `252.0` for daily
///   data or `12.0` for monthly data.
///
/// # Returns
///
/// A [`MultiFactorResult`] containing:
///
/// - `alpha`: annualized intercept
/// - `betas`: one loading per factor
/// - `r_squared` and `adjusted_r_squared`: goodness-of-fit measures
/// - `residual_vol`: annualized residual volatility
///
/// Coefficients are estimated on the overlapping sample implied by the input
/// slices. The function does not truncate mismatched factor lengths; it rejects
/// them as invalid input instead.
///
/// # Errors
///
/// Returns an error when:
///
/// - `ann_factor` is not finite or is `<= 0`
/// - no factors are supplied
/// - there are too few observations for the requested number of factors
/// - any portfolio or factor return is non-finite
/// - any factor length differs from `returns.len()`
/// - the factor matrix is singular or numerically rank deficient
///
/// # Examples
///
/// ```rust
/// use finstack_analytics::benchmark::multi_factor_greeks;
///
/// // y ≈ 2*f1 (single effective factor).
/// let y = [0.02, 0.04, 0.06, 0.08, 0.10];
/// let f1 = [0.01, 0.02, 0.03, 0.04, 0.05];
/// let result = multi_factor_greeks(&y, &[&f1], 252.0).unwrap();
/// assert!(result.r_squared > 0.99);
/// ```
///
/// # References
///
/// - Fama & French (1993): see docs/REFERENCES.md#fama-french-1993
/// - Higham: see docs/REFERENCES.md#higham-accuracy-and-stability
pub fn multi_factor_greeks(
    returns: &[f64],
    factors: &[&[f64]],
    ann_factor: f64,
) -> crate::Result<MultiFactorResult> {
    if !ann_factor.is_finite() || ann_factor <= 0.0 {
        return Err(crate::error::InputError::Invalid.into());
    }

    let n = returns.len();
    let k = factors.len();
    let p = k + 1; // intercept + k factors

    if n < p + 1 || k == 0 {
        return Err(crate::error::InputError::Invalid.into());
    }
    if returns.iter().any(|r| !r.is_finite()) {
        return Err(crate::error::InputError::Invalid.into());
    }
    if factors
        .iter()
        .any(|factor| factor.iter().any(|v| !v.is_finite()))
    {
        return Err(crate::error::InputError::Invalid.into());
    }
    if factors.iter().any(|factor| factor.len() != n) {
        return Err(crate::error::InputError::DimensionMismatch.into());
    }
    let mut columns = Vec::with_capacity(p);
    columns.push(vec![1.0_f64; n]);
    columns.extend(factors.iter().map(|factor| factor.to_vec()));

    let beta = qr_least_squares(&columns, returns)?;

    let alpha_per_period = beta[0];
    let factor_betas: Vec<f64> = beta[1..].to_vec();

    // Compute residuals and R²
    let y_mean = mean(returns);
    let mut ss_res = 0.0_f64;
    let mut ss_tot = 0.0_f64;
    for (t, &r) in returns.iter().enumerate().take(n) {
        let mut y_hat = alpha_per_period;
        for j in 0..k {
            let fj = factors[j][t];
            y_hat += factor_betas[j] * fj;
        }
        let residual = r - y_hat;
        ss_res += residual * residual;
        ss_tot += (r - y_mean) * (r - y_mean);
    }

    let r_sq = if ss_tot > 0.0 {
        1.0 - ss_res / ss_tot
    } else {
        0.0
    };
    let dof = n as f64 - k as f64 - 1.0;
    let residual_var = if dof > 0.0 { ss_res / dof } else { 0.0 };
    let residual_vol = residual_var.sqrt() * ann_factor.sqrt();
    let alpha = alpha_per_period * ann_factor;

    let adjusted_r_squared = if dof > 0.0 {
        1.0 - (1.0 - r_sq) * (n as f64 - 1.0) / dof
    } else {
        0.0
    };

    Ok(MultiFactorResult {
        alpha,
        betas: factor_betas,
        r_squared: r_sq,
        adjusted_r_squared,
        residual_vol,
    })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    use crate::dates::{Duration, Month};

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
    fn tracking_error_nan_when_annualized_with_invalid_ann_factor() {
        let r = [0.01, 0.02];
        let b = [0.01, 0.01];
        assert!(tracking_error(&r, &b, true, 0.0).is_nan());
        assert!(tracking_error(&r, &b, true, -1.0).is_nan());
        assert!(tracking_error(&r, &b, true, f64::NAN).is_nan());
    }

    #[test]
    fn information_ratio_basic() {
        let r = [0.02, 0.03, 0.01, 0.04];
        let b = [0.01, 0.01, 0.01, 0.01];
        let ir = information_ratio(&r, &b, false, 252.0);
        assert!(ir > 0.0);
    }

    #[test]
    fn information_ratio_nan_when_annualized_with_invalid_ann_factor() {
        let r = [0.02, 0.03, 0.01, 0.04];
        let b = [0.01, 0.01, 0.01, 0.01];
        assert!(information_ratio(&r, &b, true, 0.0).is_nan());
        assert!(information_ratio(&r, &b, true, f64::INFINITY).is_nan());
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
    fn calc_beta_uses_t_critical_value_for_small_samples() {
        let y = [0.020, 0.041, 0.059, 0.082, 0.099];
        let x = [0.010, 0.020, 0.030, 0.040, 0.050];
        let result = calc_beta(&y, &x);
        let expected_t_critical_df3 = 3.182_446_305_284_263_f64;
        let expected_half_width = expected_t_critical_df3 * result.std_err;
        let actual_half_width = result.ci_upper - result.beta;

        assert!(
            (actual_half_width - expected_half_width).abs() < 1e-12,
            "small-sample beta CI should use Student-t critical value: expected {}, got {}",
            expected_half_width,
            actual_half_width
        );
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
    fn rolling_greeks_stays_close_to_exact_recomputation_on_long_series() {
        fn exact_rolling_greeks(
            returns: &[f64],
            benchmark: &[f64],
            window: usize,
            ann_factor: f64,
        ) -> (Vec<f64>, Vec<f64>) {
            let n = returns.len().min(benchmark.len());
            let w = window as f64;
            let mut alphas = Vec::with_capacity(n - window + 1);
            let mut betas = Vec::with_capacity(n - window + 1);
            for end in window..=n {
                let rs = &returns[end - window..end];
                let bs = &benchmark[end - window..end];
                let sr: f64 = rs.iter().sum();
                let sb: f64 = bs.iter().sum();
                let srb: f64 = rs.iter().zip(bs.iter()).map(|(&r, &b)| r * b).sum();
                let sb2: f64 = bs.iter().map(|&b| b * b).sum();
                let denom = w * sb2 - sb * sb;
                let beta = if denom.abs() < 1e-30 {
                    0.0
                } else {
                    (w * srb - sb * sr) / denom
                };
                let alpha = (sr / w - beta * sb / w) * ann_factor;
                alphas.push(alpha);
                betas.push(beta);
            }
            (alphas, betas)
        }

        let window = 64;
        let ann_factor = 252.0;
        let n = ROLLING_GREEKS_RECOMPUTE_INTERVAL + window + 128;
        let r: Vec<f64> = (0..n)
            .map(|i| {
                let x = i as f64;
                1_000_000.0 + x * 0.125 + (x / 9.0).sin() * 0.01
            })
            .collect();
        let b: Vec<f64> = (0..n)
            .map(|i| {
                let x = i as f64;
                500_000.0 + x * 0.0625 + (x / 7.0).cos() * 0.01
            })
            .collect();
        let dates: Vec<Date> = (0..n).map(|i| jan(1) + Duration::days(i as i64)).collect();

        let rolling = rolling_greeks(&r, &b, &dates, window, ann_factor);
        let (_, expected_betas) = exact_rolling_greeks(&r, &b, window, ann_factor);

        let max_beta_diff = rolling
            .betas
            .iter()
            .zip(expected_betas.iter())
            .map(|(&actual, &expected)| (actual - expected).abs())
            .fold(0.0_f64, f64::max);
        assert!(
            max_beta_diff < 5e-4,
            "max beta diff too large: {max_beta_diff}"
        );
        assert!(rolling.alphas.iter().all(|alpha| alpha.is_finite()));
    }

    #[test]
    fn align_benchmark_basic() {
        let bd = vec![jan(1), jan(2), jan(3)];
        let br = vec![0.01, 0.02, 0.03];
        let td = vec![jan(1), jan(3), jan(5)];
        let aligned = align_benchmark(&br, &bd, &td);
        assert_eq!(aligned, vec![0.01, 0.03, 0.0]);
    }

    #[test]
    fn up_capture_hand_calc() {
        // r = [0.10, −0.05], b = [0.05, −0.10]
        // Up periods (b≥0): index 0
        // port_prod = 1.10, bench_prod = 1.05
        // up_capture = (1.10−1) / (1.05−1) = 0.10/0.05 = 2.0
        let r = [0.10, -0.05];
        let b = [0.05, -0.10];
        let uc = up_capture(&r, &b);
        assert!((uc - 2.0).abs() < 1e-12);
    }

    #[test]
    fn down_capture_hand_calc() {
        // Same data: down periods (b<0): index 1
        // port_prod = 0.95, bench_prod = 0.90
        // down_capture = (0.95−1) / (0.90−1) = −0.05/−0.10 = 0.5
        let r = [0.10, -0.05];
        let b = [0.05, -0.10];
        let dc = down_capture(&r, &b);
        assert!((dc - 0.5).abs() < 1e-12);
    }

    #[test]
    fn capture_ratio_hand_calc() {
        // up/down = 2.0/0.5 = 4.0
        let r = [0.10, -0.05];
        let b = [0.05, -0.10];
        let cr = capture_ratio(&r, &b);
        assert!((cr - 4.0).abs() < 1e-12);
    }

    #[test]
    fn up_capture_multiple_periods() {
        // r = [0.04, −0.01, 0.06], b = [0.02, −0.03, 0.03]
        // Up periods: indices 0, 2 (b[0]=0.02≥0, b[2]=0.03≥0)
        // port_prod = (1.04)(1.06) = 1.1024
        // bench_prod = (1.02)(1.03) = 1.0506
        // up_capture = (1.1024−1)/(1.0506−1) = 0.1024/0.0506
        let r = [0.04, -0.01, 0.06];
        let b = [0.02, -0.03, 0.03];
        let uc = up_capture(&r, &b);
        let expected = (1.04 * 1.06 - 1.0) / (1.02 * 1.03 - 1.0);
        assert!((uc - expected).abs() < 1e-12);
    }

    #[test]
    fn down_capture_defensive_portfolio() {
        // Portfolio loses less than benchmark → dc < 1.0 (desirable)
        let r = [0.04, -0.01, 0.06];
        let b = [0.02, -0.03, 0.03];
        let dc = down_capture(&r, &b);
        // Down periods: index 1. port_prod=0.99, bench_prod=0.97
        let expected = (0.99 - 1.0) / (0.97 - 1.0);
        assert!((dc - expected).abs() < 1e-12);
        assert!(dc < 1.0);
    }

    #[test]
    fn up_capture_no_up_periods() {
        let r = [0.01, 0.02];
        let b = [-0.01, -0.02];
        assert_eq!(up_capture(&r, &b), 0.0);
    }

    #[test]
    fn down_capture_no_down_periods() {
        let r = [0.01, 0.02];
        let b = [0.01, 0.02];
        assert_eq!(down_capture(&r, &b), 0.0);
    }

    #[test]
    fn capture_ratio_perfect_tracking() {
        // Portfolio = benchmark → up_capture=1, down_capture=1, ratio=1
        let r = [0.02, -0.03, 0.01, -0.01];
        let cr = capture_ratio(&r, &r);
        assert!((cr - 1.0).abs() < 1e-12);
    }

    #[test]
    fn multi_factor_single_factor() {
        // y = 2*x → alpha ≈ 0, beta ≈ 2, R² ≈ 1.
        let y = [0.02, 0.04, 0.06, 0.08, 0.10];
        let f1 = [0.01, 0.02, 0.03, 0.04, 0.05];
        let result = multi_factor_greeks(&y, &[&f1], 252.0).expect("single-factor regression");
        assert!((result.betas[0] - 2.0).abs() < 1e-8);
        assert!(result.r_squared > 0.999);
    }

    #[test]
    fn multi_factor_two_factors() {
        // y ≈ 1.5*f1 + 0.5*f2 (non-collinear factors).
        let f1 = [0.01, 0.02, 0.03, 0.04, 0.05];
        let f2 = [0.03, -0.01, 0.02, 0.01, -0.02];
        let y: Vec<f64> = (0..5).map(|i| 1.5 * f1[i] + 0.5 * f2[i]).collect();
        let result = multi_factor_greeks(&y, &[&f1, &f2], 252.0).expect("two-factor regression");
        assert!(result.r_squared > 0.99);
        assert_eq!(result.betas.len(), 2);
        assert!((result.betas[0] - 1.5).abs() < 1e-6);
        assert!((result.betas[1] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn multi_factor_empty_errors() {
        let result = multi_factor_greeks(&[], &[&[]], 252.0);
        assert!(result.is_err());
    }

    #[test]
    fn multi_factor_mismatched_factor_lengths_error() {
        let y = [0.02, 0.04, 0.06, 0.08, 0.10];
        let f1 = [0.01, 0.02, 0.03];
        let result = multi_factor_greeks(&y, &[&f1], 252.0);
        assert!(result.is_err());
    }

    #[test]
    fn multi_factor_adjusted_r_squared() {
        // y = 2*x → R²≈1, adj_R² should also be close to 1
        let y = [0.02, 0.04, 0.06, 0.08, 0.10];
        let f1 = [0.01, 0.02, 0.03, 0.04, 0.05];
        let result = multi_factor_greeks(&y, &[&f1], 252.0).expect("adjusted r-squared regression");
        assert!(result.adjusted_r_squared > 0.99);
        assert!(result.adjusted_r_squared <= result.r_squared);
    }

    #[test]
    fn batting_average_hand_calc() {
        // r = [0.02, 0.01, 0.03, -0.01], b = [0.01, 0.02, 0.01, 0.00]
        // Wins: r[0]>b[0] (0.02>0.01), r[2]>b[2] (0.03>0.01) → 2/4 = 0.5
        let r = [0.02, 0.01, 0.03, -0.01];
        let b = [0.01, 0.02, 0.01, 0.00];
        assert!((batting_average(&r, &b) - 0.5).abs() < 1e-12);
    }

    #[test]
    fn batting_average_all_wins() {
        let r = [0.05, 0.03, 0.04];
        let b = [0.01, 0.01, 0.01];
        assert!((batting_average(&r, &b) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn batting_average_empty() {
        assert_eq!(batting_average(&[], &[]), 0.0);
    }
}

// ── Benchmark-relative risk ratios ──

/// Treynor ratio: excess return per unit of systematic risk.
///
/// ```text
/// Treynor = (R_p − R_f) / β
/// ```
///
/// Complements the Sharpe ratio by using beta (systematic risk) rather
/// than total volatility as the risk denominator.
///
/// # Arguments
///
/// * `ann_return`     - Annualized portfolio return.
/// * `risk_free_rate` - Annualized risk-free rate.
/// * `beta`           - Portfolio beta vs benchmark.
///
/// # Returns
///
/// The Treynor ratio. Returns `0.0` if beta is zero.
///
/// # Examples
///
/// ```rust
/// use finstack_analytics::benchmark::treynor;
///
/// // 10% return, 2% risk-free, beta = 1.2 → Treynor ≈ 0.0667.
/// let t = treynor(0.10, 0.02, 1.2);
/// assert!((t - 0.0667).abs() < 0.001);
/// ```
///
/// # References
///
/// - Treynor (1965): see docs/REFERENCES.md#treynor1965
#[must_use]
pub fn treynor(ann_return: f64, risk_free_rate: f64, beta: f64) -> f64 {
    if beta.abs() < 1e-10 {
        return 0.0;
    }
    (ann_return - risk_free_rate) / beta
}

/// M-squared (Modigliani-Modigliani): risk-adjusted return on the benchmark's scale.
///
/// Leverages or deleverages the portfolio to match the benchmark's volatility,
/// then reports the resulting return. The difference `M² − R_bench` is a
/// direct measure of value added at the same risk level.
///
/// ```text
/// M² = R_f + (R_p − R_f) × (σ_bench / σ_portfolio)
/// ```
///
/// # Arguments
///
/// * `ann_return`     - Annualized portfolio return.
/// * `ann_vol`        - Annualized portfolio volatility.
/// * `bench_vol`      - Annualized benchmark volatility.
/// * `risk_free_rate` - Annualized risk-free rate.
///
/// # Returns
///
/// The M-squared return. Returns the risk-free rate if portfolio volatility is zero.
///
/// # Examples
///
/// ```rust
/// use finstack_analytics::benchmark::m_squared;
///
/// // Portfolio: 12% return, 20% vol; Benchmark: 15% vol; Rf: 2%
/// // M² = 0.02 + (0.12 − 0.02) × (0.15 / 0.20) = 0.02 + 0.075 = 0.095
/// let m2 = m_squared(0.12, 0.20, 0.15, 0.02);
/// assert!((m2 - 0.095).abs() < 1e-12);
/// ```
///
/// # References
///
/// - Modigliani & Modigliani (1997): see docs/REFERENCES.md#modigliani1997
#[must_use]
pub fn m_squared(ann_return: f64, ann_vol: f64, bench_vol: f64, risk_free_rate: f64) -> f64 {
    if ann_vol.abs() < 1e-10 {
        return risk_free_rate;
    }
    risk_free_rate + (ann_return - risk_free_rate) * (bench_vol / ann_vol)
}

/// M-squared computed directly from portfolio and benchmark return series.
///
/// This convenience wrapper annualizes the portfolio arithmetic mean return,
/// portfolio volatility, and benchmark volatility from the supplied return
/// series, then delegates to [`m_squared`].
///
/// # Arguments
///
/// * `portfolio` - Portfolio simple-return series in decimal form.
/// * `benchmark` - Benchmark simple-return series in decimal form.
/// * `ann_factor` - Number of periods per year used for annualization.
/// * `risk_free_rate` - Annualized risk-free rate in decimal form.
///
/// # Returns
///
/// The M-squared return on the benchmark-volatility scale. Returns the
/// risk-free rate if the annualized portfolio volatility is zero. Propagates
/// `NaN` from the underlying annualized return or volatility calculations when
/// `ann_factor` is invalid.
///
/// # Examples
///
/// ```rust
/// use finstack_analytics::benchmark::{m_squared, m_squared_from_returns};
/// use finstack_analytics::risk_metrics::{mean_return, volatility};
///
/// let portfolio = [0.01, -0.015, 0.012, 0.008, -0.004, 0.009];
/// let benchmark = [0.008, -0.01, 0.01, 0.006, -0.003, 0.007];
/// let ann_factor = 252.0;
/// let risk_free_rate = 0.01;
///
/// let direct = m_squared_from_returns(&portfolio, &benchmark, ann_factor, risk_free_rate);
/// let expected = m_squared(
///     mean_return(&portfolio, true, ann_factor),
///     volatility(&portfolio, true, ann_factor),
///     volatility(&benchmark, true, ann_factor),
///     risk_free_rate,
/// );
///
/// assert!((direct - expected).abs() < 1e-12);
/// ```
///
/// # References
///
/// - Modigliani & Modigliani (1997): see docs/REFERENCES.md#modigliani1997
#[must_use]
pub fn m_squared_from_returns(
    portfolio: &[f64],
    benchmark: &[f64],
    ann_factor: f64,
    risk_free_rate: f64,
) -> f64 {
    let ann_return = crate::risk_metrics::mean_return(portfolio, true, ann_factor);
    let ann_vol = crate::risk_metrics::volatility(portfolio, true, ann_factor);
    let bench_vol = crate::risk_metrics::volatility(benchmark, true, ann_factor);
    m_squared(ann_return, ann_vol, bench_vol, risk_free_rate)
}

#[cfg(test)]
mod benchmark_ratio_tests {
    use super::*;

    #[test]
    fn treynor_hand_calc() {
        let t = treynor(0.10, 0.02, 1.2);
        assert!((t - 0.08 / 1.2).abs() < 1e-14);
    }

    #[test]
    fn treynor_zero_beta() {
        assert_eq!(treynor(0.10, 0.02, 0.0), 0.0);
    }

    #[test]
    fn treynor_negative_beta() {
        let t = treynor(0.10, 0.02, -0.5);
        assert!((t - (0.08 / -0.5)).abs() < 1e-14);
    }

    #[test]
    fn m_squared_hand_calc() {
        let m2 = m_squared(0.12, 0.20, 0.15, 0.02);
        assert!((m2 - 0.095).abs() < 1e-12);
    }

    #[test]
    fn m_squared_zero_vol() {
        assert_eq!(m_squared(0.10, 0.0, 0.15, 0.02), 0.02);
    }

    #[test]
    fn m_squared_from_returns_matches_composed_formula() {
        let portfolio = [0.01, -0.015, 0.012, 0.008, -0.004, 0.009];
        let benchmark = [0.008, -0.01, 0.01, 0.006, -0.003, 0.007];
        let ann = 252.0;
        let ann_ret = crate::risk_metrics::mean_return(&portfolio, true, ann);
        let ann_vol = crate::risk_metrics::volatility(&portfolio, true, ann);
        let bench_vol = crate::risk_metrics::volatility(&benchmark, true, ann);
        let expected = m_squared(ann_ret, ann_vol, bench_vol, 0.01);
        let actual = m_squared_from_returns(&portfolio, &benchmark, ann, 0.01);
        assert!((actual - expected).abs() < 1e-12);
    }
}
