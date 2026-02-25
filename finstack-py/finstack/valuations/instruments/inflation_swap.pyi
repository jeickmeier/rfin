"""Inflation swap instrument."""

from typing import Optional
from datetime import date
from ...core.money import Money
from ..common import InstrumentType

class InflationSwapBuilder:
    """Fluent builder returned by :meth:`InflationSwap.builder`."""

    def __init__(self, instrument_id: str) -> None: ...
    def notional(self, notional: Money) -> "InflationSwapBuilder": ...
    def fixed_rate(self, fixed_rate: float) -> "InflationSwapBuilder": ...
    def start_date(self, start_date: date) -> "InflationSwapBuilder": ...
    def maturity(self, maturity: date) -> "InflationSwapBuilder": ...
    def discount_curve(self, discount_curve: str) -> "InflationSwapBuilder": ...
    def inflation_index_id(self, inflation_index_id: str) -> "InflationSwapBuilder": ...
    def inflation_curve(self, inflation_curve: str) -> "InflationSwapBuilder": ...
    def side(self, side: str) -> "InflationSwapBuilder": ...
    def day_count(self, day_count: str) -> "InflationSwapBuilder": ...
    def lag_override(self, lag_override: Optional[str] = ...) -> "InflationSwapBuilder": ...
    def build(self) -> "InflationSwap": ...

class InflationSwap:
    """Inflation swap for exchanging fixed rate for inflation-linked payments.

    InflationSwap represents a swap where one party pays a fixed rate and
    receives inflation-linked payments (or vice versa). The swap is typically
    zero-coupon, with all payments occurring at maturity.

    Inflation swaps are used to hedge inflation risk, speculate on inflation
    expectations, and create inflation-linked investment strategies. They require
    a discount curve and an inflation curve.

    Examples
    --------
    Create an inflation swap (pay fixed, receive inflation):

        >>> from finstack.valuations.instruments import InflationSwap
        >>> from finstack import Money, Currency
        >>> from datetime import date
        >>> inflation_swap = (
        ...     InflationSwap
        ...     .builder("INFLATION-SWAP-5Y")
        ...     .notional(Money(10_000_000, Currency("USD")))
        ...     .fixed_rate(0.025)  # 2.5% fixed rate
        ...     .start_date(date(2024, 1, 1))
        ...     .maturity(date(2029, 1, 1))  # 5-year swap
        ...     .discount_curve("USD")
        ...     .inflation_curve("US-CPI")
        ...     .side("pay_fixed")  # Pay fixed, receive inflation
        ...     .build()
        ... )

    Notes
    -----
    - Inflation swaps require discount curve and inflation curve
    - Fixed rate is the break-even inflation rate
    - Inflation leg pays based on CPI appreciation
    - Typically zero-coupon (all payments at maturity)
    - Side determines who pays fixed vs receives inflation

    MarketContext Requirements
    -------------------------
    - Discount curve: ``discount_curve`` (required).
    - Inflation curve/index: ``inflation_curve`` and/or ``inflation_index_id`` (required for CPI projection).

    See Also
    --------
    :class:`InflationLinkedBond`: Inflation-linked bonds
    :class:`InflationCurve`: CPI curves
    :class:`PricerRegistry`: Pricing entry point

    Sources
    -------
    - Brigo & Mercurio (2006): see ``docs/REFERENCES.md#brigoMercurio2006``.
    - Hull (text): see ``docs/REFERENCES.md#hullOptionsFuturesDerivatives``.
    """

    @classmethod
    def builder(cls, instrument_id: str) -> InflationSwapBuilder:
        """Start a fluent builder (builder-only API).

        Parameters
        ----------
        instrument_id : str
            Unique identifier for the swap (e.g., "INFLATION-SWAP-5Y").
        notional : Money
            Notional principal amount. Currency determines curve currency requirements.
        fixed_rate : float
            Fixed rate as a decimal (e.g., 0.025 for 2.5%). This is the break-even
            inflation rate. The swap has zero value if realized inflation equals this rate.
        start_date : date
            Swap start date (inflation measurement start).
        maturity : date
            Swap maturity date (payment date). Must be after start_date.
        discount_curve : str
            Discount curve identifier in MarketContext for present value calculations.
        inflation_index : str, optional
            Inflation index identifier (deprecated, use inflation_index_id).
        side : str, optional
            Swap side: "pay_fixed" (default, pay fixed, receive inflation) or
            "receive_fixed" (receive fixed, pay inflation).
        day_count : str, optional
            Day-count convention (default: "act_act").
        inflation_index_id : str, optional
            Inflation index identifier in MarketContext (e.g., "US-CPI").
        lag_override : str, optional
            Inflation lag override (e.g., "3M" for 3-month lag).
        inflation_curve : str, optional
            Inflation curve identifier in MarketContext. If None, uses inflation_index_id.

        Returns
        -------
        InflationSwap
            Configured inflation swap ready for pricing.

        Raises
        ------
        ValueError
            If dates are invalid (maturity <= start_date), if fixed_rate is negative,
            or if required curves are not found in MarketContext.

        """
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def fixed_rate(self) -> float: ...
    @property
    def maturity(self) -> date: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
