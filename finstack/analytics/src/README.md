## Analytics Module

The `finstack-analytics` crate provides **portfolio performance and risk
analytics** operating directly on numeric slices and `time::Date` values, with
no Polars or DataFrame dependency in the Rust API. It mirrors the Python
`Performance` class in capability while exposing every computation as a
standalone pure function for composability.

- **Returns**: simple returns, log returns, excess returns, compounded accumulation, geometric mean
- **Risk metrics**: Sharpe, Sortino, Calmar, VaR (historical, parametric, Cornish-Fisher), CVaR/ES, Ulcer Index, risk of ruin, tail ratios, skewness (Fisher-corrected), kurtosis (Fisher-corrected), downside deviation, Omega, Treynor, gain-to-pain, Martin ratio, M-squared, Modified Sharpe
- **Drawdown analysis**: drawdown series, episode detection, average drawdown, CDaR, max drawdown duration, recovery factor, Sterling/Burke/pain ratios
- **Benchmark-relative**: tracking error, information ratio, R², beta (with SE and CI), alpha/beta/R² greeks, rolling greeks, up/down capture ratios, batting average, multi-factor regression (with adjusted R²; invalid or singular regressions return an error)
- **Period aggregation**: group-and-compound by any `PeriodKind` (daily → annual), win rate, Kelly criterion, payoff ratio; weekly aggregation uses ISO week boundaries
- **Lookback selectors**: MTD, QTD, YTD, FYTD index ranges into sorted date arrays
- **Rolling time series**: rolling Sharpe, rolling Sortino, rolling volatility, rolling alpha/beta
- **Orchestrator**: `Performance` struct ties all sub-modules together with date-windowing and benchmark state

The crate is allocation-minimal and uses numerically stable algorithms
(Kahan/Neumaier log-space compounding, Welford-style covariance). It currently
relies on `std` for RNG-backed bootstrap routines such as ruin simulation.

---

## Module Structure

- **`lib.rs`**
  - Public entrypoint for the analytics crate.
  - Re-exports the high-level `Performance` facade and core result types
    (`PeriodStats`, `LookbackReturns`, `BacktestResult`).
  - Standalone functions remain in domain modules such as `risk_metrics`,
    `benchmark`, `drawdown`, `returns`, `aggregation`, `lookback`,
    `backtesting`, and `timeseries`.

- **`performance.rs`**
  - `Performance`: stateful orchestrator holding pre-computed returns, drawdowns, and benchmark data for a universe of tickers.
  - `LookbackReturns`: output type for MTD/QTD/YTD/FYTD compounded returns.
  - All methods delegate to the pure-function sub-modules; no analytics logic lives here directly.

- **`risk_metrics/`** (directory module)
  - **`mod.rs`**: Public facade, re-exports from all three submodules.
  - **`return_based.rs`**: `cagr`, `CagrBasis`, `mean_return`, `volatility`, `sharpe`, `sortino`, `downside_deviation`, `estimate_ruin`, `RuinDefinition`, `RuinModel`, `RuinEstimate`, `geometric_mean`, `omega_ratio`, `gain_to_pain`, `modified_sharpe`.
  - **`tail_risk.rs`**: `skewness`, `kurtosis`, `value_at_risk`, `expected_shortfall`, `parametric_var`, `cornish_fisher_var`, `tail_ratio`, `outlier_win_ratio`, `outlier_loss_ratio` (`_with_scratch` variants are crate-internal optimizations used by the `Performance` facade).
  - **`rolling.rs`**: `RollingSharpe`, `RollingVolatility`, `RollingSortino`, `rolling_sharpe`, `rolling_volatility`, `rolling_sortino`, plus `to_nan_padded()` helpers on each rolling result type.
  - All functions take `&[f64]` and return `f64` or a small struct.

- **`benchmark.rs`**
  - Benchmark-relative statistics: `tracking_error`, `information_ratio`, `r_squared`, `beta`, `greeks`, `rolling_greeks`, `up_capture`, `down_capture`, `capture_ratio`, `batting_average`, `multi_factor_greeks`.
  - Benchmark-relative risk ratios: `treynor`, `m_squared` (compose with `mean_return` / `volatility` for series-based inputs).
  - Output types: `BetaResult` (beta, std_err, CI), `GreeksResult` (alpha, beta, r², adjusted_r²), `RollingGreeks` (dates, alphas, betas), `MultiFactorResult` (alpha, betas, r², adjusted_r², residual_vol).
  - Invalid, mismatched, or singular multi-factor regressions return an error instead of silently zero-filling the output.

- **`returns.rs`**
  - Return computation: `simple_returns`, `excess_returns`, `convert_to_prices`, `rebase`.
  - Compounding: `comp_sum` (cumulative series), `comp_total` (scalar).
  - Cleaning: `clean_returns` (replace ±∞ with NaN, strip trailing NaNs).
  - Uses Neumaier log-space accumulation for long-series numerical stability.

- **`drawdown.rs`**
  - `to_drawdown_series`: per-period drawdown depth `(wealth / peak - 1)`.
  - `drawdown_details`: structured episodes (start/valley/end dates, duration, depth) sorted by severity.
  - `mean_episode_drawdown`: mean of the N worst drawdown episodes.
  - `mean_drawdown`: arithmetic mean of a drawdown series (path-weighted).
  - `max_drawdown_duration`: longest drawdown duration in calendar days.
  - `cdar`: Conditional Drawdown at Risk at a given confidence level.
  - Drawdown-derived risk ratios (compose from primitives): `ulcer_index`, `pain_index`, `calmar`, `recovery_factor`, `martin_ratio`, `sterling_ratio`, `burke_ratio`, `pain_ratio`.
  - Output type: `DrawdownEpisode { start, valley, end, duration_days, max_drawdown, near_recovery_threshold }`.

- **`aggregation.rs`**
  - `group_by_period`: compounds daily returns into period buckets keyed by `PeriodId` (`Weekly` uses ISO week numbering).
  - `period_stats`: derives win rate, best/worst, consecutive streaks, payoff ratio, Kelly criterion, profit factor from grouped returns.
  - Output type: `PeriodStats` (13 fields).

- **`lookback.rs`**
  - Date-index selectors returning `Range<usize>` into sorted date arrays: `mtd_select`, `qtd_select`, `ytd_select`, `fytd_select`.
  - All accept an `offset_days` parameter to shift window starts.
  - Uses binary search; no allocations.

- **`backtesting/`** (directory module)
  - VaR backtesting infrastructure. Submodules: `types`, `metrics`, `orchestrator`.
  - Coverage tests: `kupiec_test` (POF unconditional coverage), `christoffersen_test` (joint conditional coverage).
  - Basel-style classification: `traffic_light`, `TrafficLightZone`, `capital_multiplier()`.
  - Orchestrators: `run_backtest`, `rolling_var_forecasts`, `compare_var_backtests`.
  - FRTB P&L explanation: `pnl_explanation`.
  - Output types: `KupiecResult`, `ChristoffersenResult`, `TrafficLightResult`, `BacktestResult`, `MultiModelComparison`, `PnlExplanation`.

- **`timeseries/`** (directory module)
  - GARCH-family volatility models and diagnostics. Submodules: `garch` (trait + MLE driver), `garch11`, `gjr_garch11`, `egarch11`, `forecast`, `diagnostics`, `innovations`, `optimizer`.
  - Models: `Garch11`, `GjrGarch11`, `Egarch11` all implement the `GarchModel` trait. References: Bollerslev (1986), Glosten-Jagannathan-Runkle (1993), Nelson (1991).
  - Forecasting: `forecast_garch_fit`, `vol_term_structure`, `STANDARD_HORIZONS`.
  - Diagnostics: `ljung_box`, `arch_lm`, `aic`, `bic`, `hqic`.
  - Selection: `compare_garch_models`, `auto_garch` (BIC-ranked).
  - Innovations: `InnovationDist::Gaussian` and `InnovationDist::StudentT(nu)`.

---

## Core Types

### `Performance`

The central orchestrator. Constructed from a price matrix, it pre-computes returns and drawdowns for every ticker and caches the benchmark series:

```rust
pub struct Performance { /* opaque */ }

impl Performance {
    pub fn new(
        dates: Vec<Date>,
        prices: Vec<Vec<f64>>,      // prices[ticker_idx][time]
        ticker_names: Vec<String>,
        benchmark_ticker: Option<&str>,
        freq: PeriodKind,
    ) -> Result<Self>;

    // Date windowing — all metrics respect the active range
    pub fn reset_date_range(&mut self, start: Date, end: Date);
    pub fn reset_bench_ticker(&mut self, ticker: &str) -> Result<()>;

    // Scalar metrics (one value per ticker)
    pub fn cagr(&self) -> Result<Vec<f64>>;
    pub fn sharpe(&self, risk_free_rate: f64) -> Vec<f64>;
    pub fn sortino(&self, mar: f64) -> Vec<f64>;
    pub fn calmar(&self) -> Vec<f64>;
    pub fn volatility(&self, annualize: bool) -> Vec<f64>;
    pub fn max_drawdown(&self) -> Vec<f64>;
    pub fn value_at_risk(&self, confidence: f64) -> Vec<f64>;
    pub fn expected_shortfall(&self, confidence: f64) -> Vec<f64>;
    pub fn tail_ratio(&self, confidence: f64) -> Vec<f64>;
    pub fn ulcer_index(&self) -> Vec<f64>;
    pub fn estimate_ruin(&self, definition: RuinDefinition, model: &RuinModel) -> Vec<RuinEstimate>;

    // Distribution shape
    pub fn skewness(&self) -> Vec<f64>;
    pub fn kurtosis(&self) -> Vec<f64>;
    pub fn geometric_mean(&self) -> Vec<f64>;
    pub fn downside_deviation(&self, mar: f64) -> Vec<f64>;

    // Extended risk-adjusted ratios
    pub fn omega_ratio(&self, threshold: f64) -> Vec<f64>;
    pub fn treynor(&self, risk_free_rate: f64) -> Vec<f64>;
    pub fn gain_to_pain(&self) -> Vec<f64>;
    pub fn martin_ratio(&self) -> Vec<f64>;
    pub fn parametric_var(&self, confidence: f64) -> Vec<f64>;
    pub fn cornish_fisher_var(&self, confidence: f64) -> Vec<f64>;

    // Drawdown-family ratios
    pub fn max_drawdown_duration(&self) -> Vec<i64>;
    pub fn recovery_factor(&self) -> Vec<f64>;
    pub fn sterling_ratio(&self, risk_free_rate: f64, n: usize) -> Vec<f64>;
    pub fn burke_ratio(&self, risk_free_rate: f64, n: usize) -> Vec<f64>;
    pub fn pain_index(&self) -> Vec<f64>;
    pub fn pain_ratio(&self, risk_free_rate: f64) -> Vec<f64>;
    pub fn cdar(&self, confidence: f64) -> Vec<f64>;

    // Benchmark-relative (one value per ticker)
    pub fn tracking_error(&self) -> Vec<f64>;
    pub fn information_ratio(&self) -> Vec<f64>;
    pub fn r_squared(&self) -> Vec<f64>;
    pub fn beta(&self) -> Vec<BetaResult>;
    pub fn greeks(&self) -> Vec<GreeksResult>;
    pub fn up_capture(&self) -> Vec<f64>;
    pub fn down_capture(&self) -> Vec<f64>;
    pub fn capture_ratio(&self) -> Vec<f64>;
    pub fn batting_average(&self) -> Vec<f64>;
    pub fn m_squared(&self, risk_free_rate: f64) -> Vec<f64>;
    pub fn modified_sharpe(&self, risk_free_rate: f64, confidence: f64) -> Vec<f64>;
    pub fn multi_factor_greeks(&self, idx: usize, factors: &[&[f64]]) -> Result<MultiFactorResult>;

    // Series outputs
    pub fn cumulative_returns(&self) -> Vec<Vec<f64>>;
    pub fn drawdown_series(&self) -> Vec<Vec<f64>>;
    pub fn correlation_matrix(&self) -> Vec<Vec<f64>>;
    pub fn excess_returns(&self, rf: &[f64], nperiods: Option<f64>) -> Vec<Vec<f64>>;

    // Per-ticker rolling series
    pub fn rolling_sharpe(&self, idx: usize, window: usize, rf: f64) -> RollingSharpe;
    pub fn rolling_volatility(&self, idx: usize, window: usize) -> RollingVolatility;
    pub fn rolling_sortino(&self, idx: usize, window: usize) -> RollingSortino;
    pub fn rolling_greeks(&self, idx: usize, window: usize) -> RollingGreeks;

    // Drawdown episodes
    pub fn drawdown_details(&self, idx: usize, n: usize) -> Vec<DrawdownEpisode>;
    pub fn top_benchmark_drawdown_episodes(&self, n: usize) -> Vec<DrawdownEpisode>;

    // Lookback and aggregation
    pub fn lookback_returns(&self, ref_date: Date, fiscal: Option<FiscalConfig>) -> LookbackReturns;
    pub fn period_stats(&self, idx: usize, freq: PeriodKind, fiscal: Option<FiscalConfig>) -> PeriodStats;
}
```

### `DrawdownEpisode`

```rust
pub struct DrawdownEpisode {
    pub start: Date,                    // peak date
    pub valley: Date,                   // date of max loss
    pub end: Option<Date>,              // recovery date (None if still in drawdown)
    pub duration_days: i64,
    pub max_drawdown: f64,              // e.g. −0.25 for a 25% drawdown
    pub near_recovery_threshold: f64,   // ~1% of peak-to-trough still remaining
}
```

### `BetaResult`

OLS beta with inferential statistics:

```rust
pub struct BetaResult {
    pub beta: f64,
    pub std_err: f64,
    pub ci_lower: f64,   // lower confidence bound
    pub ci_upper: f64,   // upper confidence bound
}
```

### `MultiFactorResult`

Multi-factor OLS regression output:

```rust
pub struct MultiFactorResult {
    pub alpha: f64,              // annualized intercept
    pub betas: Vec<f64>,         // one per factor
    pub r_squared: f64,
    pub adjusted_r_squared: f64, // penalizes additional regressors
    pub residual_vol: f64,       // annualized, uses (n-k-1) DoF
}
```

### `PeriodStats`

Period-aggregated trading statistics:

```rust
pub struct PeriodStats {
    pub best: f64,
    pub worst: f64,
    pub consecutive_wins: usize,
    pub consecutive_losses: usize,
    pub win_rate: f64,
    pub avg_return: f64,
    pub avg_win: f64,
    pub avg_loss: f64,
    pub payoff_ratio: f64,    // avg_win / |avg_loss|
    pub profit_factor: f64,   // sum(wins) / sum(|losses|)
    pub cpc_ratio: f64,       // profit_factor × win_rate × payoff_ratio
    pub kelly_criterion: f64, // win_rate − loss_rate / payoff_ratio
}
```

---

## Usage Examples

### 1. Full Portfolio Analytics with `Performance`

```rust
use finstack_analytics::Performance;
use finstack_core::dates::{Date, Duration, Month, PeriodKind};

let dates: Vec<Date> = (0..252)
    .map(|i| Date::from_calendar_date(2024, Month::January, 1).unwrap()
        + Duration::days(i))
    .collect();

// Two tickers: benchmark SPY + portfolio ALPHA
let spy_prices: Vec<f64> = (0..252).map(|i| 400.0 * (1.0 + i as f64 * 0.001)).collect();
let alpha_prices: Vec<f64> = spy_prices.iter().map(|&p| p * 1.05).collect();

let mut perf = Performance::new(
    dates,
    vec![spy_prices, alpha_prices],
    vec!["SPY".into(), "ALPHA".into()],
    Some("SPY"),
    PeriodKind::Daily,
)?;

// Scalar metrics (one per ticker)
let sharpe = perf.sharpe(0.05); // 5% risk-free rate
let sortino = perf.sortino(0.0);
let max_dd  = perf.max_drawdown();
let var95   = perf.value_at_risk(0.95);

// Benchmark-relative
let ir  = perf.information_ratio();
let te  = perf.tracking_error();
let r2  = perf.r_squared();
let betas = perf.beta();

// Restrict to a sub-period
let start = Date::from_calendar_date(2024, Month::April, 1).unwrap();
let end   = Date::from_calendar_date(2024, Month::September, 30).unwrap();
perf.reset_date_range(start, end);
let windowed_cagr = perf.cagr(); // recalculated over the active window
# Ok::<(), finstack_core::Error>(())
```

### 2. Standalone Risk Metrics

All functions in the `risk_metrics/` module tree can be used independently:

```rust
use finstack_analytics::risk_metrics::{
    expected_shortfall, sharpe, sortino, value_at_risk, volatility,
};

let returns = vec![0.01, -0.005, 0.02, -0.01, 0.015, 0.008, -0.003];
let ann = 252.0_f64;

let mean_r = returns.iter().sum::<f64>() / returns.len() as f64 * ann;
let vol    = volatility(&returns, true, ann);
let sr     = sharpe(mean_r, vol, 0.05);
let so     = sortino(&returns, true, ann, 0.0);
let var    = value_at_risk(&returns, 0.95);
let es     = expected_shortfall(&returns, 0.95);

assert!(es <= var); // ES is always at least as bad as VaR
```

### 3. Drawdown Analysis

```rust
use finstack_analytics::drawdown::{
    drawdown_details, mean_episode_drawdown, to_drawdown_series,
};
use finstack_core::dates::{Date, Month};

let returns = vec![0.05, -0.12, 0.03, -0.08, 0.10, -0.20, 0.15];
let dates: Vec<Date> = (1..=7)
    .map(|d| Date::from_calendar_date(2024, Month::January, d).unwrap())
    .collect();

let dd_series = to_drawdown_series(&returns);

// Top-3 worst drawdown episodes
let episodes = drawdown_details(&dd_series, &dates, 3);
for ep in &episodes {
    println!(
        "{} → {} (valley): {:.1}% over {} days",
        ep.start,
        ep.valley,
        ep.max_drawdown * 100.0,
        ep.duration_days,
    );
}

// Average depth of the 3 worst drawdown episodes
let avg_dd = mean_episode_drawdown(&dd_series, 3);
```

### 4. Period Aggregation and Kelly Criterion

```rust
use finstack_analytics::aggregation::{group_by_period, period_stats_from_grouped};
use finstack_core::dates::PeriodKind;
use finstack_core::dates::{Date, Duration, Month};

// Daily returns + dates
let dates: Vec<Date> = (0..60)
    .map(|i| Date::from_calendar_date(2024, Month::January, 1).unwrap()
        + Duration::days(i))
    .collect();
let returns: Vec<f64> = (0..60).map(|i| (i as f64 % 7.0 - 3.0) * 0.005).collect();

// Compound daily returns into monthly buckets
let monthly = group_by_period(&dates, &returns, PeriodKind::Monthly, None);
let stats = period_stats_from_grouped(&monthly);

println!("Win rate:      {:.0}%", stats.win_rate * 100.0);
println!("Kelly:         {:.2}", stats.kelly_criterion);
println!("Profit factor: {:.2}", stats.profit_factor);
println!("Best month:    {:.1}%", stats.best * 100.0);
println!("Worst month:   {:.1}%", stats.worst * 100.0);
```

### 5. Lookback Selectors (MTD / QTD / YTD)

```rust
use finstack_analytics::{lookback::{mtd_select, ytd_select}, returns::comp_total};
use finstack_core::dates::{Date, Duration, Month};

let dates: Vec<Date> = (0..252)
    .map(|i| Date::from_calendar_date(2024, Month::January, 1).unwrap()
        + Duration::days(i))
    .collect();
let returns: Vec<f64> = vec![0.001; 252];

let today = Date::from_calendar_date(2024, Month::September, 30).unwrap();

let mtd_range = mtd_select(&dates, today);
let ytd_range = ytd_select(&dates, today);

let mtd_ret = comp_total(&returns[mtd_range]);
let ytd_ret = comp_total(&returns[ytd_range]);
println!("MTD: {:.2}%  YTD: {:.2}%", mtd_ret * 100.0, ytd_ret * 100.0);
```

### 6. Benchmark Alignment and Greeks

```rust
use finstack_analytics::benchmark::{greeks, rolling_greeks};
use finstack_core::dates::{Date, Month};

let portfolio: Vec<f64> = (0..20).map(|i| (i as f64 - 10.0) * 0.002).collect();
let benchmark: Vec<f64> = (0..20).map(|i| (i as f64 - 10.0) * 0.001).collect();
let dates: Vec<Date> = (1..=20)
    .map(|d| Date::from_calendar_date(2024, Month::January, d).unwrap())
    .collect();

// Point-in-time greeks
let g = greeks(&portfolio, &benchmark, 252.0);
println!("Alpha: {:.4}  Beta: {:.4}  R²: {:.4}", g.alpha, g.beta, g.r_squared);

// Rolling 10-day greeks
let rolling = rolling_greeks(&portfolio, &benchmark, &dates, 10, 252.0);
assert_eq!(rolling.betas.len(), 11); // 20 − 10 + 1
```

---

## Numerical Design

### Compounding in Log Space

`comp_sum` and `comp_total` accumulate in log-space using a Neumaier/Kahan compensated sum, then exponentiate:

```
log(Π (1 + rᵢ)) = Σ log(1 + rᵢ)   [numerically stable sum]
result = exp(log_sum) - 1
```

Growth factors below `1e-18` are clamped so that returns ≤ −100% produce a finite near-total-loss rather than NaN.

### Sample Statistics (n-1)

All volatility, standard deviation, variance, covariance, skewness, and kurtosis computations use **sample statistics** (divide by `n-1`), matching Bloomberg, QuantLib, and the `OnlineStats` / `OnlineCovariance` convention. Skewness and kurtosis use the Fisher bias-corrected formulas (G₁ and G₂), matching Excel `SKEW()` and `KURT()`.

A `population_variance()` function is available in `math::stats` for the rare cases where the population (n) denominator is needed (e.g., moment-matching in Monte Carlo).

### Annualization

The annualization factor is derived from `PeriodKind::annualization_factor()`:

| Frequency   | Factor |
|-------------|--------|
| Daily       | 252    |
| Weekly      | 52     |
| Monthly     | 12     |
| Quarterly   | 4      |
| Semi-annual | 2      |
| Annual      | 1      |

Return metrics scale by `factor`; volatility/tracking-error scale by `sqrt(factor)`.

---

## Adding New Features

The analytics module is **pure-function-first**: analytics logic lives in stateless functions in sub-modules; `Performance` is just an orchestrator. When adding new analytics, follow this pattern.

### Adding a New Scalar Metric

1. **Add a pure function** to the most appropriate sub-module:
   - Return or excess-return-based → `returns.rs`
   - Pure return-based risk metric → `risk_metrics/return_based.rs`
   - Tail-risk / distribution-shape metric → `risk_metrics/tail_risk.rs`
   - Drawdown-derived ratio → `drawdown.rs`
   - Benchmark-relative → `benchmark.rs`
   - Period-aggregation metric → `aggregation.rs`

2. **Follow the function signature convention**:

   ```rust
   /// Brief description.
   ///
   /// [Extended explanation with formula.]
   ///
   /// # Arguments
   ///
   /// * `returns` - ...
   /// * `ann_factor` - Number of periods per year.
   ///
   /// # Returns
   ///
   /// [What the function returns, including edge cases.]
   ///
   /// # Examples
   ///
   /// ```rust
   /// use finstack_analytics::risk_metrics::return_based::my_metric;
   /// let result = my_metric(&[0.01, -0.005, 0.02], 252.0);
   /// assert!(result.is_finite());
   /// ```
   ///
   /// # References
   ///
   /// - AuthorYear: see docs/REFERENCES.md#authorYear
   pub fn my_metric(returns: &[f64], ann_factor: f64) -> f64 { ... }
   ```

3. **Re-export from `mod.rs`** so callers reach it via `finstack_analytics::my_metric`.

4. **Add a method to `Performance`** if the metric is per-ticker:

   ```rust
   pub fn my_metric(&self) -> Vec<f64> {
       (0..self.ticker_names.len())
           .map(|i| risk_metrics::my_metric(self.active_returns(i), self.ann()))
           .collect()
   }
   ```

5. **Add the reference** (if it has a canonical academic source) to `docs/REFERENCES.md`.

6. **Wire up the bindings** in `finstack-py/src/bindings/analytics/` (PyO3) and `finstack-wasm/src/api/analytics/` (wasm-bindgen). Update the matching `finstack-py/finstack/analytics/__init__.pyi` stub and the `parity_contract.toml` entry. See `AGENTS.md` "Naming Strategy" for the triplet-update workflow.

### Adding a New Rolling Series

Rolling outputs always return a pair of parallel vectors (dates + values) in a small struct:

```rust
pub struct RollingMyMetric {
    pub values: Vec<f64>,
    pub dates: Vec<Date>,
}

pub fn rolling_my_metric(
    returns: &[f64],
    dates: &[Date],
    window: usize,
    ann_factor: f64,
) -> RollingMyMetric {
    let n = returns.len().min(dates.len());
    if n < window || window == 0 {
        return RollingMyMetric { values: vec![], dates: vec![] };
    }
    let mut values = Vec::with_capacity(n - window + 1);
    let mut out_dates = Vec::with_capacity(n - window + 1);
    for i in window..=n {
        let slice = &returns[i - window..i];
        values.push(my_metric(slice, ann_factor));
        out_dates.push(dates[i - 1]);
    }
    RollingMyMetric { values, dates: out_dates }
}
```

Produce `n - window + 1` points; the date label is always the **last date of the window**.

### Adding a New Output Type for `PeriodStats`

To expose a new period-aggregated field:

1. Add the field to `PeriodStats` in `aggregation.rs`.
2. Compute it inside `period_stats` from the `wins` / `losses` / `returns` vectors.
3. Add the zero-case default inside the early-return block.
4. Update the unit test.

### Adding a New Lookback Period

To add a custom lookback selector (e.g., `since_inception`):

1. Add a new function to `lookback.rs` returning `Range<usize>`:

   ```rust
   pub fn since_inception_select(dates: &[Date]) -> Range<usize> {
       0..dates.len()
   }
   ```

2. Expose it from `mod.rs`.
3. Add a field to `LookbackReturns` in `performance.rs` and compute it in `lookback_returns`.

### Test Requirements

Every new function needs:

- **Happy-path test**: verify the result against a known analytic value.
- **Empty-input test**: confirm the function returns `0.0` / empty without panicking.
- **Edge-case test**: zero volatility, all-positive returns, total-wipeout, etc.
- **Doc example**: at least one `# Examples` block that runs as a doctest.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn my_metric_basic() {
        let r = [0.01, 0.02, -0.005];
        let result = my_metric(&r, 252.0);
        assert!((result - EXPECTED).abs() < 1e-6);
    }

    #[test]
    fn my_metric_empty() {
        assert_eq!(my_metric(&[], 252.0), 0.0);
    }
}
```

---

## When to Use This Module vs. Others

| Need | Use |
|------|-----|
| Portfolio performance analytics on `Vec<f64>` returns | `finstack_analytics` ✓ |
| Python-facing analytics on a Polars DataFrame | `finstack-py` (`Performance`) |
| Realized volatility from OHLC prices | `core::math::stats` (`realized_variance`) |
| Pricing a derivative or computing Greeks | `valuations` |
| NPV or IRR of cashflow schedules | `core::cashflow` |
| Curve interpolation or discount factors | `core::math::interp`, `market_data` |

Keep analytics functions in this module **instrument-agnostic** and **dependency-free**: no Polars, no market data curves, no instrument types. If a function needs a curve or a price model, it belongs in `valuations` or `scenarios`.

Data-quality note: compounded-return helpers now propagate `NaN` inputs rather
than coercing them into synthetic wipeouts, and `Performance::new(...)` rejects
ragged price matrices or ticker/date mismatches instead of silently aligning
them with truncated series.
