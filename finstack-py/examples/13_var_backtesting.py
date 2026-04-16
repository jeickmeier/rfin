"""VaR Backtesting example.

Simulates a year of portfolio P&L, generates rolling historical VaR
forecasts at 95% and 99%, then runs the full backtesting suite:

1. Breach classification
2. Kupiec Proportion of Failures (unconditional coverage) test
3. Christoffersen conditional coverage test
4. Basel Committee traffic-light classification

Run standalone:

    python finstack-py/examples/13_var_backtesting.py
"""

from __future__ import annotations

import numpy as np

from finstack.analytics import (
    christoffersen_test,
    classify_breaches,
    kupiec_test,
    run_backtest,
    traffic_light,
    value_at_risk,
)


def simulate_pnl(n_days: int, mu: float, sigma: float, seed: int) -> np.ndarray:
    """Simulate `n_days` of daily portfolio P&L from N(mu, sigma)."""
    rng = np.random.default_rng(seed)
    return rng.normal(loc=mu, scale=sigma, size=n_days)


def rolling_historical_var(
    pnl: np.ndarray, lookback: int, confidence: float
) -> tuple[np.ndarray, np.ndarray]:
    """Compute rolling historical VaR forecasts aligned with next-day P&L.

    Returns (forecasts, realized) where forecasts[i] is the VaR from
    pnl[i - lookback : i] and realized[i] is pnl[i].
    """
    n = len(pnl)
    forecasts = []
    realized = []
    for i in range(lookback, n):
        window = pnl[i - lookback : i].tolist()
        forecasts.append(value_at_risk(window, confidence=confidence))
        realized.append(float(pnl[i]))
    return np.asarray(forecasts), np.asarray(realized)


def print_report(label: str, confidence: float, forecasts: np.ndarray, realized: np.ndarray) -> None:
    """Run and print a full backtest report for one VaR series."""
    print(f"\n{'=' * 70}")
    print(f"  VaR Backtest — {label} (confidence = {confidence:.0%})")
    print(f"{'=' * 70}")

    n = len(forecasts)
    print(f"  Observations:               {n}")

    # 1. Breaches
    breaches = classify_breaches(forecasts.tolist(), realized.tolist())
    breach_count = len(breaches)
    expected = n * (1.0 - confidence)
    print(f"  Breaches:                   {breach_count}  (expected ~ {expected:.1f})")
    if breaches:
        first = breaches[0]
        print(
            f"  First breach:               day {first[0]}, "
            f"VaR={first[1]:.4f}, P&L={first[2]:.4f}"
        )

    # 2. Kupiec POF
    lr_k, p_k, reject_k = kupiec_test(breach_count, n, confidence)
    print("\n  Kupiec POF (unconditional coverage):")
    print(f"    LR statistic:             {lr_k:.4f}")
    print(f"    p-value:                  {p_k:.4f}")
    print(f"    Reject H0 at 5%:          {reject_k}")

    # 3. Christoffersen
    breach_series = [realized[i] < forecasts[i] for i in range(n)]
    lr_cc, p_cc, reject_cc = christoffersen_test(breach_series, confidence)
    print("\n  Christoffersen (conditional coverage):")
    print(f"    LR_cc statistic:          {lr_cc:.4f}")
    print(f"    p-value:                  {p_cc:.4f}")
    print(f"    Reject H0 at 5%:          {reject_cc}")

    # 4. Traffic light
    zone, mult = traffic_light(breach_count, n, confidence)
    print("\n  Basel traffic light:")
    print(f"    Zone:                     {zone}")
    print(f"    Capital multiplier:       {mult:.2f}")

    # 5. One-shot aggregated report
    full = run_backtest(
        forecasts.tolist(),
        realized.tolist(),
        confidence=confidence,
        window_size=min(250, n),
    )
    print("\n  Aggregated run_backtest():")
    print(
        f"    kupiec.observed_rate:     {full['kupiec']['observed_rate']:.4f} "
        f"(expected {1.0 - confidence:.4f})"
    )
    print(f"    christoffersen.lr_ind:    {full['christoffersen']['lr_ind']:.4f}")
    print(f"    traffic_light.zone:       {full['traffic_light']['zone']}")


def main() -> None:
    # Simulate 1 year of data plus a lookback warm-up window
    lookback = 250
    total_days = lookback + 250  # 250 forecastable days
    daily_mu = 0.0005  # ~12.5% annualized drift
    daily_sigma = 0.01  # 1% daily vol

    pnl = simulate_pnl(n_days=total_days, mu=daily_mu, sigma=daily_sigma, seed=42)

    print("VaR Backtesting Example")
    print(f"  Simulated days:             {total_days}")
    print(f"  Lookback window:            {lookback}")
    print(f"  Forecast days:              {total_days - lookback}")
    print(f"  Daily drift:                {daily_mu:+.4f}")
    print(f"  Daily volatility:           {daily_sigma:.4f}")

    for confidence, label in [(0.95, "95% Historical VaR"), (0.99, "99% Historical VaR")]:
        forecasts, realized = rolling_historical_var(pnl, lookback, confidence)
        print_report(label, confidence, forecasts, realized)

    print()


if __name__ == "__main__":
    main()
