"""Forecast helpers mirroring finstack.statements.forecast (Rust)."""

from typing import Dict, List

from finstack.core.dates.periods import PeriodId
from finstack.statements.types import ForecastSpec

def apply_forecast(
    spec: ForecastSpec,
    base_value: float,
    forecast_periods: List[PeriodId],
) -> Dict[PeriodId, float]: ...
