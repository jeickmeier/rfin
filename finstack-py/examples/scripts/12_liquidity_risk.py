"""Liquidity risk example.

Demonstrates the ``finstack.portfolio`` liquidity risk bindings by working
through a full single-stock workflow:

1. Simulate 100 trading days of prices, returns, and volumes.
2. Estimate Roll (1984) effective spread and Amihud (2002) illiquidity.
3. Compute days-to-liquidate for a $5M position at 10% participation.
4. Compute Bangia et al. (1999) liquidity-adjusted VaR.
5. Estimate Almgren-Chriss (2001) market impact for a 5-day liquidation.
6. Estimate Kyle's (1985) lambda from the simulated data.
7. Show portfolio-level liquidity tier classification.

Run standalone:

    python finstack-py/examples/12_liquidity_risk.py
"""

from __future__ import annotations

import numpy as np

from finstack.portfolio import (
    almgren_chriss_impact,
    amihud_illiquidity,
    days_to_liquidate,
    kyle_lambda,
    liquidity_tier,
    lvar_bangia,
    roll_effective_spread,
)


def simulate_price_path(
    n_days: int = 100,
    initial_price: float = 100.0,
    annual_vol: float = 0.25,
    avg_daily_volume: float = 2_000_000.0,
    seed: int = 42,
) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    """Simulate a geometric-Brownian price path with a bid-ask bounce overlay.

    The bid-ask bounce injects negative serial covariance into observed
    returns so the Roll estimator produces a meaningful spread. Daily
    volumes are drawn from a log-normal distribution centered on
    ``avg_daily_volume``.

    Returns
    -------
    prices : ndarray of shape (n_days,)
    returns : ndarray of shape (n_days - 1,)
    volumes : ndarray of shape (n_days - 1,)
    """
    rng = np.random.default_rng(seed)
    daily_vol = annual_vol / np.sqrt(252.0)

    # Efficient-price log-returns.
    eff_returns = rng.normal(0.0, daily_vol, size=n_days)

    # Bid-ask bounce: alternating +/- half-spread of 10bp.
    half_spread = 0.0005
    bounce = np.where(rng.random(n_days) < 0.5, half_spread, -half_spread)

    log_prices = np.cumsum(eff_returns) + bounce
    prices = initial_price * np.exp(log_prices)

    returns = np.diff(prices) / prices[:-1]
    # Log-normal daily volumes with modest dispersion.
    volumes = rng.lognormal(mean=np.log(avg_daily_volume), sigma=0.35, size=n_days - 1)

    return prices, returns, volumes


def section(title: str) -> None:
    print("\n" + "=" * 72)
    print(title)
    print("=" * 72)


def main() -> None:
    # ------------------------------------------------------------------
    # 1. Simulate market data
    # ------------------------------------------------------------------
    section("Simulated market data (100 trading days)")
    prices, returns, volumes = simulate_price_path()
    print(f"  Starting price:     {prices[0]:>12,.4f}")
    print(f"  Ending price:       {prices[-1]:>12,.4f}")
    print(f"  Mean daily return:  {returns.mean():>12.4%}")
    print(f"  Realized vol (daily): {returns.std():>10.4%}")
    print(f"  Mean daily volume:  {volumes.mean():>12,.0f} shares")

    # ------------------------------------------------------------------
    # 2. Spread / illiquidity estimators
    # ------------------------------------------------------------------
    section("Spread and illiquidity estimators")
    roll = roll_effective_spread(returns.tolist())
    amihud = amihud_illiquidity(returns.tolist(), volumes.tolist())
    print(f"  Roll effective spread:    {roll:>12.6f}  (relative to price)")
    print(f"  Roll spread (bps):        {roll * 1e4:>12.2f} bps")
    print(f"  Amihud illiquidity:       {amihud:>12.4e}  (|r| / volume)")

    # ------------------------------------------------------------------
    # 3. Days to liquidate a $5M position
    # ------------------------------------------------------------------
    section("Position sizing and tier classification")
    position_value = 5_000_000.0
    participation = 0.10
    # Convert ADV from shares to dollar-notional using the ending price so
    # units line up with `position_value`.
    adv_notional = volumes.mean() * prices[-1]

    dtl = days_to_liquidate(position_value, adv_notional, participation)
    tier = liquidity_tier(dtl)
    print(f"  Position value:           ${position_value:>14,.0f}")
    print(f"  ADV (notional):           ${adv_notional:>14,.0f}")
    print(f"  Participation rate:       {participation:>14.0%}")
    print(f"  Days to liquidate:        {dtl:>14.4f}")
    print(f"  Liquidity tier:           {tier:>14}")

    # Show how the tier shifts if the position is 50x bigger (illiquid).
    big_dtl = days_to_liquidate(position_value * 50.0, adv_notional, participation)
    print(
        f"  (hypothetical $250M: dtl={big_dtl:,.2f} days, tier={liquidity_tier(big_dtl)})"
    )

    # ------------------------------------------------------------------
    # 4. Liquidity-adjusted VaR (Bangia et al. 1999)
    # ------------------------------------------------------------------
    section("Liquidity-adjusted VaR (Bangia et al. 1999)")
    # Use a 2.33-sigma 99% daily VaR as the input risk number.
    daily_vol = float(returns.std())
    var_99 = 2.326 * daily_vol * position_value
    spread_mean = 0.0010  # 10bp mean relative spread (given).
    spread_vol = 0.0003  # 3bp spread volatility.

    lvar = lvar_bangia(
        var=var_99,
        spread_mean=spread_mean,
        spread_vol=spread_vol,
        confidence=0.99,
        position_value=position_value,
    )
    print(f"  Base 99% VaR:             ${lvar['var']:>14,.2f}")
    print(f"  Spread cost add-on:       ${lvar['spread_cost']:>14,.2f}")
    print(f"  Bangia LVaR:              ${lvar['lvar']:>14,.2f}")
    print(f"  LVaR / VaR ratio:         {lvar['lvar_ratio']:>14.4f}x")

    # ------------------------------------------------------------------
    # 5. Almgren-Chriss impact for liquidating over 5 days
    # ------------------------------------------------------------------
    section("Almgren-Chriss market impact (5-day liquidation)")
    # Size in shares at the ending price.
    shares = position_value / prices[-1]
    # Empirical calibration: gamma ~ spread / (2 * ADV), eta ~ vol * sqrt(mid / ADV).
    gamma = roll / (2.0 * volumes.mean()) if np.isfinite(roll) else 1e-8
    eta = daily_vol * np.sqrt(prices[-1] / volumes.mean())

    impact = almgren_chriss_impact(
        position_size=shares,
        avg_daily_volume=volumes.mean(),
        volatility=daily_vol,
        execution_horizon_days=5.0,
        permanent_impact_coef=gamma,
        temporary_impact_coef=eta,
    )
    print(f"  Shares to liquidate:      {shares:>14,.0f}")
    print(f"  Permanent impact coef:    {gamma:>14.4e}")
    print(f"  Temporary impact coef:    {eta:>14.4e}")
    print(f"  Permanent impact cost:    {impact['permanent_impact']:>14,.4f}")
    print(f"  Temporary impact cost:    {impact['temporary_impact']:>14,.4f}")
    print(f"  Total impact cost:        {impact['total_impact']:>14,.4f}")
    print(f"  Cost (bps of notional):   {impact['expected_cost_bps']:>14.2f} bps")

    # ------------------------------------------------------------------
    # 6. Kyle's lambda from simulated data
    # ------------------------------------------------------------------
    section("Kyle's lambda (price impact per unit of volume)")
    kyle = kyle_lambda(volumes.tolist(), returns.tolist())
    print(f"  Estimated Kyle lambda:    {kyle:>14.4e}")
    print(f"  Expected impact for 10k-share order: {kyle * 10_000:>8.4f} (price units)")

    # ------------------------------------------------------------------
    # 7. Portfolio tier classification
    # ------------------------------------------------------------------
    section("Portfolio tier classification across positions")
    portfolio = [
        ("AAPL_5M", 5_000_000.0, 8e9),
        ("SMALL_CAP_5M", 5_000_000.0, 20_000_000.0),
        ("MICRO_CAP_2M", 2_000_000.0, 500_000.0),
        ("MEGA_CAP_50M", 50_000_000.0, 50e9),
    ]
    print(f"  {'Position':<18} {'PV':>14} {'ADV':>16} {'DTL':>10} {'Tier':>8}")
    for name, pv, adv in portfolio:
        d = days_to_liquidate(pv, adv, 0.10)
        t = liquidity_tier(d)
        print(f"  {name:<18} ${pv:>13,.0f} ${adv:>15,.0f} {d:>10.3f} {t:>8}")

    section("Done")


if __name__ == "__main__":
    main()
