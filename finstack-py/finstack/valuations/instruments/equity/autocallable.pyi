"""Autocallable structured product instrument."""

from __future__ import annotations
from typing import List, Dict, Any
from datetime import date
from ....core.money import Money

class Autocallable:
    """Autocallable structured product with early redemption features.

    Autocallable represents a structured product that automatically redeems
    (calls) if the underlying asset price exceeds a barrier on observation dates.
    If not called, it pays a final payoff based on underlying performance.

    Autocallables are popular retail structured products providing enhanced
    coupons with downside protection. They require discount curves, spot prices,
    and volatility surfaces.

    Examples
    --------
    Create an autocallable:

        >>> from finstack.valuations.instruments import Autocallable
        >>> from finstack import Money, Currency
        >>> from datetime import date
        >>> observation_dates = [date(2024, 6, 30), date(2024, 12, 31), date(2025, 6, 30), date(2025, 12, 31)]
        >>> autocallable = Autocallable.builder(
        ...     "AUTOCALLABLE-SPX",
        ...     ticker="SPX",
        ...     observation_dates=observation_dates,
        ...     autocall_barriers=[1.0, 1.0, 1.0, 1.0],  # 100% of initial
        ...     coupons=[0.08, 0.16, 0.24, 0.32],  # Cumulative coupons
        ...     final_barrier=0.70,  # 70% barrier for final payoff
        ...     final_payoff_type={"type": "capital_protection", "floor": 0.7},  # Capital protection with 70% floor
        ...     participation_rate=1.0,  # 100% participation
        ...     cap_level=1.30,  # 130% cap
        ...     notional=Money(1_000_000, Currency("USD")),
        ...     discount_curve="USD",
        ...     spot_id="SPX",
        ...     vol_surface="SPX-VOL",
        ...     expiry=date(2025, 12, 31),
        ...     div_yield_id=None,
        ... )

    Notes
    -----
    - Autocallables require discount curve, spot price, and volatility surface
    - Product automatically calls if barrier is breached on observation date
    - If called early, pays coupon and returns principal
    - If not called, pays final payoff based on underlying performance
    - Final payoff can be put (downside protection) or call (upside participation)

    MarketContext Requirements
    -------------------------
    - Discount curve: ``discount_curve`` (required).
    - Spot price: ``spot_id`` (required).
    - Volatility surface: ``vol_surface`` (required).
    - Dividend yield: ``div_yield_id`` (optional; used when provided).

    See Also
    --------
    :class:`Bond`: Standard bonds
    :class:`RangeAccrual`: Range accrual notes
    :class:`PricerRegistry`: Pricing entry point

    Sources
    -------
    - Hull (text): see ``docs/REFERENCES.md#hullOptionsFuturesDerivatives``.
    """

    @classmethod
    def builder(
        cls,
        instrument_id: str,
        ticker: str,
        observation_dates: List[date],
        autocall_barriers: List[float],
        coupons: List[float],
        final_barrier: float,
        final_payoff_type: str | Dict[str, Any],
        participation_rate: float,
        cap_level: float,
        notional: Money,
        discount_curve: str,
        spot_id: str,
        vol_surface: str,
        *,
        expiry: date | None = None,
        div_yield_id: str | None = None,
    ) -> "Autocallable":
        """Create an autocallable structured product.

        Parameters
        ----------
        instrument_id : str
            Unique identifier for the product (e.g., "AUTOCALLABLE-SPX").
        ticker : str
            Underlying equity ticker symbol.
        observation_dates : List[date]
            Dates when autocall barriers are checked. Must match autocall_barriers
            and coupons lengths.
        autocall_barriers : List[float]
            Barrier levels for each observation date (as fraction of initial price,
            e.g., 1.0 for 100%).
        coupons : List[float]
            Cumulative coupon rates for each observation date (as decimals,
            e.g., [0.08, 0.16, 0.24, 0.32] for 8%, 16%, 24%, 32%).
        final_barrier : float
            Final barrier level for final payoff (as fraction of initial price).
        final_payoff_type : str or Dict[str, Any]
            Final payoff type: "put" (downside protection), "call" (upside),
            or custom payoff specification.
        participation_rate : float
            Participation rate for final payoff (e.g., 1.0 for 100%).
        cap_level : float
            Cap level for final payoff (as fraction of initial, e.g., 1.30 for 130%).
        notional : Money
            Notional principal amount.
        discount_curve : str
            Discount curve identifier in MarketContext.
        spot_id : str
            Spot price identifier in MarketContext.
        vol_surface : str
            Volatility surface identifier in MarketContext.
        expiry : date, optional
            Explicit expiry date. Defaults to the last observation date.
        div_yield_id : str, optional
            Dividend yield identifier in MarketContext.

        Returns
        -------
        Autocallable
            Configured autocallable ready for pricing.

        Raises
        ------
        ValueError
            If parameters are invalid (list length mismatches, etc.) or if
            required market data is missing.

        Examples
        --------
            >>> autocallable = Autocallable.builder(
            ...     "AUTOCALLABLE-SPX",
            ...     "SPX",
            ...     observation_dates,
            ...     autocall_barriers,
            ...     coupons,
            ...     final_barrier=0.70,
            ...     final_payoff_type="put",
            ...     participation_rate=1.0,
            ...     cap_level=1.30,
            ...     notional=Money(1_000_000, Currency("USD")),
            ...     discount_curve="USD",
            ...     spot_id="SPX",
            ...     vol_surface="SPX-VOL",
            ... )
        """
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def ticker(self) -> str: ...
    @property
    def final_barrier(self) -> float: ...
    @property
    def participation_rate(self) -> float: ...
    @property
    def cap_level(self) -> float: ...
    @property
    def notional(self) -> Money: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
