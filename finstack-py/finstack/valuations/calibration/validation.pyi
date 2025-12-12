"""Validation API exposed by :mod:`finstack.valuations.calibration`."""

from __future__ import annotations

from typing import Any, Dict, Optional


class ValidationError:
    def __init__(
        self,
        constraint: str,
        location: str,
        details: str,
        values: Optional[Dict[str, float]] = None,
    ) -> None: ...

    @property
    def constraint(self) -> str: ...

    @property
    def location(self) -> str: ...

    @property
    def details(self) -> str: ...

    @property
    def values(self) -> Dict[str, float]: ...

    def __repr__(self) -> str: ...


class ValidationConfig:
    def __init__(
        self,
        check_forward_positivity: bool = True,
        min_forward_rate: float = -0.01,
        max_forward_rate: float = 0.50,
        check_monotonicity: bool = True,
        check_arbitrage: bool = True,
        tolerance: float = 1e-10,
        max_hazard_rate: float = 0.50,
        min_cpi_growth: float = -0.10,
        max_cpi_growth: float = 0.50,
        min_fwd_inflation: float = -0.20,
        max_fwd_inflation: float = 0.50,
        max_volatility: float = 5.0,
        allow_negative_rates: bool = False,
        lenient_arbitrage: bool = False,
    ) -> None: ...

    @classmethod
    def standard(cls) -> ValidationConfig: ...

    @property
    def check_forward_positivity(self) -> bool: ...

    @property
    def min_forward_rate(self) -> float: ...

    @property
    def max_forward_rate(self) -> float: ...

    @property
    def check_monotonicity(self) -> bool: ...

    @property
    def check_arbitrage(self) -> bool: ...

    @property
    def tolerance(self) -> float: ...

    @property
    def max_hazard_rate(self) -> float: ...

    @property
    def min_cpi_growth(self) -> float: ...

    @property
    def max_cpi_growth(self) -> float: ...

    @property
    def min_fwd_inflation(self) -> float: ...

    @property
    def max_fwd_inflation(self) -> float: ...

    @property
    def max_volatility(self) -> float: ...

    @property
    def allow_negative_rates(self) -> bool: ...

    @property
    def lenient_arbitrage(self) -> bool: ...

    def __repr__(self) -> str: ...


def validate_discount_curve(curve: Any, config: Optional[ValidationConfig] = None) -> None: ...

def validate_forward_curve(curve: Any, config: Optional[ValidationConfig] = None) -> None: ...

def validate_hazard_curve(curve: Any, config: Optional[ValidationConfig] = None) -> None: ...

def validate_inflation_curve(curve: Any, config: Optional[ValidationConfig] = None) -> None: ...

def validate_vol_surface(surface: Any, config: Optional[ValidationConfig] = None) -> None: ...
