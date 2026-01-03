"""CDS option instrument."""

from typing import Optional
from datetime import date
from ...core.money import Money
from ..common import InstrumentType

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
        >>> cds_option = CdsOption.create(
        ...     "CDS-OPT-CORP-A",
        ...     notional=Money(10_000_000, Currency("USD")),
        ...     strike_spread_bp=150.0,  # 150bp strike
        ...     expiry=date(2024, 12, 20),
        ...     cds_maturity=date(2029, 1, 1),  # 5-year underlying CDS
        ...     discount_curve="USD",
        ...     credit_curve="CORP-A-HAZARD",
        ...     vol_surface="CDS-VOL",
        ...     option_type="call",  # Right to buy protection at strike
        ... )

    Notes
    -----
    - CDS options require discount curve, credit curve, and volatility surface
    - Strike is the CDS spread in basis points
    - Option type: "call" (right to buy protection) or "put" (right to sell protection)
    - Underlying CDS maturity determines the protection period
    - Forward adjustment accounts for forward spread vs spot spread

    Conventions
    -----------
    - ``strike_spread_bp`` and ``forward_adjust_bp`` are quoted in basis points (bp).
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
    def create(
        cls,
        instrument_id: str,
        notional: Money,
        strike_spread_bp: float,
        expiry: date,
        cds_maturity: date,
        discount_curve: str,
        credit_curve: str,
        vol_surface: str,
        *,
        option_type: Optional[str] = "call",
        recovery_rate: Optional[float] = 0.4,
        underlying_is_index: Optional[bool] = False,
        index_factor: Optional[float] = None,
        forward_adjust_bp: Optional[float] = 0.0,
    ) -> "CdsOption":
        """Create a CDS option referencing a standard CDS contract.

        Parameters
        ----------
        instrument_id : str
            Unique identifier for the option (e.g., "CDS-OPT-CORP-A").
        notional : Money
            Notional principal amount.
        strike_spread_bp : float
            Strike CDS spread in basis points (e.g., 150.0 for 150bp).
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
        forward_adjust_bp : float, optional
            Forward spread adjustment in basis points (default: 0.0).

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
            >>> cds_option = CdsOption.create(
            ...     "CDS-OPT-CORP-A",
            ...     Money(10_000_000, Currency("USD")),
            ...     150.0,  # 150bp strike
            ...     date(2024, 12, 20),
            ...     date(2029, 1, 1),
            ...     discount_curve="USD",
            ...     credit_curve="CORP-A-HAZARD",
            ...     vol_surface="CDS-VOL",
            ... )
        """
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def notional(self) -> Money: ...
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
