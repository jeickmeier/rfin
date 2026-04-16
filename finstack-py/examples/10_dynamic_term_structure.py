"""Dynamic term structure modeling example.

Demonstrates the dynamic term-structure primitives exposed from the Rust
``finstack-core`` crate through ``finstack.core.market_data.dtsm``:

1. Generate 200 days of synthetic 10-tenor yield curves with time-varying
   Nelson-Siegel (level/slope/curvature) dynamics.
2. Extract Diebold-Li factors from the panel (cross-sectional OLS on NS
   loadings).
3. Fit VAR(1) dynamics on the factors and forecast the yield curve 12 days
   ahead, including the 95% forecast band.
4. Run PCA on first-differenced yields and show that the top 3 PCs explain
   more than 95% of the variance.
5. Generate a +2 sigma PC1 (level) shock scenario and apply it to the final
   observed curve.

Run:

    python finstack-py/examples/10_dynamic_term_structure.py
"""

from __future__ import annotations

import numpy as np

from finstack.core.market_data import dtsm


def banner(title: str) -> None:
    """Print a labelled section separator."""
    print()
    print("=" * 72)
    print(f"  {title}")
    print("=" * 72)


def ns_curve(beta1: float, beta2: float, beta3: float, lam: float, tenors: np.ndarray) -> np.ndarray:
    """Nelson-Siegel curve on ``tenors`` given factors and decay ``lam``."""
    lt = lam * tenors
    # Protect against tau -> 0 (slope loading tends to 1).
    slope_load = np.where(np.abs(lt) < 1e-10, 1.0, (1.0 - np.exp(-lt)) / lt)
    curv_load = slope_load - np.exp(-lt)
    return beta1 + beta2 * slope_load + beta3 * curv_load


def simulate_panel(
    n_dates: int,
    tenors: np.ndarray,
    lam: float,
    rng: np.random.Generator,
) -> np.ndarray:
    """Simulate an (n_dates x len(tenors)) yield panel with AR(1) NS factors.

    beta1 mean-reverts around 4%, beta2 around -1.5%, beta3 around 0.5%.
    """
    # Targets and persistence
    mu = np.array([0.040, -0.015, 0.005])
    phi = np.array([0.98, 0.95, 0.92])
    sigma = np.array([0.0015, 0.0020, 0.0025])

    betas = np.empty((n_dates, 3))
    betas[0] = mu
    for t in range(1, n_dates):
        eps = rng.normal(size=3) * sigma
        betas[t] = mu + phi * (betas[t - 1] - mu) + eps

    panel = np.empty((n_dates, len(tenors)))
    for t in range(n_dates):
        panel[t] = ns_curve(betas[t, 0], betas[t, 1], betas[t, 2], lam, tenors)
    return panel


def main() -> None:
    rng = np.random.default_rng(seed=20260416)

    # Standard tenor grid in years (10 tenors from 3M to 30Y).
    tenors = np.array([0.25, 0.5, 1.0, 2.0, 3.0, 5.0, 7.0, 10.0, 20.0, 30.0])
    lam = 0.0609  # Diebold-Li canonical decay.
    n_dates = 200

    yields = simulate_panel(n_dates, tenors, lam, rng)

    banner("Synthetic yield panel")
    print(f"  Shape                      : {yields.shape} (dates x tenors)")
    print(f"  Tenors (years)             : {tenors.tolist()}")
    print(f"  Lambda (Diebold-Li decay)  : {lam}")
    print(
        f"  Mean 10Y yield             : {yields[:, 7].mean():.4%}  "
        f"(min {yields[:, 7].min():.4%}, max {yields[:, 7].max():.4%})"
    )

    # ------------------------------------------------------------------
    # 1. Extract Diebold-Li factors
    # ------------------------------------------------------------------
    banner("Diebold-Li factor extraction")

    # Note: `lambda` is a Python reserved word so we pass all three arguments
    # positionally.
    factors = dtsm.diebold_li_fit_factors(
        tenors.tolist(),
        yields.tolist(),
        lam,
    )

    beta1 = np.asarray(factors["beta1"])
    beta2 = np.asarray(factors["beta2"])
    beta3 = np.asarray(factors["beta3"])
    r_squared = np.asarray(factors["r_squared"])

    print(
        f"  beta1 (level)              : "
        f"mean {beta1.mean():+.4%}, std {beta1.std():.4%}"
    )
    print(
        f"  beta2 (slope)              : "
        f"mean {beta2.mean():+.4%}, std {beta2.std():.4%}"
    )
    print(
        f"  beta3 (curvature)          : "
        f"mean {beta3.mean():+.4%}, std {beta3.std():.4%}"
    )
    print(f"  Avg R^2 across tenors      : {factors['r_squared_avg']:.6f}")
    print(f"  Min/Max per-tenor R^2      : {r_squared.min():.6f} / {r_squared.max():.6f}")

    # ------------------------------------------------------------------
    # 2. VAR(1) + 12-step forecast
    # ------------------------------------------------------------------
    banner("Diebold-Li 12-day forecast (VAR(1) on factors)")

    horizon = 12
    fc = dtsm.diebold_li_forecast(
        tenors.tolist(),
        yields.tolist(),
        horizon,
        lam,
    )

    fc_yields = np.asarray(fc["forecast_yields"])
    lower = np.asarray(fc["confidence_bands"]["lower_95"])
    upper = np.asarray(fc["confidence_bands"]["upper_95"])
    last_obs = yields[-1]

    fb = fc["forecast_factors"]
    print(
        f"  Forecast factors (b1/b2/b3): "
        f"{fb[0]:+.4%} / {fb[1]:+.4%} / {fb[2]:+.4%}"
    )
    print(f"  {'Tenor':>8s}  {'Last':>10s}  {'Forecast':>10s}  {'Lower95':>10s}  {'Upper95':>10s}  {'BandWidth':>10s}")
    print("  " + "-" * 68)
    for i, tau in enumerate(tenors):
        print(
            f"  {tau:>8.2f}  {last_obs[i]:>10.4%}  {fc_yields[i]:>10.4%}  "
            f"{lower[i]:>10.4%}  {upper[i]:>10.4%}  {(upper[i] - lower[i]):>10.4%}"
        )

    # ------------------------------------------------------------------
    # 3. PCA on yield changes
    # ------------------------------------------------------------------
    banner("PCA on daily yield changes")

    changes = np.diff(yields, axis=0)  # (n_dates - 1) x n_tenors
    print(f"  Yield-change panel shape   : {changes.shape}")

    pca = dtsm.yield_pca_fit(changes.tolist(), 3)

    evr = np.asarray(pca["explained_variance_ratio"])
    cum = np.asarray(pca["cumulative_variance"])
    loadings = np.asarray(pca["loadings"])  # (N x n_components)

    print(f"  PC1 / PC2 / PC3 var share  : {evr[0]:.4f} / {evr[1]:.4f} / {evr[2]:.4f}")
    print(f"  Cumulative (PC1..PC3)      : {cum[0]:.4f} / {cum[1]:.4f} / {cum[2]:.4f}")
    if cum[-1] > 0.95:
        print(f"  Top 3 PCs explain > 95% variance (check passed: {cum[-1]:.4%}).")
    else:
        print(f"  WARNING: Top 3 PCs explain only {cum[-1]:.4%} of variance.")

    print()
    print(f"  {'Tenor':>8s}  {'PC1':>10s}  {'PC2':>10s}  {'PC3':>10s}")
    print("  " + "-" * 44)
    for i, tau in enumerate(tenors):
        print(
            f"  {tau:>8.2f}  {loadings[i, 0]:>+10.4f}  "
            f"{loadings[i, 1]:>+10.4f}  {loadings[i, 2]:>+10.4f}"
        )

    # ------------------------------------------------------------------
    # 4. +2 sigma PC1 (level) shock scenario
    # ------------------------------------------------------------------
    banner("PC1 (level) +2 sigma shock scenario")

    shock = dtsm.yield_pca_scenario(
        changes.tolist(),
        0,     # component_index: PC1
        2.0,   # sigma_shock
        3,     # n_components
    )
    shock = np.asarray(shock)
    shocked_curve = last_obs + shock

    print(f"  {'Tenor':>8s}  {'Last':>10s}  {'Shifted':>10s}  {'Delta (bp)':>12s}")
    print("  " + "-" * 48)
    for i, tau in enumerate(tenors):
        print(
            f"  {tau:>8.2f}  {last_obs[i]:>10.4%}  "
            f"{shocked_curve[i]:>10.4%}  {shock[i] * 1e4:>+12.2f}"
        )

    print()
    print(
        "  Interpretation: PC1 is the near-parallel level factor. A +2 sigma "
        "shock shifts the curve roughly uniformly across tenors, with small "
        "deviations reflecting the loading's deviation from perfectly flat."
    )


if __name__ == "__main__":
    main()
