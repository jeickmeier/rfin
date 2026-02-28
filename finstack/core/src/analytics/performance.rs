//! Stateful `Performance` struct that orchestrates all analytics sub-modules.
//!
//! Mirrors the Python `Performance` class 1:1 (minus plotting), operating on
//! internal slices and returning numeric results.

use crate::dates::{Date, FiscalConfig, PeriodKind};

use super::aggregation::{group_by_period, period_stats, PeriodStats};
use super::benchmark::{
    calc_beta, greeks, information_ratio, r_squared, rolling_greeks, tracking_error, BetaResult,
    GreeksResult, RollingGreeks,
};
use super::drawdown::{drawdown_details, to_drawdown_series, DrawdownEpisode};
use super::lookback;
use super::returns::{clean_returns, comp_sum, comp_total, excess_returns, simple_returns};
use super::risk_metrics::{self, rolling_sharpe, RollingSharpe};

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
    /// The first column or the `benchmark_ticker` column is the benchmark.
    /// Computes returns, drawdowns, and aligns the benchmark automatically.
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

    /// Reset the date range for all subsequent analytics.
    pub fn reset_date_range(&mut self, start: Date, end: Date) {
        self.start_idx = self.dates.partition_point(|&d| d < start);
        self.end_idx = self.dates.partition_point(|&d| d <= end);
    }

    /// Reset which ticker is the benchmark.
    pub fn reset_bench_ticker(&mut self, ticker: &str) {
        if let Some(idx) = self.ticker_names.iter().position(|t| t == ticker) {
            self.benchmark_idx = idx;
            self.bench_returns = self.returns.get(idx).cloned().unwrap_or_default();
            self.bench_drawdown = self.drawdowns.get(idx).cloned().unwrap_or_default();
        }
    }

    fn active_range(&self) -> core::ops::Range<usize> {
        self.start_idx..self.end_idx
    }

    fn active_returns(&self, ticker_idx: usize) -> &[f64] {
        let range = self.active_range();
        self.returns
            .get(ticker_idx)
            .map(|r| &r[range.start..range.end.min(r.len())])
            .unwrap_or(&[])
    }

    fn active_bench(&self) -> &[f64] {
        let range = self.active_range();
        &self.bench_returns[range.start..range.end.min(self.bench_returns.len())]
    }

    fn active_dates(&self) -> &[Date] {
        let range = self.active_range();
        &self.dates[range.start..range.end.min(self.dates.len())]
    }

    fn active_drawdown(&self, ticker_idx: usize) -> &[f64] {
        let range = self.active_range();
        self.drawdowns
            .get(ticker_idx)
            .map(|d| &d[range.start..range.end.min(d.len())])
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
    pub fn mean_return(&self, annualize: bool) -> Vec<f64> {
        (0..self.ticker_names.len())
            .map(|i| risk_metrics::mean_return(self.active_returns(i), annualize, self.ann()))
            .collect()
    }

    /// Volatility for each ticker.
    pub fn volatility(&self, annualize: bool) -> Vec<f64> {
        (0..self.ticker_names.len())
            .map(|i| risk_metrics::volatility(self.active_returns(i), annualize, self.ann()))
            .collect()
    }

    /// Sharpe ratio for each ticker.
    pub fn sharpe(&self) -> Vec<f64> {
        let ann = self.ann();
        (0..self.ticker_names.len())
            .map(|i| {
                let r = self.active_returns(i);
                let m = risk_metrics::mean_return(r, true, ann);
                let v = risk_metrics::volatility(r, true, ann);
                risk_metrics::sharpe(m, v)
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

    /// Value-at-Risk for each ticker.
    pub fn value_at_risk(&self, confidence: f64) -> Vec<f64> {
        (0..self.ticker_names.len())
            .map(|i| risk_metrics::value_at_risk(self.active_returns(i), confidence, None))
            .collect()
    }

    /// Expected shortfall for each ticker.
    pub fn expected_shortfall(&self, confidence: f64) -> Vec<f64> {
        (0..self.ticker_names.len())
            .map(|i| risk_metrics::expected_shortfall(self.active_returns(i), confidence, None))
            .collect()
    }

    /// Tail ratio for each ticker.
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

    /// Drawdown episode details for a specific ticker.
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

    /// Rolling greeks for a specific ticker vs benchmark.
    pub fn rolling_greeks(&self, ticker_idx: usize, window: usize) -> RollingGreeks {
        rolling_greeks(
            self.active_returns(ticker_idx),
            self.active_bench(),
            self.active_dates(),
            window,
            self.ann(),
        )
    }

    /// Rolling Sharpe for a specific ticker.
    pub fn rolling_sharpe(&self, ticker_idx: usize, window: usize) -> RollingSharpe {
        rolling_sharpe(
            self.active_returns(ticker_idx),
            self.active_dates(),
            window,
            self.ann(),
        )
    }

    // ── Lookback selectors ──

    /// Returns for each lookback period (MTD, QTD, YTD, FYTD) at `ref_date`.
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

    /// Group returns by period and compute stats for a specific ticker.
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

    /// Correlation matrix of all tickers.
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
        let bench_dd = &self.bench_drawdown
            [self.active_range().start..self.active_range().end.min(self.bench_drawdown.len())];
        (0..self.ticker_names.len())
            .map(|i| {
                let dd = self.active_drawdown(i);
                dd.iter().zip(bench_dd.iter()).map(|(p, b)| p - b).collect()
            })
            .collect()
    }

    /// Stats of each ticker during benchmark drawdown episodes.
    pub fn stats_during_bench_drawdowns(&self, n: usize) -> Vec<DrawdownEpisode> {
        let bench_dd = &self.bench_drawdown
            [self.active_range().start..self.active_range().end.min(self.bench_drawdown.len())];
        drawdown_details(bench_dd, self.active_dates(), n)
    }

    // ── Excess returns ──

    /// Compute excess returns for each ticker given a risk-free rate series.
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

        let sharpe = perf.sharpe();
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
