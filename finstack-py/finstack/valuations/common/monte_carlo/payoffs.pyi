"""Vanilla Monte Carlo payoff types."""

from __future__ import annotations

class EuropeanCall:
    """European call payoff: max(S_T - K, 0) * notional."""

    def __init__(self, strike: float, notional: float = 1.0) -> None: ...
    @property
    def strike(self) -> float: ...
    @property
    def notional(self) -> float: ...

class EuropeanPut:
    """European put payoff: max(K - S_T, 0) * notional."""

    def __init__(self, strike: float, notional: float = 1.0) -> None: ...
    @property
    def strike(self) -> float: ...
    @property
    def notional(self) -> float: ...

class Digital:
    """Digital (binary) option payoff.

    Call: pays *payout* if S_T > strike.
    Put:  pays *payout* if S_T < strike.
    """

    def __init__(self, strike: float, payout: float, is_call: bool = True) -> None: ...
    @property
    def strike(self) -> float: ...
    @property
    def payout(self) -> float: ...
    @property
    def is_call(self) -> bool: ...

class Forward:
    """Forward contract payoff.

    Long:  (S_T - F) * notional.
    Short: (F - S_T) * notional.
    """

    def __init__(
        self,
        forward_price: float,
        notional: float = 1.0,
        is_long: bool = True,
    ) -> None: ...
    @property
    def forward_price(self) -> float: ...
    @property
    def notional(self) -> float: ...
    @property
    def is_long(self) -> bool: ...
