# Analytics

The `finstack-analytics` crate provides 30+ statistical and portfolio analytics
functions, plus a DSL-based expression engine for defining computed metrics.

## Built-in Functions

### Return & Performance

| Function | Description |
|----------|-------------|
| `returns(prices)` | Simple returns from price series |
| `log_returns(prices)` | Log returns |
| `cumulative_return(returns)` | Cumulative compounded return |
| `annualized_return(returns, freq)` | Annualized return |
| `total_return(prices)` | Total return over period |

### Risk Metrics

| Function | Description |
|----------|-------------|
| `volatility(returns, freq)` | Annualized volatility |
| `downside_deviation(returns, mar)` | Downside semi-deviation |
| `var(returns, confidence)` | Value at Risk (historical) |
| `cvar(returns, confidence)` | Conditional VaR / Expected Shortfall |
| `parametric_var(returns, conf)` | Parametric (Gaussian) VaR |
| `cornish_fisher_var(returns, conf)` | CF-adjusted VaR |

### Ratios

| Function | Description |
|----------|-------------|
| `sharpe_ratio(returns, rf, freq)` | Sharpe ratio |
| `sortino_ratio(returns, rf, freq)` | Sortino ratio |
| `calmar_ratio(returns, freq)` | Calmar ratio |
| `information_ratio(returns, bench)` | Information ratio |
| `treynor_ratio(returns, bench, rf)` | Treynor ratio |

### Drawdown

| Function | Description |
|----------|-------------|
| `max_drawdown(returns)` | Maximum drawdown |
| `drawdown_series(returns)` | Running drawdown time series |
| `drawdown_duration(returns)` | Max drawdown duration (periods) |
| `underwater_series(returns)` | Underwater equity curve |

```python
from finstack.analytics import sharpe_ratio, max_drawdown, var

sr = sharpe_ratio(returns, rf=0.05, freq=252)
dd = max_drawdown(returns)
var_95 = var(returns, confidence=0.95)
```

## Detail Pages

- [Expressions](expressions.md) — Expression DSL syntax and evaluation
