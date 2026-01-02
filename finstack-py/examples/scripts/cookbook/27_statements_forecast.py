"""Title: Statements Forecast Helpers (apply_forecast)
Persona: Quant / Analyst
Complexity: Beginner
Runtime: <1 second.

Description:
Demonstrates calling the forecast helper directly:
- Build a ForecastSpec (growth)
- Provide a list of PeriodIds
- Call statements.apply_forecast(...) -> dict[PeriodId, float]

Notes:
- This is straight-through to Rust; no Python-side forecasting logic.
"""

from __future__ import annotations

from finstack.core.dates.periods import PeriodId
from finstack.statements import ForecastSpec, apply_forecast


def main() -> None:
    # Forecast a quarterly series for 2025 using a simple growth spec.
    spec = ForecastSpec.growth(0.05)
    periods = [PeriodId.quarter(2025, q) for q in (1, 2, 3, 4)]

    values = apply_forecast(spec, base_value=100.0, forecast_periods=periods)

    print("Forecast values:")
    for period_id, value in values.items():
        print(f"  - {period_id}: {value:.6f}")


if __name__ == "__main__":
    main()
