"""GARCH family volatility modeling example.

Simulates 1000 daily log-returns from a known GARCH(1,1) data-generating
process, fits three volatility models, compares them by AIC/BIC, forecasts
the 21-day variance term structure, and runs residual diagnostics
(Ljung-Box on squared standardized residuals, Engle's ARCH-LM).

Run standalone::

    python finstack-py/examples/07_garch_volatility.py

Requires the ``finstack`` extension module to be built (``maturin develop``
inside ``finstack-py``).
"""

from __future__ import annotations

import math

import numpy as np

from finstack.analytics import (
    GarchFit,
    arch_lm,
    fit_egarch11,
    fit_garch11,
    fit_gjr_garch11,
    garch11_forecast,
    ljung_box,
)


# ---------------------------------------------------------------------------
# 1. Simulate a GARCH(1,1) return series with known parameters
# ---------------------------------------------------------------------------

def simulate_garch11(
    omega: float,
    alpha: float,
    beta: float,
    n: int,
    seed: int,
) -> np.ndarray:
    """Simulate ``n`` daily returns from a GARCH(1,1) process with Gaussian
    innovations. Uses the unconditional variance as the initial state.
    """
    persistence = alpha + beta
    if persistence >= 1.0:
        raise ValueError("alpha + beta must be < 1 for a stationary process")
    rng = np.random.default_rng(seed)
    uncond_var = omega / (1.0 - persistence)

    sigma2 = uncond_var
    returns = np.empty(n, dtype=np.float64)
    for t in range(n):
        z = rng.standard_normal()
        r = z * math.sqrt(sigma2)
        returns[t] = r
        sigma2 = omega + alpha * r * r + beta * sigma2
    return returns


# ---------------------------------------------------------------------------
# 2. Pretty printers
# ---------------------------------------------------------------------------

def fmt(value: float | None, width: int = 10, precision: int = 6) -> str:
    """Format a possibly-None float for tabular display."""
    if value is None or (isinstance(value, float) and math.isnan(value)):
        return " " * (width - 1) + "-"
    return f"{value:{width}.{precision}f}"


def _se_index(fit: GarchFit) -> dict[str, int]:
    """Map parameter names to std_errors vector indices for the fitted model."""
    idx: dict[str, int] = {"omega": 0, "alpha": 1, "beta": 2}
    next_i = 3
    if fit.gamma is not None:
        idx["gamma"] = next_i
        next_i += 1
    if fit.nu is not None:
        idx["nu"] = next_i
    return idx


def print_fit_summary(label: str, fit: GarchFit) -> None:
    """Print parameter estimates with optional standard errors."""
    print(f"\n{'=' * 72}")
    print(f"  {label}")
    print(f"{'=' * 72}")
    print(f"  converged:        {fit.converged}  (iters={fit.iterations})")
    print(f"  n_obs:            {fit.n_obs}  n_params: {fit.n_params}")

    se = fit.std_errors
    se_idx = _se_index(fit)
    print("\n  {:<10} {:>12}  {:>12}".format("parameter", "estimate", "std_error"))
    print("  " + "-" * 38)
    for name in ("omega", "alpha", "beta", "gamma", "nu"):
        est = getattr(fit, name, None)
        if est is None:
            continue
        se_val = se[se_idx[name]] if se and name in se_idx else None
        print(f"  {name:<10} {fmt(est, 12):>12}  {fmt(se_val, 12):>12}")

    print(f"\n  log-likelihood:   {fit.log_likelihood:.4f}")
    print(f"  AIC / BIC / HQIC: {fit.aic:.4f}  /  {fit.bic:.4f}  /  {fit.hqic:.4f}")
    print(f"  persistence:      {fit.persistence:.6f}")
    uv = fit.unconditional_variance
    if uv is not None:
        print(f"  uncond. variance: {uv:.8f}  (ann. vol = {math.sqrt(uv * 252):.4f})")
    hl = fit.half_life
    if hl is not None:
        print(f"  shock half-life:  {hl:.2f} periods")


# ---------------------------------------------------------------------------
# 3. Main
# ---------------------------------------------------------------------------

def main() -> None:
    true_omega = 2.0e-5
    true_alpha = 0.08
    true_beta = 0.88
    n = 1000
    seed = 42

    returns = simulate_garch11(true_omega, true_alpha, true_beta, n, seed)

    print("GARCH Volatility Modeling Demo")
    print(f"  DGP:     GARCH(1,1)  omega={true_omega:.2e}  alpha={true_alpha}  beta={true_beta}")
    print(f"  Sample:  n={n}  mean={returns.mean():.6f}  std={returns.std(ddof=1):.6f}")

    # ---- Fit three models --------------------------------------------------
    returns_list = returns.tolist()
    garch = fit_garch11(returns_list, distribution="gaussian")
    egarch = fit_egarch11(returns_list, distribution="gaussian")
    gjr = fit_gjr_garch11(returns_list, distribution="gaussian")

    print_fit_summary("GARCH(1,1) — Gaussian innovations", garch)
    print_fit_summary("EGARCH(1,1) — Gaussian innovations", egarch)
    print_fit_summary("GJR-GARCH(1,1) — Gaussian innovations", gjr)

    # ---- Model comparison by IC -------------------------------------------
    print(f"\n{'=' * 72}")
    print("  Model Comparison (lower is better)")
    print(f"{'=' * 72}")
    print("  {:<16} {:>14} {:>14} {:>14}".format("model", "log-lik", "AIC", "BIC"))
    print("  " + "-" * 60)
    for name, fit in (("GARCH(1,1)", garch), ("EGARCH(1,1)", egarch), ("GJR-GARCH(1,1)", gjr)):
        print(
            f"  {name:<16} {fit.log_likelihood:>14.4f}"
            f" {fit.aic:>14.4f} {fit.bic:>14.4f}"
        )
    best = min([("GARCH(1,1)", garch), ("EGARCH(1,1)", egarch), ("GJR-GARCH(1,1)", gjr)],
               key=lambda kv: kv[1].bic)
    print(f"\n  Best by BIC: {best[0]}")

    # ---- Forecast 21-day variance term structure from GARCH(1,1) ----------
    horizon = 21
    forecasts = garch11_forecast(
        omega=garch.omega,
        alpha=garch.alpha,
        beta=garch.beta,
        last_variance=garch.terminal_variance,
        last_return=returns_list[-1],
        horizon=horizon,
    )
    ann_factor = 252.0
    print(f"\n{'=' * 72}")
    print(f"  GARCH(1,1) Volatility Term Structure (1..{horizon} days)")
    print(f"{'=' * 72}")
    print("  {:>8} {:>16} {:>16}".format("horizon", "variance", "ann. vol"))
    print("  " + "-" * 44)
    uv = garch.unconditional_variance
    for h, var in enumerate(forecasts, start=1):
        ann_vol = math.sqrt(max(var, 0.0) * ann_factor)
        if h in (1, 5, 10, 21):
            print(f"  {h:>8} {var:>16.10f} {ann_vol:>16.6f}")
    if uv is not None:
        print(f"\n  Long-run uncond. ann. vol: {math.sqrt(uv * ann_factor):.6f}")

    # ---- Residual diagnostics on standardized residuals -------------------
    resid = garch.standardized_residuals
    sq = [z * z for z in resid]

    q10, p10 = ljung_box(sq, 10)
    q20, p20 = ljung_box(sq, 20)
    lm5, plm5 = arch_lm(resid, 5)
    lm10, plm10 = arch_lm(resid, 10)

    print(f"\n{'=' * 72}")
    print("  Residual Diagnostics (GARCH(1,1) standardized residuals)")
    print(f"{'=' * 72}")
    print(f"  Ljung-Box (squared resid, lag=10):  Q={q10:.4f}   p={p10:.4f}")
    print(f"  Ljung-Box (squared resid, lag=20):  Q={q20:.4f}   p={p20:.4f}")
    print(f"  ARCH-LM   (resid,         lag=5 ):  LM={lm5:.4f}  p={plm5:.4f}")
    print(f"  ARCH-LM   (resid,         lag=10):  LM={lm10:.4f}  p={plm10:.4f}")
    print("  (high p-values indicate the model has absorbed the volatility clustering)")


if __name__ == "__main__":
    main()
