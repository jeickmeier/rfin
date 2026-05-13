//! Stateful `Performance` struct that orchestrates all analytics sub-modules.
//!
//! Mirrors the Python `Performance` class 1:1 (minus plotting), operating on
//! internal slices and returning numeric results.

use crate::dates::{Date, Duration, FiscalConfig, HolidayCalendar, PeriodKind};

use super::aggregation::{group_by_period, period_stats_from_grouped, PeriodStats};
use super::benchmark::{
    batting_average, beta, capture_ratio, down_capture, greeks, information_ratio,
    multi_factor_greeks, r_squared, rolling_greeks, tracking_error, up_capture, BetaResult,
    GreeksResult, MultiFactorResult, RollingGreeks,
};
use super::benchmark::{m_squared, treynor};
use super::drawdown::{
    burke_ratio, calmar, cdar, drawdown_details, martin_ratio, max_drawdown,
    max_drawdown_duration as dd_max_duration, mean_drawdown, pain_index, pain_ratio,
    recovery_factor, sterling_ratio, to_drawdown_series, ulcer_index, DrawdownEpisode,
};
use super::lookback;
use super::returns::{clean_returns, comp_sum, comp_total, excess_returns, simple_returns};
use super::risk_metrics::{
    self, rolling_sharpe, rolling_sortino, rolling_volatility, DatedSeries, RollingSharpe,
    RollingSortino, RollingVolatility,
};

/// Central performance analytics engine.
///
/// Holds pre-computed returns, drawdowns, and benchmark data for a universe of
/// tickers. Methods delegate to the pure-function sub-modules.
///
/// The facade follows one shape convention throughout: scalar methods return
/// one value per ticker in the same order as `ticker_names`, while per-ticker
/// rolling and episode methods take a zero-based ticker index.
///
/// # Examples
///
/// ```rust
/// use finstack_analytics::Performance;
/// use finstack_core::dates::{Date, Month, PeriodKind};
///
/// let dates: Vec<Date> = (1..=6)
///     .map(|d| Date::from_calendar_date(2025, Month::January, d).unwrap())
///     .collect();
/// let benchmark = vec![100.0, 101.0, 99.0, 102.0, 101.0, 103.0];
/// let portfolio = vec![100.0, 103.0, 100.0, 104.0, 102.0, 106.0];
///
/// let mut perf = Performance::new(
///     dates,
///     vec![benchmark, portfolio],
///     vec!["SPY".to_string(), "ALPHA".to_string()],
///     Some("SPY"),
///     PeriodKind::Daily,
/// )?;
///
/// let sharpe = perf.sharpe(0.02);
/// let beta = perf.beta();
/// let rolling = perf.rolling_sharpe(1, 3, 0.02);
/// assert_eq!(sharpe.len(), 2);
/// assert_eq!(beta.len(), 2);
/// assert_eq!(rolling.values.len(), 3);
///
/// perf.reset_date_range(
///     Date::from_calendar_date(2025, Month::January, 3).unwrap(),
///     Date::from_calendar_date(2025, Month::January, 6).unwrap(),
/// );
/// assert_eq!(perf.cagr()?.len(), 2);
/// # Ok::<(), finstack_core::Error>(())
/// ```
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Performance {
    price_dates: Vec<Date>,
    dates: Vec<Date>,
    returns: Vec<Vec<f64>>,
    ticker_names: Vec<String>,
    benchmark_idx: usize,
    drawdowns: Vec<Vec<f64>>,
    active_window_drawdowns: Option<Vec<Vec<f64>>>,
    freq: PeriodKind,
    start_idx: usize,
    end_idx: usize,
}

impl Performance {
    /// Construct from a price matrix (columns = tickers).
    ///
    /// Computes simple returns for each ticker, builds the drawdown
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
    /// * `benchmark_ticker` - Name of the benchmark ticker. Uses column 0 if
    ///   `None`; returns an error if a non-`None` ticker name is not found.
    /// * `freq` - Observation frequency, used to derive the annualization factor.
    /// # Returns
    ///
    /// A fully initialized [`Performance`] instance, or an error if
    /// validation fails.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::InputError::Invalid`] when:
    ///
    /// * `prices` or `dates` is empty.
    /// * `ticker_names.len() != prices.len()`.
    /// * any price column length differs from `dates.len()`.
    /// * `benchmark_ticker` is supplied but not found in `ticker_names`.
    /// * derived returns are non-finite or below `-1.0`.
    ///
    /// # Tracing
    ///
    /// Emits a `debug`-level `tracing` span named `Performance::new` with
    /// `n_tickers`, `n_dates`, and `freq` fields.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_analytics::Performance;
    /// use finstack_core::dates::{Date, Month, PeriodKind};
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
    /// ).unwrap();
    /// assert_eq!(perf.ticker_names(), &["SPY"]);
    /// ```
    #[tracing::instrument(level = "debug", skip(dates, prices, ticker_names, benchmark_ticker), fields(n_tickers = prices.len(), n_dates = dates.len(), freq = ?freq))]
    pub fn new(
        dates: Vec<Date>,
        prices: Vec<Vec<f64>>,
        ticker_names: Vec<String>,
        benchmark_ticker: Option<&str>,
        freq: PeriodKind,
    ) -> crate::Result<Self> {
        if prices.is_empty() || dates.is_empty() {
            return Err(crate::error::InputError::InvalidReturnSeries {
                ticker: "<panel>".into(),
                index: 0,
                reason: "prices or dates is empty".into(),
            }
            .into());
        }
        if let Some((col_idx, bad)) = prices
            .iter()
            .enumerate()
            .find(|(_, price_col)| price_col.len() != dates.len())
        {
            let ticker = ticker_names
                .get(col_idx)
                .cloned()
                .unwrap_or_else(|| format!("col[{col_idx}]"));
            return Err(crate::error::InputError::InvalidReturnSeries {
                ticker,
                index: 0,
                reason: format!(
                    "price column length {} does not match dates length {}",
                    bad.len(),
                    dates.len()
                ),
            }
            .into());
        }

        let returns_matrix: Vec<Vec<f64>> = prices
            .iter()
            .map(|price_col| simple_returns(price_col)[1..].to_vec())
            .collect();
        let return_dates = if dates.len() > 1 {
            dates[1..].to_vec()
        } else {
            dates.clone()
        };
        Self::assemble(
            dates,
            return_dates,
            returns_matrix,
            ticker_names,
            benchmark_ticker,
            freq,
        )
    }

    /// Construct from a pre-computed return matrix (columns = tickers).
    ///
    /// Use this when you already have a return panel and want to skip the
    /// price → return conversion handled by [`Self::new`]. The supplied
    /// `dates` are the return-aligned observation dates (one entry per
    /// return row).
    ///
    /// A synthetic prior date is prepended to the internal price-date grid
    /// so that CAGR and other date-aware metrics see a holding period of
    /// `dates.len()` periods. The prior date is derived from the first
    /// observed gap (`dates[1] - dates[0]`) when at least two dates are
    /// supplied, and otherwise falls back to `dates[0]`.
    ///
    /// # Arguments
    ///
    /// * `dates` - Chronologically sorted return-aligned dates.
    /// * `returns` - Return matrix: `returns[i]` is the simple-return series
    ///   for ticker `i`, with one entry per `dates` row.
    /// * `ticker_names` - Names corresponding to each column of `returns`.
    /// * `benchmark_ticker` - Name of the benchmark ticker. Uses column 0 if
    ///   `None`; returns an error if a non-`None` ticker name is not found.
    /// * `freq` - Observation frequency, used to derive the annualization factor.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::InputError::Invalid`] when inputs are empty,
    /// the column count does not match `ticker_names`, any return column has
    /// the wrong length, the benchmark name is unknown, or any return value
    /// is non-finite or `< -1.0`.
    pub fn from_returns(
        dates: Vec<Date>,
        returns: Vec<Vec<f64>>,
        ticker_names: Vec<String>,
        benchmark_ticker: Option<&str>,
        freq: PeriodKind,
    ) -> crate::Result<Self> {
        if returns.is_empty() || dates.is_empty() {
            return Err(crate::error::InputError::InvalidReturnSeries {
                ticker: "<panel>".into(),
                index: 0,
                reason: "returns or dates is empty".into(),
            }
            .into());
        }

        let prior_date = if dates.len() >= 2 {
            let gap = (dates[1] - dates[0]).whole_days();
            dates[0]
                .checked_sub(Duration::days(gap))
                .unwrap_or(dates[0])
        } else {
            dates[0]
        };
        let mut price_dates = Vec::with_capacity(dates.len() + 1);
        price_dates.push(prior_date);
        price_dates.extend_from_slice(&dates);

        Self::assemble(
            price_dates,
            dates,
            returns,
            ticker_names,
            benchmark_ticker,
            freq,
        )
    }

    /// Validate return columns, build per-ticker drawdown caches, and finalize state.
    ///
    /// Shared by [`Self::new`] (which pre-computes simple returns from prices)
    /// and [`Self::from_returns`] (which receives a return matrix directly).
    fn assemble(
        price_dates: Vec<Date>,
        return_dates: Vec<Date>,
        returns: Vec<Vec<f64>>,
        ticker_names: Vec<String>,
        benchmark_ticker: Option<&str>,
        freq: PeriodKind,
    ) -> crate::Result<Self> {
        if ticker_names.len() != returns.len() {
            return Err(crate::error::InputError::InvalidReturnSeries {
                ticker: "<panel>".into(),
                index: 0,
                reason: format!(
                    "ticker_names.len() = {} does not match returns.len() = {}",
                    ticker_names.len(),
                    returns.len()
                ),
            }
            .into());
        }

        let benchmark_idx = match benchmark_ticker {
            Some(name) => ticker_names.iter().position(|t| t == name).ok_or_else(|| {
                crate::error::InputError::InvalidReturnSeries {
                    ticker: name.to_string(),
                    index: 0,
                    reason: "benchmark ticker not found among supplied ticker_names".into(),
                }
            })?,
            None => 0,
        };

        let expected_len = return_dates.len();
        let mut all_returns: Vec<Vec<f64>> = Vec::with_capacity(returns.len());
        let mut all_drawdowns: Vec<Vec<f64>> = Vec::with_capacity(returns.len());
        for (col_idx, col) in returns.into_iter().enumerate() {
            let ticker = ticker_names[col_idx].clone();
            if col.len() != expected_len {
                return Err(crate::error::InputError::InvalidReturnSeries {
                    ticker,
                    index: col.len().min(expected_len),
                    reason: format!(
                        "column length {} does not match return-date grid length {}",
                        col.len(),
                        expected_len
                    ),
                }
                .into());
            }
            let mut clean = col;
            clean_returns(&mut clean, &ticker);
            if clean.len() != expected_len {
                return Err(crate::error::InputError::InvalidReturnSeries {
                    ticker,
                    index: clean.len(),
                    reason: format!(
                        "{} trailing NaN/non-finite rows stripped; series no longer aligned with date grid (length {} vs expected {})",
                        expected_len - clean.len(),
                        clean.len(),
                        expected_len
                    ),
                }
                .into());
            }
            if let Some((index, value)) = clean
                .iter()
                .enumerate()
                .find(|(_, &v)| !v.is_finite() || v <= -1.0)
            {
                let reason = if !value.is_finite() {
                    format!("non-finite return ({value})")
                } else {
                    format!(
                        "return <= -1.0 ({value}); total wipeout makes downstream metrics meaningless"
                    )
                };
                return Err(crate::error::InputError::InvalidReturnSeries {
                    ticker,
                    index,
                    reason,
                }
                .into());
            }
            let dd = to_drawdown_series(&clean);
            all_drawdowns.push(dd);
            all_returns.push(clean);
        }

        let end_idx = all_returns.first().map_or(0, Vec::len);

        Ok(Self {
            price_dates,
            dates: return_dates,
            returns: all_returns,
            ticker_names,
            benchmark_idx,
            drawdowns: all_drawdowns,
            active_window_drawdowns: None,
            freq,
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
    /// # Drawdown semantics on a windowed range
    ///
    /// Drawdown caches are **rebuilt from scratch** within the new window:
    /// the peak watermark is reset to the first observation of the active
    /// range, so any drawdown that began before `start` is *not* carried
    /// over. As a consequence:
    ///
    /// - [`Self::max_drawdown`], [`Self::mean_drawdown`],
    ///   [`Self::drawdown_series`], [`Self::drawdown_details`],
    ///   [`Self::ulcer_index`], [`Self::pain_index`], [`Self::cdar`],
    ///   [`Self::recovery_factor`], [`Self::sterling_ratio`],
    ///   [`Self::burke_ratio`], [`Self::martin_ratio`],
    ///   [`Self::pain_ratio`], [`Self::calmar`], and
    ///   [`Self::max_drawdown_duration`] all reflect drawdowns measured
    ///   *only* over `[start, end]`.
    /// - To preserve a watermark from before `start`, call these methods on
    ///   the un-windowed `Performance` first or fork the instance.
    ///
    /// # Arguments
    ///
    /// * `start` - First date to include (inclusive).
    /// * `end`   - Last date to include (inclusive).
    pub fn reset_date_range(&mut self, start: Date, end: Date) {
        self.start_idx = self.dates.partition_point(|&d| d < start);
        self.end_idx = self.dates.partition_point(|&d| d <= end);
        self.refresh_active_drawdown_cache();
    }

    /// Designate a different ticker as the benchmark for all subsequent analytics.
    ///
    /// Updates `benchmark_idx`; all benchmark-aware accessors derive their
    /// series from `returns[benchmark_idx]` / `drawdowns[benchmark_idx]`
    /// (or the active windowed-drawdown cache when a date range is set).
    ///
    /// # Arguments
    ///
    /// * `ticker` - Name of the ticker to use as benchmark. Must match one
    ///   of the names provided at construction time.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or an [`crate::error::InputError::Invalid`] if `ticker` is
    /// not found among the loaded tickers.
    pub fn reset_bench_ticker(&mut self, ticker: &str) -> crate::Result<()> {
        let idx = self
            .ticker_names
            .iter()
            .position(|t| t == ticker)
            .ok_or_else(|| crate::error::InputError::InvalidReturnSeries {
                ticker: ticker.to_string(),
                index: 0,
                reason: "ticker not found among loaded ticker_names".into(),
            })?;
        self.benchmark_idx = idx;
        Ok(())
    }

    fn active_range(&self) -> core::ops::Range<usize> {
        self.start_idx..self.end_idx
    }

    fn full_range_len(&self) -> usize {
        self.returns.first().map_or(0, Vec::len)
    }

    fn using_full_range(&self) -> bool {
        self.start_idx == 0 && self.end_idx >= self.full_range_len()
    }

    fn refresh_active_drawdown_cache(&mut self) {
        if self.using_full_range() {
            self.active_window_drawdowns = None;
            return;
        }

        self.active_window_drawdowns = Some(
            self.returns
                .iter()
                .map(|series| {
                    let end = self.end_idx.min(series.len());
                    let start = self.start_idx.min(end);
                    to_drawdown_series(&series[start..end])
                })
                .collect(),
        );
    }

    fn active_holding_period(&self) -> Option<(Date, Date)> {
        let range = self.active_range();
        if range.start >= range.end || self.price_dates.len() < 2 {
            return None;
        }
        let last_price_idx = self.price_dates.len() - 1;
        let start_idx = range.start.min(last_price_idx);
        let end_idx = range.end.min(last_price_idx);
        if start_idx >= end_idx {
            return None;
        }
        Some((self.price_dates[start_idx], self.price_dates[end_idx]))
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
        self.active_returns(self.benchmark_idx)
    }

    /// Date slice corresponding to the currently active analysis window.
    pub fn active_dates(&self) -> &[Date] {
        let range = self.active_range();
        let end = range.end.min(self.dates.len());
        &self.dates[range.start.min(end)..end]
    }

    fn active_drawdown_values(&self, ticker_idx: usize) -> &[f64] {
        if self.using_full_range() {
            return self
                .drawdowns
                .get(ticker_idx)
                .map(Vec::as_slice)
                .unwrap_or(&[]);
        }

        self.active_window_drawdowns
            .as_ref()
            .and_then(|drawdowns| drawdowns.get(ticker_idx))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    fn active_bench_drawdown_values(&self) -> &[f64] {
        self.active_drawdown_values(self.benchmark_idx)
    }

    fn ann(&self) -> f64 {
        self.freq.annualization_factor()
    }

    /// Map a per-ticker closure over all tickers in column order.
    ///
    /// Centralises the `(0..n_tickers).map(..).collect()` idiom used
    /// throughout the scalar-metric API.
    #[inline]
    fn map_tickers<T, F>(&self, f: F) -> Vec<T>
    where
        F: FnMut(usize) -> T,
    {
        (0..self.ticker_names.len()).map(f).collect()
    }

    // ── Scalar metrics per ticker ──

    /// Compound annual growth rate for each ticker.
    ///
    /// Uses the active date window and annualizes from the actual holding
    /// period implied by the price-date grid.
    ///
    /// # Returns
    ///
    /// One CAGR per ticker in column order.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::InputError::Invalid`] if the active range has
    /// no valid positive holding period.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use finstack_analytics::Performance;
    /// # use finstack_core::dates::{Date, Month, PeriodKind};
    /// # let dates: Vec<Date> = (1..=4)
    /// #     .map(|d| Date::from_calendar_date(2025, Month::January, d).unwrap())
    /// #     .collect();
    /// # let perf = Performance::new(
    /// #     dates,
    /// #     vec![vec![100.0, 101.0, 102.0, 103.0]],
    /// #     vec!["SPY".to_string()],
    /// #     None,
    /// #     PeriodKind::Daily,
    /// # )?;
    /// let cagr = perf.cagr()?;
    /// assert_eq!(cagr.len(), 1);
    /// # Ok::<(), finstack_core::Error>(())
    /// ```
    pub fn cagr(&self) -> crate::Result<Vec<f64>> {
        let Some((start, end)) = self.active_holding_period() else {
            return Err(crate::error::InputError::InvalidReturnSeries {
                ticker: "<panel>".into(),
                index: self.start_idx,
                reason: format!(
                    "active range [{}..{}] has no positive holding period on the price-date grid",
                    self.start_idx, self.end_idx
                ),
            }
            .into());
        };
        let basis = risk_metrics::CagrBasis::dates(start, end);
        (0..self.ticker_names.len())
            .map(|i| risk_metrics::cagr(self.active_returns(i), basis))
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
        self.map_tickers(|i| {
            risk_metrics::mean_return(self.active_returns(i), annualize, self.ann())
        })
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
        self.map_tickers(|i| {
            risk_metrics::volatility(self.active_returns(i), annualize, self.ann())
        })
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
        self.map_tickers(|i| {
            let r = self.active_returns(i);
            let m = risk_metrics::mean_return(r, true, ann);
            let v = risk_metrics::volatility(r, true, ann);
            risk_metrics::sharpe(m, v, risk_free_rate)
        })
    }

    /// Annualized Sortino ratio for each ticker.
    ///
    /// Uses the active date window, annualizes with the observation frequency
    /// configured on this [`Performance`] instance, and uses the supplied
    /// minimum acceptable return.
    ///
    /// # Returns
    ///
    /// One Sortino ratio per ticker in column order. May return `±∞` for
    /// tickers with zero downside deviation and nonzero mean return.
    pub fn sortino(&self, mar: f64) -> Vec<f64> {
        self.map_tickers(|i| risk_metrics::sortino(self.active_returns(i), true, self.ann(), mar))
    }

    /// Calmar ratio for each ticker.
    ///
    /// Computes CAGR over the active date window and divides by the absolute
    /// value of each ticker's worst drawdown over that same window.
    ///
    /// # Returns
    ///
    /// One Calmar ratio per ticker in column order. Returns `0.0` for tickers
    /// with no observed drawdown.
    ///
    /// # Errors
    ///
    /// Propagates errors from [`Self::cagr`] when the active range cannot be
    /// annualized.
    pub fn calmar(&self) -> crate::Result<Vec<f64>> {
        let cagrs = self.cagr()?;
        Ok(self.map_tickers(|i| calmar(cagrs[i], max_drawdown(self.active_drawdown_values(i)))))
    }

    /// Maximum drawdown for each ticker.
    ///
    /// # Returns
    ///
    /// One non-positive maximum drawdown per ticker in column order.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use finstack_analytics::Performance;
    /// # use finstack_core::dates::{Date, Month, PeriodKind};
    /// # let dates: Vec<Date> = (1..=4)
    /// #     .map(|d| Date::from_calendar_date(2025, Month::January, d).unwrap())
    /// #     .collect();
    /// # let perf = Performance::new(
    /// #     dates,
    /// #     vec![vec![100.0, 105.0, 99.0, 106.0]],
    /// #     vec!["SPY".to_string()],
    /// #     None,
    /// #     PeriodKind::Daily,
    /// # )?;
    /// let max_dd = perf.max_drawdown();
    /// assert!(max_dd[0] <= 0.0);
    /// # Ok::<(), finstack_core::Error>(())
    /// ```
    pub fn max_drawdown(&self) -> Vec<f64> {
        self.map_tickers(|i| max_drawdown(self.active_drawdown_values(i)))
    }

    /// Mean drawdown (arithmetic mean of the drawdown path) for each ticker.
    ///
    /// # Returns
    ///
    /// One non-positive mean drawdown per ticker in column order.
    pub fn mean_drawdown(&self) -> Vec<f64> {
        self.map_tickers(|i| mean_drawdown(self.active_drawdown_values(i)))
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
        self.map_tickers(|i| risk_metrics::value_at_risk(self.active_returns(i), confidence))
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
        self.map_tickers(|i| risk_metrics::expected_shortfall(self.active_returns(i), confidence))
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
        self.map_tickers(|i| risk_metrics::tail_ratio(self.active_returns(i), confidence))
    }

    /// Ulcer Index for each ticker.
    ///
    /// Measures drawdown-based risk from the active drawdown path rather than
    /// return volatility.
    ///
    /// # Returns
    ///
    /// One non-negative Ulcer Index per ticker in column order.
    pub fn ulcer_index(&self) -> Vec<f64> {
        self.map_tickers(|i| ulcer_index(self.active_drawdown_values(i)))
    }

    /// Bias-corrected sample skewness for each ticker.
    ///
    /// # Returns
    ///
    /// One skewness estimate per ticker in column order. Positive values
    /// indicate a heavier right tail.
    pub fn skewness(&self) -> Vec<f64> {
        self.map_tickers(|i| risk_metrics::skewness(self.active_returns(i)))
    }

    /// Bias-corrected sample excess kurtosis for each ticker.
    ///
    /// # Returns
    ///
    /// One excess-kurtosis estimate per ticker in column order. Positive
    /// values indicate fatter tails than a normal distribution.
    pub fn kurtosis(&self) -> Vec<f64> {
        self.map_tickers(|i| risk_metrics::kurtosis(self.active_returns(i)))
    }

    /// Geometric mean return for each ticker.
    ///
    /// # Returns
    ///
    /// One per-period geometric mean return per ticker in column order, using
    /// the active return window.
    pub fn geometric_mean(&self) -> Vec<f64> {
        self.map_tickers(|i| risk_metrics::geometric_mean(self.active_returns(i)))
    }

    /// Annualized downside deviation for each ticker.
    ///
    /// # Arguments
    ///
    /// * `mar` - Minimum acceptable per-period return threshold in decimal
    ///   form (for example, `0.0` or `0.001`).
    ///
    /// # Returns
    ///
    /// One downside-deviation value per ticker in column order, annualized
    /// using the configured observation frequency.
    pub fn downside_deviation(&self, mar: f64) -> Vec<f64> {
        self.map_tickers(|i| {
            risk_metrics::downside_deviation(self.active_returns(i), mar, true, self.ann())
        })
    }

    /// Maximum drawdown duration in calendar days for each ticker.
    ///
    /// Duration is measured on the active date grid, so irregular observation
    /// spacing is reflected in the reported day counts.
    ///
    /// # Returns
    ///
    /// One maximum drawdown duration per ticker in column order.
    pub fn max_drawdown_duration(&self) -> Vec<i64> {
        self.map_tickers(|i| dd_max_duration(self.active_drawdown_values(i), self.active_dates()))
    }

    /// Up-market capture ratio for each ticker versus the active benchmark.
    ///
    /// # Returns
    ///
    /// One up-capture ratio per ticker in column order. Values greater than
    /// `1.0` indicate stronger participation than the benchmark in benchmark-up periods.
    pub fn up_capture(&self) -> Vec<f64> {
        let bench = self.active_bench();
        self.map_tickers(|i| up_capture(self.active_returns(i), bench))
    }

    /// Down-market capture ratio for each ticker versus the active benchmark.
    ///
    /// # Returns
    ///
    /// One down-capture ratio per ticker in column order. Values below `1.0`
    /// indicate the ticker loses less than the benchmark in benchmark-down periods.
    pub fn down_capture(&self) -> Vec<f64> {
        let bench = self.active_bench();
        self.map_tickers(|i| down_capture(self.active_returns(i), bench))
    }

    /// Capture ratio (up-capture divided by down-capture) for each ticker.
    ///
    /// # Returns
    ///
    /// One capture ratio per ticker in column order versus the active benchmark.
    pub fn capture_ratio(&self) -> Vec<f64> {
        let bench = self.active_bench();
        self.map_tickers(|i| capture_ratio(self.active_returns(i), bench))
    }

    /// Rolling compounded returns for a specific ticker.
    ///
    /// Computes the total compounded return over each `window`-length slice
    /// of the active return series, right-labelled by the window-end date.
    /// Produces `n - window + 1` values where `n = active_returns.len()`.
    ///
    /// # Arguments
    ///
    /// * `ticker_idx` - Zero-based column index of the ticker.
    /// * `window`     - Look-back window length in periods (e.g. 252 for 1Y daily).
    pub fn rolling_returns(&self, ticker_idx: usize, window: usize) -> DatedSeries {
        let returns = self.active_returns(ticker_idx);
        let dates = self.active_dates();
        let n = returns.len().min(dates.len());
        if window == 0 || window > n {
            return DatedSeries::default();
        }
        let count = n - window + 1;
        let mut values = Vec::with_capacity(count);
        let mut out_dates = Vec::with_capacity(count);
        for end in window..=n {
            let start = end - window;
            values.push(comp_total(&returns[start..end]));
            out_dates.push(dates[end - 1]);
        }
        DatedSeries {
            values,
            dates: out_dates,
        }
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
    /// * `threshold` - Per-period threshold return in decimal form, typically `0.0`.
    ///
    /// # Returns
    ///
    /// One Omega ratio per ticker in column order over the active window.
    pub fn omega_ratio(&self, threshold: f64) -> Vec<f64> {
        self.map_tickers(|i| risk_metrics::omega_ratio(self.active_returns(i), threshold))
    }

    /// Treynor ratio for each ticker.
    ///
    /// # Arguments
    ///
    /// * `risk_free_rate` - Annualized risk-free rate in decimal form.
    ///
    /// # Returns
    ///
    /// One Treynor ratio per ticker in column order, using the active
    /// benchmark to estimate beta.
    pub fn treynor(&self, risk_free_rate: f64) -> Vec<f64> {
        let ann = self.ann();
        let bench = self.active_bench();
        self.map_tickers(|i| {
            let r = self.active_returns(i);
            let ann_ret = risk_metrics::mean_return(r, true, ann);
            let beta = super::benchmark::greeks(r, bench, ann).beta;
            treynor(ann_ret, risk_free_rate, beta)
        })
    }

    /// Gain-to-pain ratio for each ticker (sum of returns / sum of |losses|).
    ///
    /// # Returns
    ///
    /// One gain-to-pain ratio per ticker in column order over the active
    /// return window.
    pub fn gain_to_pain(&self) -> Vec<f64> {
        self.map_tickers(|i| risk_metrics::gain_to_pain(self.active_returns(i)))
    }

    /// Martin ratio (CAGR divided by Ulcer Index) for each ticker.
    ///
    /// # Returns
    ///
    /// One Martin ratio per ticker in column order. Returns `0.0` for tickers
    /// with zero Ulcer Index.
    ///
    /// # Errors
    ///
    /// Propagates errors from [`Self::cagr`] when the active range cannot be
    /// annualized.
    pub fn martin_ratio(&self) -> crate::Result<Vec<f64>> {
        let cagrs = self.cagr()?;
        Ok(self
            .map_tickers(|i| martin_ratio(cagrs[i], ulcer_index(self.active_drawdown_values(i)))))
    }

    /// Parametric (Gaussian) VaR for each ticker.
    ///
    /// # Arguments
    ///
    /// * `confidence` - Confidence level in `(0, 1)`, e.g. `0.95`.
    ///
    /// # Returns
    ///
    /// One non-positive parametric VaR per ticker in column order.
    pub fn parametric_var(&self, confidence: f64) -> Vec<f64> {
        self.map_tickers(|i| risk_metrics::parametric_var(self.active_returns(i), confidence, None))
    }

    /// Cornish-Fisher adjusted VaR for each ticker.
    ///
    /// # Arguments
    ///
    /// * `confidence` - Confidence level in `(0, 1)`, e.g. `0.95`.
    ///
    /// # Returns
    ///
    /// One non-positive Cornish-Fisher VaR per ticker in column order.
    pub fn cornish_fisher_var(&self, confidence: f64) -> Vec<f64> {
        self.map_tickers(|i| {
            risk_metrics::cornish_fisher_var(self.active_returns(i), confidence, None)
        })
    }

    /// Rolling Sortino ratio for a specific ticker.
    ///
    /// # Arguments
    ///
    /// * `ticker_idx` - Zero-based column index of the ticker.
    /// * `window`     - Look-back window length in periods.
    /// * `mar`        - Minimum acceptable per-period return (decimal). Pass
    ///   `0.0` for the conventional zero-MAR Sortino, or e.g. a risk-free or
    ///   target rate scaled to the observation frequency.
    pub fn rolling_sortino(&self, ticker_idx: usize, window: usize, mar: f64) -> RollingSortino {
        rolling_sortino(
            self.active_returns(ticker_idx),
            self.active_dates(),
            window,
            self.ann(),
            mar,
        )
    }

    // ── Batch 3: Drawdown-family ratios ──

    /// Recovery factor for each ticker.
    ///
    /// # Returns
    ///
    /// One recovery factor per ticker in column order, computed as total
    /// compounded return divided by absolute maximum drawdown.
    pub fn recovery_factor(&self) -> Vec<f64> {
        self.map_tickers(|i| {
            let total_ret = comp_total(self.active_returns(i));
            let max_dd = max_drawdown(self.active_drawdown_values(i));
            recovery_factor(total_ret, max_dd)
        })
    }

    /// Sterling ratio for each ticker.
    ///
    /// # Arguments
    ///
    /// * `risk_free_rate` - Annualized risk-free rate.
    /// * `n` - Number of worst drawdowns to average.
    ///
    /// # Returns
    ///
    /// One Sterling ratio per ticker in column order.
    ///
    /// # Errors
    ///
    /// Propagates errors from [`Self::cagr`] when the active range cannot be
    /// annualized.
    pub fn sterling_ratio(&self, risk_free_rate: f64, n: usize) -> crate::Result<Vec<f64>> {
        let cagrs = self.cagr()?;
        Ok(self.map_tickers(|i| {
            let avg = super::drawdown::mean_episode_drawdown(self.active_drawdown_values(i), n);
            sterling_ratio(cagrs[i], avg, risk_free_rate)
        }))
    }

    /// Burke ratio for each ticker.
    ///
    /// # Arguments
    ///
    /// * `risk_free_rate` - Annualized risk-free rate.
    /// * `n` - Number of worst drawdown episodes to use.
    ///
    /// # Returns
    ///
    /// One Burke ratio per ticker in column order.
    ///
    /// # Errors
    ///
    /// Propagates errors from [`Self::cagr`] when the active range cannot be
    /// annualized.
    pub fn burke_ratio(&self, risk_free_rate: f64, n: usize) -> crate::Result<Vec<f64>> {
        let cagrs = self.cagr()?;
        let dates = self.active_dates();
        Ok(self.map_tickers(|i| {
            let episodes =
                super::drawdown::drawdown_details(self.active_drawdown_values(i), dates, n);
            let dd_vals: Vec<f64> = episodes.iter().map(|e| e.max_drawdown).collect();
            burke_ratio(cagrs[i], &dd_vals, risk_free_rate)
        }))
    }

    /// Pain index for each ticker.
    ///
    /// # Returns
    ///
    /// One non-negative pain index per ticker in column order.
    pub fn pain_index(&self) -> Vec<f64> {
        self.map_tickers(|i| pain_index(self.active_drawdown_values(i)))
    }

    /// Pain ratio for each ticker.
    ///
    /// # Arguments
    ///
    /// * `risk_free_rate` - Annualized risk-free rate.
    ///
    /// # Returns
    ///
    /// One pain ratio per ticker in column order.
    ///
    /// # Errors
    ///
    /// Propagates errors from [`Self::cagr`] when the active range cannot be
    /// annualized.
    pub fn pain_ratio(&self, risk_free_rate: f64) -> crate::Result<Vec<f64>> {
        let cagrs = self.cagr()?;
        Ok(self.map_tickers(|i| {
            let pain = pain_index(self.active_drawdown_values(i));
            pain_ratio(cagrs[i], pain, risk_free_rate)
        }))
    }

    // ── Batch 4: Multi-factor and CDaR ──

    /// Multi-factor regression for a specific ticker.
    ///
    /// # Arguments
    ///
    /// * `ticker_idx`     - Zero-based column index of the ticker.
    /// * `factor_returns` - Slice of factor return series aligned with the
    ///   active date window.
    ///
    /// # Returns
    ///
    /// A [`MultiFactorResult`] for the requested ticker, with annualized alpha
    /// and residual volatility.
    ///
    /// # Errors
    ///
    /// Propagates errors from [`multi_factor_greeks`] when factor inputs are
    /// mismatched, non-finite, insufficient, or numerically singular.
    pub fn multi_factor_greeks(
        &self,
        ticker_idx: usize,
        factor_returns: &[&[f64]],
    ) -> crate::Result<MultiFactorResult> {
        multi_factor_greeks(self.active_returns(ticker_idx), factor_returns, self.ann())
    }

    /// Conditional Drawdown at Risk for each ticker.
    ///
    /// # Arguments
    ///
    /// * `confidence` - Confidence level in `(0, 1)`, e.g. `0.95`.
    ///
    /// # Returns
    ///
    /// One non-positive CDaR value per ticker in column order, matching the
    /// sign convention of [`Self::max_drawdown`] / [`Self::mean_drawdown`].
    /// A 95% CDaR of `-0.25` means the average drawdown in the worst 5% tail
    /// is 25%.
    pub fn cdar(&self, confidence: f64) -> Vec<f64> {
        self.map_tickers(|i| cdar(self.active_drawdown_values(i), confidence))
    }

    // ── Series outputs ──

    /// Cumulative compounded returns for each ticker.
    ///
    /// # Returns
    ///
    /// One cumulative-return path per ticker in column order, computed over
    /// the active return window.
    pub fn cumulative_returns(&self) -> Vec<Vec<f64>> {
        self.map_tickers(|i| comp_sum(self.active_returns(i)))
    }

    /// Drawdown series for each ticker.
    ///
    /// # Returns
    ///
    /// One drawdown path per ticker in column order. Values are non-positive
    /// fractions such as `-0.25` for a 25% drawdown.
    pub fn drawdown_series(&self) -> Vec<Vec<f64>> {
        self.map_tickers(|i| self.active_drawdown_values(i).to_vec())
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
        let dd = self.active_drawdown_values(ticker_idx);
        let dates = self.active_dates();
        drawdown_details(dd, dates, n)
    }

    // ── Benchmark-relative ──

    /// Annualized tracking error for each ticker versus the active benchmark.
    ///
    /// # Returns
    ///
    /// One annualized tracking-error value per ticker in column order.
    pub fn tracking_error(&self) -> Vec<f64> {
        let bench = self.active_bench();
        self.map_tickers(|i| tracking_error(self.active_returns(i), bench, true, self.ann()))
    }

    /// Annualized information ratio for each ticker versus the active benchmark.
    ///
    /// # Returns
    ///
    /// One information ratio per ticker in column order.
    pub fn information_ratio(&self) -> Vec<f64> {
        let bench = self.active_bench();
        self.map_tickers(|i| information_ratio(self.active_returns(i), bench, true, self.ann()))
    }

    /// R-squared for each ticker versus the active benchmark.
    ///
    /// # Returns
    ///
    /// One coefficient-of-determination value in `[0, 1]` per ticker.
    pub fn r_squared(&self) -> Vec<f64> {
        let bench = self.active_bench();
        self.map_tickers(|i| r_squared(self.active_returns(i), bench))
    }

    /// OLS beta estimates for each ticker versus the active benchmark.
    ///
    /// # Returns
    ///
    /// One [`BetaResult`] per ticker in column order, including standard error
    /// and confidence interval information.
    pub fn beta(&self) -> Vec<BetaResult> {
        let bench = self.active_bench();
        self.map_tickers(|i| beta(self.active_returns(i), bench))
    }

    /// Single-factor greeks for each ticker versus the active benchmark.
    ///
    /// Alpha is annualized using the configured observation frequency.
    ///
    /// # Returns
    ///
    /// One [`GreeksResult`] per ticker in column order.
    pub fn greeks(&self) -> Vec<GreeksResult> {
        let bench = self.active_bench();
        self.map_tickers(|i| greeks(self.active_returns(i), bench, self.ann()))
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

    /// Batting average for each ticker versus the active benchmark.
    ///
    /// Fraction of periods where the ticker's return exceeds the benchmark's
    /// return over the active window.
    ///
    /// # Returns
    ///
    /// One batting-average fraction per ticker in column order.
    pub fn batting_average(&self) -> Vec<f64> {
        let bench = self.active_bench();
        self.map_tickers(|i| batting_average(self.active_returns(i), bench))
    }

    /// M-squared (Modigliani-Modigliani) for each ticker.
    ///
    /// # Arguments
    ///
    /// * `risk_free_rate` - Annualized risk-free rate in decimal form.
    ///
    /// # Returns
    ///
    /// One M-squared return per ticker in column order, scaled to the active
    /// benchmark's volatility.
    pub fn m_squared(&self, risk_free_rate: f64) -> Vec<f64> {
        let ann = self.ann();
        let bench = self.active_bench();
        let bench_vol = risk_metrics::volatility(bench, true, ann);
        self.map_tickers(|i| {
            let r = self.active_returns(i);
            let ann_ret = risk_metrics::mean_return(r, true, ann);
            let ann_vol = risk_metrics::volatility(r, true, ann);
            m_squared(ann_ret, ann_vol, bench_vol, risk_free_rate)
        })
    }

    /// Modified Sharpe ratio for each ticker.
    ///
    /// # Arguments
    ///
    /// * `risk_free_rate` - Annualized risk-free rate in decimal form.
    /// * `confidence`     - Cornish-Fisher VaR confidence level in `(0, 1)`.
    ///
    /// # Returns
    ///
    /// One modified Sharpe ratio per ticker in column order.
    pub fn modified_sharpe(&self, risk_free_rate: f64, confidence: f64) -> Vec<f64> {
        self.map_tickers(|i| {
            risk_metrics::modified_sharpe(
                self.active_returns(i),
                risk_free_rate,
                confidence,
                self.ann(),
            )
        })
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
    /// Each horizon is computed by selecting the corresponding index range on
    /// the active date grid, then compounding the in-range simple returns for
    /// every ticker.
    ///
    /// # Arguments
    ///
    /// * `ref_date`      - Reference date (typically the most recent business day).
    /// * `fiscal_config` - Fiscal year configuration used for FYTD.
    /// * `calendar`      - Holiday calendar used to align the FYTD start.
    ///
    /// # Returns
    ///
    /// A [`LookbackReturns`] with per-ticker compounded returns for each
    /// horizon. If a lookback range is empty for a given ticker, the
    /// corresponding compounded return is `0.0`.
    ///
    /// FYTD start is aligned to the next business day per the supplied
    /// calendar.
    ///
    /// # Errors
    /// Returns an error if the fiscal start cannot be adjusted on the supplied
    /// calendar.
    pub fn lookback_returns(
        &self,
        ref_date: Date,
        fiscal_config: FiscalConfig,
        calendar: &dyn HolidayCalendar,
    ) -> crate::Result<LookbackReturns> {
        let dates = self.active_dates();
        let mtd = lookback::mtd_select(dates, ref_date);
        let qtd = lookback::qtd_select(dates, ref_date);
        let ytd = lookback::ytd_select(dates, ref_date);
        let fytd = lookback::fytd_select(dates, ref_date, fiscal_config, calendar)?;

        let compute = |range: &core::ops::Range<usize>| -> Vec<f64> {
            self.map_tickers(|i| {
                let r = self.active_returns(i);
                let start = range.start.min(r.len());
                let end = range.end.min(r.len()).max(start);
                comp_total(&r[start..end])
            })
        };

        Ok(LookbackReturns {
            mtd: compute(&mtd),
            qtd: compute(&qtd),
            ytd: compute(&ytd),
            fytd: Some(compute(&fytd)),
        })
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
    /// A [`PeriodStats`] struct covering all aggregation buckets in the active
    /// date range for the requested ticker.
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
        period_stats_from_grouped(&grouped)
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

        let variances: Vec<f64> = (0..n)
            .map(|i| crate::math::stats::variance(self.active_returns(i)))
            .collect();

        for i in 0..n {
            matrix[i][i] = 1.0;
            let ri = self.active_returns(i);
            for j in (i + 1)..n {
                let rj = self.active_returns(j);
                let len = ri.len().min(rj.len());
                let cov = crate::math::stats::covariance(&ri[..len], &rj[..len]);
                let denom = variances[i].sqrt() * variances[j].sqrt();
                let corr = if denom == 0.0 { 0.0 } else { cov / denom };
                matrix[i][j] = corr;
                matrix[j][i] = corr;
            }
        }
        matrix
    }

    // ── Outperformance ──

    /// Cumulative outperformance versus the active benchmark.
    ///
    /// # Returns
    ///
    /// One relative cumulative return path per ticker, computed as
    /// `(1 + portfolio_cumulative) / (1 + benchmark_cumulative) - 1`.
    pub fn cumulative_returns_outperformance(&self) -> Vec<Vec<f64>> {
        let bench_cum = comp_sum(self.active_bench());
        self.map_tickers(|i| {
            let port_cum = comp_sum(self.active_returns(i));
            port_cum
                .iter()
                .zip(bench_cum.iter())
                .map(|(p, b)| ((1.0 + p) / (1.0 + b)) - 1.0)
                .collect()
        })
    }

    /// Drawdown difference versus the active benchmark.
    ///
    /// # Returns
    ///
    /// One path per ticker, computed as ticker drawdown minus benchmark
    /// drawdown over the active window.
    pub fn drawdown_difference(&self) -> Vec<Vec<f64>> {
        let bench_dd = self.active_bench_drawdown_values();
        self.map_tickers(|i| {
            let dd = self.active_drawdown_values(i);
            dd.iter().zip(bench_dd.iter()).map(|(p, b)| p - b).collect()
        })
    }

    // ── Excess returns ──

    /// Excess returns (portfolio minus risk-free) for each ticker.
    ///
    /// # Arguments
    ///
    /// * `rf` - Risk-free rate series aligned with the active date window.
    /// * `nperiods` - If `Some(n)`, de-compounds the risk-free rate from annual
    ///   to the observation frequency before subtraction. Non-finite or
    ///   non-positive values propagate as `NaN` outputs.
    ///
    /// # Returns
    ///
    /// One excess-return series per ticker.
    pub fn excess_returns(&self, rf: &[f64], nperiods: Option<f64>) -> Vec<Vec<f64>> {
        self.map_tickers(|i| excess_returns(self.active_returns(i), rf, nperiods))
    }

    // ── Accessors ──

    /// Full return-aligned date vector (independent of any active window).
    ///
    /// Returns the date grid that pairs with each row of internal returns,
    /// covering the full constructed range. To get just the dates inside
    /// the currently selected analysis window, use [`Self::active_dates`].
    pub fn dates(&self) -> &[Date] {
        &self.dates
    }
    /// Ticker names in column order.
    ///
    /// # Returns
    ///
    /// The names supplied at construction time.
    pub fn ticker_names(&self) -> &[String] {
        &self.ticker_names
    }
    /// Index of the benchmark ticker.
    ///
    /// # Returns
    ///
    /// The zero-based index of the active benchmark ticker.
    pub fn benchmark_idx(&self) -> usize {
        self.benchmark_idx
    }
    /// Observation frequency.
    ///
    /// # Returns
    ///
    /// The frequency used to annualize facade-level metrics.
    pub fn freq(&self) -> PeriodKind {
        self.freq
    }
}

/// Lookback returns for each period horizon.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
