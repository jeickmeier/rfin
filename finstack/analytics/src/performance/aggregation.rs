//! Series, correlation, lookback, and period-aggregation methods on
//! [`Performance`].
//!
//! Pure layout split from `performance.rs`; no behavior changes.

use super::{LookbackReturns, Performance};
use crate::aggregation::{group_by_period, period_stats_from_grouped, PeriodStats};
use crate::dates::{Date, FiscalConfig, HolidayCalendar, PeriodKind};
use crate::drawdown::{drawdown_details, DrawdownEpisode};
use crate::lookback;
use crate::returns::{comp_sum, comp_total, excess_returns};

impl Performance {
    /// Cumulative compounded returns for each ticker.
    pub fn cumulative_returns(&self) -> Vec<Vec<f64>> {
        self.map_tickers(|i| comp_sum(self.active_returns(i)))
    }

    /// Drawdown series for each ticker.
    ///
    /// Values are non-positive fractions such as `-0.25` for a 25% drawdown.
    pub fn drawdown_series(&self) -> Vec<Vec<f64>> {
        self.map_tickers(|i| self.active_drawdown_values(i).to_vec())
    }

    /// Top-N drawdown episodes for a specific ticker.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::InputError::InvalidReturnSeries`] when
    /// `ticker_idx` is outside the loaded ticker columns.
    pub fn drawdown_details(
        &self,
        ticker_idx: usize,
        n: usize,
    ) -> crate::Result<Vec<DrawdownEpisode>> {
        self.ensure_ticker_idx(ticker_idx)?;
        let dd = self.active_drawdown_values(ticker_idx);
        let dates = self.active_dates();
        Ok(drawdown_details(dd, dates, n))
    }

    /// Pearson correlation matrix of all tickers.
    ///
    /// Computes pairwise correlations over the active date window.
    /// The diagonal is always `1.0`.
    pub fn correlation_matrix(&self) -> Vec<Vec<f64>> {
        let n = self.ticker_names().len();
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

    /// Cumulative outperformance versus the active benchmark.
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
    pub fn drawdown_difference(&self) -> Vec<Vec<f64>> {
        let bench_dd = self.active_bench_drawdown_values();
        self.map_tickers(|i| {
            let dd = self.active_drawdown_values(i);
            dd.iter().zip(bench_dd.iter()).map(|(p, b)| p - b).collect()
        })
    }

    /// Excess returns (portfolio minus risk-free) for each ticker.
    pub fn excess_returns(&self, rf: &[f64], nperiods: Option<f64>) -> Vec<Vec<f64>> {
        self.map_tickers(|i| excess_returns(self.active_returns(i), rf, nperiods))
    }

    /// Compounded returns for each lookback period (MTD, QTD, YTD, FYTD) at `ref_date`.
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

    /// Period-aggregated statistics for a specific ticker.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::InputError::InvalidReturnSeries`] when
    /// `ticker_idx` is outside the loaded ticker columns.
    pub fn period_stats(
        &self,
        ticker_idx: usize,
        agg_freq: PeriodKind,
        fiscal_config: Option<FiscalConfig>,
    ) -> crate::Result<PeriodStats> {
        self.ensure_ticker_idx(ticker_idx)?;
        let grouped = group_by_period(
            self.active_dates(),
            self.active_returns(ticker_idx),
            agg_freq,
            fiscal_config,
        );
        Ok(period_stats_from_grouped(&grouped))
    }
}
