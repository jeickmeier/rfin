"""Position-level VaR decomposition example.

Demonstrates the ``finstack.portfolio`` position-risk bindings:

1. Build a 4-position portfolio (weights + covariance).
2. Compute parametric component / marginal VaR and verify the Euler
   exhaustion property (sum of component VaRs equals portfolio VaR).
3. Simulate a year of historical scenarios (per-position P&L) and run
   the historical decomposition.
4. Evaluate a simple equal-weight (25%/position) risk budget and flag
   breaches above a 120% utilization threshold.
5. Print the top 3 risk contributors.

Run standalone:

    python finstack-py/examples/08_position_var_decomposition.py
"""

from __future__ import annotations

import numpy as np

from finstack.portfolio import (
    evaluate_risk_budget,
    historical_var_decomposition,
    parametric_es_decomposition,
    parametric_var_decomposition,
)


def build_portfolio() -> tuple[list[str], list[float], list[list[float]]]:
    """Construct a 4-position portfolio with a PSD covariance matrix.

    Factor structure: positions roughly sized as equity, credit, rates,
    and a small FX overlay. Volatilities span 8% to 30% annualized and
    pairwise correlations are a mix of positive (equity/credit) and
    negative (rates vs. equity) to showcase diversification.
    """
    ids = ["EQ_SPX", "CR_HY", "RT_10Y", "FX_EUR"]

    vols = np.array([0.20, 0.12, 0.08, 0.10])
    corr = np.array(
        [
            [1.00, 0.60, -0.30, 0.20],
            [0.60, 1.00, -0.15, 0.10],
            [-0.30, -0.15, 1.00, -0.05],
            [0.20, 0.10, -0.05, 1.00],
        ]
    )
    cov = np.outer(vols, vols) * corr

    weights = [0.45, 0.25, 0.20, 0.10]
    return ids, weights, cov.tolist()


def print_var_contributions(label: str, result: dict) -> None:
    """Pretty-print a VaR decomposition returned by the bindings."""
    print(f"\n[{label}]")
    print(f"  Portfolio VaR:     {result['portfolio_var']:>12,.4f}")
    print(f"  Portfolio ES:      {result['portfolio_es']:>12,.4f}")
    print(f"  Confidence:        {result['confidence']:.2%}")
    print(f"  Euler residual:    {result['euler_residual']:>12.2e}")
    print(f"  {'Position':<10} {'Component':>12} {'Marginal':>12} {'% of VaR':>10}")
    for c in result["contributions"]:
        print(
            f"  {c['position_id']:<10} "
            f"{c['component_var']:>12,.4f} "
            f"{c['marginal_var']:>12,.4f} "
            f"{c['pct_contribution']:>9.2%}"
        )


def verify_euler(result: dict) -> None:
    """Confirm the Euler exhaustion property: sum(component) == portfolio."""
    total = result["portfolio_var"]
    summed = sum(c["component_var"] for c in result["contributions"])
    diff = abs(total - summed)
    print("\n  Euler verification:")
    print(f"    sum(component_var) = {summed:,.6f}")
    print(f"    portfolio_var      = {total:,.6f}")
    print(f"    |residual|         = {diff:.2e}")
    assert diff < 1e-9, f"Parametric Euler exhaustion failed: residual={diff:.2e}"


def simulate_history(
    ids: list[str],
    weights: list[float],
    covariance: list[list[float]],
    n_scenarios: int = 500,
    seed: int = 42,
) -> list[list[float]]:
    """Simulate per-position P&L scenarios from N(0, cov) returns.

    Returns a list of length ``n_positions`` where each row contains the
    position's P&L under each scenario. A fat left tail is injected into
    the equity leg to make the historical decomposition interesting.
    """
    rng = np.random.default_rng(seed)
    cov = np.asarray(covariance)
    # Daily vol from annual: divide by sqrt(252). (Returns represent one day.)
    daily_cov = cov / 252.0

    # Base multivariate-normal returns: shape (n_scenarios, n_positions).
    returns = rng.multivariate_normal(np.zeros(len(ids)), daily_cov, size=n_scenarios)

    # Inject 5 severe equity drawdowns to create a clear tail.
    tail_idx = rng.choice(n_scenarios, size=5, replace=False)
    returns[tail_idx, 0] -= 0.08  # -8% equity shocks.

    # P&L = weight * return (unit portfolio notional).
    pnl = returns * np.asarray(weights)
    # Transpose to (n_positions, n_scenarios) as the binding expects.
    return pnl.T.tolist()


def print_top_contributors(result: dict, top_n: int = 3) -> None:
    """Print the top-N risk contributors by component VaR."""
    ranked = sorted(
        result["contributions"], key=lambda c: c["component_var"], reverse=True
    )
    print(f"\n  Top {top_n} risk contributors:")
    for rank, c in enumerate(ranked[:top_n], start=1):
        print(
            f"    #{rank} {c['position_id']:<10} "
            f"component_var={c['component_var']:,.4f} "
            f"({c['pct_contribution']:.2%} of total)"
        )


def run_risk_budget(ids: list[str], var_result: dict) -> None:
    """Evaluate a flat 25%/position risk budget."""
    print("\n" + "=" * 72)
    print("Risk budget evaluation (target: 25% per position, threshold: 120%)")
    print("=" * 72)

    actuals = [c["component_var"] for c in var_result["contributions"]]
    targets = [0.25] * len(ids)

    budget = evaluate_risk_budget(
        position_ids=ids,
        actual_var=actuals,
        target_var_pct=targets,
        portfolio_var=var_result["portfolio_var"],
        utilization_threshold=1.20,
    )

    print(f"  Portfolio VaR:      {budget['portfolio_var']:>12,.4f}")
    print(f"  Total over-budget:  {budget['total_overbudget']:>12,.4f}")
    print(f"  Any breach?         {budget['has_breach']}")
    print(
        f"  {'Position':<10} {'Target':>10} {'Actual':>10} {'Util':>8} {'Excess':>10} {'Breach':>8}"
    )
    for p in budget["positions"]:
        print(
            f"  {p['position_id']:<10} "
            f"{p['target_component_var']:>10,.4f} "
            f"{p['actual_component_var']:>10,.4f} "
            f"{p['utilization']:>7.2%} "
            f"{p['excess']:>10,.4f} "
            f"{str(p['breach']):>8}"
        )


def main() -> None:
    ids, weights, covariance = build_portfolio()

    print("=" * 72)
    print("Parametric position-level VaR / ES decomposition")
    print("=" * 72)
    print(f"  Positions: {ids}")
    print(f"  Weights:   {weights}")

    # Parametric VaR.
    var_result = parametric_var_decomposition(
        position_ids=ids,
        weights=weights,
        covariance=covariance,
        confidence=0.95,
    )
    print_var_contributions("Parametric VaR (95%)", var_result)
    verify_euler(var_result)
    print_top_contributors(var_result, top_n=3)

    # Parametric ES (same structure but ES metrics).
    es_result = parametric_es_decomposition(
        position_ids=ids,
        weights=weights,
        covariance=covariance,
        confidence=0.95,
    )
    print("\n[Parametric ES (95%)]")
    print(f"  Portfolio ES:      {es_result['portfolio_es']:>12,.4f}")
    print(f"  {'Position':<10} {'Component ES':>14} {'Marginal ES':>14} {'% of ES':>10}")
    for c in es_result["contributions"]:
        print(
            f"  {c['position_id']:<10} "
            f"{c['component_es']:>14,.4f} "
            f"{c['marginal_es']:>14,.4f} "
            f"{c['pct_contribution']:>9.2%}"
        )

    # Historical VaR.
    print("\n" + "=" * 72)
    print("Historical VaR decomposition (500 scenarios)")
    print("=" * 72)
    pnls = simulate_history(ids, weights, covariance, n_scenarios=500, seed=42)
    hist_result = historical_var_decomposition(
        position_ids=ids,
        position_pnls=pnls,
        confidence=0.95,
    )
    print_var_contributions("Historical VaR (95%)", hist_result)
    print_top_contributors(hist_result, top_n=3)

    # Risk budget against parametric component VaRs.
    run_risk_budget(ids, var_result)


if __name__ == "__main__":
    main()
