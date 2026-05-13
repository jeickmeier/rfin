//! Rolling per-ticker metrics on [`Performance`].
//!
//! Pure layout split from `performance.rs`; no behavior changes.

use super::Performance;
use crate::returns::comp_total;
use crate::risk_metrics::{
    rolling_sharpe, rolling_sortino, rolling_volatility, DatedSeries, RollingSharpe,
    RollingSortino, RollingVolatility,
};

impl Performance {
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
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::InputError::InvalidReturnSeries`] when
    /// `ticker_idx` is outside the loaded ticker columns.
    pub fn rolling_returns(&self, ticker_idx: usize, window: usize) -> crate::Result<DatedSeries> {
        self.ensure_ticker_idx(ticker_idx)?;
        let returns = self.active_returns(ticker_idx);
        let dates = self.active_dates();
        let n = returns.len().min(dates.len());
        if window == 0 || window > n {
            return Ok(DatedSeries::default());
        }
        let count = n - window + 1;
        let mut values = Vec::with_capacity(count);
        let mut out_dates = Vec::with_capacity(count);
        for end in window..=n {
            let start = end - window;
            values.push(comp_total(&returns[start..end]));
            out_dates.push(dates[end - 1]);
        }
        Ok(DatedSeries {
            values,
            dates: out_dates,
        })
    }

    /// Rolling annualized volatility for a specific ticker.
    ///
    /// # Arguments
    ///
    /// * `ticker_idx` - Zero-based column index of the ticker.
    /// * `window`     - Look-back window length in periods.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::InputError::InvalidReturnSeries`] when
    /// `ticker_idx` is outside the loaded ticker columns.
    pub fn rolling_volatility(
        &self,
        ticker_idx: usize,
        window: usize,
    ) -> crate::Result<RollingVolatility> {
        self.ensure_ticker_idx(ticker_idx)?;
        Ok(rolling_volatility(
            self.active_returns(ticker_idx),
            self.active_dates(),
            window,
            self.ann(),
        ))
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
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::InputError::InvalidReturnSeries`] when
    /// `ticker_idx` is outside the loaded ticker columns.
    pub fn rolling_sortino(
        &self,
        ticker_idx: usize,
        window: usize,
        mar: f64,
    ) -> crate::Result<RollingSortino> {
        self.ensure_ticker_idx(ticker_idx)?;
        Ok(rolling_sortino(
            self.active_returns(ticker_idx),
            self.active_dates(),
            window,
            self.ann(),
            mar,
        ))
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
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::InputError::InvalidReturnSeries`] when
    /// `ticker_idx` is outside the loaded ticker columns.
    pub fn rolling_sharpe(
        &self,
        ticker_idx: usize,
        window: usize,
        risk_free_rate: f64,
    ) -> crate::Result<RollingSharpe> {
        self.ensure_ticker_idx(ticker_idx)?;
        Ok(rolling_sharpe(
            self.active_returns(ticker_idx),
            self.active_dates(),
            window,
            self.ann(),
            risk_free_rate,
        ))
    }
}
