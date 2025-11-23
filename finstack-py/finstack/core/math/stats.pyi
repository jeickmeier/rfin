"""Statistical helper functions."""

from typing import List


class RealizedVarMethod:
    """Realized variance estimation methods."""

    CLOSE_TO_CLOSE: "RealizedVarMethod"
    PARKINSON: "RealizedVarMethod"
    GARMAN_KLASS: "RealizedVarMethod"
    ROGERS_SATCHELL: "RealizedVarMethod"
    YANG_ZHANG: "RealizedVarMethod"

    @classmethod
    def from_name(cls, name: str) -> "RealizedVarMethod": ...

    @property
    def name(self) -> str: ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...


def mean(data: List[float]) -> float: ...
def variance(data: List[float]) -> float: ...
def covariance(x: List[float], y: List[float]) -> float: ...
def correlation(x: List[float], y: List[float]) -> float: ...
def mean_var(data: List[float]) -> tuple[float, float]: ...
def log_returns(prices: List[float]) -> List[float]: ...

def realized_variance(
    prices: List[float],
    method: RealizedVarMethod | str = ...,
    annualization_factor: float = ...,
) -> float: ...

def realized_variance_ohlc(
    open: List[float],
    high: List[float],
    low: List[float],
    close: List[float],
    method: RealizedVarMethod | str = ...,
    annualization_factor: float = ...,
) -> float: ...

