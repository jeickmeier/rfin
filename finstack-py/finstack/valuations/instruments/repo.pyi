"""Repo instrument."""

from typing import Optional
from datetime import date
from ...core.money import Money
from ...core.dates.daycount import DayCount
from ...core.dates.calendar import BusinessDayConvention
from ..common import InstrumentType

class RepoCollateral:
    """Collateral specification for Repo."""
    def __init__(
        self,
        instrument_id: str,
        quantity: float,
        market_value_id: str,
        *,
        collateral_type: str = "general",
        special_security_id: Optional[str] = None,
        special_rate_adjust_bp: Optional[float] = None,
    ) -> None: ...
    @property
    def instrument_id(self) -> str: ...
    @property
    def quantity(self) -> float: ...
    @property
    def market_value_id(self) -> str: ...

class Repo:
    """Repurchase agreement for secured funding.

    Repo represents a repurchase agreement where one party sells securities
    and agrees to repurchase them at a future date at a higher price. The
    difference is the repo rate (funding cost).

    Repos are used for secured funding, collateral management, and short
    selling. They require discount curves and collateral valuations.

    Examples
    --------
    Create a repo:

        >>> from finstack.valuations.instruments import Repo, RepoCollateral
        >>> from finstack import Money, Currency
        >>> from datetime import date
        >>> collateral = RepoCollateral(
        ...     instrument_id="UST-5Y",
        ...     quantity=1000.0,  # 1000 bonds
        ...     market_value_id="UST-5Y-PRICE",
        ... )
        >>> repo = Repo.create(
        ...     "REPO-UST-5Y",
        ...     cash_amount=Money(1_000_000, Currency("USD")),
        ...     collateral=collateral,
        ...     repo_rate=0.03,  # 3% repo rate
        ...     start_date=date(2024, 1, 1),
        ...     maturity=date(2024, 1, 8),  # 1-week repo
        ...     discount_curve="USD",
        ...     repo_type="term",
        ... )

    Notes
    -----
    - Repos require discount curve and collateral market value
    - Repo rate is the funding cost (typically lower than unsecured rates)
    - Haircut provides margin protection for the lender
    - Repo type: "term" (fixed maturity) or "open" (overnight, rolling)
    - Triparty repos use a third-party custodian

    MarketContext Requirements
    -------------------------
    - Discount curve: ``discount_curve`` (required).
    - Collateral market value: ``collateral.market_value_id`` (required).

    See Also
    --------
    :class:`Deposit`: Unsecured deposits
    :class:`Bond`: Bond collateral
    :class:`PricerRegistry`: Pricing entry point

    Sources
    -------
    - Hull (text): see ``docs/REFERENCES.md#hullOptionsFuturesDerivatives``.
    - Brigo & Mercurio (2006): see ``docs/REFERENCES.md#brigoMercurio2006``.
    """

    @classmethod
    def create(
        cls,
        instrument_id: str,
        cash_amount: Money,
        collateral: RepoCollateral,
        repo_rate: float,
        start_date: date,
        maturity: date,
        discount_curve: str,
        *,
        repo_type: str = "term",
        haircut: float = 0.0,
        day_count: Optional[DayCount] = None,
        business_day_convention: Optional[BusinessDayConvention] = None,
        calendar: Optional[str] = None,
        triparty: bool = False,
    ) -> "Repo":
        """Create a repo.

        Parameters
        ----------
        instrument_id : str
            Unique identifier for the repo (e.g., "REPO-UST-5Y").
        cash_amount : Money
            Cash lent (repo proceeds). Currency determines curve currency.
        collateral : RepoCollateral
            Collateral specification (instrument, quantity, market value).
        repo_rate : float
            Repo rate as a decimal (e.g., 0.03 for 3%). This is the funding cost.
        start_date : date
            Repo start date (cash lent, collateral delivered).
        maturity : date
            Repo maturity date (cash repaid, collateral returned). Must be after start_date.
        discount_curve : str
            Discount curve identifier in MarketContext.
        repo_type : str, optional
            Repo type: "term" (default, fixed maturity) or "open" (overnight, rolling).
        haircut : float, optional
            Haircut percentage (default: 0.0). Provides margin protection (e.g., 0.02 = 2%).
        day_count : DayCount, optional
            Day-count convention (default: ACT/360 for money market).
        business_day_convention : BusinessDayConvention, optional
            Business day convention for maturity date adjustment.
        calendar : str, optional
            Holiday calendar identifier.
        triparty : bool, optional
            If True, uses triparty repo structure (default: False).

        Returns
        -------
        Repo
            Configured repo ready for pricing.

        Raises
        ------
        ValueError
            If parameters are invalid (maturity <= start_date, repo_rate < 0,
            etc.) or if required market data is missing.

        Examples
        --------
            >>> repo = Repo.create(
            ...     "REPO-UST-5Y",
            ...     Money(1_000_000, Currency("USD")),
            ...     collateral,
            ...     0.03,  # 3% repo rate
            ...     date(2024, 1, 1),
            ...     date(2024, 1, 8),
            ...     discount_curve="USD",
            ... )
        """
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def cash_amount(self) -> Money: ...
    @property
    def repo_rate(self) -> float: ...
    @property
    def start_date(self) -> date: ...
    @property
    def maturity(self) -> date: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
