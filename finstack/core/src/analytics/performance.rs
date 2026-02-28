//! Stateful `Performance` struct that orchestrates all analytics sub-modules.
//!
//! Mirrors the Python `Performance` class 1:1 (minus plotting), operating on
//! internal slices and returning numeric results.

use crate::dates::{Date, FiscalConfig, PeriodKind};

use super::aggregation::{group_by_period, period_stats, PeriodStats};
use super::benchmark::{
    batting_average, calc_beta, capture_ratio, down_capture, greeks, information_ratio,
    multi_factor_greeks, r_squared, rolling_greeks, tracking_error, up_capture, BetaResult,
    GreeksResult, MultiFactorResult, RollingGreeks,
};
use super::drawdown::{
    cdar, drawdown_details, max_drawdown_duration as dd_max_duration, to_drawdown_series,
    DrawdownEpisode,
};
use super::lookback;
use super::returns::{clean_returns, comp_sum, comp_total, excess_returns, simple_returns};
use super::risk_metrics::{
    self, rolling_sharpe, rolling_sortino, rolling_volatility, RollingSharpe, RollingSortino,
    RollingVolatility,
};

/// Central performance analytics engine.
///
/// Holds pre-computed returns, drawdowns, and benchmark data for a universe of
/// tickers. Methods delegate to the pure-function sub-modules.
pub struct Performance {
    dates: Vec<Date>,
    returns: Vec<Vec<f64>>,
    ticker_names: Vec<String>,
    benchmark_idx: usize,
    drawdowns: Vec<Vec<f64>>,
    bench_returns: Vec<f64>,
    bench_drawdown: Vec<f64>,
    freq: PeriodKind,
    log_returns: bool,
    start_idx: usize,
    end_idx: usize,
}

impl Performance {
    /// Construct from a price matrix (columns = tickers).
    ///
    /// Computes simple or log returns for each ticker, builds the drawdown
    /// series, and designates one ticker as the benchmark. The `dates`
    /// vector should have one entry per price row; internally the date and
    /// return series are trimmed by one element to align with the return
    /// computation (returns have length `n_prices - 1`).
    ///
    /// # Arguments
    ///
    /// * `dates` - Chronologically sorted date vector, one entry per price
    ///   observation.
    /// * `prices` - Price matrix: `prices[i]` is the full price series for
    ///   ticker `i`.
    /// * `ticker_names` - Names corresponding to each column of `prices`.
    /// * `benchmark_ticker` - Name of the benchmark ticker. Falls back to
    ///   column 0 if `None` or not found.
    /// * `freq` - Observation frequency, used to derive the annualization factor.
    /// * `use_log_returns` - If `true`, uses log returns (`ln(p[t]/p[t-1])`);
    ///   if `false`, uses simple returns (`p[t]/p[t-1] - 1`).
    ///
    /// # Returns
    ///
    /// A fully initialized [`Performance`] instance, or an error if
    /// `prices` or `dates` is empty.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_core::analytics::performance::Performance;
    /// use finstack_core::dates::PeriodKind;
    /// use time::{Date, Month};
    ///
    /// let dates: Vec<Date> = (1..=10)
    ///     .map(|d| Date::from_calendar_date(2025, Month::January, d).unwrap())
    ///     .collect();
    /// let prices = vec![(0..10).map(|i| 100.0 + i as f64).collect::<Vec<_>>()];
    /// let perf = Performance::new(
    ///     dates,
    ///     prices,
    ///     vec!["SPY".into()],
    ///     None,
    ///     PeriodKind::Daily,
    ///     false,
    /// ).unwrap();
    /// assert_eq!(perf.ticker_names(), &["SPY"]);
    /// ```
    pub fn new(
        dates: Vec<Date>,
        prices: Vec<Vec<f64>>,
        ticker_names: Vec<String>,
        benchmark_ticker: Option<&str>,
        freq: PeriodKind,
        use_log_returns: bool,
    ) -> crate::Result<Self> {
        if prices.is_empty() || dates.is_empty() {
            return Err(crate::error::InputError::Invalid.into());
        }
        let n_tickers = prices.len();

        let benchmark_idx = match benchmark_ticker {
            Some(name) => ticker_names.iter().position(|t| t == name).unwrap_or(0),
            None => 0,
        };

        let mut all_returns: Vec<Vec<f64>> = Vec::with_capacity(n_tickers);
        let mut all_drawdowns: Vec<Vec<f64>> = Vec::with_capacity(n_tickers);

        for price_col in &prices {
            let mut r = if use_log_returns {
                crate::math::stats::log_returns(price_col)
            } else {
                let sr = simple_returns(price_col);
                sr[1..].to_vec()
            };
            clean_returns(&mut r);
            let dd = to_drawdown_series(&r);
            all_drawdowns.push(dd);
            all_returns.push(r);
        }

        let bench_returns = all_returns.get(benchmark_idx).cloned().unwrap_or_default();
        let bench_drawdown = all_drawdowns
            .get(benchmark_idx)
            .cloned()
            .unwrap_or_default();

        let adj_dates = if dates.len() > 1 {
            dates[1..].to_vec()
        } else {
            dates.clone()
        };

        let end_idx = all_returns.first().map(|r| r.len()).unwrap_or(0);

        Ok(Self {
            dates: adj_dates,
            returns: all_returns,
            ticker_names,
            benchmark_idx,
            drawdowns: all_drawdowns,
            bench_returns,
            bench_drawdown,
            freq,
            log_returns: use_log_returns,
            start_idx: 0,
            end_idx,
        })
    }

    /// Restrict all subsequent analytics to the `[start, end]` date window.
    ///
    /// Finds the index boundaries in the internal date vector using binary
    /// search and stores them as `start_idx`/`end_idx`. All `active_*`
    /// accessors respect this range until it is changed again.
    ///
    /// # Arguments
    ///
    /// * `start` - First date to include (inclusive).
    /// * `end`   - Last date to include (inclusive).
    pub fn reset_date_range(&mut self, start: Date, end: Date) {
        self.start_idx = self.dates.partition_point(|&d| d < start);
        self.end_idx = self.dates.partition_point(|&d| d <= end);
    }

    /// Designate a different ticker as the benchmark for all subsequent analytics.
    ///
    /// Updates the internal benchmark return and drawdown caches to point to
    /// the new ticker's pre-computed series.
    ///
    /// # Arguments
    ///
    /// * `ticker` - Name of the ticker to use as benchmark. Must match one
    ///   of the names provided at construction time.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or an [`InputError::Invalid`] if `ticker` is
    /// not found among the loaded tickers.
    pub fn reset_bench_ticker(&mut self, ticker: &str) -> crate::Result<()> {
        let idx = self
            .ticker_names
            .iter()
            .position(|t| t == ticker)
            .ok_or(crate::error::InputError::Invalid)?;
        self.benchmark_idx = idx;
        self.bench_returns = self.returns.get(idx).cloned().unwrap_or_default();
        self.bench_drawdown = self.drawdowns.get(idx).cloned().unwrap_or_default();
        Ok(())
    }

    fn active_range(&self) -> core::ops::Range<usize> {
        self.start_idx..self.end_idx
    }

    fn active_returns(&self, ticker_idx: usize) -> &[f64] {
        let range = self.active_range();
        self.returns
            .get(ticker_idx)
            .map(|r| {
                let end = range.end.min(r.len());
                &r[range.start.min(end)..end]
            })
            .unwrap_or(&[])
    }

    fn active_bench(&self) -> &[f64] {
        let range = self.active_range();
        let end = range.end.min(self.bench_returns.len());
        &self.bench_returns[range.start.min(end)..end]
    }

    /// Date slice corresponding to the currently active analysis window.
    pub fn active_dates(&self) -> &[Date] {
        let range = self.active_range();
        let end = range.end.min(self.dates.len());
        &self.dates[range.start.min(end)..end]
    }

    fn active_drawdown(&self, ticker_idx: usize) -> &[f64] {
        let range = self.active_range();
        self.drawdowns
            .get(ticker_idx)
            .map(|d| {
                let end = range.end.min(d.len());
                &d[range.start.min(end)..end]
            })
            .unwrap_or(&[])
    }

    fn ann(&self) -> f64 {
        self.freq.annualization_factor()
    }

    // ── Scalar metrics per ticker ──

    /// CAGR for each ticker.
    pub fn cagr(&self) -> Vec<f64> {
        let dates = self.active_dates();
        if dates.is_empty() {
            return vec![0.0; self.ticker_names.len()];
        }
        let start = dates[0];
        let end = *dates.last().unwrap_or(&start);
        (0..self.ticker_names.len())
            .map(|i| risk_metrics::cagr(self.active_returns(i), start, end))
            .collect()
    }

    /// Mean return for each ticker.
    ///
    /// # Arguments
    ///
    /// * `annualize` - If `true`, scales the mean by the annualization factor.
    ///
    /// # Returns
    ///
    /// One value per ticker in column order.
    pub fn mean_return(&self, annualize: bool) -> Vec<f64> {
        (0..self.ticker_names.len())
            .map(|i| risk_metrics::mean_return(self.active_returns(i), annualize, self.ann()))
            .collect()
    }

    /// Volatility (sample standard deviation) for each ticker.
    ///
    /// # Arguments
    ///
    /// * `annualize` - If `true`, scales by `sqrt(ann_factor)`.
    ///
    /// # Returns
    ///
    /// One value per ticker in column order.
    pub fn volatility(&self, annualize: bool) -> Vec<f64> {
        (0..self.ticker_names.len())
            .map(|i| risk_metrics::volatility(self.active_returns(i), annualize, self.ann()))
            .collect()
    }

    /// Sharpe ratio for each ticker.
    ///
    /// # Arguments
    ///
    /// * `risk_free_rate` - Annualized risk-free rate (e.g. `0.02` for 2%).
    ///
    /// # Returns
    ///
    /// One Sharpe ratio per ticker. Returns `0.0` for tickers with zero volatility.
    pub fn sharpe(&self, risk_free_rate: f64) -> Vec<f64> {
        let ann = self.ann();
        (0..self.ticker_names.len())
            .map(|i| {
                let r = self.active_returns(i);
                let m = risk_metrics::mean_return(r, true, ann);
                let v = risk_metrics::volatility(r, true, ann);
                risk_metrics::sharpe(m, v, risk_free_rate)
            })
            .collect()
    }

    /// Sortino ratio for each ticker.
    pub fn sortino(&self) -> Vec<f64> {
        (0..self.ticker_names.len())
            .map(|i| risk_metrics::sortino(self.active_returns(i), true, self.ann()))
            .collect()
    }

    /// Calmar ratio for each ticker.
    pub fn calmar(&self) -> Vec<f64> {
        let cagrs = self.cagr();
        (0..self.ticker_names.len())
            .map(|i| {
                let dd = self.active_drawdown(i);
                let max_dd = dd.iter().copied().fold(0.0_f64, f64::min);
                risk_metrics::calmar(cagrs[i], max_dd)
            })
            .collect()
    }

    /// Max drawdown for each ticker.
    pub fn max_drawdown(&self) -> Vec<f64> {
        (0..self.ticker_names.len())
            .map(|i| {
                let dd = self.active_drawdown(i);
                dd.iter().copied().fold(0.0_f64, f64::min)
            })
            .collect()
    }

    /// Historical Value-at-Risk for each ticker (not annualized).
    ///
    /// # Arguments
    ///
    /// * `confidence` - Confidence level in `(0, 1)`, e.g. `0.95` for 95% VaR.
    ///
    /// # Returns
    ///
    /// One VaR value per ticker (non-positive).
    pub fn value_at_risk(&self, confidence: f64) -> Vec<f64> {
        (0..self.ticker_names.len())
            .map(|i| risk_metrics::value_at_risk(self.active_returns(i), confidence, None))
            .collect()
    }

    /// Expected Shortfall (CVaR) for each ticker (not annualized).
    ///
    /// # Arguments
    ///
    /// * `confidence` - Confidence level in `(0, 1)`, e.g. `0.95`.
    ///
    /// # Returns
    ///
    /// One ES value per ticker (non-positive, always ≤ corresponding VaR).
    pub fn expected_shortfall(&self, confidence: f64) -> Vec<f64> {
        (0..self.ticker_names.len())
            .map(|i| risk_metrics::expected_shortfall(self.active_returns(i), confidence, None))
            .collect()
    }

    /// Tail ratio for each ticker.
    ///
    /// # Arguments
    ///
    /// * `confidence` - Quantile level for the upper tail (e.g., `0.95`).
    ///
    /// # Returns
    ///
    /// One tail ratio per ticker.
    pub fn tail_ratio(&self, confidence: f64) -> Vec<f64> {
        (0..self.ticker_names.len())
            .map(|i| risk_metrics::tail_ratio(self.active_returns(i), confidence))
            .collect()
    }

    /// Ulcer index for each ticker.
    pub fn ulcer_index(&self) -> Vec<f64> {
        (0..self.ticker_names.len())
            .map(|i| risk_metrics::ulcer_index(self.active_drawdown(i)))
            .collect()
    }

    /// Risk of ruin for each ticker.
    pub fn risk_of_ruin(&self) -> Vec<f64> {
        (0..self.ticker_names.len())
            .map(|i| {
                let r = self.active_returns(i);
                let m = crate::math::stats::mean(r);
                let v = crate::math::stats::variance(r).sqrt();
                risk_metrics::risk_of_ruin(m, v)
            })
            .collect()
    }

    /// Skewness for each ticker.
    pub fn skewness(&self) -> Vec<f64> {
        (0..self.ticker_names.len())
            .map(|i| risk_metrics::skewness(self.active_returns(i)))
            .collect()
    }

    /// Excess kurtosis for each ticker.
    pub fn kurtosis(&self) -> Vec<f64> {
        (0..self.ticker_names.len())
            .map(|i| risk_metrics::kurtosis(self.active_returns(i)))
            .collect()
    }

    /// Geometric mean return for each ticker.
    pub fn geometric_mean(&self) -> Vec<f64> {
        (0..self.ticker_names.len())
            .map(|i| risk_metrics::geometric_mean(self.active_returns(i)))
            .collect()
    }

    /// Downside deviation for each ticker.
    ///
    /// # Arguments
    ///
    /// * `mar` - Minimum acceptable return threshold (e.g., `0.0`).
    pub fn downside_deviation(&self, mar: f64) -> Vec<f64> {
        (0..self.ticker_names.len())
            .map(|i| {
                risk_metrics::downside_deviation(self.active_returns(i), mar, true, self.ann())
            })
            .collect()
    }

    /// Maximum drawdown duration (calendar days) for each ticker.
    pub fn max_drawdown_duration(&self) -> Vec<i64> {
        (0..self.ticker_names.len())
            .map(|i| dd_max_duration(self.active_drawdown(i), self.active_dates()))
            .collect()
    }

    /// Up-market capture ratio for each ticker vs benchmark.
    pub fn up_capture(&self) -> Vec<f64> {
        let bench = self.active_bench();
        (0..self.ticker_names.len())
            .map(|i| up_capture(self.active_returns(i), bench))
            .collect()
    }

    /// Down-market capture ratio for each ticker vs benchmark.
    pub fn down_capture(&self) -> Vec<f64> {
        let bench = self.active_bench();
        (0..self.ticker_names.len())
            .map(|i| down_capture(self.active_returns(i), bench))
            .collect()
    }

    /// Capture ratio (up/down) for each ticker vs benchmark.
    pub fn capture_ratio(&self) -> Vec<f64> {
        let bench = self.active_bench();
        (0..self.ticker_names.len())
            .map(|i| capture_ratio(self.active_returns(i), bench))
            .collect()
    }

    /// Rolling annualized volatility for a specific ticker.
    ///
    /// # Arguments
    ///
    /// * `ticker_idx` - Zero-based column index of the ticker.
    /// * `window`     - Look-back window length in periods.
    pub fn rolling_volatility(&self, ticker_idx: usize, window: usize) -> RollingVolatility {
        rolling_volatility(
            self.active_returns(ticker_idx),
            self.active_dates(),
            window,
            self.ann(),
        )
    }

    // ── Batch 2: Standard ratios and VaR extensions ──

    /// Omega ratio for each ticker.
    ///
    /// # Arguments
    ///
    /// * `threshold` - Return threshold (typically `0.0`).
    pub fn omega_ratio(&self, threshold: f64) -> Vec<f64> {
        (0..self.ticker_names.len())
            .map(|i| risk_metrics::omega_ratio(self.active_returns(i), threshold))
            .collect()
    }

    /// Treynor ratio for each ticker.
    ///
    /// # Arguments
    ///
    /// * `risk_free_rate` - Annualized risk-free rate.
    pub fn treynor(&self, risk_free_rate: f64) -> Vec<f64> {
        let ann = self.ann();
        let bench = self.active_bench();
        (0..self.ticker_names.len())
            .map(|i| {
                let r = self.active_returns(i);
                let ann_ret = risk_metrics::mean_return(r, true, ann);
                let beta = super::benchmark::greeks(r, bench, ann).beta;
                risk_metrics::treynor(ann_ret, risk_free_rate, beta)
            })
            .collect()
    }

    /// Gain-to-pain ratio for each ticker.
    pub fn gain_to_pain(&self) -> Vec<f64> {
        (0..self.ticker_names.len())
            .map(|i| risk_metrics::gain_to_pain(self.active_returns(i)))
            .collect()
    }

    /// Martin ratio (CAGR / Ulcer Index) for each ticker.
    pub fn martin_ratio(&self) -> Vec<f64> {
        let cagrs = self.cagr();
        (0..self.ticker_names.len())
            .map(|i| {
                let ulcer = risk_metrics::ulcer_index(self.active_drawdown(i));
                risk_metrics::martin_ratio(cagrs[i], ulcer)
            })
            .collect()
    }

    /// Parametric (Gaussian) VaR for each ticker.
    ///
    /// # Arguments
    ///
    /// * `confidence` - Confidence level in `(0, 1)`, e.g. `0.95`.
    pub fn parametric_var(&self, confidence: f64) -> Vec<f64> {
        (0..self.ticker_names.len())
            .map(|i| risk_metrics::parametric_var(self.active_returns(i), confidence, None))
            .collect()
    }

    /// Cornish-Fisher adjusted VaR for each ticker.
    ///
    /// # Arguments
    ///
    /// * `confidence` - Confidence level in `(0, 1)`, e.g. `0.95`.
    pub fn cornish_fisher_var(&self, confidence: f64) -> Vec<f64> {
        (0..self.ticker_names.len())
            .map(|i| risk_metrics::cornish_fisher_var(self.active_returns(i), confidence, None))
            .collect()
    }

    /// Rolling Sortino ratio for a specific ticker.
    ///
    /// # Arguments
    ///
    /// * `ticker_idx` - Zero-based column index of the ticker.
    /// * `window`     - Look-back window length in periods.
    pub fn rolling_sortino(&self, ticker_idx: usize, window: usize) -> RollingSortino {
        rolling_sortino(
            self.active_returns(ticker_idx),
            self.active_dates(),
            window,
            self.ann(),
        )
    }

    // ── Batch 3: Drawdown-family ratios ──

    /// Recovery factor for each ticker.
    pub fn recovery_factor(&self) -> Vec<f64> {
        (0..self.ticker_names.len())
            .map(|i| {
                let total_ret = comp_total(self.active_returns(i));
                let dd = self.active_drawdown(i);
                let max_dd = dd.iter().copied().fold(0.0_f64, f64::min);
                risk_metrics::recovery_factor(total_ret, max_dd)
            })
            .collect()
    }

    /// Sterling ratio for each ticker.
    ///
    /// # Arguments
    ///
    /// * `risk_free_rate` - Annualized risk-free rate.
    /// * `n` - Number of worst drawdowns to average.
    pub fn sterling_ratio(&self, risk_free_rate: f64, n: usize) -> Vec<f64> {
        let cagrs = self.cagr();
        (0..self.ticker_names.len())
            .map(|i| {
                let dd = self.active_drawdown(i);
                let dates = self.active_dates();
                let avg = super::drawdown::avg_drawdown(dd, dates, n);
                risk_metrics::sterling_ratio(cagrs[i], avg, risk_free_rate)
            })
            .collect()
    }

    /// Burke ratio for each ticker.
    ///
    /// # Arguments
    ///
    /// * `risk_free_rate` - Annualized risk-free rate.
    /// * `n` - Number of worst drawdown episodes to use.
    pub fn burke_ratio(&self, risk_free_rate: f64, n: usize) -> Vec<f64> {
        let cagrs = self.cagr();
        (0..self.ticker_names.len())
            .map(|i| {
                let dd = self.active_drawdown(i);
                let dates = self.active_dates();
                let episodes = super::drawdown::drawdown_details(dd, dates, n);
                let dd_vals: Vec<f64> = episodes.iter().map(|e| e.max_drawdown).collect();
                risk_metrics::burke_ratio(cagrs[i], &dd_vals, risk_free_rate)
            })
            .collect()
    }

    /// Pain index for each ticker.
    pub fn pain_index(&self) -> Vec<f64> {
        (0..self.ticker_names.len())
            .map(|i| risk_metrics::pain_index(self.active_drawdown(i)))
            .collect()
    }

    /// Pain ratio for each ticker.
    ///
    /// # Arguments
    ///
    /// * `risk_free_rate` - Annualized risk-free rate.
    pub fn pain_ratio(&self, risk_free_rate: f64) -> Vec<f64> {
        let cagrs = self.cagr();
        (0..self.ticker_names.len())
            .map(|i| {
                let pain = risk_metrics::pain_index(self.active_drawdown(i));
                risk_metrics::pain_ratio(cagrs[i], pain, risk_free_rate)
            })
            .collect()
    }

    // ── Batch 4: Multi-factor and CDaR ──

    /// Multi-factor regression for a specific ticker.
    ///
    /// # Arguments
    ///
    /// * `ticker_idx`     - Zero-based column index of the ticker.
    /// * `factor_returns` - Slice of factor return series.
    pub fn multi_factor_greeks(
        &self,
        ticker_idx: usize,
        factor_returns: &[&[f64]],
    ) -> MultiFactorResult {
        multi_factor_greeks(self.active_returns(ticker_idx), factor_returns, self.ann())
    }

    /// Conditional Drawdown at Risk for each ticker.
    ///
    /// # Arguments
    ///
    /// * `confidence` - Confidence level in `(0, 1)`, e.g. `0.95`.
    pub fn cdar(&self, confidence: f64) -> Vec<f64> {
        (0..self.ticker_names.len())
            .map(|i| cdar(self.active_drawdown(i), confidence))
            .collect()
    }

    // ── Series outputs ──

    /// Cumulative returns for each ticker.
    pub fn cumulative_returns(&self) -> Vec<Vec<f64>> {
        (0..self.ticker_names.len())
            .map(|i| comp_sum(self.active_returns(i)))
            .collect()
    }

    /// Drawdown series for each ticker.
    pub fn drawdown_series(&self) -> Vec<Vec<f64>> {
        (0..self.ticker_names.len())
            .map(|i| self.active_drawdown(i).to_vec())
            .collect()
    }

    /// Top-N drawdown episodes for a specific ticker.
    ///
    /// # Arguments
    ///
    /// * `ticker_idx` - Zero-based column index of the ticker.
    /// * `n` - Maximum number of episodes to return, sorted by severity
    ///   (worst first).
    ///
    /// # Returns
    ///
    /// Up to `n` [`DrawdownEpisode`] structs.
    pub fn drawdown_details(&self, ticker_idx: usize, n: usize) -> Vec<DrawdownEpisode> {
        let dd = self.active_drawdown(ticker_idx);
        let dates = self.active_dates();
        drawdown_details(dd, dates, n)
    }

    // ── Benchmark-relative ──

    /// Tracking error for each ticker vs benchmark.
    pub fn tracking_error(&self) -> Vec<f64> {
        let bench = self.active_bench();
        (0..self.ticker_names.len())
            .map(|i| tracking_error(self.active_returns(i), bench, true, self.ann()))
            .collect()
    }

    /// Information ratio for each ticker vs benchmark.
    pub fn information_ratio(&self) -> Vec<f64> {
        let bench = self.active_bench();
        (0..self.ticker_names.len())
            .map(|i| information_ratio(self.active_returns(i), bench, true, self.ann()))
            .collect()
    }

    /// R-squared for each ticker vs benchmark.
    pub fn r_squared(&self) -> Vec<f64> {
        let bench = self.active_bench();
        (0..self.ticker_names.len())
            .map(|i| r_squared(self.active_returns(i), bench))
            .collect()
    }

    /// Beta for each ticker vs benchmark.
    pub fn beta(&self) -> Vec<BetaResult> {
        let bench = self.active_bench();
        (0..self.ticker_names.len())
            .map(|i| calc_beta(self.active_returns(i), bench))
            .collect()
    }

    /// Greeks (alpha, beta, r²) for each ticker vs benchmark.
    pub fn greeks(&self) -> Vec<GreeksResult> {
        let bench = self.active_bench();
        (0..self.ticker_names.len())
            .map(|i| greeks(self.active_returns(i), bench, self.ann()))
            .collect()
    }

    /// Rolling greeks (alpha, beta) for a specific ticker vs the benchmark.
    ///
    /// # Arguments
    ///
    /// * `ticker_idx` - Zero-based column index of the portfolio ticker.
    /// * `window`     - Look-back window length in periods.
    ///
    /// # Returns
    ///
    /// A [`RollingGreeks`] with parallel date, alpha, and beta vectors.
    pub fn rolling_greeks(&self, ticker_idx: usize, window: usize) -> RollingGreeks {
        rolling_greeks(
            self.active_returns(ticker_idx),
            self.active_bench(),
            self.active_dates(),
            window,
            self.ann(),
        )
    }

    /// Batting average for each ticker vs benchmark.
    ///
    /// Fraction of periods where the ticker outperforms the benchmark.
    pub fn batting_average(&self) -> Vec<f64> {
        let bench = self.active_bench();
        (0..self.ticker_names.len())
            .map(|i| batting_average(self.active_returns(i), bench))
            .collect()
    }

    /// M-squared (Modigliani-Modigliani) for each ticker.
    ///
    /// # Arguments
    ///
    /// * `risk_free_rate` - Annualized risk-free rate.
    pub fn m_squared(&self, risk_free_rate: f64) -> Vec<f64> {
        let ann = self.ann();
        let bench = self.active_bench();
        let bench_vol = risk_metrics::volatility(bench, true, ann);
        (0..self.ticker_names.len())
            .map(|i| {
                let r = self.active_returns(i);
                let ann_ret = risk_metrics::mean_return(r, true, ann);
                let ann_vol = risk_metrics::volatility(r, true, ann);
                risk_metrics::m_squared(ann_ret, ann_vol, bench_vol, risk_free_rate)
            })
            .collect()
    }

    /// Modified Sharpe ratio for each ticker.
    ///
    /// # Arguments
    ///
    /// * `risk_free_rate` - Annualized risk-free rate.
    /// * `confidence`     - VaR confidence level (e.g., `0.95`).
    pub fn modified_sharpe(&self, risk_free_rate: f64, confidence: f64) -> Vec<f64> {
        (0..self.ticker_names.len())
            .map(|i| {
                risk_metrics::modified_sharpe(
                    self.active_returns(i),
                    risk_free_rate,
                    confidence,
                    self.ann(),
                )
            })
            .collect()
    }

    /// Rolling Sharpe ratio for a specific ticker.
    ///
    /// # Arguments
    ///
    /// * `ticker_idx`     - Zero-based column index of the ticker.
    /// * `window`         - Look-back window length in periods.
    /// * `risk_free_rate` - Annualized risk-free rate to subtract.
    ///
    /// # Returns
    ///
    /// A [`RollingSharpe`] with parallel date and Sharpe value vectors.
    pub fn rolling_sharpe(
        &self,
        ticker_idx: usize,
        window: usize,
        risk_free_rate: f64,
    ) -> RollingSharpe {
        rolling_sharpe(
            self.active_returns(ticker_idx),
            self.active_dates(),
            window,
            self.ann(),
            risk_free_rate,
        )
    }

    // ── Lookback selectors ──

    /// Compounded returns for each lookback period (MTD, QTD, YTD, FYTD) at `ref_date`.
    ///
    /// # Arguments
    ///
    /// * `ref_date` - Reference date (typically the most recent business day).
    /// * `fiscal_config` - Optional fiscal year configuration. If `None`,
    ///   `fytd` in the result will be `None`.
    ///
    /// # Returns
    ///
    /// A [`LookbackReturns`] with per-ticker compounded returns for each horizon.
    pub fn lookback_returns(
        &self,
        ref_date: Date,
        fiscal_config: Option<FiscalConfig>,
    ) -> LookbackReturns {
        let dates = self.active_dates();
        let mtd = lookback::mtd_select(dates, ref_date, 0);
        let qtd = lookback::qtd_select(dates, ref_date, 0);
        let ytd = lookback::ytd_select(dates, ref_date, 0);
        let fytd = fiscal_config.map(|fc| lookback::fytd_select(dates, ref_date, fc, 0));

        let compute = |range: &core::ops::Range<usize>| -> Vec<f64> {
            (0..self.ticker_names.len())
                .map(|i| {
                    let r = self.active_returns(i);
                    let slice = &r[range.start..range.end.min(r.len())];
                    comp_total(slice)
                })
                .collect()
        };

        LookbackReturns {
            mtd: compute(&mtd),
            qtd: compute(&qtd),
            ytd: compute(&ytd),
            fytd: fytd.map(|r| compute(&r)),
        }
    }

    // ── Aggregation ──

    /// Period-aggregated statistics for a specific ticker.
    ///
    /// Groups daily returns into `agg_freq` buckets, compounds within each
    /// bucket, then derives win rate, payoff ratio, Kelly criterion, and
    /// more from the resulting period-level return series.
    ///
    /// # Arguments
    ///
    /// * `ticker_idx` - Zero-based column index of the ticker.
    /// * `agg_freq` - Aggregation frequency (e.g., `Monthly`, `Annual`).
    /// * `fiscal_config` - Fiscal year configuration, used when `agg_freq`
    ///   is `Annual` and a non-calendar year is needed.
    ///
    /// # Returns
    ///
    /// A [`PeriodStats`] struct covering all periods in the active date range.
    pub fn period_stats(
        &self,
        ticker_idx: usize,
        agg_freq: PeriodKind,
        fiscal_config: Option<FiscalConfig>,
    ) -> PeriodStats {
        let grouped = group_by_period(
            self.active_dates(),
            self.active_returns(ticker_idx),
            agg_freq,
            fiscal_config,
        );
        period_stats(&grouped)
    }

    /// Pearson correlation matrix of all tickers.
    ///
    /// Computes pairwise correlations over the active date window.
    /// The diagonal is always `1.0`.
    ///
    /// # Returns
    ///
    /// An `n × n` matrix (outer Vec = rows, inner Vec = columns) where
    /// `n = ticker_names().len()`.
    pub fn correlation_matrix(&self) -> Vec<Vec<f64>> {
        let n = self.ticker_names.len();
        let mut matrix = vec![vec![0.0; n]; n];
        for (i, row) in matrix.iter_mut().enumerate().take(n) {
            let ri = self.active_returns(i);
            for (j, cell) in row.iter_mut().enumerate().take(n) {
                let rj = self.active_returns(j);
                let len = ri.len().min(rj.len());
                *cell = crate::math::stats::correlation(&ri[..len], &rj[..len]);
            }
        }
        matrix
    }

    // ── Outperformance ──

    /// Cumulative outperformance (portfolio cumulative return − benchmark cumulative return).
    pub fn cumulative_returns_outperformance(&self) -> Vec<Vec<f64>> {
        let bench_cum = comp_sum(self.active_bench());
        (0..self.ticker_names.len())
            .map(|i| {
                let port_cum = comp_sum(self.active_returns(i));
                port_cum
                    .iter()
                    .zip(bench_cum.iter())
                    .map(|(p, b)| p - b)
                    .collect()
            })
            .collect()
    }

    /// Drawdown outperformance (portfolio drawdown − benchmark drawdown).
    pub fn drawdown_outperformance(&self) -> Vec<Vec<f64>> {
        let range = self.active_range();
        let end = range.end.min(self.bench_drawdown.len());
        let bench_dd = &self.bench_drawdown[range.start.min(end)..end];
        (0..self.ticker_names.len())
            .map(|i| {
                let dd = self.active_drawdown(i);
                dd.iter().zip(bench_dd.iter()).map(|(p, b)| p - b).collect()
            })
            .collect()
    }

    /// The top-N benchmark drawdown episodes (for stress-test analysis).
    ///
    /// Identifies the `n` worst drawdown episodes in the benchmark series.
    /// Useful for examining how the portfolio performs during the benchmark's
    /// worst historical periods.
    ///
    /// # Arguments
    ///
    /// * `n` - Maximum number of episodes to return, sorted by severity.
    ///
    /// # Returns
    ///
    /// Up to `n` [`DrawdownEpisode`] structs from the benchmark series.
    pub fn stats_during_bench_drawdowns(&self, n: usize) -> Vec<DrawdownEpisode> {
        let range = self.active_range();
        let end = range.end.min(self.bench_drawdown.len());
        let bench_dd = &self.bench_drawdown[range.start.min(end)..end];
        drawdown_details(bench_dd, self.active_dates(), n)
    }

    // ── Excess returns ──

    /// Excess returns (portfolio minus risk-free) for each ticker.
    ///
    /// # Arguments
    ///
    /// * `rf` - Risk-free rate series aligned with the active date window.
    /// * `nperiods` - If `Some(n)`, de-compounds the risk-free rate from annual
    ///   to the observation frequency before subtraction.
    ///
    /// # Returns
    ///
    /// One excess-return series per ticker.
    pub fn excess_returns(&self, rf: &[f64], nperiods: Option<f64>) -> Vec<Vec<f64>> {
        (0..self.ticker_names.len())
            .map(|i| excess_returns(self.active_returns(i), rf, nperiods))
            .collect()
    }

    // ── Accessors ──

    /// Active date vector (adjusted for return computation).
    pub fn dates(&self) -> &[Date] {
        &self.dates
    }
    /// Ticker names in column order.
    pub fn ticker_names(&self) -> &[String] {
        &self.ticker_names
    }
    /// Index of the benchmark ticker.
    pub fn benchmark_idx(&self) -> usize {
        self.benchmark_idx
    }
    /// Observation frequency.
    pub fn freq(&self) -> PeriodKind {
        self.freq
    }
    /// Whether log returns are used internally.
    pub fn uses_log_returns(&self) -> bool {
        self.log_returns
    }
}

/// Lookback returns for each period horizon.
#[derive(Debug, Clone)]
pub struct LookbackReturns {
    /// Month-to-date compounded return per ticker.
    pub mtd: Vec<f64>,
    /// Quarter-to-date compounded return per ticker.
    pub qtd: Vec<f64>,
    /// Year-to-date compounded return per ticker.
    pub ytd: Vec<f64>,
    /// Fiscal-year-to-date compounded return per ticker (None if no fiscal config).
    pub fytd: Option<Vec<f64>>,
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use time::Month;

    fn make_dates(n: usize) -> Vec<Date> {
        (0..n)
            .map(|i| {
                Date::from_calendar_date(2025, Month::January, 1).expect("valid")
                    + time::Duration::days(i as i64)
            })
            .collect()
    }

    fn make_prices(n: usize) -> Vec<f64> {
        let mut prices = Vec::with_capacity(n);
        let mut p = 100.0;
        prices.push(p);
        for i in 1..n {
            p *= 1.0 + (i as f64 * 0.001);
            prices.push(p);
        }
        prices
    }

    #[test]
    fn performance_construction() {
        let dates = make_dates(50);
        let p1 = make_prices(50);
        let p2: Vec<f64> = p1.iter().map(|&x| x * 0.95).collect();
        let perf = Performance::new(
            dates,
            vec![p1, p2],
            vec!["A".into(), "B".into()],
            Some("A"),
            PeriodKind::Daily,
            false,
        )
        .expect("construction");

        let cagrs = perf.cagr();
        assert_eq!(cagrs.len(), 2);
        assert!(cagrs[0] > 0.0);
    }

    #[test]
    fn sharpe_sortino_calmar() {
        let dates = make_dates(100);
        let p1 = make_prices(100);
        let perf = Performance::new(
            dates,
            vec![p1],
            vec!["A".into()],
            None,
            PeriodKind::Daily,
            false,
        )
        .expect("construction");

        let sharpe = perf.sharpe(0.0);
        assert_eq!(sharpe.len(), 1);

        let sortino = perf.sortino();
        assert_eq!(sortino.len(), 1);

        let calmar = perf.calmar();
        assert_eq!(calmar.len(), 1);
    }

    #[test]
    fn benchmark_relative_metrics() {
        let dates = make_dates(100);
        let p1 = make_prices(100);
        let p2: Vec<f64> = p1.iter().map(|&x| x * 1.05).collect();
        let perf = Performance::new(
            dates,
            vec![p1, p2],
            vec!["bench".into(), "port".into()],
            Some("bench"),
            PeriodKind::Daily,
            false,
        )
        .expect("construction");

        let te = perf.tracking_error();
        assert_eq!(te.len(), 2);

        let ir = perf.information_ratio();
        assert_eq!(ir.len(), 2);

        let r2 = perf.r_squared();
        assert_eq!(r2.len(), 2);
    }

    #[test]
    fn correlation_matrix_square() {
        let dates = make_dates(50);
        let p1 = make_prices(50);
        let p2: Vec<f64> = p1.iter().map(|&x| x * 0.9).collect();
        let perf = Performance::new(
            dates,
            vec![p1, p2],
            vec!["A".into(), "B".into()],
            None,
            PeriodKind::Daily,
            false,
        )
        .expect("construction");

        let corr = perf.correlation_matrix();
        assert_eq!(corr.len(), 2);
        assert_eq!(corr[0].len(), 2);
        assert!((corr[0][0] - 1.0).abs() < 1e-10);
    }
}
