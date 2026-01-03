"""CDS tranche instrument."""

from typing import Optional
from datetime import date
from ...core.money import Money
from ...core.dates.daycount import DayCount
from ...core.dates.calendar import BusinessDayConvention
from ..common import InstrumentType

class CdsTranche:
    """CDS tranche for structured credit exposure.

    CdsTranche represents a tranche of a CDS index, providing exposure to
    a specific loss layer (attachment point to detachment point). Tranches
    are used in structured credit products (CDO, CLO).

    CDS tranches provide leveraged credit exposure and are priced using
    portfolio credit models. They require discount curves and credit index
    curves.

    Examples
    --------
    Create a CDS tranche (equity tranche):

        >>> from finstack.valuations.instruments import CdsTranche
        >>> from finstack import Money, Currency
        >>> from datetime import date
        >>> tranche = CdsTranche.create(
        ...     "CDX-IG-0-3",
        ...     index_name="CDX.NA.IG",
        ...     series=40,
        ...     attach_pct=0.0,  # 0% attachment (equity)
        ...     detach_pct=3.0,  # 3% detachment
        ...     notional=Money(10_000_000, Currency("USD")),
        ...     maturity=date(2029, 1, 1),
        ...     running_coupon_bp=500.0,  # 500bp running coupon
        ...     discount_curve="USD",
        ...     credit_index_curve="CDX-IG-40",
        ... )

    Notes
    -----
    - CDS tranches require discount curve and credit index curve
    - Attachment and detachment define the loss layer
    - Equity tranche (0-3%) has highest risk and return
    - Senior tranches (e.g., 15-30%) have lower risk
    - Running coupon is paid on remaining notional

    Conventions
    -----------
    - ``attach_pct`` and ``detach_pct`` are expressed in percent points (e.g., 3.0 means 3% subordination).
    - ``running_coupon_bp`` is quoted in basis points (bp).
    - Required market data is identified by string IDs (``discount_curve``, ``credit_index_curve``) and must be
      present in ``MarketContext``.
    - The exact tranche pricing model is selected by the runtime/pricer; inputs such as ``payments_per_year``,
      ``day_count``, and calendar/BDC parameters control the cashflow schedule.

    MarketContext Requirements
    -------------------------
    - Discount curve: ``discount_curve`` (required).
    - Credit index curve: ``credit_index_curve`` (required).

    See Also
    --------
    :class:`CDSIndex`: CDS indices
    :class:`CreditDefaultSwap`: Single-name CDS
    :class:`PricerRegistry`: Pricing entry point

    Sources
    -------
    - Li (2000): see ``docs/REFERENCES.md#liGaussianCopula2000``.
    - O'Kane (2008): see ``docs/REFERENCES.md#okane2008``.
    - ISDA (2006) Definitions: see ``docs/REFERENCES.md#isda2006Definitions``.
    """

    @classmethod
    def create(
        cls,
        instrument_id: str,
        index_name: str,
        series: int,
        attach_pct: float,
        detach_pct: float,
        notional: Money,
        maturity: date,
        running_coupon_bp: float,
        discount_curve: str,
        credit_index_curve: str,
        *,
        side: Optional[str] = "buy_protection",
        payments_per_year: Optional[int] = 4,
        day_count: Optional[DayCount] = None,
        business_day_convention: Optional[BusinessDayConvention] = None,
        calendar: Optional[str] = None,
        effective_date: Optional[date] = None,
    ) -> "CdsTranche":
        """Create a CDS tranche referencing a credit index.

        Parameters
        ----------
        instrument_id : str
            Unique identifier for the tranche (e.g., "CDX-IG-0-3").
        index_name : str
            Index name (e.g., "CDX.NA.IG", "iTraxx.Europe").
        series : int
            Index series number.
        attach_pct : float
            Attachment point as percentage (e.g., 0.0 for equity, 3.0 for mezzanine).
        detach_pct : float
            Detachment point as percentage (e.g., 3.0, 7.0, 15.0, 30.0).
            Must be > attach_pct.
        notional : Money
            Notional principal amount.
        maturity : date
            Tranche maturity date.
        running_coupon_bp : float
            Running coupon in basis points paid on remaining notional.
        discount_curve : str
            Discount curve identifier in MarketContext.
        credit_index_curve : str
            Credit index curve identifier in MarketContext.
        side : str, optional
            Position side: "buy_protection" (default) or "sell_protection".
        payments_per_year : int, optional
            Coupon payment frequency per year (default: 4 for quarterly).
        day_count : DayCount, optional
            Day-count convention (default: ACT/360).
        business_day_convention : BusinessDayConvention, optional
            Business day convention for payment dates.
        calendar : str, optional
            Holiday calendar identifier.
        effective_date : date, optional
            Effective date (default: trade date).

        Returns
        -------
        CdsTranche
            Configured CDS tranche ready for pricing.

        Raises
        ------
        ValueError
            If parameters are invalid (detach_pct <= attach_pct, etc.) or if
            required curves are not found.

        Examples
        --------
            >>> tranche = CdsTranche.create(
            ...     "CDX-IG-0-3",
            ...     "CDX.NA.IG",
            ...     40,
            ...     0.0,  # 0% attachment
            ...     3.0,  # 3% detachment
            ...     Money(10_000_000, Currency("USD")),
            ...     date(2029, 1, 1),
            ...     500.0,  # 500bp
            ...     discount_curve="USD",
            ...     credit_index_curve="CDX-IG-40",
            ... )
        """
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def attach_pct(self) -> float: ...
    @property
    def detach_pct(self) -> float: ...
    @property
    def running_coupon_bp(self) -> float: ...
    @property
    def maturity(self) -> date: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def credit_index_curve(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
