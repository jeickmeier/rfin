//! WASM bindings for the `Performance` analytics engine.
//!
//! Wraps `finstack_analytics::Performance`, accepting JS arrays of dates and
//! prices and returning JS-typed results.

use crate::core::dates::FsDate;
use crate::core::error::{core_to_js, js_error};
use finstack_analytics::Performance;
use finstack_core::dates::PeriodKind;
use wasm_bindgen::prelude::*;

/// Parse a frequency string into a `PeriodKind`.
fn parse_freq(s: &str) -> Result<PeriodKind, JsValue> {
    s.parse::<PeriodKind>().map_err(|_| {
        js_error(format!(
            "Unknown frequency '{s}'. Expected: daily, weekly, monthly, quarterly, semiannual, annual"
        ))
    })
}

/// Performance analytics engine.
///
/// Construct from parallel arrays of dates and prices for one or more tickers.
/// The first price column is used as the benchmark unless overridden.
///
/// @example
/// ```javascript
/// const dates = [new FsDate(2024, 1, 2), new FsDate(2024, 1, 3), ...];
/// const prices = new Float64Array([100, 101, 99, ...]); // flat: nDates * nTickers
/// const tickers = ["SPY", "QQQ"];
/// const perf = new Performance(dates, prices, tickers, 2, "daily", false);
/// const cagrs = perf.cagr();
/// ```
#[wasm_bindgen(js_name = Performance)]
pub struct JsPerformance {
    inner: Performance,
}

impl JsPerformance {
    /// Resolve a ticker name to its column index.
    fn resolve_ticker(&self, ticker: &str) -> Result<usize, JsValue> {
        self.inner
            .ticker_names()
            .iter()
            .position(|t| t == ticker)
            .ok_or_else(|| {
                js_error(format!(
                    "Unknown ticker '{}'. Available: {:?}",
                    ticker,
                    self.inner.ticker_names()
                ))
            })
    }
}

#[wasm_bindgen(js_class = Performance)]
impl JsPerformance {
    /// Create a new Performance analytics engine.
    ///
    /// @param {FsDate[]} dates - Observation dates (one per price row)
    /// @param {Float64Array} pricesFlat - Flattened price matrix (nDates * nTickers, column-major)
    /// @param {string[]} tickerNames - Ticker names
    /// @param {number} nTickers - Number of tickers
    /// @param {string} freq - Observation frequency: "daily", "weekly", "monthly", etc.
    /// @param {boolean} logReturns - If true, compute log returns internally
    /// @param {string | undefined} benchmarkTicker - Optional benchmark ticker name
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        dates: Vec<FsDate>,
        prices_flat: &[f64],
        ticker_names: Vec<String>,
        n_tickers: usize,
        freq: &str,
        log_returns: bool,
        benchmark_ticker: Option<String>,
    ) -> Result<JsPerformance, JsValue> {
        let period_kind = parse_freq(freq)?;
        let n_dates = dates.len();

        if n_tickers == 0 {
            return Err(js_error("At least one ticker is required"));
        }
        if ticker_names.len() != n_tickers {
            return Err(js_error("tickerNames length must equal nTickers"));
        }
        if prices_flat.len() != n_dates * n_tickers {
            return Err(js_error(
                "pricesFlat length must equal dates.length * nTickers",
            ));
        }

        let core_dates: Vec<finstack_core::dates::Date> =
            dates.iter().map(|d| d.inner()).collect();

        let price_cols: Vec<Vec<f64>> = (0..n_tickers)
            .map(|col| {
                (0..n_dates)
                    .map(|row| prices_flat[col * n_dates + row])
                    .collect()
            })
            .collect();

        let inner = Performance::new(
            core_dates,
            price_cols,
            ticker_names,
            benchmark_ticker.as_deref(),
            period_kind,
            log_returns,
        )
        .map_err(core_to_js)?;

        Ok(JsPerformance { inner })
    }

    /// Ticker names in column order.
    #[wasm_bindgen(getter, js_name = tickerNames)]
    pub fn ticker_names(&self) -> Vec<String> {
        self.inner.ticker_names().to_vec()
    }

    /// Index of the current benchmark ticker.
    #[wasm_bindgen(getter, js_name = benchmarkIdx)]
    pub fn benchmark_idx(&self) -> usize {
        self.inner.benchmark_idx()
    }

    /// Observation frequency string.
    #[wasm_bindgen(getter)]
    pub fn freq(&self) -> String {
        self.inner.freq().to_string()
    }

    /// Whether log returns were used at construction.
    #[wasm_bindgen(getter, js_name = logReturns)]
    pub fn log_returns(&self) -> bool {
        self.inner.uses_log_returns()
    }

    /// Number of active dates in the current analysis window.
    #[wasm_bindgen(getter, js_name = activeDateCount)]
    pub fn active_date_count(&self) -> usize {
        self.inner.active_dates().len()
    }

    /// Active dates as an array of FsDate.
    #[wasm_bindgen(js_name = activeDates)]
    pub fn active_dates(&self) -> Vec<FsDate> {
        self.inner
            .active_dates()
            .iter()
            .map(|&d| FsDate::from_core(d))
            .collect()
    }

    /// Restrict the analysis date range.
    ///
    /// @param {FsDate} start - First date (inclusive)
    /// @param {FsDate} end - Last date (inclusive)
    #[wasm_bindgen(js_name = resetDateRange)]
    pub fn reset_date_range(&mut self, start: &FsDate, end: &FsDate) {
        self.inner.reset_date_range(start.inner(), end.inner());
    }

    /// Change the benchmark ticker.
    ///
    /// @param {string} ticker - Name of the new benchmark ticker
    #[wasm_bindgen(js_name = resetBenchTicker)]
    pub fn reset_bench_ticker(&mut self, ticker: &str) -> Result<(), JsValue> {
        self.inner.reset_bench_ticker(ticker).map_err(core_to_js)
    }

    // ── Scalar metrics (per ticker) ──

    /// CAGR for each ticker.
    pub fn cagr(&self) -> Vec<f64> {
        self.inner.cagr()
    }

    /// Mean return for each ticker.
    #[wasm_bindgen(js_name = meanReturn)]
    pub fn mean_return(&self, annualize: bool) -> Vec<f64> {
        self.inner.mean_return(annualize)
    }

    /// Volatility for each ticker.
    pub fn volatility(&self, annualize: bool) -> Vec<f64> {
        self.inner.volatility(annualize)
    }

    /// Sharpe ratio for each ticker.
    pub fn sharpe(&self, risk_free_rate: f64) -> Vec<f64> {
        self.inner.sharpe(risk_free_rate)
    }

    /// Sortino ratio for each ticker.
    pub fn sortino(&self) -> Vec<f64> {
        self.inner.sortino()
    }

    /// Calmar ratio for each ticker.
    pub fn calmar(&self) -> Vec<f64> {
        self.inner.calmar()
    }

    /// Max drawdown for each ticker.
    #[wasm_bindgen(js_name = maxDrawdown)]
    pub fn max_drawdown(&self) -> Vec<f64> {
        self.inner.max_drawdown()
    }

    /// Value-at-Risk for each ticker.
    #[wasm_bindgen(js_name = valueAtRisk)]
    pub fn value_at_risk(&self, confidence: f64) -> Vec<f64> {
        self.inner.value_at_risk(confidence)
    }

    /// Expected Shortfall (CVaR) for each ticker.
    #[wasm_bindgen(js_name = expectedShortfall)]
    pub fn expected_shortfall(&self, confidence: f64) -> Vec<f64> {
        self.inner.expected_shortfall(confidence)
    }

    /// Tail ratio for each ticker.
    #[wasm_bindgen(js_name = tailRatio)]
    pub fn tail_ratio(&self, confidence: f64) -> Vec<f64> {
        self.inner.tail_ratio(confidence)
    }

    /// Ulcer index for each ticker.
    #[wasm_bindgen(js_name = ulcerIndex)]
    pub fn ulcer_index(&self) -> Vec<f64> {
        self.inner.ulcer_index()
    }

    /// Skewness for each ticker.
    pub fn skewness(&self) -> Vec<f64> {
        self.inner.skewness()
    }

    /// Excess kurtosis for each ticker.
    pub fn kurtosis(&self) -> Vec<f64> {
        self.inner.kurtosis()
    }

    /// Geometric mean return for each ticker.
    #[wasm_bindgen(js_name = geometricMean)]
    pub fn geometric_mean(&self) -> Vec<f64> {
        self.inner.geometric_mean()
    }

    /// Downside deviation for each ticker.
    #[wasm_bindgen(js_name = downsideDeviation)]
    pub fn downside_deviation(&self, mar: f64) -> Vec<f64> {
        self.inner.downside_deviation(mar)
    }

    /// Max drawdown duration (calendar days) for each ticker.
    #[wasm_bindgen(js_name = maxDrawdownDuration)]
    pub fn max_drawdown_duration(&self) -> Vec<i64> {
        self.inner.max_drawdown_duration()
    }

    // ── Benchmark-relative scalar metrics ──

    /// Tracking error for each ticker vs benchmark.
    #[wasm_bindgen(js_name = trackingError)]
    pub fn tracking_error(&self) -> Vec<f64> {
        self.inner.tracking_error()
    }

    /// Information ratio for each ticker vs benchmark.
    #[wasm_bindgen(js_name = informationRatio)]
    pub fn information_ratio(&self) -> Vec<f64> {
        self.inner.information_ratio()
    }

    /// R-squared for each ticker vs benchmark.
    #[wasm_bindgen(js_name = rSquared)]
    pub fn r_squared(&self) -> Vec<f64> {
        self.inner.r_squared()
    }

    /// Up-market capture ratio for each ticker.
    #[wasm_bindgen(js_name = upCapture)]
    pub fn up_capture(&self) -> Vec<f64> {
        self.inner.up_capture()
    }

    /// Down-market capture ratio for each ticker.
    #[wasm_bindgen(js_name = downCapture)]
    pub fn down_capture(&self) -> Vec<f64> {
        self.inner.down_capture()
    }

    /// Capture ratio (up/down) for each ticker.
    #[wasm_bindgen(js_name = captureRatio)]
    pub fn capture_ratio(&self) -> Vec<f64> {
        self.inner.capture_ratio()
    }

    /// Batting average for each ticker vs benchmark.
    #[wasm_bindgen(js_name = battingAverage)]
    pub fn batting_average(&self) -> Vec<f64> {
        self.inner.batting_average()
    }

    // ── Standard ratios ──

    /// Omega ratio for each ticker.
    #[wasm_bindgen(js_name = omegaRatio)]
    pub fn omega_ratio(&self, threshold: f64) -> Vec<f64> {
        self.inner.omega_ratio(threshold)
    }

    /// Treynor ratio for each ticker.
    pub fn treynor(&self, risk_free_rate: f64) -> Vec<f64> {
        self.inner.treynor(risk_free_rate)
    }

    /// Gain-to-pain ratio for each ticker.
    #[wasm_bindgen(js_name = gainToPain)]
    pub fn gain_to_pain(&self) -> Vec<f64> {
        self.inner.gain_to_pain()
    }

    /// Martin ratio for each ticker.
    #[wasm_bindgen(js_name = martinRatio)]
    pub fn martin_ratio(&self) -> Vec<f64> {
        self.inner.martin_ratio()
    }

    /// M-squared for each ticker.
    #[wasm_bindgen(js_name = mSquared)]
    pub fn m_squared(&self, risk_free_rate: f64) -> Vec<f64> {
        self.inner.m_squared(risk_free_rate)
    }

    /// Modified Sharpe ratio for each ticker.
    #[wasm_bindgen(js_name = modifiedSharpe)]
    pub fn modified_sharpe(&self, risk_free_rate: f64, confidence: f64) -> Vec<f64> {
        self.inner.modified_sharpe(risk_free_rate, confidence)
    }

    // ── VaR variants ──

    /// Parametric (Gaussian) VaR for each ticker.
    #[wasm_bindgen(js_name = parametricVar)]
    pub fn parametric_var(&self, confidence: f64) -> Vec<f64> {
        self.inner.parametric_var(confidence)
    }

    /// Cornish-Fisher adjusted VaR for each ticker.
    #[wasm_bindgen(js_name = cornishFisherVar)]
    pub fn cornish_fisher_var(&self, confidence: f64) -> Vec<f64> {
        self.inner.cornish_fisher_var(confidence)
    }

    // ── Drawdown ratios ──

    /// Recovery factor for each ticker.
    #[wasm_bindgen(js_name = recoveryFactor)]
    pub fn recovery_factor(&self) -> Vec<f64> {
        self.inner.recovery_factor()
    }

    /// Sterling ratio for each ticker.
    #[wasm_bindgen(js_name = sterlingRatio)]
    pub fn sterling_ratio(&self, risk_free_rate: f64, n: usize) -> Vec<f64> {
        self.inner.sterling_ratio(risk_free_rate, n)
    }

    /// Burke ratio for each ticker.
    #[wasm_bindgen(js_name = burkeRatio)]
    pub fn burke_ratio(&self, risk_free_rate: f64, n: usize) -> Vec<f64> {
        self.inner.burke_ratio(risk_free_rate, n)
    }

    /// Pain index for each ticker.
    #[wasm_bindgen(js_name = painIndex)]
    pub fn pain_index(&self) -> Vec<f64> {
        self.inner.pain_index()
    }

    /// Pain ratio for each ticker.
    #[wasm_bindgen(js_name = painRatio)]
    pub fn pain_ratio(&self, risk_free_rate: f64) -> Vec<f64> {
        self.inner.pain_ratio(risk_free_rate)
    }

    /// Conditional Drawdown at Risk for each ticker.
    pub fn cdar(&self, confidence: f64) -> Vec<f64> {
        self.inner.cdar(confidence)
    }

    // ── Series outputs ──

    /// Cumulative returns for each ticker (flat: nDates * nTickers, column-major).
    #[wasm_bindgen(js_name = cumulativeReturns)]
    pub fn cumulative_returns(&self) -> Vec<f64> {
        let data = self.inner.cumulative_returns();
        flatten_columns(&data)
    }

    /// Drawdown series for each ticker (flat: nDates * nTickers, column-major).
    #[wasm_bindgen(js_name = drawdownSeries)]
    pub fn drawdown_series(&self) -> Vec<f64> {
        let data = self.inner.drawdown_series();
        flatten_columns(&data)
    }

    /// Correlation matrix (flat: nTickers * nTickers, row-major).
    #[wasm_bindgen(js_name = correlationMatrix)]
    pub fn correlation_matrix(&self) -> Vec<f64> {
        let matrix = self.inner.correlation_matrix();
        matrix.into_iter().flatten().collect()
    }

    // ── Rolling metrics ──

    /// Rolling volatility for a specific ticker.
    ///
    /// @param {string} ticker - Ticker name
    /// @param {number} window - Look-back window in periods
    /// @returns {Float64Array} Rolling volatility values
    #[wasm_bindgen(js_name = rollingVolatility)]
    pub fn rolling_volatility(&self, ticker: &str, window: usize) -> Result<Vec<f64>, JsValue> {
        let idx = self.resolve_ticker(ticker)?;
        let rv = self.inner.rolling_volatility(idx, window);
        Ok(rv.values)
    }

    /// Rolling Sortino ratio for a specific ticker.
    ///
    /// @param {string} ticker - Ticker name
    /// @param {number} window - Look-back window in periods
    /// @returns {Float64Array} Rolling Sortino values
    #[wasm_bindgen(js_name = rollingSortino)]
    pub fn rolling_sortino(&self, ticker: &str, window: usize) -> Result<Vec<f64>, JsValue> {
        let idx = self.resolve_ticker(ticker)?;
        let rs = self.inner.rolling_sortino(idx, window);
        Ok(rs.values)
    }

    /// Rolling Sharpe ratio for a specific ticker.
    ///
    /// @param {string} ticker - Ticker name
    /// @param {number} window - Look-back window in periods
    /// @param {number} riskFreeRate - Annualized risk-free rate
    /// @returns {Float64Array} Rolling Sharpe values
    #[wasm_bindgen(js_name = rollingSharpe)]
    pub fn rolling_sharpe(
        &self,
        ticker: &str,
        window: usize,
        risk_free_rate: f64,
    ) -> Result<Vec<f64>, JsValue> {
        let idx = self.resolve_ticker(ticker)?;
        let rs = self.inner.rolling_sharpe(idx, window, risk_free_rate);
        Ok(rs.values)
    }

    /// Rolling greeks (alpha, beta) for a ticker vs the benchmark.
    ///
    /// Returns a flat array: `[alpha_0, beta_0, alpha_1, beta_1, ...]`
    ///
    /// @param {string} ticker - Ticker name
    /// @param {number} window - Look-back window in periods
    /// @returns {Float64Array} Interleaved alpha/beta values
    #[wasm_bindgen(js_name = rollingGreeks)]
    pub fn rolling_greeks(
        &self,
        ticker: &str,
        window: usize,
    ) -> Result<Vec<f64>, JsValue> {
        let idx = self.resolve_ticker(ticker)?;
        let rg = self.inner.rolling_greeks(idx, window);
        let mut out = Vec::with_capacity(rg.alphas.len() * 2);
        for (a, b) in rg.alphas.iter().zip(rg.betas.iter()) {
            out.push(*a);
            out.push(*b);
        }
        Ok(out)
    }

    // ── Outperformance series ──

    /// Cumulative outperformance vs benchmark (flat: nDates * nTickers, column-major).
    #[wasm_bindgen(js_name = cumulativeReturnsOutperformance)]
    pub fn cumulative_returns_outperformance(&self) -> Vec<f64> {
        let data = self.inner.cumulative_returns_outperformance();
        flatten_columns(&data)
    }

    /// Drawdown outperformance vs benchmark (flat: nDates * nTickers, column-major).
    #[wasm_bindgen(js_name = drawdownOutperformance)]
    pub fn drawdown_outperformance(&self) -> Vec<f64> {
        let data = self.inner.drawdown_outperformance();
        flatten_columns(&data)
    }

    /// Excess returns (portfolio minus risk-free) for each ticker
    /// (flat: nDates * nTickers, column-major).
    ///
    /// @param {Float64Array} rf - Risk-free rate series
    /// @param {number | undefined} nperiods - Optional de-compounding periods
    #[wasm_bindgen(js_name = excessReturns)]
    pub fn excess_returns(&self, rf: &[f64], nperiods: Option<f64>) -> Vec<f64> {
        let data = self.inner.excess_returns(rf, nperiods);
        flatten_columns(&data)
    }

    /// Beta results for each ticker vs benchmark.
    ///
    /// Returns a flat array: `[beta_0, stdErr_0, ciLower_0, ciUpper_0, beta_1, ...]`
    pub fn beta(&self) -> Vec<f64> {
        let betas = self.inner.beta();
        let mut out = Vec::with_capacity(betas.len() * 4);
        for br in &betas {
            out.push(br.beta);
            out.push(br.std_err);
            out.push(br.ci_lower);
            out.push(br.ci_upper);
        }
        out
    }

    /// Greeks results for each ticker vs benchmark.
    ///
    /// Returns a flat array: `[alpha_0, beta_0, rSquared_0, alpha_1, ...]`
    pub fn greeks(&self) -> Vec<f64> {
        let g = self.inner.greeks();
        let mut out = Vec::with_capacity(g.len() * 3);
        for gr in &g {
            out.push(gr.alpha);
            out.push(gr.beta);
            out.push(gr.r_squared);
        }
        out
    }

    /// Lookback returns for standard periods.
    ///
    /// Returns a flat array: `[mtd_0..mtd_n, qtd_0..qtd_n, ytd_0..ytd_n]`
    ///
    /// @param {FsDate} refDate - Reference date
    #[wasm_bindgen(js_name = lookbackReturns)]
    pub fn lookback_returns(&self, ref_date: &FsDate) -> Vec<f64> {
        let lb = self.inner.lookback_returns(ref_date.inner(), None);
        let n = lb.mtd.len();
        let mut out = Vec::with_capacity(n * 3);
        out.extend_from_slice(&lb.mtd);
        out.extend_from_slice(&lb.qtd);
        out.extend_from_slice(&lb.ytd);
        out
    }

    /// String representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        let tickers = self.inner.ticker_names();
        let bench = tickers
            .get(self.inner.benchmark_idx())
            .map(|s| s.as_str())
            .unwrap_or("?");
        format!(
            "Performance(tickers={:?}, freq={}, benchmark='{}', nDates={})",
            tickers,
            self.inner.freq(),
            bench,
            self.inner.active_dates().len(),
        )
    }
}

/// Flatten column-major data: `data[col][row]` → flat `[col0_r0, col0_r1, ..., col1_r0, ...]`
fn flatten_columns(data: &[Vec<f64>]) -> Vec<f64> {
    let total: usize = data.iter().map(|v| v.len()).sum();
    let mut out = Vec::with_capacity(total);
    for col in data {
        out.extend_from_slice(col);
    }
    out
}
