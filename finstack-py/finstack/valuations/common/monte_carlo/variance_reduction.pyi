"""Variance-reduction utilities for Monte Carlo pricing."""

from __future__ import annotations

class AntitheticConfig:
    """Configuration for antithetic-variates variance reduction."""

    def __init__(
        self,
        num_pairs: int,
        discount_factor: float = 1.0,
        currency: str = "USD",
    ) -> None: ...
    @property
    def num_pairs(self) -> int: ...
    @property
    def discount_factor(self) -> float: ...
    @property
    def currency(self) -> str: ...

def black_scholes_call(
    spot: float,
    strike: float,
    time_to_maturity: float,
    rate: float,
    dividend_yield: float,
    volatility: float,
) -> float:
    """Black-Scholes analytical price for a European call."""
    ...

def black_scholes_put(
    spot: float,
    strike: float,
    time_to_maturity: float,
    rate: float,
    dividend_yield: float,
    volatility: float,
) -> float:
    """Black-Scholes analytical price for a European put."""
    ...
