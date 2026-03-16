"""Title: Analytics as Polars Expressions
Persona: Portfolio Manager, Quant Analyst
Complexity: Beginner
Runtime: <1 second.

Description:
Demonstrates using finstack analytics as native Polars expression plugins:
- Compute risk metrics (Sharpe, Sortino, VaR) directly in .select() / .with_columns()
- Transform price series to returns and drawdowns as column expressions
- Benchmark-relative metrics (tracking error, beta, information ratio)
- Rolling analytics (rolling Sharpe, rolling volatility)
- Compare expression plugin results with the Performance class
"""

import math
from datetime import date, timedelta

import polars as pl

from finstack.core.analytics import Performance
from finstack.core.analytics.expr import (
    batting_average,
    beta,
    calmar,
    cumulative_returns,
    drawdown_series,
    expected_shortfall,
    gain_to_pain,
    geometric_mean,
    estimate_ruin,
    information_ratio,
    kurtosis,
    max_drawdown,
    mean_return,
    omega_ratio,
    r_squared,
    recovery_factor,
    rolling_sharpe,
    rolling_volatility,
    sharpe,
    simple_returns,
    skewness,
    sortino,
    tracking_error,
    ulcer_index,
    value_at_risk,
    volatility,
)

# ── 1. Sample data ──

n = 500
base_date = date(2023, 1, 2)
dates = [base_date + timedelta(days=i) for i in range(n)]

price_aapl = [150.0]
price_msft = [250.0]
price_spy = [400.0]
for i in range(1, n):
    price_aapl.append(price_aapl[-1] * (1.0 + 0.001 * math.sin(i * 0.3) + 0.0004))
    price_msft.append(price_msft[-1] * (1.0 + 0.0008 * math.cos(i * 0.2) + 0.0003))
    price_spy.append(price_spy[-1] * (1.0 + 0.0002))

prices = pl.DataFrame(
    {"date": dates, "AAPL": price_aapl, "MSFT": price_msft, "SPY": price_spy}
).with_columns(pl.col("date").cast(pl.Date))

print("Price DataFrame:")
print(prices.head())
print()

# ── 2. Tier 1: Scalar risk metrics in a single .select() ──

returns = prices.select(
    simple_returns("AAPL").alias("AAPL"),
    simple_returns("MSFT").alias("MSFT"),
    simple_returns("SPY").alias("SPY"),
).slice(1)

risk_summary = returns.select(
    sharpe("AAPL", freq="daily").alias("aapl_sharpe"),
    sharpe("MSFT", freq="daily").alias("msft_sharpe"),
    sortino("AAPL", freq="daily").alias("aapl_sortino"),
    calmar("AAPL", freq="daily").alias("aapl_calmar"),
    volatility("AAPL", freq="daily").alias("aapl_vol"),
    mean_return("AAPL", freq="daily").alias("aapl_ann_ret"),
    value_at_risk("AAPL", confidence=0.95).alias("aapl_var95"),
    expected_shortfall("AAPL", confidence=0.95).alias("aapl_es95"),
    skewness("AAPL").alias("aapl_skew"),
    kurtosis("AAPL").alias("aapl_kurt"),
    geometric_mean("AAPL").alias("aapl_geo_mean"),
    max_drawdown("AAPL").alias("aapl_max_dd"),
    omega_ratio("AAPL", threshold=0.0).alias("aapl_omega"),
    gain_to_pain("AAPL").alias("aapl_gtp"),
    ulcer_index("AAPL").alias("aapl_ulcer"),
    estimate_ruin(
        "AAPL",
        definition="drawdown_breach",
        threshold=0.2,
        horizon_periods=63,
        n_paths=512,
        block_size=5,
        seed=42,
    ).alias("aapl_ruin"),
    recovery_factor("AAPL").alias("aapl_recovery"),
)
print("Tier 1 — Scalar Risk Metrics (single .select() call):")
print(risk_summary)
print()

# ── 3. Tier 2: Series transforms as column expressions ──

enriched = prices.with_columns(
    simple_returns("AAPL").alias("aapl_returns"),
    cumulative_returns(simple_returns("AAPL")).alias("aapl_cum"),
    drawdown_series(simple_returns("AAPL")).alias("aapl_drawdown"),
)
print("Tier 2 — Series Transforms (enriched DataFrame):")
print(enriched.head(10))
print()

# ── 4. Tier 3: Benchmark-relative metrics ──

benchmark_metrics = returns.select(
    tracking_error("AAPL", "SPY", freq="daily").alias("te_aapl"),
    tracking_error("MSFT", "SPY", freq="daily").alias("te_msft"),
    beta("AAPL", "SPY").alias("beta_aapl"),
    beta("MSFT", "SPY").alias("beta_msft"),
    information_ratio("AAPL", "SPY", freq="daily").alias("ir_aapl"),
    information_ratio("MSFT", "SPY", freq="daily").alias("ir_msft"),
    r_squared("AAPL", "SPY").alias("r2_aapl"),
    batting_average("AAPL", "SPY").alias("ba_aapl"),
)
print("Tier 3 — Benchmark-Relative Metrics:")
print(benchmark_metrics)
print()

# ── 5. Tier 4: Rolling analytics ──

rolling = returns.with_columns(
    rolling_sharpe("AAPL", window=60, freq="daily").alias("roll_sharpe_60d"),
    rolling_volatility("AAPL", window=60, freq="daily").alias("roll_vol_60d"),
)
print("Tier 4 — Rolling Analytics (tail):")
print(rolling.tail(10))
print()

# ── 6. Side-by-side: Expression plugins vs Performance class ──

perf = Performance(prices, benchmark_ticker="SPY", freq="daily")
perf_sharpe = perf.sharpe()

expr_sharpe = returns.select(
    sharpe("AAPL", freq="daily").alias("AAPL"),
    sharpe("MSFT", freq="daily").alias("MSFT"),
)

print("Side-by-Side Comparison:")
print("Performance class (sharpe):")
print(perf_sharpe)
print()
print("Expression plugin (sharpe):")
print(expr_sharpe)
print()

# Verify exact match
perf_val = perf_sharpe.filter(pl.col("ticker") == "AAPL")["sharpe"].item()
expr_val = expr_sharpe["AAPL"].item()
assert abs(perf_val - expr_val) < 1e-10, f"Mismatch: {perf_val} != {expr_val}"
print(f"Sharpe ratios match exactly: {expr_val:.6f}")
