"""End-to-end covenant forward-projection with headroom analytics (Python).

This example builds a minimal financial model with EBITDA and Total Debt nodes,
computes a Debt/EBITDA ratio node, evaluates the model, and then forecasts a
Max Debt/EBITDA covenant for upcoming quarters, including headroom and an
optional Monte Carlo breach probability overlay.

Requirements
------------
- finstack (Python bindings built from this repository)
- polars (optional, for DataFrame visualization)

Run
---
uv run python finstack-py/examples/scripts/valuations/covenants_forecast_example.py
"""

from __future__ import annotations

try:
    import polars as pl  # type: ignore
except Exception:  # pragma: no cover - optional
    pl = None

from finstack.core.dates.periods import build_periods
from finstack.statements.evaluator import Evaluator
from finstack.statements.types import AmountOrScalar, FinancialModelSpec, ForecastSpec, NodeSpec, NodeType

from finstack import Covenant, CovenantForecastConfig, CovenantSpec, CovenantType, forecast_covenant


def build_demo_model() -> tuple[FinancialModelSpec, list]:
    """Create a small model with EBITDA, Total Debt, and Debt/EBITDA nodes.

    Returns a model and the full list of periods.
    """
    # Build 12 quarters from 2024Q1 through 2026Q4
    plan = build_periods("2024Q1..2026Q4", None)
    periods = plan.periods

    model = FinancialModelSpec("demo_model", periods)

    # EBITDA grows 5% q/q from a starting value
    ebitda = (
        NodeSpec("ebitda", NodeType.MIXED)
        .with_values([(periods[0].id, AmountOrScalar.scalar(50_000.0))])  # seed level
        .with_forecast(ForecastSpec.growth(0.05))
    )

    # Total Debt stays flat and then amortizes slightly
    # You can also forward-fill a starting value and override later periods
    debt_values = [
        (
            periods[i].id,
            AmountOrScalar.scalar(240_000.0 if i < 8 else 220_000.0),
        )
        for i in range(len(periods))
    ]
    debt_total = NodeSpec("debt_total", NodeType.MIXED).with_values(debt_values)

    # Debt/EBITDA covenant metric node
    debt_to_ebitda = NodeSpec("debt_to_ebitda", NodeType.CALCULATED).with_formula("debt_total / ebitda")

    model.add_node(ebitda)
    model.add_node(debt_total)
    model.add_node(debt_to_ebitda)

    return model, periods


def main() -> int:
    model, periods = build_demo_model()

    # Evaluate the base case deterministically
    evaluator = Evaluator.new()
    results = evaluator.evaluate(model)

    # Choose an 8-quarter forecast window (next 8 quarters of the plan)
    window = periods[4:12]  # 2025Q1..2026Q4 given the plan above
    window_ids = [p.id for p in window]

    # Covenant: Max Debt/EBITDA <= 5.0x, explicitly tied to the ratio node
    cov_type = CovenantType.max_debt_to_ebitda(5.0)
    cov = Covenant(cov_type)
    cov_spec = CovenantSpec.with_metric(cov, "debt_to_ebitda")

    # Optional stochastic overlay for breach probability (lognormal shocks on metric)
    cfg = CovenantForecastConfig(True, 10_000, 0.25, 42, False)

    forecast = forecast_covenant(cov_spec, model, results, window_ids, cfg)

    # Warn where headroom < 10%
    warn_indices = [i for i, h in enumerate(forecast.headroom) if h < 0.10]
    if warn_indices:
        [forecast.test_dates[i] for i in warn_indices]
    else:
        pass

    # Human-readable quarter-by-quarter explanation

    # Optional: export to Polars for visualization
    if pl is not None:
        df = forecast.to_polars().to_pandas() if hasattr(forecast, "to_polars") else None
        if df is not None:
            pass

    return 0


if __name__ == "__main__":  # pragma: no cover - manual example
    raise SystemExit(main())
