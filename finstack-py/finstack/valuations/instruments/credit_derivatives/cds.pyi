"""Credit default swap instrument."""

from __future__ import annotations
from typing import Self
from datetime import date
from ....core.money import Money
from ....core.currency import Currency
from ....core.market_data.context import MarketContext
from ...common import InstrumentType
from ...conventions import CdsDocClause

class CDSConvention:
    """ISDA CDS convention for regional market standards.

    Provides access to standard convention parameters (day count, payment
    frequency, business day convention, settlement delay) for regional CDS
    markets.

    Examples
    --------
        >>> CDSConvention.ISDA_NA.day_count
        'act_360'
        >>> CDSConvention.detect_from_currency("EUR")
        CDSConvention('isda_eu')
    """

    ISDA_NA: "CDSConvention"
    ISDA_EU: "CDSConvention"
    ISDA_AS: "CDSConvention"
    CUSTOM: "CDSConvention"

    @classmethod
    def from_name(cls, name: str) -> "CDSConvention":
        """Parse a convention name.

        Parameters
        ----------
        name : str
            Convention name such as ``"isda_na"``, ``"isda_eu"``, ``"isda_as"``.

        Returns
        -------
        CDSConvention

        Raises
        ------
        ValueError
            If the name is not recognized.
        """
        ...

    @classmethod
    def detect_from_currency(cls, currency: Currency | str) -> "CDSConvention":
        """Detect the appropriate convention based on currency.

        Parameters
        ----------
        currency : Currency or str
            Currency object or ISO code string.

        Returns
        -------
        CDSConvention
        """
        ...

    @property
    def name(self) -> str: ...
    @property
    def day_count(self) -> str: ...
    @property
    def frequency(self) -> str: ...
    @property
    def business_day_convention(self) -> str: ...
    @property
    def settlement_delay(self) -> int: ...
    @property
    def default_calendar(self) -> str: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
    def __hash__(self) -> int: ...
    def __eq__(self, other: object) -> bool: ...
    def __ne__(self, other: object) -> bool: ...

class CDSPayReceive:
    """Pay/receive indicator for CDS premium leg."""

    PAY_PROTECTION: "CDSPayReceive"
    RECEIVE_PROTECTION: "CDSPayReceive"

    @classmethod
    def from_name(cls, name: str) -> "CDSPayReceive": ...
    @property
    def name(self) -> str: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
    def __hash__(self) -> int: ...
    def __eq__(self, other: object) -> bool: ...
    def __ne__(self, other: object) -> bool: ...

class CreditDefaultSwap:
    """Credit default swap for credit risk transfer and pricing.

    CreditDefaultSwap (CDS) is a credit derivative that provides protection
    against default of a reference entity. The protection buyer pays a periodic
    premium (spread) and receives a payment if the reference entity defaults.

    Examples
    --------
    Create a CDS buying protection:

        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.money import Money
        >>> from finstack.valuations.instruments import CreditDefaultSwap
        >>> cds = CreditDefaultSwap.buy_protection(
        ...     "CDS-CORP-A-5Y",
        ...     notional=Money(10_000_000, Currency("USD")),
        ...     spread_bp=150.0,
        ...     start_date=date(2024, 1, 1),
        ...     maturity=date(2029, 1, 1),
        ...     discount_curve="USD-OIS",
        ...     credit_curve="CORP-A-HAZARD",
        ...     recovery_rate=0.40,
        ... )

    Notes
    -----
    - Spread is quoted in basis points (e.g., 150bp = 1.5% annual).
    - Recovery rate affects the protection leg payment: ``(1 - recovery_rate) * notional``.
    - Premium leg pays spread quarterly (standard ISDA convention).
    - Convention defaults to ISDA North American; use ``convention`` kwarg for EU/Asian CDS.

    Sources
    -------
    - ISDA (2006) Definitions: see ``docs/REFERENCES.md#isda2006Definitions``.
    - O'Kane (2008): see ``docs/REFERENCES.md#okane2008``.
    """

    @classmethod
    def buy_protection(
        cls,
        instrument_id: str,
        notional: Money,
        spread_bp: float,
        start_date: date,
        maturity: date,
        discount_curve: str,
        credit_curve: str,
        *,
        recovery_rate: float | None = None,
        settlement_delay: int | None = None,
        convention: CDSConvention | None = None,
    ) -> "CreditDefaultSwap":
        """Create a CDS where the caller buys protection.

        Parameters
        ----------
        instrument_id : str
            Unique identifier for the CDS.
        notional : Money
            Notional principal amount.
        spread_bp : float
            CDS spread in basis points.
        start_date : date
            CDS start date.
        maturity : date
            CDS maturity date.
        discount_curve : str
            Discount curve identifier in MarketContext.
        credit_curve : str
            Credit (hazard) curve identifier in MarketContext.
        recovery_rate : float, optional
            Recovery rate (default: 0.40).
        settlement_delay : int, optional
            Settlement delay in days.
        convention : CDSConvention, optional
            ISDA convention (default: ISDA_NA).

        Returns
        -------
        CreditDefaultSwap
        """
        ...

    @classmethod
    def sell_protection(
        cls,
        instrument_id: str,
        notional: Money,
        spread_bp: float,
        start_date: date,
        maturity: date,
        discount_curve: str,
        credit_curve: str,
        *,
        recovery_rate: float | None = None,
        settlement_delay: int | None = None,
        convention: CDSConvention | None = None,
    ) -> "CreditDefaultSwap":
        """Create a CDS where the caller sells protection.

        Parameters
        ----------
        instrument_id : str
            Unique identifier for the CDS.
        notional : Money
            Notional principal amount.
        spread_bp : float
            CDS spread in basis points.
        start_date : date
            CDS start date.
        maturity : date
            CDS maturity date.
        discount_curve : str
            Discount curve identifier in MarketContext.
        credit_curve : str
            Credit (hazard) curve identifier in MarketContext.
        recovery_rate : float, optional
            Recovery rate (default: 0.40).
        settlement_delay : int, optional
            Settlement delay in days.
        convention : CDSConvention, optional
            ISDA convention (default: ISDA_NA).

        Returns
        -------
        CreditDefaultSwap
        """
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def side(self) -> CDSPayReceive: ...
    @property
    def notional(self) -> Money: ...
    @property
    def spread_bp(self) -> float: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def credit_curve(self) -> str: ...
    @property
    def recovery_rate(self) -> float: ...
    @property
    def settlement_delay(self) -> int: ...
    @property
    def start_date(self) -> date: ...
    @property
    def maturity(self) -> date: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    @property
    def convention(self) -> CDSConvention: ...
    @property
    def day_count(self) -> str: ...
    @property
    def frequency(self) -> str: ...
    @property
    def calendar(self) -> str | None: ...
    def isda_coupon_schedule(self) -> list[date]:
        """Return ISDA-standard coupon date schedule."""
        ...

    @classmethod
    def from_isda(
        cls,
        instrument_id: str,
        notional: Money,
        side: CDSPayReceive | str,
        convention: CDSConvention,
        spread_bp: float,
        start: date,
        end: date,
        recovery_rate: float,
        discount_curve_id: str,
        credit_curve_id: str,
    ) -> "CreditDefaultSwap":
        """Create a CDS with standard ISDA conventions."""
        ...

    @property
    def protection_start(self) -> date:
        """Effective protection start date."""
        ...

    @property
    def doc_clause_effective(self) -> CdsDocClause:
        """Effective ISDA documentation clause."""
        ...

    def build_premium_schedule(
        self,
        market: MarketContext,
        as_of: date,
    ) -> list[tuple[date, Money]]:
        """Build the premium leg cashflow schedule."""
        ...

    def to_json(self) -> str:
        """Serialize to JSON in envelope format.

        Returns:
            str: JSON string with schema version and tagged instrument spec.
        """
        ...

    @classmethod
    def from_json(cls, json_str: str) -> "Self":
        """Deserialize from JSON in envelope format.

        Args:
            json_str: JSON string in envelope format.

        Returns:
            The deserialized instrument.

        Raises:
            ValueError: If JSON is malformed or contains a different instrument type.
        """
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
