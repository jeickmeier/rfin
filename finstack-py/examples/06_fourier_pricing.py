"""Fourier-transform European option pricing example.

Demonstrates the Fourier pricing bindings exposed from the Rust
``finstack-valuations`` crate through ``finstack.valuations``:

1. ATM European call under Black-Scholes via the COS and Lewis methods,
   cross-checked against the closed-form Black-Scholes price.
2. A Variance Gamma smile across strikes (sigma=0.2, theta=-0.1, nu=0.2).
3. A Merton jump-diffusion price (sigma=0.2, lambda=0.5, mu_j=-0.1,
   sigma_j=0.15) across strikes.
4. Verification that put-call parity holds for each model/method.
5. A compact pricing grid summarising the full experiment.

Run:

    python finstack-py/examples/06_fourier_pricing.py
"""

from __future__ import annotations

import math
from typing import Callable

import numpy as np

from finstack.valuations import (
    bs_cos_price,
    bs_lewis_price,
    merton_jump_cos_price,
    vg_cos_price,
)


# ---------------------------------------------------------------------------
# Closed-form Black-Scholes reference (for cross-checking the Fourier methods)
# ---------------------------------------------------------------------------


def _norm_cdf(x: float) -> float:
    """Standard-normal CDF using ``math.erf`` (no SciPy dependency)."""
    return 0.5 * (1.0 + math.erf(x / math.sqrt(2.0)))


def bs_closed_form(
    spot: float,
    strike: float,
    rate: float,
    dividend: float,
    vol: float,
    maturity: float,
    is_call: bool,
) -> float:
    """Black-Scholes-Merton closed-form price with continuous dividend yield."""
    if maturity <= 0.0 or vol <= 0.0:
        intrinsic = max(spot - strike, 0.0) if is_call else max(strike - spot, 0.0)
        return intrinsic
    fwd = spot * math.exp((rate - dividend) * maturity)
    sqrt_t = math.sqrt(maturity)
    d1 = (math.log(fwd / strike) + 0.5 * vol * vol * maturity) / (vol * sqrt_t)
    d2 = d1 - vol * sqrt_t
    df = math.exp(-rate * maturity)
    if is_call:
        return df * (fwd * _norm_cdf(d1) - strike * _norm_cdf(d2))
    return df * (strike * _norm_cdf(-d2) - fwd * _norm_cdf(-d1))


# ---------------------------------------------------------------------------
# Utility printing helpers
# ---------------------------------------------------------------------------


def banner(title: str) -> None:
    """Print a labelled section separator."""
    print()
    print("=" * 78)
    print(f"  {title}")
    print("=" * 78)


def check_parity(
    pricer: Callable[..., float],
    spot: float,
    strike: float,
    rate: float,
    dividend: float,
    maturity: float,
    *extra,
) -> tuple[float, float, float]:
    """Return (call, put, parity residual) for a given pricing callable.

    The callable signature is expected to be
    ``fn(spot, strike, rate, dividend, *extra, maturity, is_call, ...)``
    where ``*extra`` captures any model-specific parameters between the
    discount inputs and the maturity/is_call flags.
    """
    call = pricer(spot, strike, rate, dividend, *extra, maturity, True)
    put = pricer(spot, strike, rate, dividend, *extra, maturity, False)
    forward_disc = spot * math.exp(-dividend * maturity) - strike * math.exp(-rate * maturity)
    residual = (call - put) - forward_disc
    return call, put, residual


# ---------------------------------------------------------------------------
# Main example
# ---------------------------------------------------------------------------


def main() -> None:
    # Common market inputs.
    spot = 100.0
    rate = 0.05
    dividend = 0.02
    maturity = 1.0

    # -----------------------------------------------------------------
    # 1. ATM Black-Scholes via COS, Lewis, and closed-form.
    # -----------------------------------------------------------------
    banner("Black-Scholes ATM call: COS vs. Lewis vs. closed form")

    bs_vol = 0.2
    strike_atm = 100.0

    cos_price = bs_cos_price(
        spot, strike_atm, rate, dividend, bs_vol, maturity, True
    )
    lewis_price = bs_lewis_price(
        spot, strike_atm, rate, dividend, bs_vol, maturity, True
    )
    cf_price = bs_closed_form(
        spot, strike_atm, rate, dividend, bs_vol, maturity, True
    )

    print(f"  Spot={spot:.2f}  Strike={strike_atm:.2f}  r={rate:.2%}  q={dividend:.2%}")
    print(f"  sigma={bs_vol:.2%}  T={maturity:.2f}y")
    print()
    print(f"  {'Method':<18s} {'Price':>12s} {'Error vs. CF':>18s}")
    print("  " + "-" * 50)
    print(f"  {'Closed form':<18s} {cf_price:>12.6f} {'-':>18s}")
    print(f"  {'COS (N=128)':<18s} {cos_price:>12.6f} {abs(cos_price - cf_price):>18.2e}")
    print(f"  {'Lewis':<18s} {lewis_price:>12.6f} {abs(lewis_price - cf_price):>18.2e}")

    # Confirm COS agrees with closed form at high-precision.
    assert abs(cos_price - cf_price) < 1e-4, (
        f"COS disagrees with Black-Scholes: {cos_price} vs {cf_price}"
    )

    # -----------------------------------------------------------------
    # 2. Variance Gamma smile across strikes.
    # -----------------------------------------------------------------
    banner("Variance Gamma call prices across strikes "
           "(sigma=0.20, theta=-0.10, nu=0.20)")

    vg_sigma, vg_theta, vg_nu = 0.20, -0.10, 0.20
    strikes = np.array([80.0, 90.0, 95.0, 100.0, 105.0, 110.0, 120.0])

    vg_calls = np.array(
        [
            vg_cos_price(
                spot, float(k), rate, dividend, vg_sigma, vg_theta, vg_nu,
                maturity, True,
            )
            for k in strikes
        ]
    )
    bs_calls = np.array(
        [bs_closed_form(spot, float(k), rate, dividend, bs_vol, maturity, True)
         for k in strikes]
    )

    print(f"  {'Strike':>8s} {'VG Call':>12s} {'BS Call':>12s} {'VG - BS':>12s}")
    print("  " + "-" * 48)
    for k, vg, bs in zip(strikes, vg_calls, bs_calls):
        print(f"  {k:>8.2f} {vg:>12.6f} {bs:>12.6f} {vg - bs:>+12.6f}")

    # Monotonicity in strike (calls must be non-increasing).
    assert np.all(np.diff(vg_calls) <= 1e-8), "VG calls should decrease with strike"

    # -----------------------------------------------------------------
    # 3. Merton jump-diffusion prices across strikes.
    # -----------------------------------------------------------------
    banner("Merton jump-diffusion call prices "
           "(sigma=0.20, lambda=0.50, mu_j=-0.10, sigma_j=0.15)")

    m_sigma, m_lambda, m_mu_j, m_sigma_j = 0.20, 0.50, -0.10, 0.15
    merton_calls = np.array(
        [
            merton_jump_cos_price(
                spot, float(k), rate, dividend,
                m_sigma, m_mu_j, m_sigma_j, m_lambda,
                maturity, True,
            )
            for k in strikes
        ]
    )

    print(f"  {'Strike':>8s} {'Merton Call':>14s} {'BS Call':>12s} {'Merton - BS':>14s}")
    print("  " + "-" * 52)
    for k, m, bs in zip(strikes, merton_calls, bs_calls):
        print(f"  {k:>8.2f} {m:>14.6f} {bs:>12.6f} {m - bs:>+14.6f}")

    # Jump-diffusion with negative mean jumps typically fattens left tail,
    # lifting OTM put prices (equivalently, the low-strike calls via parity).
    # At minimum, all prices must be finite and non-negative.
    assert np.all(np.isfinite(merton_calls))
    assert np.all(merton_calls >= 0.0)

    # -----------------------------------------------------------------
    # 4. Put-call parity checks.
    # -----------------------------------------------------------------
    banner("Put-call parity check: C - P = S*exp(-qT) - K*exp(-rT)")

    strike_parity = 105.0

    # Black-Scholes / COS
    bs_cos_call, bs_cos_put, bs_cos_res = check_parity(
        bs_cos_price, spot, strike_parity, rate, dividend, maturity, bs_vol,
    )
    # Black-Scholes / Lewis
    bs_lewis_call, bs_lewis_put, bs_lewis_res = check_parity(
        bs_lewis_price, spot, strike_parity, rate, dividend, maturity, bs_vol,
    )
    # Variance Gamma / COS
    vg_call, vg_put, vg_res = check_parity(
        vg_cos_price, spot, strike_parity, rate, dividend, maturity,
        vg_sigma, vg_theta, vg_nu,
    )
    # Merton / COS
    m_call, m_put, m_res = check_parity(
        merton_jump_cos_price, spot, strike_parity, rate, dividend, maturity,
        m_sigma, m_mu_j, m_sigma_j, m_lambda,
    )

    print(f"  Strike = {strike_parity:.2f}")
    print()
    print(f"  {'Model/Method':<22s} {'Call':>10s} {'Put':>10s} {'Residual':>14s}")
    print("  " + "-" * 58)
    print(f"  {'Black-Scholes / COS':<22s} {bs_cos_call:>10.6f} {bs_cos_put:>10.6f} {bs_cos_res:>14.2e}")
    print(f"  {'Black-Scholes / Lewis':<22s} {bs_lewis_call:>10.6f} {bs_lewis_put:>10.6f} {bs_lewis_res:>14.2e}")
    print(f"  {'Variance Gamma / COS':<22s} {vg_call:>10.6f} {vg_put:>10.6f} {vg_res:>14.2e}")
    print(f"  {'Merton jump / COS':<22s} {m_call:>10.6f} {m_put:>10.6f} {m_res:>14.2e}")

    for label, residual, tol in [
        ("BS/COS", bs_cos_res, 1e-6),
        ("BS/Lewis", bs_lewis_res, 5e-4),
        ("VG/COS", vg_res, 1e-6),
        ("Merton/COS", m_res, 1e-6),
    ]:
        assert abs(residual) < tol, f"{label} parity residual {residual:.3e} > {tol:.1e}"

    # -----------------------------------------------------------------
    # 5. Combined pricing grid (all three models, call+put).
    # -----------------------------------------------------------------
    banner("Combined pricing grid (T=1y, S=100, r=5%, q=2%)")

    grid_strikes = [90.0, 100.0, 110.0]
    print(
        f"  {'Strike':>8s} | "
        f"{'BS-COS C':>10s} {'BS-COS P':>10s} | "
        f"{'VG C':>10s} {'VG P':>10s} | "
        f"{'Merton C':>10s} {'Merton P':>10s}"
    )
    print("  " + "-" * 86)
    for k in grid_strikes:
        bc = bs_cos_price(spot, k, rate, dividend, bs_vol, maturity, True)
        bp = bs_cos_price(spot, k, rate, dividend, bs_vol, maturity, False)
        vc = vg_cos_price(spot, k, rate, dividend, vg_sigma, vg_theta, vg_nu,
                          maturity, True)
        vp = vg_cos_price(spot, k, rate, dividend, vg_sigma, vg_theta, vg_nu,
                          maturity, False)
        mc = merton_jump_cos_price(
            spot, k, rate, dividend, m_sigma, m_mu_j, m_sigma_j, m_lambda,
            maturity, True,
        )
        mp = merton_jump_cos_price(
            spot, k, rate, dividend, m_sigma, m_mu_j, m_sigma_j, m_lambda,
            maturity, False,
        )
        print(
            f"  {k:>8.2f} | "
            f"{bc:>10.4f} {bp:>10.4f} | "
            f"{vc:>10.4f} {vp:>10.4f} | "
            f"{mc:>10.4f} {mp:>10.4f}"
        )

    print()
    print("  All Fourier-pricing sanity checks passed.")


if __name__ == "__main__":
    main()
