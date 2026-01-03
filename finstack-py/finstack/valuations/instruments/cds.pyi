"""Credit default swap instrument."""

from typing import Optional
from datetime import date
from ...core.money import Money
from ..common import InstrumentType

class CDSPayReceive:
    """Pay/receive indicator for CDS premium leg."""

    PAY_PROTECTION: "CDSPayReceive"
    RECEIVE_PROTECTION: "CDSPayReceive"

    @classmethod
    def from_name(cls, name: str) -> "CDSPayReceive": ...
    @property
    def name(self) -> str: ...

class CreditDefaultSwap:
    """Credit default swap for credit risk transfer and pricing.

    CreditDefaultSwap (CDS) is a credit derivative that provides protection
    against default of a reference entity. The protection buyer pays a periodic
    premium (spread) and receives a payment if the reference entity defaults.

    CDS pricing requires both a discount curve (for time value) and a credit
    curve (hazard curve) for default probability. The recovery rate determines
    the protection payment amount in case of default.

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

    Price the CDS:

        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.market_data.context import MarketContext
        >>> from finstack.core.market_data.term_structures import DiscountCurve, HazardCurve
        >>> from finstack.core.money import Money
        >>> from finstack.valuations.instruments import CreditDefaultSwap
        >>> from finstack.valuations.pricer import create_standard_registry
        >>> cds = CreditDefaultSwap.buy_protection(
        ...     "CDS-CORP-A-5Y",
        ...     Money(5_000_000, Currency("USD")),
        ...     120.0,
        ...     date(2024, 1, 1),
        ...     date(2029, 1, 1),
        ...     discount_curve="USD-OIS",
        ...     credit_curve="CORP-A-HAZARD",
        ...     recovery_rate=0.40,
        ... )
        >>> ctx = MarketContext()
        >>> ctx.insert_discount(DiscountCurve("USD-OIS", date(2024, 1, 1), [(0.0, 1.0), (5.0, 0.95)]))
        >>> ctx.insert_hazard(
        ...     HazardCurve("CORP-A-HAZARD", date(2024, 1, 1), [(0.5, 0.01), (5.0, 0.02)], recovery_rate=0.40)
        ... )
        >>> registry = create_standard_registry()
        >>> pv = registry.price(cds, "hazard_rate", ctx).value
        >>> pv.currency.code
        'USD'

    Notes
    -----
    - CDS requires discount curve and credit (hazard) curve
    - Spread is quoted in basis points (e.g., 150bp = 1.5% annual)
    - Recovery rate affects protection leg value (typically 40% for senior debt)
    - Settlement delay is the number of days between default and payment
    - Premium leg pays spread quarterly (standard convention)
    - Protection leg pays (1 - recovery_rate) * notional on default

    Conventions
    -----------
    - ``spread_bp`` is quoted in basis points (bp). Convert to decimal rate with ``spread_bp / 10_000``.
    - ``recovery_rate`` is a decimal fraction in [0, 1].
    - Required market data is identified by string IDs (``discount_curve``, ``credit_curve``) and must be present
      in ``MarketContext``.
    - The concrete schedule/roll conventions are determined by the runtime implementation and the inputs
      provided (e.g., dates and settlement_delay).

    MarketContext Requirements
    -------------------------
    - Discount curve: ``discount_curve`` (required).
    - Credit/hazard curve: ``credit_curve`` (required).

    See Also
    --------
    :class:`HazardCurve`: Credit curve for default probability
    :class:`Bond`: Bonds with credit risk
    :class:`PricerRegistry`: Pricing entry point

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
        recovery_rate: Optional[float] = None,
        settlement_delay: Optional[int] = None,
    ) -> "CreditDefaultSwap":
        """Create a CDS where the caller buys protection (pays premium, receives protection).

        Buying protection means paying the CDS spread (premium) in exchange for
        receiving a payment if the reference entity defaults. This is equivalent
        to being long credit risk (benefiting from credit improvement) or hedging
        a long credit position.

        Parameters
        ----------
        instrument_id : str
            Unique identifier for the CDS (e.g., "CDS-CORP-A-5Y").
        notional : Money
            Notional principal amount. The currency determines curve currency
            requirements.
        spread_bp : float
            CDS spread in basis points (e.g., 150.0 for 150bp = 1.5% annual).
            This is the premium paid quarterly by the protection buyer.
        start_date : date
            CDS start date (first accrual date for premium payments).
        maturity : date
            CDS maturity date (last premium payment date). Must be after start_date.
        discount_curve : str
            Discount curve identifier in MarketContext for present value calculations.
        credit_curve : str
            Credit (hazard) curve identifier in MarketContext for default probability.
            The curve should be calibrated to the reference entity's credit risk.
        recovery_rate : float, optional
            Recovery rate assumed in case of default, as a decimal (e.g., 0.40
            for 40%). Defaults to 0.40 if not specified. Affects the protection
            leg payment: (1 - recovery_rate) * notional.
        settlement_delay : int, optional
            Number of days between default event and protection payment (settlement
            delay). Defaults to standard market convention if not specified.

        Returns
        -------
        CreditDefaultSwap
            Configured CDS where the caller buys protection (pays premium).

        Raises
        ------
        ValueError
            If dates are invalid (maturity <= start_date), if spread_bp is negative,
            if recovery_rate is not in [0, 1], or if notional is invalid.

        Examples
        --------
            >>> from finstack import Money, Currency
            >>> from datetime import date
            >>> cds = CreditDefaultSwap.buy_protection(
            ...     "CDS-CORP-A-5Y",
            ...     Money(10_000_000, Currency("USD")),
            ...     150.0,  # 150bp spread
            ...     date(2024, 1, 1),
            ...     date(2029, 1, 1),  # 5-year CDS
            ...     discount_curve="USD",
            ...     credit_curve="CORP-A-HAZARD",
            ...     recovery_rate=0.40,
            ... )
            >>> cds.spread_bp
            150.0
            >>> cds.side
            CDSPayReceive.PAY_PROTECTION
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
        recovery_rate: Optional[float] = None,
        settlement_delay: Optional[int] = None,
    ) -> "CreditDefaultSwap":
        """Create a CDS where the caller sells protection (receives premium).

        Selling protection means receiving the CDS spread (premium) in exchange for
        paying a protection payment if the reference entity defaults. This is
        equivalent to being short credit risk (benefiting from credit deterioration)
        or taking a credit position.

        Parameters
        ----------
        instrument_id : str
            Unique identifier for the CDS.
        notional : Money
            Notional principal amount.
        spread_bp : float
            CDS spread in basis points. This is the premium received quarterly
            by the protection seller.
        start_date : date
            CDS start date.
        maturity : date
            CDS maturity date. Must be after start_date.
        discount_curve : str
            Discount curve identifier in MarketContext.
        credit_curve : str
            Credit (hazard) curve identifier in MarketContext.
        recovery_rate : float, optional
            Recovery rate (default: 0.40).
        settlement_delay : int, optional
            Settlement delay in days (default: market convention).

        Returns
        -------
        CreditDefaultSwap
            Configured CDS where the caller sells protection (receives premium).

        Raises
        ------
        ValueError
            If parameters are invalid.

        Examples
        --------
            >>> cds = CreditDefaultSwap.sell_protection(
            ...     "CDS-SELL-CORP-A",
            ...     Money(10_000_000, Currency("USD")),
            ...     150.0,
            ...     date(2024, 1, 1),
            ...     date(2029, 1, 1),
            ...     discount_curve="USD",
            ...     credit_curve="CORP-A-HAZARD",
            ... )
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
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
