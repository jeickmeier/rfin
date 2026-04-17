"""Volatility surface arbitrage detection example.

Demonstrates the ``finstack.core.market_data.arbitrage`` bindings that
screen a volatility surface for three classes of model-free arbitrage:

1. **Butterfly** -- strike convexity of total variance (non-negative
   implied density).
2. **Calendar spread** -- expiry monotonicity of total variance
   (w(k, T2) >= w(k, T1) for T2 > T1).
3. **Local-vol density** -- Dupire local variance positivity.

The script:

1. Builds a well-behaved synthetic surface (SABR-like smile plus a mild
   term-structure tilt) and runs all three checks; expects zero
   violations.
2. Corrupts the surface with a non-convex strike slice at one expiry
   and re-runs the butterfly check; expects violations.
3. Builds a decreasing-in-T surface and runs the calendar-spread check;
   expects violations.

Run standalone::

    python finstack-py/examples/09_vol_surface_arbitrage.py
"""

from __future__ import annotations

from collections import Counter

import numpy as np

from finstack.core.market_data.arbitrage import (
    check_all,
    check_butterfly,
    check_calendar_spread,
    check_local_vol_density,
)


# ---------------------------------------------------------------------------
# Synthetic surface builders
# ---------------------------------------------------------------------------


def sabr_like_smile(
    strikes: np.ndarray,
    expiries: np.ndarray,
    atm_vol: float = 0.20,
    smile_curvature: float = 0.8,
    term_slope: float = 0.03,
    forward: float = 100.0,
) -> np.ndarray:
    """Return an ``[n_expiries, n_strikes]`` array of implied vols.

    The shape is a quadratic smile in log-moneyness with a small upward
    term-structure tilt, mimicking a well-behaved SABR calibration:

        vol(K, T) = atm_vol + term_slope * sqrt(T)
                    + smile_curvature * (ln(K/F))^2 / sqrt(T)
    """
    vols = np.empty((len(expiries), len(strikes)), dtype=float)
    k = np.log(strikes / forward)
    for i, t in enumerate(expiries):
        term = atm_vol + term_slope * np.sqrt(t)
        smile = smile_curvature * k**2 / np.sqrt(max(t, 1e-6))
        vols[i, :] = term + smile
    return vols


def inject_butterfly_violation(
    vols: np.ndarray, expiry_idx: int, strikes: np.ndarray
) -> np.ndarray:
    """Return a copy of ``vols`` with a concave (non-convex) slice at
    ``expiry_idx``.

    A convex (arbitrage-free) smile has wings that are *higher* than ATM.
    Inverting that -- making the center higher than the wings -- creates
    a concave total-variance profile and thus butterfly arbitrage.
    """
    out = vols.copy()
    n = len(strikes)
    mid = n // 2
    # Force a concave bump: wings low, center high.
    for j in range(n):
        d = abs(j - mid)
        # Peak at center, dropping off linearly towards the wings.
        out[expiry_idx, j] = 0.30 - 0.015 * d
    return out


def decreasing_in_time_surface(
    strikes: np.ndarray, expiries: np.ndarray
) -> np.ndarray:
    """Return a surface whose total variance *decreases* with maturity --
    a textbook calendar-spread violation.
    """
    vols = np.empty((len(expiries), len(strikes)), dtype=float)
    base = 0.35
    for i, _t in enumerate(expiries):
        # Same raw vol at every expiry means total variance grows with T,
        # which is fine. To force a violation we need total variance to
        # *decrease* with T, so reduce vol faster than 1/sqrt(T).
        vols[i, :] = base - 0.08 * i
    return vols


# ---------------------------------------------------------------------------
# Reporting helpers
# ---------------------------------------------------------------------------


def banner(title: str) -> None:
    print()
    print("=" * 72)
    print(f"  {title}")
    print("=" * 72)


def summarize(label: str, violations: list[dict]) -> None:
    """Print a one-line summary and a per-type/severity breakdown."""
    n = len(violations)
    print(f"{label}: {n} violation(s)")
    if not violations:
        return
    by_type = Counter(v["type"] for v in violations)
    by_sev = Counter(v["severity"] for v in violations)
    print("  by type     :", dict(by_type))
    print("  by severity :", dict(by_sev))
    # Show the top 3 by magnitude for a taste of the detail.
    top = sorted(violations, key=lambda v: -abs(v["magnitude"]))[:3]
    for v in top:
        print(
            f"    [{v['severity']:>9}] {v['type']:<18} "
            f"T={v['expiry']:.3f} K={v['strike']:.2f} "
            f"mag={v['magnitude']:.3e}  {v['message']}"
        )


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------


def main() -> None:
    strikes = np.array([80.0, 90.0, 100.0, 110.0, 120.0])
    expiries = np.array([0.25, 0.5, 1.0, 2.0])
    forward = 100.0
    forward_prices = [forward] * len(expiries)

    # -----------------------------------------------------------------
    # 1. Clean SABR-like surface -- should pass all three checks
    # -----------------------------------------------------------------
    banner("Clean SABR-like surface (expect zero violations)")
    clean = sabr_like_smile(strikes, expiries, forward=forward)
    print("vols (rows=expiries, cols=strikes):")
    with np.printoptions(precision=4, suppress=True):
        print(clean)

    bf = check_butterfly(
        list(strikes), list(expiries), clean.tolist(), forward=forward, tolerance=1e-6
    )
    cs = check_calendar_spread(
        list(strikes), list(expiries), clean.tolist(), forward=forward, tolerance=1e-6
    )
    lv = check_local_vol_density(
        list(strikes), list(expiries), clean.tolist(), forward_prices
    )
    summarize("butterfly     ", bf)
    summarize("calendar spread", cs)
    summarize("local vol density", lv)

    report = check_all(list(strikes), list(expiries), clean.tolist(), forward=forward)
    print(
        f"\ncheck_all summary: passed={report['passed']}, "
        f"total={report['total_violations']}, "
        f"by_severity={report['by_severity']}, "
        f"by_type={report['by_type']}"
    )

    # -----------------------------------------------------------------
    # 2. Butterfly violation at one expiry
    # -----------------------------------------------------------------
    banner("Corrupted surface: butterfly violation at T=1.0 (expiry_idx=2)")
    bad_bf = inject_butterfly_violation(clean, expiry_idx=2, strikes=strikes)
    print("vols (rows=expiries, cols=strikes):")
    with np.printoptions(precision=4, suppress=True):
        print(bad_bf)

    bf = check_butterfly(
        list(strikes), list(expiries), bad_bf.tolist(), forward=forward, tolerance=1e-6
    )
    summarize("butterfly", bf)
    report = check_all(list(strikes), list(expiries), bad_bf.tolist(), forward=forward)
    print(
        f"check_all summary: passed={report['passed']}, "
        f"total={report['total_violations']}, "
        f"by_severity={report['by_severity']}, "
        f"by_type={report['by_type']}"
    )

    # -----------------------------------------------------------------
    # 3. Calendar-spread violation: total variance decreasing in T
    # -----------------------------------------------------------------
    banner("Decreasing-in-T surface (expect calendar-spread violations)")
    bad_cs = decreasing_in_time_surface(strikes, expiries)
    print("vols (rows=expiries, cols=strikes):")
    with np.printoptions(precision=4, suppress=True):
        print(bad_cs)
    # Sanity check: show total variance at ATM falling with T.
    atm_idx = len(strikes) // 2
    w_atm = bad_cs[:, atm_idx] ** 2 * expiries
    print(f"total variance at K={strikes[atm_idx]:.0f}, by expiry: {w_atm}")

    cs = check_calendar_spread(
        list(strikes), list(expiries), bad_cs.tolist(), forward=forward, tolerance=1e-6
    )
    summarize("calendar spread", cs)
    report = check_all(list(strikes), list(expiries), bad_cs.tolist(), forward=forward)
    print(
        f"check_all summary: passed={report['passed']}, "
        f"total={report['total_violations']}, "
        f"by_severity={report['by_severity']}, "
        f"by_type={report['by_type']}"
    )

    print()
    print("Done.")


if __name__ == "__main__":
    main()
