"""Monte Carlo engine and European pricer bindings."""

from __future__ import annotations

from typing import Union

from .estimate import Estimate
from .payoffs import Digital, EuropeanCall, EuropeanPut, Forward
from .result import MonteCarloResult

class EuropeanPricerConfig:
    """Configuration for the European Monte Carlo pricer."""

    def __init__(
        self,
        num_paths: int = 100_000,
        seed: int = 42,
        use_parallel: bool = True,
    ) -> None: ...
    @property
    def num_paths(self) -> int: ...
    @property
    def seed(self) -> int: ...
    @property
    def use_parallel(self) -> bool: ...

class EuropeanMcPricer:
    """Compact GBM-only Monte Carlo pricer for European-style payoffs."""

    def __init__(self, config: EuropeanPricerConfig) -> None: ...
    def price_call(
        self,
        spot: float,
        strike: float,
        r: float,
        q: float,
        sigma: float,
        time_to_maturity: float,
        num_steps: int,
        currency: str,
        discount_factor: float,
    ) -> Estimate:
        """Price a European call under GBM."""
        ...

    def price_put(
        self,
        spot: float,
        strike: float,
        r: float,
        q: float,
        sigma: float,
        time_to_maturity: float,
        num_steps: int,
        currency: str,
        discount_factor: float,
    ) -> Estimate:
        """Price a European put under GBM."""
        ...

Payoff = Union[EuropeanCall, EuropeanPut, Digital, Forward]

def price_european(
    spot: float,
    r: float,
    q: float,
    sigma: float,
    time_to_maturity: float,
    num_steps: int,
    num_paths: int,
    payoff: Payoff,
    currency: str,
    discount_factor: float,
    seed: int = 42,
    antithetic: bool = False,
) -> MonteCarloResult:
    """Price any supported payoff under GBM via the generic McEngine."""
    ...
