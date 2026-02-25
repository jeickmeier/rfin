"""CDS option instrument."""

from typing import Optional
from datetime import date
from ...core.money import Money
from ..common import InstrumentType

class CdsOptionBuilder:
    """Fluent builder returned by :meth:`CdsOption.builder`."""

    def __init__(self, instrument_id: str) -> None: ...
    def notional(self, notional: Money) -> "CdsOptionBuilder": ...
    def money(self, money: Money) -> "CdsOptionBuilder": ...
    def strike(self, strike: float) -> "CdsOptionBuilder": ...
    def strike_spread_bp(self, strike_spread_bp: float) -> "CdsOptionBuilder": ...
    def expiry(self, expiry: date) -> "CdsOptionBuilder": ...
    def cds_maturity(self, cds_maturity: date) -> "CdsOptionBuilder": ...
    def discount_curve(self, discount_curve: str) -> "CdsOptionBuilder": ...
    def credit_curve(self, credit_curve: str) -> "CdsOptionBuilder": ...
    def vol_surface(self, vol_surface: str) -> "CdsOptionBuilder": ...
    def option_type(self, option_type: Optional[str]) -> "CdsOptionBuilder": ...
    def recovery_rate(self, recovery_rate: float) -> "CdsOptionBuilder": ...
    def underlying_is_index(self, underlying_is_index: bool) -> "CdsOptionBuilder": ...
    def index_factor(self, index_factor: Optional[float] = ...) -> "CdsOptionBuilder": ...
    def forward_adjust(self, forward_adjust: float) -> "CdsOptionBuilder": ...
    def forward_adjust_bp(self, forward_adjust_bp: float) -> "CdsOptionBuilder": ...
    def build(self) -> "CdsOption": ...

class CdsOption:
    """Option on CDS spread for credit volatility exposure.

    CdsOption represents an option to enter into a CDS at a specified spread
    (strike) on or before expiry. The option can be on a single-name CDS or
    a CDS index.

    CDS options are used for credit volatility trading, hedging spread risk,
    and creating structured credit products. They require discount curves,
    credit curves, and volatility surfaces.

    Examples
    --------
    Create a CDS call option:

        >>> from finstack.valuations.instruments import CdsOption
        >>> from finstack import Money, Currency
        >>> from datetime import date
        >>> cds_option = (
        ...     CdsOption
        ...     .builder("CDS-OPT-CORP-A")
        ...     .money(Money(10_000_000, Currency("USD")))
        ...     .strike(0.015)  # 150bp as decimal rate
        ...     .expiry(date(2024, 12, 20))
        ...     .cds_maturity(date(2029, 1, 1))  # 5-year underlying CDS
        ...     .discount_curve("USD")
        ...     .credit_curve("CORP-A-HAZARD")
        ...     .vol_surface("CDS-VOL")
        ...     .option_type("call")  # Right to buy protection at strike
        ...     .build()
        ... )

    Notes
    -----
    - CDS options require discount curve, credit curve, and volatility surface
    - Strike is a decimal rate (e.g., 0.01 = 100bp)
    - Option type: "call" (right to buy protection) or "put" (right to sell protection)
    - Underlying CDS maturity determines the protection period
    - Forward adjustment accounts for forward spread vs spot spread

    Conventions
    -----------
    - ``strike`` and ``forward_adjust`` are decimal rates (e.g., 0.01 = 100bp).
    - ``recovery_rate`` is a decimal fraction in [0, 1].
    - Required market data is identified by string IDs (``discount_curve``, ``credit_curve``, ``vol_surface``) and
      must be present in ``MarketContext``.

    MarketContext Requirements
    -------------------------
    - Discount curve: ``discount_curve`` (required).
    - Credit/hazard curve: ``credit_curve`` (required).
    - Volatility surface: ``vol_surface`` (required).

    See Also
    --------
    :class:`CreditDefaultSwap`: Single-name CDS
    :class:`CDSIndex`: CDS indices
    :class:`PricerRegistry`: Pricing entry point

    Sources
    -------
    - O'Kane (2008): see ``docs/REFERENCES.md#okane2008``.
    - ISDA (2006) Definitions: see ``docs/REFERENCES.md#isda2006Definitions``.
    """

    @classmethod
    def builder(cls, instrument_id: str) -> CdsOptionBuilder:
        """Start a fluent builder (builder-only API).

        Parameters
        ----------
        instrument_id : str
            Unique identifier for the option (e.g., "CDS-OPT-CORP-A").
        notional : Money
            Notional principal amount.
        strike : float
            Strike CDS spread as a decimal rate (e.g., 0.015 for 150bp).
        expiry : date
            Option expiration date.
        cds_maturity : date
            Maturity date of the underlying CDS if exercised. Must be after expiry.
        discount_curve : str
            Discount curve identifier in MarketContext.
        credit_curve : str
            Credit curve identifier in MarketContext.
        vol_surface : str
            Volatility surface identifier for CDS option pricing.
        option_type : str, optional
            Option type: "call" (default, right to buy protection) or "put"
            (right to sell protection).
        recovery_rate : float, optional
            Recovery rate (default: 0.40).
        underlying_is_index : bool, optional
            If True, underlying is a CDS index (default: False, single-name).
        index_factor : float, optional
            Index factor if underlying is an index (default: 1.0).
        forward_adjust : float, optional
            Forward spread adjustment as decimal rate (default: 0.0).

        Returns
        -------
        CdsOption
            Configured CDS option ready for pricing.

        Raises
        ------
        ValueError
            If parameters are invalid or if required market data is missing.

        Examples
        --------
            >>> cds_option = (
            ...     CdsOption
            ...     .builder("CDS-OPT-CORP-A")
            ...     .notional(Money(10_000_000, Currency("USD")))
            ...     .strike(0.015)  # 150bp as decimal rate
            ...     .expiry(date(2024, 12, 20))
            ...     .cds_maturity(date(2029, 1, 1))
            ...     .discount_curve("USD")
            ...     .credit_curve("CORP-A-HAZARD")
            ...     .vol_surface("CDS-VOL")
            ...     .build()
            ... )
        """
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def strike(self) -> float: ...
    @property
    def strike_spread_bp(self) -> float: ...
    @property
    def expiry(self) -> date: ...
    @property
    def cds_maturity(self) -> date: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def credit_curve(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
