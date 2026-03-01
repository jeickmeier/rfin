"""Hull-White one-factor model calibration types exposed by :mod:`finstack.valuations.calibration`."""

from __future__ import annotations

from typing import Callable

from .report import CalibrationReport

class HullWhiteParams:
    """Hull-White one-factor model parameters (kappa, sigma)."""

    def __init__(self, kappa: float, sigma: float) -> None: ...
    @property
    def kappa(self) -> float: ...
    @property
    def sigma(self) -> float: ...
    def b_function(self, t1: float, t2: float) -> float: ...
    def bond_option_vol(self, t: float, big_t: float, s: float) -> float: ...
    def __repr__(self) -> str: ...

class SwaptionQuote:
    """Market quote for a European swaption used in HW1F calibration."""

    def __init__(
        self,
        expiry: float,
        tenor: float,
        volatility: float,
        is_normal_vol: bool,
    ) -> None: ...
    @property
    def expiry(self) -> float: ...
    @property
    def tenor(self) -> float: ...
    @property
    def volatility(self) -> float: ...
    @property
    def is_normal_vol(self) -> bool: ...
    def __repr__(self) -> str: ...

def calibrate_hull_white(
    df: Callable[[float], float],
    quotes: list[SwaptionQuote],
) -> tuple[HullWhiteParams, CalibrationReport]: ...
