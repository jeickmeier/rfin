"""Exotic rate products: coupon profiles and intrinsic payoffs.

This script exercises the deterministic coupon helpers exposed by
``finstack.valuations`` for the exotic rate instrument family:

1. **TARN** (Target Redemption Note) — pays ``max(fixed - L, 0)`` each
   period until the cumulative coupon reaches a target, then redeems.
   Demonstrates early redemption under a path of falling rates.
2. **Snowball** — path-dependent coupon ``c_i = c_{i-1} + fixed - L_i``
   floored at zero.
3. **Inverse Floater** — non-path-dependent ``c = fixed - leverage * L``
   floored at zero.
4. **CMS Spread Option** — intrinsic payoff on a ``10Y - 2Y`` CMS spread
   given realised CMS fixings.
5. **Callable Range Accrual** — accrued coupon over a period when the
   reference rate is observed in a fixed range, simulated over 250
   daily observations.

Full Monte-Carlo pricing for these products is exposed via the standard
``price_instrument_with_metrics`` pipeline; this example focuses on the
deterministic coupon / payoff logic that underlies those pricers.

Run standalone:

    python finstack-py/examples/14_exotic_rates.py
"""

from __future__ import annotations

import numpy as np

from finstack.valuations import (
    callable_range_accrual_accrued,
    cms_spread_option_intrinsic,
    snowball_coupon_profile,
    tarn_coupon_profile,
)


# ---------------------------------------------------------------------------
# 1) TARN
# ---------------------------------------------------------------------------


def demo_tarn() -> dict:
    """8% fixed vs 6M LIBOR, floor 0%, cumulative target 20%.

    Uses semi-annual coupons with a path of gradually falling floating
    rates: the coupon ``fixed - L`` grows each period, and the note
    redeems once the cumulative hits 20%.
    """
    fixed_rate = 0.08
    coupon_floor = 0.0
    target_coupon = 0.20
    day_count_fraction = 0.5  # semi-annual Act/360 ~= 0.5

    # 10 semi-annual periods = 5Y; rates start at 6% and fall by 50bp/period
    floating = np.linspace(0.06, 0.015, 10).tolist()

    result = tarn_coupon_profile(
        fixed_rate=fixed_rate,
        coupon_floor=coupon_floor,
        floating_fixings=floating,
        target_coupon=target_coupon,
        day_count_fraction=day_count_fraction,
    )
    print("=== TARN: 8% vs 6M L, floor 0%, target 20% cumulative ===")
    print(f"  Floating path (%): {[round(100 * x, 2) for x in floating]}")
    print(f"  Coupons paid (%):  {[round(100 * x, 3) for x in result['coupons_paid']]}")
    print(f"  Cumulative (%):    {[round(100 * x, 3) for x in result['cumulative']]}")
    print(f"  Redemption index:  {result['redemption_index']}")
    print(f"  Redeemed early:    {result['redeemed_early']}")
    print()
    return result


# ---------------------------------------------------------------------------
# 2) Snowball
# ---------------------------------------------------------------------------


def demo_snowball() -> list[float]:
    """Initial 5% coupon, accumulates with ``fixed - floating`` each step."""
    initial_coupon = 0.05
    fixed_rate = 0.06
    floor = 0.0
    cap = float("inf")

    # Rates drift down from 5% to 1% — coupon snowballs upward
    floating = np.linspace(0.05, 0.01, 8).tolist()

    coupons = snowball_coupon_profile(
        initial_coupon=initial_coupon,
        fixed_rate=fixed_rate,
        floating_fixings=floating,
        floor=floor,
        cap=cap,
        is_inverse_floater=False,
        leverage=1.0,
    )
    print("=== Snowball: c_0=5%, fixed=6%, floor=0%, no cap ===")
    print(f"  Floating path (%): {[round(100 * x, 2) for x in floating]}")
    print(f"  Coupons (%):       {[round(100 * x, 3) for x in coupons]}")
    print()
    return coupons


# ---------------------------------------------------------------------------
# 3) Inverse Floater
# ---------------------------------------------------------------------------


def demo_inverse_floater() -> list[float]:
    """``max(8% - 2 * LIBOR, 0)`` — coupon collapses as rates rise."""
    fixed_rate = 0.08
    leverage = 2.0
    floor = 0.0
    cap = float("inf")

    # Rates sweep from 1% up to 6% — coupon falls from 6% toward the floor
    floating = np.linspace(0.01, 0.06, 8).tolist()

    coupons = snowball_coupon_profile(
        initial_coupon=0.0,  # ignored for inverse floater
        fixed_rate=fixed_rate,
        floating_fixings=floating,
        floor=floor,
        cap=cap,
        is_inverse_floater=True,
        leverage=leverage,
    )
    print("=== Inverse Floater: max(8% - 2*L, 0%) ===")
    print(f"  Floating path (%): {[round(100 * x, 2) for x in floating]}")
    print(f"  Coupons (%):       {[round(100 * x, 3) for x in coupons]}")
    print()
    return coupons


# ---------------------------------------------------------------------------
# 4) CMS Spread Option
# ---------------------------------------------------------------------------


def demo_cms_spread_option() -> float:
    """Cap on the 10Y-2Y CMS spread at 50bp; evaluate at 1.2% realised."""
    long_cms = 0.045   # 10Y CMS = 4.5%
    short_cms = 0.033  # 2Y CMS = 3.3%
    strike = 0.005     # 50bp strike
    notional = 10_000_000.0

    payoff = cms_spread_option_intrinsic(
        long_cms=long_cms,
        short_cms=short_cms,
        strike=strike,
        is_call=True,
        notional=notional,
    )
    spread = long_cms - short_cms
    print("=== CMS Spread Option: call on 10Y-2Y, strike=50bp, notional $10MM ===")
    print(f"  Long CMS:     {100 * long_cms:.2f}%")
    print(f"  Short CMS:    {100 * short_cms:.2f}%")
    print(f"  Spread:       {100 * spread:.2f}%")
    print(f"  Strike:       {100 * strike:.2f}%")
    print(f"  Intrinsic:    ${payoff:,.2f}")
    print()
    return payoff


# ---------------------------------------------------------------------------
# 5) Callable Range Accrual
# ---------------------------------------------------------------------------


def demo_callable_range_accrual() -> float:
    """Accrue 5% when LIBOR in [2%, 5%]; 250 daily observations."""
    lower = 0.02
    upper = 0.05
    coupon_rate = 0.05
    day_count_fraction = 250.0 / 360.0  # Act/360 full-year observation window

    # Mean 3.5% (mid-range) with 1% vol — most obs will land in range,
    # but tails leak out on both sides.
    rng = np.random.default_rng(seed=42)
    observations = rng.normal(loc=0.035, scale=0.01, size=250).tolist()

    accrued = callable_range_accrual_accrued(
        lower=lower,
        upper=upper,
        observations=observations,
        coupon_rate=coupon_rate,
        day_count_fraction=day_count_fraction,
    )
    in_range = sum(1 for o in observations if lower <= o <= upper)
    fraction = in_range / len(observations)
    print("=== Callable Range Accrual: 5% when LIBOR in [2%, 5%], 250 daily obs ===")
    print(f"  Observations:       {len(observations)}")
    print(f"  In-range:           {in_range} ({100 * fraction:.1f}%)")
    print(f"  Coupon rate:        {100 * coupon_rate:.2f}%")
    print(f"  Day-count fraction: {day_count_fraction:.4f}")
    print(f"  Accrued coupon:     {100 * accrued:.3f}% of notional")
    print()
    return accrued


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------


if __name__ == "__main__":
    demo_tarn()
    demo_snowball()
    demo_inverse_floater()
    demo_cms_spread_option()
    demo_callable_range_accrual()
