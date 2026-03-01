"""Forecast method bindings."""

from __future__ import annotations
from typing import Dict, Any

class ForecastMethod:
    """Forecast method enumeration.

    Defines how to forecast future values for a node.
    """

    # Class attributes
    FORWARD_FILL: ForecastMethod
    GROWTH_PCT: ForecastMethod
    CURVE_PCT: ForecastMethod
    OVERRIDE: ForecastMethod
    NORMAL: ForecastMethod
    LOG_NORMAL: ForecastMethod
    TIME_SERIES: ForecastMethod
    SEASONAL: ForecastMethod

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class SeasonalMode:
    """Seasonal forecast mode.

    Determines how seasonal patterns are applied.
    """

    # Class attributes
    ADDITIVE: SeasonalMode
    MULTIPLICATIVE: SeasonalMode

    def __repr__(self) -> str: ...

class ForecastSpec:
    """Forecast specification.

    Defines how to forecast future values for a node using a specific method
    and parameters.
    """

    def __init__(self, method: ForecastMethod, params: Dict[str, Any] | None = None) -> None:
        """Create a forecast specification.

        Args:
            method: Forecast method to use
            params: Method-specific parameters

        Returns:
            ForecastSpec: Forecast specification
        """
        ...

    @classmethod
    def forward_fill(cls) -> ForecastSpec:
        """Create a forward-fill forecast (carry last value forward).

        Returns:
            ForecastSpec: Forward-fill forecast spec
        """
        ...

    @classmethod
    def growth(cls, rate: float) -> ForecastSpec:
        """Create a growth percentage forecast.

        Args:
            rate: Growth rate (e.g., 0.05 for 5% growth)

        Returns:
            ForecastSpec: Growth forecast spec
        """
        ...

    @classmethod
    def curve(cls, rates: list[float]) -> ForecastSpec:
        """Create a curve percentage forecast with period-specific rates.

        Args:
            rates: Period-specific growth rates

        Returns:
            ForecastSpec: Curve forecast spec
        """
        ...

    @classmethod
    def normal(cls, mean: float, std: float, seed: int) -> ForecastSpec:
        """Create a normal distribution forecast (deterministic with seed).

        Args:
            mean: Mean of the distribution
            std: Standard deviation
            seed: Random seed for determinism

        Returns:
            ForecastSpec: Normal forecast spec
        """
        ...

    @classmethod
    def lognormal(cls, mean: float, std: float, seed: int) -> ForecastSpec:
        """Create a log-normal distribution forecast (always positive).

        Values follow ``X = exp(mu + sigma * Z)`` where ``Z ~ N(0,1)``.
        The physical-space expected value is ``E[X] = exp(mu + sigma^2/2)``.

        Args:
            mean: ``mu`` -- mean of the underlying **log-space** normal
                (not the expected value in physical space).
            std: ``sigma`` -- standard deviation in **log-space**.
            seed: Random seed for determinism

        Returns:
            ForecastSpec: Log-normal forecast spec
        """
        ...

    @property
    def method(self) -> ForecastMethod:
        """Get the forecast method.

        Returns:
            ForecastMethod: Forecast method
        """
        ...

    @property
    def params(self) -> Dict[str, Any]:
        """Get the forecast parameters.

        Returns:
            dict: Parameters dictionary
        """
        ...

    def to_json(self) -> str:
        """Convert to JSON string.

        Returns:
            str: JSON representation
        """
        ...

    @classmethod
    def from_json(cls, json_str: str) -> ForecastSpec:
        """Create from JSON string.

        Args:
            json_str: JSON string

        Returns:
            ForecastSpec: Deserialized forecast spec
        """
        ...

    def __repr__(self) -> str: ...
