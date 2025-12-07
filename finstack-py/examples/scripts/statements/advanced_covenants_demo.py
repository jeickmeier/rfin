"""
Advanced covenant modeling demo: springing leverage tests and basket tracking.

This script extends the standard covenant forward-projection example by adding:

- A **springing maintenance covenant** that only activates when revolver
  utilization exceeds a threshold (35% in this demo).
- A **basket utilization covenant** that tracks headroom for a General Debt basket.
- Explicit `CovenantScope` metadata (Maintenance vs. Incurrence) so downstream
  tooling can distinguish recurring tests from action-triggered covenants.

Run
---
uv run python finstack-py/examples/scripts/valuations/covenants/advanced_covenants_demo.py
"""

from __future__ import annotations

from finstack import (
    Covenant,
    CovenantForecastConfig,
    CovenantScope,
    CovenantSpec,
    CovenantType,
    SpringingCondition,
    forecast_covenant,
)
from finstack.core.dates.periods import build_periods
from finstack.statements.evaluator import Evaluator
from finstack.statements.types import AmountOrScalar, FinancialModelSpec, ForecastSpec, NodeSpec, NodeType


def build_extended_model() -> tuple[FinancialModelSpec, list]:
    """Create a small multi-node model with utilization + basket tracking."""
    plan = build_periods("2024Q1..2026Q4", None)
    periods = plan.periods
    model = FinancialModelSpec("advanced_covenants_demo", periods)

    ebitda = (
        NodeSpec("ebitda", NodeType.MIXED)
        .with_values([(periods[0].id, AmountOrScalar.scalar(55_000.0))])
        .with_forecast(ForecastSpec.growth(0.04))
    )

    debt_levels = [
        (
            periods[i].id,
            AmountOrScalar.scalar(250_000.0 if i < 6 else 225_000.0),
        )
        for i in range(len(periods))
    ]
    debt_total = NodeSpec("debt_total", NodeType.MIXED).with_values(debt_levels)

    total_leverage = NodeSpec("total_leverage", NodeType.CALCULATED).with_formula("debt_total / ebitda")

    utilization_values = []
    for idx, period in enumerate(periods):
        utilization = 0.25 if idx < 4 else (0.42 if idx < 8 else 0.58)
        utilization_values.append((period.id, AmountOrScalar.scalar(utilization)))
    rcf_utilization = NodeSpec("rcf_utilization", NodeType.MIXED).with_values(utilization_values)

    basket_usage = []
    for idx, period in enumerate(periods):
        drawn = 60.0 + idx * 5.0  # simple upward trend
        basket_usage.append((period.id, AmountOrScalar.scalar(drawn)))
    general_debt = NodeSpec("general_debt_basket", NodeType.MIXED).with_values(basket_usage)

    for node in (ebitda, debt_total, total_leverage, rcf_utilization, general_debt):
        model.add_node(node)

    return model, periods


def describe_forecast(label: str, forecast, warn_threshold: float = 0.1) -> None:
    print(f"\n== {label} ==")
    print(f"First breach date: {forecast.first_breach_date}")
    print(f"Minimum headroom: {forecast.min_headroom_value:.1%} on {forecast.min_headroom_date}")

    warn_indices = [i for i, headroom in enumerate(forecast.headroom) if headroom < warn_threshold]
    if warn_indices:
        warn_dates = [forecast.test_dates[i] for i in warn_indices]
        print(f"Warning: headroom < {warn_threshold:.0%} on {warn_dates}")
    else:
        print(f"No periods under {warn_threshold:.0%} headroom.")

    print("\n-- Explain --")
    print(forecast.explain())


def main() -> int:
    model, periods = build_extended_model()
    evaluator = Evaluator.new()
    results = evaluator.evaluate(model)

    window = periods[4:12]  # focus on 2025Q1..2026Q4
    window_ids = [p.id for p in window]
    cfg = CovenantForecastConfig(
        stochastic=False, num_paths=0, volatility=None, seed=None, antithetic=False
    )  # deterministic forward projection

    springing = SpringingCondition("rcf_utilization", "minimum", 0.35)
    leverage_cov = (
        Covenant(CovenantType.max_total_leverage(4.5))
        .with_scope(CovenantScope.maintenance())
        .with_springing_condition(springing)
    )
    leverage_spec = CovenantSpec.with_metric(leverage_cov, "total_leverage")
    leverage_forecast = forecast_covenant(leverage_spec, model, results, window_ids, cfg)
    describe_forecast("Springing Maintenance Covenant (Leverage)", leverage_forecast)

    basket_cov = Covenant(CovenantType.basket("general_debt_basket", 120.0)).with_scope(CovenantScope.incurrence())
    basket_spec = CovenantSpec.with_metric(basket_cov, "general_debt_basket")
    basket_forecast = forecast_covenant(basket_spec, model, results, window_ids, cfg)
    describe_forecast("General Debt Basket Headroom", basket_forecast, warn_threshold=0.2)

    return 0


if __name__ == "__main__":  # pragma: no cover - manual example
    raise SystemExit(main())
