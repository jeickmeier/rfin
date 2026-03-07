"""Repo instrument (builder-only API)."""

from __future__ import annotations
from datetime import date
from ....core.currency import Currency
from ....core.money import Money
from ....core.dates.daycount import DayCount
from ....core.dates.calendar import BusinessDayConvention
from ...common import InstrumentType

class RepoType:
    """Repo type classification."""

    TERM: RepoType
    OPEN: RepoType
    OVERNIGHT: RepoType
    @classmethod
    def from_name(cls, name: str) -> RepoType: ...
    @property
    def name(self) -> str: ...

class RepoCollateral:
    """Collateral specification for Repo."""
    def __init__(
        self,
        instrument_id: str,
        quantity: float,
        market_value_id: str,
        *,
        collateral_type: str = "general",
        special_security_id: str | None = None,
        special_rate_adjust_bp: float | None = None,
    ) -> None: ...
    @property
    def instrument_id(self) -> str: ...
    @property
    def quantity(self) -> float: ...
    @property
    def market_value_id(self) -> str: ...

class RepoBuilder:
    """Fluent builder returned by :meth:`Repo.builder`."""

    def __init__(self, instrument_id: str) -> None: ...
    def cash(self, amount: float) -> RepoBuilder: ...
    def currency(self, currency: str | Currency) -> RepoBuilder: ...
    def cash_amount(self, money: Money) -> RepoBuilder: ...
    def collateral(self, collateral: RepoCollateral) -> RepoBuilder: ...
    def repo_rate(self, repo_rate: float) -> RepoBuilder: ...
    def start_date(self, start_date: date) -> RepoBuilder: ...
    def maturity(self, maturity: date) -> RepoBuilder: ...
    def discount_curve(self, curve_id: str) -> RepoBuilder: ...
    def disc_id(self, curve_id: str) -> RepoBuilder:
        """Deprecated: use :meth:`discount_curve` instead."""
        ...
    def repo_type(self, repo_type: str | RepoType | None = ...) -> RepoBuilder: ...
    def haircut(self, haircut: float) -> RepoBuilder: ...
    def day_count(self, day_count: DayCount | str) -> RepoBuilder: ...
    def business_day_convention(self, business_day_convention: BusinessDayConvention | str) -> RepoBuilder: ...
    def calendar(self, calendar: str | None = ...) -> RepoBuilder: ...
    def triparty(self, triparty: bool) -> RepoBuilder: ...
    def build(self) -> "Repo": ...

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
        >>> repo = (
        ...     Repo
        ...     .builder("REPO-UST-5Y")
        ...     .cash_amount(Money(1_000_000, Currency("USD")))
        ...     .get_collateral(collateral)
        ...     .repo_rate(0.03)  # 3% repo rate
        ...     .start_date(date(2024, 1, 1))
        ...     .maturity(date(2024, 1, 8))  # 1-week repo
        ...     .discount_curve("USD")
        ...     .repo_type("term")
        ...     .build()
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
    def builder(cls, instrument_id: str) -> RepoBuilder: ...
    @classmethod
    def overnight(cls, instrument_id: str) -> RepoBuilder:
        """Create an overnight repo builder (repo_type pre-set to overnight)."""
        ...
    @classmethod
    def term(cls, instrument_id: str) -> RepoBuilder:
        """Create a term repo builder (repo_type pre-set to term)."""
        ...
    @classmethod
    def open(cls, instrument_id: str) -> RepoBuilder:
        """Create an open repo builder (repo_type pre-set to open)."""
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
    def effective_rate(self) -> float: ...
    @property
    def interest_amount(self) -> Money: ...
    @property
    def total_repayment(self) -> Money: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
