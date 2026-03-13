"""Statistical helper functions."""

from __future__ import annotations
from typing import List

class RealizedVarMethod:
    """Realized variance estimation methods."""

    CLOSE_TO_CLOSE: RealizedVarMethod
    PARKINSON: RealizedVarMethod
    GARMAN_KLASS: RealizedVarMethod
    ROGERS_SATCHELL: RealizedVarMethod
    YANG_ZHANG: RealizedVarMethod

    @classmethod
    def from_name(cls, name: str) -> RealizedVarMethod: ...
    @property
    def name(self) -> str: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class OnlineStats:
    """Streaming mean / variance accumulator (Welford's algorithm)."""

    def __init__(self) -> None: ...
    def update(self, value: float) -> None: ...
    def merge(self, other: OnlineStats) -> None: ...
    @property
    def count(self) -> int: ...
    @property
    def mean(self) -> float: ...
    @property
    def variance(self) -> float: ...
    @property
    def std_dev(self) -> float: ...
    @property
    def stderr(self) -> float: ...
    def reset(self) -> None: ...
    def __repr__(self) -> str: ...

class OnlineCovariance:
    """Streaming covariance / correlation accumulator (Welford's algorithm)."""

    def __init__(self) -> None: ...
    def update(self, x: float, y: float) -> None: ...
    def merge(self, other: OnlineCovariance) -> None: ...
    @property
    def count(self) -> int: ...
    @property
    def covariance(self) -> float: ...
    @property
    def correlation(self) -> float: ...
    @property
    def mean_x(self) -> float: ...
    @property
    def mean_y(self) -> float: ...
    def reset(self) -> None: ...
    def __repr__(self) -> str: ...

def mean(data: list[float]) -> float: ...
def variance(data: list[float]) -> float: ...
def population_variance(data: list[float]) -> float: ...
def covariance(x: list[float], y: list[float]) -> float: ...
def correlation(x: list[float], y: list[float]) -> float: ...
def mean_var(data: list[float]) -> tuple[float, float]: ...
def quantile(data: list[float], p: float) -> float: ...
def log_returns(prices: list[float]) -> list[float]: ...
def realized_variance(
    prices: list[float],
    method: RealizedVarMethod | str = ...,
    annualization_factor: float = ...,
) -> float:
    """Compute annualized realized variance from a close-price series.

    Parameters
    ----------
    prices:
        Closing prices in chronological order.
    method:
        Estimator to use. Must be ``CloseToClose``; OHLC-only estimators
        (``Parkinson``, ``GarmanKlass``, ``RogersSatchell``, ``YangZhang``)
        raise ``ValueError`` — use :func:`realized_variance_ohlc` instead.
    annualization_factor:
        Trading periods per year (default 252).

    Raises
    ------
    ValueError
        If ``method`` requires OHLC data.
    """
    ...

def realized_variance_ohlc(
    open: list[float],
    high: list[float],
    low: list[float],
    close: list[float],
    method: RealizedVarMethod | str = ...,
    annualization_factor: float = ...,
) -> float:
    """Compute annualized realized variance from an OHLC price series.

    Parameters
    ----------
    open:
        Opening prices in chronological order.
    high:
        Daily high prices in chronological order.
    low:
        Daily low prices in chronological order.
    close:
        Closing prices in chronological order.
    method:
        Estimator to use (any :class:`RealizedVarMethod` value).
    annualization_factor:
        Trading periods per year (default 252).

    Raises
    ------
    ValueError
        If ``open``, ``high``, ``low``, and ``close`` do not all have the
        same length.
    """
    ...
