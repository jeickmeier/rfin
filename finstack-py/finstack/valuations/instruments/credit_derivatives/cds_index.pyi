"""CDS index instrument."""

from __future__ import annotations
from datetime import date
from ....core.money import Money
from ...common import InstrumentType

class CdsIndexBuilder:
    """Fluent builder returned by :meth:`CDSIndex.builder`."""

    def __init__(self, instrument_id: str) -> None: ...
    def index_name(self, index_name: str) -> "CdsIndexBuilder": ...
    def series(self, series: int) -> "CdsIndexBuilder": ...
    def version(self, version: int) -> "CdsIndexBuilder": ...
    def notional(self, notional: Money) -> "CdsIndexBuilder": ...
    def money(self, money: Money) -> "CdsIndexBuilder": ...
    def fixed_coupon_bp(self, fixed_coupon_bp: float) -> "CdsIndexBuilder": ...
    def start_date(self, start_date: date) -> "CdsIndexBuilder": ...
    def maturity(self, maturity: date) -> "CdsIndexBuilder": ...
    def discount_curve(self, discount_curve: str) -> "CdsIndexBuilder": ...
    def credit_curve(self, credit_curve: str) -> "CdsIndexBuilder": ...
    def side(self, side: str) -> "CdsIndexBuilder": ...
    def recovery_rate(self, recovery_rate: float) -> "CdsIndexBuilder": ...
    def index_factor(self, index_factor: float | None = ...) -> "CdsIndexBuilder": ...
    def build(self) -> "CDSIndex": ...

class CDSIndex:
    """CDS index for portfolio credit risk exposure.

    CDSIndex represents a credit default swap on a standardized index of
    reference entities (e.g., CDX, iTraxx). The index provides diversified
    credit exposure and is more liquid than single-name CDS.

    CDS indices are used for portfolio credit risk management, hedging, and
    speculation. They follow ISDA conventions and require discount and credit
    curves for pricing.

    Examples
    --------
    Create a CDS index position:

        >>> from finstack.valuations.instruments import CDSIndex
        >>> from finstack import Money, Currency
        >>> from datetime import date
        >>> cds_index = (
        ...     CDSIndex
        ...     .builder("CDX-IG-5Y")
        ...     .index_name("CDX.NA.IG")
        ...     .series(40)
        ...     .version(1)
        ...     .money(Money(10_000_000, Currency("USD")))
        ...     .fixed_coupon_bp(100.0)  # 100bp fixed coupon
        ...     .start_date(date(2024, 1, 1))
        ...     .maturity(date(2029, 1, 1))  # 5-year index
        ...     .discount_curve("USD")
        ...     .credit_curve("CDX-IG-40")
        ...     .side("pay_protection")
        ...     .build()
        ... )

    Notes
    -----
    - CDS indices require discount curve and credit (index) curve
    - Fixed coupon is the standard coupon for the index series
    - Index factor accounts for defaults and roll-downs
    - Series and version identify the specific index iteration
    - Side determines protection buyer vs seller

    Conventions
    -----------
    - ``fixed_coupon_bp`` is quoted in basis points (bp). Convert to decimal rate with ``fixed_coupon_bp / 10_000``.
    - ``recovery_rate`` is a decimal fraction in [0, 1] when provided.
    - Required market data is identified by string IDs (``discount_curve``, ``credit_curve``) and must be present
      in ``MarketContext``.

    MarketContext Requirements
    -------------------------
    - Discount curve: ``discount_curve`` (required).
    - Credit/index curve: ``credit_curve`` (required).

    See Also
    --------
    :class:`CreditDefaultSwap`: Single-name CDS
    :class:`CdsTranche`: CDS index tranches
    :class:`PricerRegistry`: Pricing entry point

    Sources
    -------
    - ISDA (2006) Definitions: see ``docs/REFERENCES.md#isda2006Definitions``.
    - O'Kane (2008): see ``docs/REFERENCES.md#okane2008``.
    """

    @classmethod
    def builder(cls, instrument_id: str) -> CdsIndexBuilder:
        """Start a fluent builder (builder-only API).

        Parameters
        ----------
        instrument_id : str
            Unique identifier for the CDS index (e.g., "CDX-IG-5Y").
        index_name : str
            Index name (e.g., "CDX.NA.IG", "iTraxx.Europe").
        series : int
            Index series number (e.g., 40 for CDX.NA.IG Series 40).
        version : int
            Index version number (typically 1 for new series).
        notional : Money
            Notional principal amount.
        fixed_coupon_bp : float
            Fixed coupon in basis points (e.g., 100.0 for 100bp = 1%).
            This is the standard coupon for the index series.
        start_date : date
            CDS index start date.
        maturity : date
            CDS index maturity date. Must be after start_date.
        discount_curve : str
            Discount curve identifier in MarketContext.
        credit_curve : str
            Credit (index) curve identifier in MarketContext.
        side : str, optional
            Position side: "pay_protection" (default, pay premium, receive protection)
            or "receive_protection" (receive premium, pay protection).
        recovery_rate : float, optional
            Recovery rate (default: index standard, typically 0.40).
        index_factor : float, optional
            Index factor accounting for defaults and roll-downs (default: 1.0).

        Returns
        -------
        CDSIndex
            Configured CDS index ready for pricing.

        Raises
        ------
        ValueError
            If parameters are invalid or if required curves are not found.

        """
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def index_name(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def fixed_coupon_bp(self) -> float: ...
    @property
    def side(self) -> str: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def credit_curve(self) -> str: ...
    @property
    def maturity(self) -> date: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
