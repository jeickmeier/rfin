# finstack-analytics

`finstack-analytics` provides portfolio performance and risk analytics on numeric
return series and `finstack_core::dates::Date`, with no DataFrame or Polars
dependency in the Rust API.

The crate is intentionally **pure-function-first**:

- Every metric is available as a standalone function over `&[f64]`.
- The `Performance` facade adds date windowing, benchmark state, caching, and
  per-ticker batch evaluation over a panel of price series.
- Common APIs are re-exported at the crate root, so users can write
  `use finstack_analytics::{Performance, value_at_risk, to_drawdown_series};`
  without importing every submodule directly.

## What This Crate Covers

- **Returns and compounding**: simple returns, excess returns, rebasing, price
  reconstruction, cumulative compounding, total compounded return.
- **Return-based risk metrics**: CAGR, mean return, volatility, Sharpe, Sortino,
  downside deviation, Omega, gain-to-pain, modified Sharpe, geometric mean.
- **Tail-risk analytics**: historical VaR, Expected Shortfall, parametric VaR,
  Cornish-Fisher VaR, skewness, kurtosis, tail ratios, outlier win/loss ratios.
- **Drawdown analytics**: drawdown paths, drawdown episodes, max drawdown,
  average drawdown, Ulcer Index, CDaR, Calmar, Martin, Sterling, Burke, Pain,
  recovery factor.
- **Benchmark-relative analytics**: tracking error, information ratio, beta,
  alpha/beta/R-squared greeks, capture ratios, batting average, Treynor,
  M-squared, multi-factor regression.
- **Rolling series**: rolling Sharpe, Sortino, volatility, and rolling greeks.
- **Aggregation and lookbacks**: period compounding, win/loss streak metrics,
  Kelly criterion, MTD/QTD/YTD/FYTD range selection.
- **Ruin modeling**: seedable Monte Carlo ruin estimates with confidence
  intervals for wealth-floor, terminal-floor, and drawdown-breach definitions.
- **Serde-friendly results**: `Performance`, rolling outputs, drawdown episodes,
  benchmark regression outputs, lookback outputs, and ruin configs/results all
  support serialization.

This crate is for **instrument-agnostic analytics** on returns and prices. If a
workflow needs curves, pricing models, or instrument valuation logic, it likely
belongs in `finstack-valuations` or `finstack-core`, not here.

## Dependencies

Use the analytics crate directly:

```toml
[dependencies]
finstack-analytics = { path = "../finstack/analytics" }
finstack-core = { path = "../finstack/core" }
```

Or use the umbrella crate:

```toml
[dependencies]
finstack = { path = "../finstack" }
```

Import paths use underscores even though the package name uses hyphens:

```rust
use finstack_analytics::Performance;
use finstack_core::dates::{Date, Month, PeriodKind};
```

## Quick Start

### Stateful portfolio analytics with `Performance`

Use `Performance` when you have a full panel of prices and want one object that
can:

- cache returns and drawdowns,
- switch benchmarks,
- reset the active date window,
- return one metric per ticker,
- produce rolling series and drawdown episode reports.

```rust
use finstack_analytics::Performance;
use finstack_core::dates::{Date, Month, PeriodKind};

let dates: Vec<Date> = (1..=6)
    .map(|d| Date::from_calendar_date(2025, Month::January, d).unwrap())
    .collect();

let benchmark = vec![100.0, 101.0, 99.0, 102.0, 101.0, 103.0];
let portfolio = vec![100.0, 103.0, 100.0, 104.0, 102.0, 106.0];

let mut perf = Performance::new(
    dates,
    vec![benchmark, portfolio],
    vec!["SPY".to_string(), "ALPHA".to_string()],
    Some("SPY"),
    PeriodKind::Daily,
    false,
)
.expect("price matrix should be aligned and valid");

let sharpe = perf.sharpe(0.02);
let max_drawdown = perf.max_drawdown();
let beta = perf.beta();
let info_ratio = perf.information_ratio();
let rolling = perf.rolling_sharpe(1, 3, 0.02);

assert_eq!(sharpe.len(), 2);
assert_eq!(beta.len(), 2);
assert_eq!(rolling.values.len(), 3);

perf.reset_date_range(
    Date::from_calendar_date(2025, Month::January, 3).unwrap(),
    Date::from_calendar_date(2025, Month::January, 6).unwrap(),
);

let windowed_cagr = perf.cagr();
assert_eq!(windowed_cagr.len(), 2);
assert_eq!(max_drawdown.len(), 2);
```

### Standalone analytics on return slices

Use the standalone functions when you already have aligned return slices and
want allocation-light, composable building blocks.

```rust
use finstack_analytics::{
    comp_total, expected_shortfall, group_by_period, period_stats, sortino,
    to_drawdown_series, tracking_error, value_at_risk,
};
use finstack_core::dates::{Date, Month, PeriodKind};

let returns = vec![0.01, -0.005, 0.02, -0.01, 0.015, 0.008];
let benchmark = vec![0.008, -0.004, 0.012, -0.009, 0.01, 0.006];
let dates: Vec<Date> = (1..=6)
    .map(|d| Date::from_calendar_date(2025, Month::January, d).unwrap())
    .collect();

let total = comp_total(&returns);
let sortino = sortino(&returns, true, 252.0);
let var_95 = value_at_risk(&returns, 0.95, None);
let es_95 = expected_shortfall(&returns, 0.95, None);
let drawdown = to_drawdown_series(&returns);
let tracking_err = tracking_error(&returns, &benchmark, true, 252.0);

let monthly = group_by_period(&dates, &returns, PeriodKind::Monthly, None);
let stats = period_stats(&monthly);

assert!(total.is_finite());
assert!(sortino.is_finite() || sortino.is_infinite());
assert!(es_95 <= var_95);
assert_eq!(drawdown.len(), returns.len());
assert!(tracking_err >= 0.0);
assert!(stats.win_rate >= 0.0);
```

## Main Modules

| Module | Use It For | Notable APIs |
|--------|------------|--------------|
| `performance` | Stateful portfolio analytics over multiple tickers | `Performance`, `LookbackReturns` |
| `returns` | Return transforms and compounding | `simple_returns`, `excess_returns`, `comp_sum`, `comp_total`, `convert_to_prices`, `rebase`, `clean_returns` |
| `risk_metrics::return_based` | Return-level metrics | `cagr`, `CagrBasis`, `mean_return`, `volatility`, `sharpe`, `sortino`, `omega_ratio`, `estimate_ruin` |
| `risk_metrics::tail_risk` | Distribution and downside-tail metrics | `value_at_risk`, `expected_shortfall`, `parametric_var`, `cornish_fisher_var`, `skewness`, `kurtosis`, `tail_ratio` |
| `risk_metrics::rolling` | Rolling series outputs | `rolling_sharpe`, `rolling_sortino`, `rolling_volatility` |
| `drawdown` | Drawdown paths, episodes, and drawdown-derived ratios | `to_drawdown_series`, `drawdown_details`, `cdar`, `ulcer_index`, `calmar`, `martin_ratio`, `sterling_ratio` |
| `benchmark` | Benchmark-relative and regression-style analytics | `tracking_error`, `information_ratio`, `beta`, `greeks`, `rolling_greeks`, `multi_factor_greeks`, `align_benchmark` |
| `aggregation` | Period compounding and trading statistics | `group_by_period`, `period_stats`, `PeriodStats` |
| `lookback` | Index-range selectors for dated arrays | `mtd_select`, `qtd_select`, `ytd_select`, `fytd_select` |

## Core Types

### `Performance`

`Performance` is the high-level API for panel analytics. It:

- accepts a chronologically sorted date vector and one price series per ticker,
- computes returns and drawdowns once at construction time,
- stores the active benchmark series,
- supports resetting the active analysis window with `reset_date_range`,
- supports switching benchmarks with `reset_bench_ticker`,
- exposes both scalar metrics and rolling/episode outputs.

Important shape convention:

- `prices[i]` is the full price history for ticker `i`.
- `ticker_names[i]` names that same column.
- `dates.len()` must match each price vector length.
- The internal return grid has length `dates.len() - 1`, so methods like
  `active_dates()` and `dates()` refer to the return-aligned date grid.

### `PeriodStats`

`PeriodStats` summarizes grouped returns with:

- best and worst period,
- win rate,
- average return, average win, average loss,
- longest win and loss streaks,
- payoff ratio, profit ratio, profit factor,
- CPC ratio,
- Kelly criterion.

### Regression and rolling outputs

The benchmark module exposes structured outputs rather than returning opaque
tuples:

- `BetaResult` for beta, standard error, and confidence interval,
- `GreeksResult` for annualized alpha, beta, and R-squared,
- `MultiFactorResult` for annualized alpha, factor loadings, R-squared,
  adjusted R-squared, and residual volatility,
- `RollingGreeks`, `RollingSharpe`, `RollingSortino`, and
  `RollingVolatility` for date-labeled rolling series.

### Drawdown and ruin outputs

- `DrawdownEpisode` captures start, valley, recovery date, duration, and max
  drawdown for an episode.
- `RuinDefinition`, `RuinModel`, and `RuinEstimate` define and report ruin
  simulations with seedable Monte Carlo settings and confidence intervals.

## Core Conventions

- **Returns are simple decimal returns** unless a function explicitly says
  otherwise. `0.01` means `+1%`.
- **Drawdown depths are non-positive fractions**. A 25% drawdown is `-0.25`.
- **CDaR is non-negative** because it is reported as an absolute tail drawdown
  depth.
- **Benchmark alignment operates in return space**. Missing benchmark dates can
  be zero-filled or rejected explicitly with
  `BenchmarkAlignmentPolicy::ErrorOnMissingDates`.
- **Annualization is explicit**. Pure functions either take `ann_factor`
  directly or derive it from `PeriodKind` when called through `Performance`.
- **Rolling series are right-labeled**. The date attached to each rolling value
  is the last date in that window.
- **`*_values` rolling helpers are NaN-padded** while structured rolling
  outputs contain only active windows.
- **`Performance::new(..., use_log_returns = true, ...)` still stores simple
  returns internally** after converting from log returns, so downstream
  compounding and drawdown logic remains coherent.

## Numerical Behavior and Validation

The crate favors explicit, stable behavior over silent coercion:

- Compounding uses compensated summation in log space for long-series stability.
- `Performance::new` rejects ragged price matrices, mismatched ticker names,
  non-finite inputs, negative price-domain issues in simple-return mode, and
  interior invalid returns.
- Multi-factor regression rejects mismatched factor lengths, non-finite factors,
  non-positive annualization factors, and singular or near-singular factor
  matrices instead of returning zero-filled coefficients.
- Ruin estimation is seedable and reproducible for a fixed `RuinModel::seed`.
  Invalid ruin definitions yield `NaN` estimates rather than clipping inputs.
- Degenerate cases often return `0.0`, `NaN`, or `±∞` rather than panicking.
  For example, zero-volatility Sharpe-style denominators or zero-drawdown
  ratios intentionally surface boundary behavior.

## Serialization

Most public result/config types derive `Serialize` and `Deserialize`, including:

- `Performance`
- `LookbackReturns`
- `PeriodStats`
- `DrawdownEpisode`
- `BetaResult`, `GreeksResult`, `MultiFactorResult`, `RollingGreeks`
- `RollingSharpe`, `RollingSortino`, `RollingVolatility`
- `BenchmarkAlignmentPolicy`
- `RuinDefinition`, `RuinModel`, `RuinEstimate`

This makes the crate easy to use in snapshot tests, REST or IPC payloads, and
Python/WASM-adjacent serialization layers.

## When To Use This Crate

Use `finstack-analytics` when you need:

- analytics on return slices or price panels,
- benchmark-relative portfolio statistics,
- drawdown studies and rolling metrics,
- pure Rust analytics without DataFrame dependencies,
- serde-friendly output types for downstream reporting.

Reach for adjacent crates when you need:

- `finstack-core` for low-level dates, calendars, math, and market-data types,
- `finstack-valuations` for pricing, Greeks, and instrument-specific models,
- `finstack-py` for Python-facing analytics workflows and DataFrame ergonomics.

## References

Canonical quantitative references used across the project live in
[`docs/REFERENCES.md`](../../docs/REFERENCES.md).

Notable methods in this crate map to standard finance literature, including:

- VaR / Expected Shortfall
- Calmar, Martin, Sterling, and Burke ratios
- Treynor and M-squared
- active risk / information ratio
- multi-factor regression
- drawdown-tail measures such as CDaR

## Verification

Useful crate-local checks:

```bash
cargo fmt -p finstack-analytics
cargo clippy -p finstack-analytics --all-features -- -D warnings
cargo test -p finstack-analytics
cargo test -p finstack-analytics --doc
RUSTDOCFLAGS='-D warnings' cargo doc -p finstack-analytics --no-deps --all-features
```
