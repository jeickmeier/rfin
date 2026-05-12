//! WASM `Performance` class — the sole analytics entry point.
//!
//! Mirrors the Python `Performance` API (price- or return-panel construction,
//! every metric exposed as an instance method). Complex result types are
//! serialized to plain JS objects via `serde_wasm_bindgen` rather than
//! exposed as classes, keeping the JS facade simple.

use crate::utils::{date_to_iso, to_js_err};
use finstack_analytics as fa;
use finstack_core::dates::{CalendarRegistry, FiscalConfig, HolidayCalendar, PeriodKind};
use wasm_bindgen::prelude::*;

use super::support::{parse_f64_matrix, parse_f64_vec, parse_iso_date};

const DEFAULT_FISCAL_START_MONTH: u8 = 1;
const DEFAULT_FISCAL_START_DAY: u8 = 1;
const DEFAULT_FISCAL_CALENDAR_ID: &str = "nyse";

fn parse_freq(freq: &str) -> Result<PeriodKind, JsValue> {
    freq.parse::<PeriodKind>().map_err(|_| {
        to_js_err(format!(
            "Unknown frequency {freq:?}; expected one of: \
             daily, weekly, monthly, quarterly, semiannual, annual"
        ))
    })
}

fn make_fiscal_config(month: Option<u8>) -> Result<FiscalConfig, JsValue> {
    FiscalConfig::new(
        month.unwrap_or(DEFAULT_FISCAL_START_MONTH),
        DEFAULT_FISCAL_START_DAY,
    )
    .map_err(to_js_err)
}

fn resolve_fiscal_calendar() -> Result<&'static dyn HolidayCalendar, JsValue> {
    CalendarRegistry::global()
        .resolve_str(DEFAULT_FISCAL_CALENDAR_ID)
        .ok_or_else(|| to_js_err(format!("calendar {DEFAULT_FISCAL_CALENDAR_ID:?} not found")))
}

fn parse_dates(dates: JsValue) -> Result<Vec<time::Date>, JsValue> {
    let strs: Vec<String> = serde_wasm_bindgen::from_value(dates).map_err(to_js_err)?;
    strs.iter().map(|s| parse_iso_date(s)).collect()
}

fn to_js<T: serde::Serialize>(value: &T) -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(value).map_err(to_js_err)
}

/// Stateful performance analytics engine over a panel of ticker price (or return) series.
#[wasm_bindgen(js_name = Performance)]
pub struct WasmPerformance {
    inner: fa::Performance,
}

#[wasm_bindgen(js_class = Performance)]
impl WasmPerformance {
    /// Construct from a price matrix. `dates` is an array of ISO date strings,
    /// `prices` is `prices[i]` = column for ticker `i`.
    #[wasm_bindgen(constructor)]
    pub fn new(
        dates: JsValue,
        prices: JsValue,
        ticker_names: JsValue,
        benchmark_ticker: Option<String>,
        freq: Option<String>,
    ) -> Result<WasmPerformance, JsValue> {
        let date_vec = parse_dates(dates)?;
        let price_matrix = parse_f64_matrix(prices)?;
        let names: Vec<String> = serde_wasm_bindgen::from_value(ticker_names).map_err(to_js_err)?;
        let freq = parse_freq(freq.as_deref().unwrap_or("daily"))?;
        let inner = fa::Performance::new(
            date_vec,
            price_matrix,
            names,
            benchmark_ticker.as_deref(),
            freq,
        )
        .map_err(to_js_err)?;
        Ok(WasmPerformance { inner })
    }

    /// Construct from a return matrix (one row per `dates` entry per ticker).
    #[wasm_bindgen(js_name = fromReturns)]
    pub fn from_returns(
        dates: JsValue,
        returns: JsValue,
        ticker_names: JsValue,
        benchmark_ticker: Option<String>,
        freq: Option<String>,
    ) -> Result<WasmPerformance, JsValue> {
        let date_vec = parse_dates(dates)?;
        let return_matrix = parse_f64_matrix(returns)?;
        let names: Vec<String> = serde_wasm_bindgen::from_value(ticker_names).map_err(to_js_err)?;
        let freq = parse_freq(freq.as_deref().unwrap_or("daily"))?;
        let inner = fa::Performance::from_returns(
            date_vec,
            return_matrix,
            names,
            benchmark_ticker.as_deref(),
            freq,
        )
        .map_err(to_js_err)?;
        Ok(WasmPerformance { inner })
    }

    // ── Mutators ──

    /// Restrict subsequent analytics to `[start, end]`.
    #[wasm_bindgen(js_name = resetDateRange)]
    pub fn reset_date_range(&mut self, start: &str, end: &str) -> Result<(), JsValue> {
        self.inner
            .reset_date_range(parse_iso_date(start)?, parse_iso_date(end)?);
        Ok(())
    }

    /// Change the benchmark ticker.
    #[wasm_bindgen(js_name = resetBenchTicker)]
    pub fn reset_bench_ticker(&mut self, ticker: &str) -> Result<(), JsValue> {
        self.inner.reset_bench_ticker(ticker).map_err(to_js_err)
    }

    // ── Accessors ──

    /// Ticker names in column order.
    #[wasm_bindgen(js_name = tickerNames)]
    pub fn ticker_names(&self) -> Result<JsValue, JsValue> {
        to_js(&self.inner.ticker_names().to_vec())
    }

    /// Benchmark column index.
    #[wasm_bindgen(js_name = benchmarkIdx)]
    pub fn benchmark_idx(&self) -> usize {
        self.inner.benchmark_idx()
    }

    /// Observation frequency token.
    #[wasm_bindgen(js_name = freq)]
    pub fn freq(&self) -> String {
        self.inner.freq().to_string()
    }

    /// Active date grid as ISO date strings (`"YYYY-MM-DD"`).
    #[wasm_bindgen(js_name = dates)]
    pub fn dates(&self) -> Vec<String> {
        self.inner
            .active_dates()
            .iter()
            .map(|&d| date_to_iso(d))
            .collect()
    }

    // ── Scalar metrics ──

    pub fn cagr(&self) -> Result<JsValue, JsValue> {
        to_js(&self.inner.cagr().map_err(to_js_err)?)
    }

    #[wasm_bindgen(js_name = meanReturn)]
    pub fn mean_return(&self, annualize: Option<bool>) -> Result<JsValue, JsValue> {
        to_js(&self.inner.mean_return(annualize.unwrap_or(true)))
    }

    pub fn volatility(&self, annualize: Option<bool>) -> Result<JsValue, JsValue> {
        to_js(&self.inner.volatility(annualize.unwrap_or(true)))
    }

    pub fn sharpe(&self, risk_free_rate: Option<f64>) -> Result<JsValue, JsValue> {
        to_js(&self.inner.sharpe(risk_free_rate.unwrap_or(0.0)))
    }

    pub fn sortino(&self, mar: Option<f64>) -> Result<JsValue, JsValue> {
        to_js(&self.inner.sortino(mar.unwrap_or(0.0)))
    }

    pub fn calmar(&self) -> Result<JsValue, JsValue> {
        to_js(&self.inner.calmar().map_err(to_js_err)?)
    }

    #[wasm_bindgen(js_name = maxDrawdown)]
    pub fn max_drawdown(&self) -> Result<JsValue, JsValue> {
        to_js(&self.inner.max_drawdown())
    }

    #[wasm_bindgen(js_name = valueAtRisk)]
    pub fn value_at_risk(&self, confidence: Option<f64>) -> Result<JsValue, JsValue> {
        to_js(&self.inner.value_at_risk(confidence.unwrap_or(0.95)))
    }

    #[wasm_bindgen(js_name = expectedShortfall)]
    pub fn expected_shortfall(&self, confidence: Option<f64>) -> Result<JsValue, JsValue> {
        to_js(&self.inner.expected_shortfall(confidence.unwrap_or(0.95)))
    }

    #[wasm_bindgen(js_name = trackingError)]
    pub fn tracking_error(&self) -> Result<JsValue, JsValue> {
        to_js(&self.inner.tracking_error())
    }

    #[wasm_bindgen(js_name = informationRatio)]
    pub fn information_ratio(&self) -> Result<JsValue, JsValue> {
        to_js(&self.inner.information_ratio())
    }

    pub fn skewness(&self) -> Result<JsValue, JsValue> {
        to_js(&self.inner.skewness())
    }

    pub fn kurtosis(&self) -> Result<JsValue, JsValue> {
        to_js(&self.inner.kurtosis())
    }

    #[wasm_bindgen(js_name = geometricMean)]
    pub fn geometric_mean(&self) -> Result<JsValue, JsValue> {
        to_js(&self.inner.geometric_mean())
    }

    #[wasm_bindgen(js_name = downsideDeviation)]
    pub fn downside_deviation(&self, mar: Option<f64>) -> Result<JsValue, JsValue> {
        to_js(&self.inner.downside_deviation(mar.unwrap_or(0.0)))
    }

    #[wasm_bindgen(js_name = maxDrawdownDuration)]
    pub fn max_drawdown_duration(&self) -> Result<JsValue, JsValue> {
        to_js(&self.inner.max_drawdown_duration())
    }

    #[wasm_bindgen(js_name = upCapture)]
    pub fn up_capture(&self) -> Result<JsValue, JsValue> {
        to_js(&self.inner.up_capture())
    }

    #[wasm_bindgen(js_name = downCapture)]
    pub fn down_capture(&self) -> Result<JsValue, JsValue> {
        to_js(&self.inner.down_capture())
    }

    #[wasm_bindgen(js_name = captureRatio)]
    pub fn capture_ratio(&self) -> Result<JsValue, JsValue> {
        to_js(&self.inner.capture_ratio())
    }

    #[wasm_bindgen(js_name = omegaRatio)]
    pub fn omega_ratio(&self, threshold: Option<f64>) -> Result<JsValue, JsValue> {
        to_js(&self.inner.omega_ratio(threshold.unwrap_or(0.0)))
    }

    pub fn treynor(&self, risk_free_rate: Option<f64>) -> Result<JsValue, JsValue> {
        to_js(&self.inner.treynor(risk_free_rate.unwrap_or(0.0)))
    }

    #[wasm_bindgen(js_name = gainToPain)]
    pub fn gain_to_pain(&self) -> Result<JsValue, JsValue> {
        to_js(&self.inner.gain_to_pain())
    }

    #[wasm_bindgen(js_name = ulcerIndex)]
    pub fn ulcer_index(&self) -> Result<JsValue, JsValue> {
        to_js(&self.inner.ulcer_index())
    }

    #[wasm_bindgen(js_name = martinRatio)]
    pub fn martin_ratio(&self) -> Result<JsValue, JsValue> {
        to_js(&self.inner.martin_ratio().map_err(to_js_err)?)
    }

    #[wasm_bindgen(js_name = recoveryFactor)]
    pub fn recovery_factor(&self) -> Result<JsValue, JsValue> {
        to_js(&self.inner.recovery_factor())
    }

    #[wasm_bindgen(js_name = painIndex)]
    pub fn pain_index(&self) -> Result<JsValue, JsValue> {
        to_js(&self.inner.pain_index())
    }

    #[wasm_bindgen(js_name = painRatio)]
    pub fn pain_ratio(&self, risk_free_rate: Option<f64>) -> Result<JsValue, JsValue> {
        to_js(
            &self
                .inner
                .pain_ratio(risk_free_rate.unwrap_or(0.0))
                .map_err(to_js_err)?,
        )
    }

    #[wasm_bindgen(js_name = tailRatio)]
    pub fn tail_ratio(&self, confidence: Option<f64>) -> Result<JsValue, JsValue> {
        to_js(&self.inner.tail_ratio(confidence.unwrap_or(0.95)))
    }

    #[wasm_bindgen(js_name = rSquared)]
    pub fn r_squared(&self) -> Result<JsValue, JsValue> {
        to_js(&self.inner.r_squared())
    }

    #[wasm_bindgen(js_name = battingAverage)]
    pub fn batting_average(&self) -> Result<JsValue, JsValue> {
        to_js(&self.inner.batting_average())
    }

    #[wasm_bindgen(js_name = parametricVar)]
    pub fn parametric_var(&self, confidence: Option<f64>) -> Result<JsValue, JsValue> {
        to_js(&self.inner.parametric_var(confidence.unwrap_or(0.95)))
    }

    #[wasm_bindgen(js_name = cornishFisherVar)]
    pub fn cornish_fisher_var(&self, confidence: Option<f64>) -> Result<JsValue, JsValue> {
        to_js(&self.inner.cornish_fisher_var(confidence.unwrap_or(0.95)))
    }

    pub fn cdar(&self, confidence: Option<f64>) -> Result<JsValue, JsValue> {
        to_js(&self.inner.cdar(confidence.unwrap_or(0.95)))
    }

    #[wasm_bindgen(js_name = mSquared)]
    pub fn m_squared(&self, risk_free_rate: Option<f64>) -> Result<JsValue, JsValue> {
        to_js(&self.inner.m_squared(risk_free_rate.unwrap_or(0.0)))
    }

    #[wasm_bindgen(js_name = modifiedSharpe)]
    pub fn modified_sharpe(
        &self,
        risk_free_rate: Option<f64>,
        confidence: Option<f64>,
    ) -> Result<JsValue, JsValue> {
        to_js(
            &self
                .inner
                .modified_sharpe(risk_free_rate.unwrap_or(0.0), confidence.unwrap_or(0.95)),
        )
    }

    #[wasm_bindgen(js_name = sterlingRatio)]
    pub fn sterling_ratio(
        &self,
        risk_free_rate: Option<f64>,
        n: Option<usize>,
    ) -> Result<JsValue, JsValue> {
        to_js(
            &self
                .inner
                .sterling_ratio(risk_free_rate.unwrap_or(0.0), n.unwrap_or(5))
                .map_err(to_js_err)?,
        )
    }

    #[wasm_bindgen(js_name = burkeRatio)]
    pub fn burke_ratio(
        &self,
        risk_free_rate: Option<f64>,
        n: Option<usize>,
    ) -> Result<JsValue, JsValue> {
        to_js(
            &self
                .inner
                .burke_ratio(risk_free_rate.unwrap_or(0.0), n.unwrap_or(5))
                .map_err(to_js_err)?,
        )
    }

    // ── Vector outputs ──

    #[wasm_bindgen(js_name = cumulativeReturns)]
    pub fn cumulative_returns(&self) -> Result<JsValue, JsValue> {
        to_js(&self.inner.cumulative_returns())
    }

    #[wasm_bindgen(js_name = drawdownSeries)]
    pub fn drawdown_series(&self) -> Result<JsValue, JsValue> {
        to_js(&self.inner.drawdown_series())
    }

    #[wasm_bindgen(js_name = correlationMatrix)]
    pub fn correlation_matrix(&self) -> Result<JsValue, JsValue> {
        to_js(&self.inner.correlation_matrix())
    }

    #[wasm_bindgen(js_name = cumulativeReturnsOutperformance)]
    pub fn cumulative_returns_outperformance(&self) -> Result<JsValue, JsValue> {
        to_js(&self.inner.cumulative_returns_outperformance())
    }

    #[wasm_bindgen(js_name = drawdownDifference)]
    pub fn drawdown_difference(&self) -> Result<JsValue, JsValue> {
        to_js(&self.inner.drawdown_difference())
    }

    #[wasm_bindgen(js_name = excessReturns)]
    pub fn excess_returns(&self, rf: JsValue, nperiods: Option<f64>) -> Result<JsValue, JsValue> {
        let rf = parse_f64_vec(rf)?;
        to_js(&self.inner.excess_returns(&rf, nperiods))
    }

    // ── Benchmark ──

    pub fn beta(&self) -> Result<JsValue, JsValue> {
        to_js(&self.inner.beta())
    }

    pub fn greeks(&self) -> Result<JsValue, JsValue> {
        to_js(&self.inner.greeks())
    }

    #[wasm_bindgen(js_name = rollingGreeks)]
    pub fn rolling_greeks(
        &self,
        ticker_idx: usize,
        window: Option<usize>,
    ) -> Result<JsValue, JsValue> {
        to_js(&self.inner.rolling_greeks(ticker_idx, window.unwrap_or(63)))
    }

    #[wasm_bindgen(js_name = rollingVolatility)]
    pub fn rolling_volatility(
        &self,
        ticker_idx: usize,
        window: Option<usize>,
    ) -> Result<JsValue, JsValue> {
        to_js(
            &self
                .inner
                .rolling_volatility(ticker_idx, window.unwrap_or(63)),
        )
    }

    #[wasm_bindgen(js_name = rollingSortino)]
    pub fn rolling_sortino(
        &self,
        ticker_idx: usize,
        window: Option<usize>,
    ) -> Result<JsValue, JsValue> {
        to_js(&self.inner.rolling_sortino(ticker_idx, window.unwrap_or(63)))
    }

    #[wasm_bindgen(js_name = rollingSharpe)]
    pub fn rolling_sharpe(
        &self,
        ticker_idx: usize,
        window: Option<usize>,
        risk_free_rate: Option<f64>,
    ) -> Result<JsValue, JsValue> {
        to_js(&self.inner.rolling_sharpe(
            ticker_idx,
            window.unwrap_or(63),
            risk_free_rate.unwrap_or(0.0),
        ))
    }

    #[wasm_bindgen(js_name = rollingReturns)]
    pub fn rolling_returns(&self, ticker_idx: usize, window: usize) -> Result<JsValue, JsValue> {
        to_js(&self.inner.rolling_returns(ticker_idx, window))
    }

    #[wasm_bindgen(js_name = drawdownDetails)]
    pub fn drawdown_details(
        &self,
        ticker_idx: usize,
        n: Option<usize>,
    ) -> Result<JsValue, JsValue> {
        to_js(&self.inner.drawdown_details(ticker_idx, n.unwrap_or(5)))
    }

    #[wasm_bindgen(js_name = topBenchmarkDrawdownEpisodes)]
    pub fn top_benchmark_drawdown_episodes(&self, n: Option<usize>) -> Result<JsValue, JsValue> {
        to_js(&self.inner.top_benchmark_drawdown_episodes(n.unwrap_or(5)))
    }

    #[wasm_bindgen(js_name = multiFactorGreeks)]
    pub fn multi_factor_greeks(
        &self,
        ticker_idx: usize,
        factor_returns: JsValue,
    ) -> Result<JsValue, JsValue> {
        let factors = parse_f64_matrix(factor_returns)?;
        let refs: Vec<&[f64]> = factors.iter().map(|v| v.as_slice()).collect();
        to_js(
            &self
                .inner
                .multi_factor_greeks(ticker_idx, &refs)
                .map_err(to_js_err)?,
        )
    }

    // ── Lookback & aggregation ──

    #[wasm_bindgen(js_name = lookbackReturns)]
    pub fn lookback_returns(
        &self,
        ref_date: &str,
        fiscal_year_start_month: Option<u8>,
    ) -> Result<JsValue, JsValue> {
        let d = parse_iso_date(ref_date)?;
        let fc = make_fiscal_config(fiscal_year_start_month)?;
        let cal = resolve_fiscal_calendar()?;
        to_js(
            &self
                .inner
                .lookback_returns_with_calendar(d, fc, cal)
                .map_err(to_js_err)?,
        )
    }

    #[wasm_bindgen(js_name = periodStats)]
    pub fn period_stats(
        &self,
        ticker_idx: usize,
        agg_freq: Option<String>,
        fiscal_year_start_month: Option<u8>,
    ) -> Result<JsValue, JsValue> {
        let pk = parse_freq(agg_freq.as_deref().unwrap_or("monthly"))?;
        let fc = make_fiscal_config(fiscal_year_start_month)?;
        to_js(&self.inner.period_stats(ticker_idx, pk, Some(fc)))
    }
}
