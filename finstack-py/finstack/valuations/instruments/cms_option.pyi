"""CMS option instrument."""

from __future__ import annotations
from typing import List
from datetime import date
from ...core.money import Money
from ...core.dates.schedule import Frequency
from ...core.dates.daycount import DayCount

class CmsOption:
    """CMS (Constant Maturity Swap) option for interest rate optionality.

    CmsOption represents an option on a constant maturity swap rate (e.g.,
    10-year swap rate). The option pays based on the difference between the
    CMS rate and strike rate on each fixing date.

    CMS options are used for interest rate optionality and structured products.
    They require discount curves and optionally volatility surfaces.

    Examples
    --------
    Create a CMS cap (call option on CMS rate):

        >>> from finstack.valuations.instruments import CmsOption
        >>> from finstack import Money, Currency
        >>> from datetime import date
        >>> fixing_dates = [date(2024, 3, 15), date(2024, 6, 15), date(2024, 9, 15)]
        >>> cms_option = CmsOption.builder(
        ...     "CMS-CAP-10Y",
        ...     strike=0.035,  # 3.5% strike
        ...     cms_tenor=10.0,  # 10-year CMS rate
        ...     fixing_dates=fixing_dates,
        ...     accrual_fractions=[0.25, 0.25, 0.25],  # Quarterly
        ...     option_type="call",  # Cap (call on rate)
        ...     notional=Money(10_000_000, Currency("USD")),
        ...     discount_curve="USD",
        ...     forward_curve="USD-SOFR-3M",
        ...     vol_surface="USD-CMS10Y-VOL",
        ... )

    Notes
    -----
    - CMS options require discount curve and swap curve
    - CMS tenor is the maturity of the underlying swap rate (e.g., 10 years)
    - Option type: "call" (cap) or "put" (floor)
    - Each fixing date creates a separate CMS option (caplet/floorlet)
    - Convexity adjustment accounts for CMS vs forward rate differences

    Conventions
    -----------
    - Rates are expressed as decimals (e.g., 0.035 for 3.5%).
    - ``cms_tenor`` is expressed in years.
    - ``accrual_fractions`` are year fractions for each fixing period (must align with ``fixing_dates``).
    - If ``vol_surface`` is provided, it must be present in ``MarketContext`` and quoted in decimal volatility.

    MarketContext Requirements
    -------------------------
    - Discount curve: ``discount_curve`` (required).
    - Volatility surface: ``vol_surface`` (optional; required by pricing models that use it).

    See Also
    --------
    :class:`Swaption`: Swaptions
    :class:`InterestRateOption`: Interest rate caps/floors
    :class:`PricerRegistry`: Pricing entry point

    Sources
    -------
    - Brigo & Mercurio (2006): see ``docs/REFERENCES.md#brigoMercurio2006``.
    - Hull (text): see ``docs/REFERENCES.md#hullOptionsFuturesDerivatives``.
    """

    @classmethod
    def builder(
        cls,
        instrument_id: str,
        strike: float,
        cms_tenor: float,
        fixing_dates: List[date],
        accrual_fractions: List[float],
        option_type: str,
        notional: Money,
        discount_curve: str,
        forward_curve: str,
        vol_surface: str,
        *,
        payment_dates: List[date] | None = None,
        swap_fixed_freq: Frequency | None = None,
        swap_float_freq: Frequency | None = None,
        swap_day_count: DayCount | None = None,
    ) -> "CmsOption":
        """Create a CMS option.

        Parameters
        ----------
        instrument_id : str
            Unique identifier for the option (e.g., "CMS-CAP-10Y").
        strike : float
            Strike rate as a decimal (e.g., 0.035 for 3.5%).
        cms_tenor : float
            CMS tenor in years (e.g., 10.0 for 10-year swap rate).
        fixing_dates : List[date]
            Dates when CMS rates are observed. Must match accrual_fractions length.
        accrual_fractions : List[float]
            Accrual fractions for each period (e.g., [0.25, 0.25, 0.25] for quarterly).
        option_type : str
            Option type: "call" (cap) or "put" (floor).
        notional : Money
            Notional principal amount.
        discount_curve : str
            Discount curve identifier in MarketContext.
        forward_curve : str
            Forward/projection curve identifier for CMS rate estimation.
        vol_surface : str
            Volatility surface identifier for CMS option pricing.
        payment_dates : List[date], optional
            Payment dates (default: fixing_dates).
        swap_fixed_freq : Frequency, optional
            Fixed leg frequency of underlying swap (default: semi-annual).
        swap_float_freq : Frequency, optional
            Floating leg frequency of underlying swap (default: quarterly).
        swap_day_count : DayCount, optional
            Day-count convention for underlying swap (default: 30/360).

        Returns
        -------
        CmsOption
            Configured CMS option ready for pricing.

        Raises
        ------
        ValueError
            If parameters are invalid (fixing_dates/accrual_fractions mismatch, etc.)
            or if required market data is missing.

        Examples
        --------
            >>> option = CmsOption.builder(
            ...     "CMS-CAP-10Y",
            ...     0.035,  # 3.5% strike
            ...     10.0,  # 10-year CMS
            ...     fixing_dates,
            ...     accrual_fractions,
            ...     "call",
            ...     Money(10_000_000, Currency("USD")),
            ...     discount_curve="USD",
            ...     forward_curve="USD-SOFR-3M",
            ...     vol_surface="USD-CMS10Y-VOL",
            ... )
        """
        ...

    @classmethod
    def from_schedule(
        cls,
        instrument_id: str,
        start_date: date,
        maturity: date,
        frequency: Frequency,
        cms_tenor: float,
        strike: float,
        option_type: str,
        notional: Money,
        discount_curve: str,
        forward_curve: str,
        vol_surface: str,
        *,
        swap_fixed_freq: Frequency | None = None,
        swap_float_freq: Frequency | None = None,
        swap_day_count: DayCount | None = None,
        day_count: DayCount | None = None,
    ) -> "CmsOption":
        """Create a CMS option from a schedule specification.

        Generates fixing and payment dates from ``start_date``, ``maturity``,
        and ``frequency`` using standard market conventions (Modified Following
        BDC, weekends-only calendar).

        Parameters
        ----------
        instrument_id : str
            Unique identifier for the option.
        start_date : date
            Start of the first accrual period.
        maturity : date
            End of the last accrual period.
        frequency : Frequency
            Coupon/observation frequency (e.g. quarterly).
        cms_tenor : float
            Tenor of the underlying CMS swap in years (e.g. ``10.0`` for 10Y).
        strike : float
            Strike rate as a decimal (e.g. ``0.035`` for 3.5%).
        option_type : str
            ``"call"`` for a CMS cap or ``"put"`` for a CMS floor.
        notional : Money
            Notional principal amount.
        discount_curve : str
            Discount curve identifier in MarketContext.
        forward_curve : str
            Forward/projection curve identifier for CMS rate estimation.
        vol_surface : str
            Volatility surface identifier.
        swap_fixed_freq : Frequency, optional
            Fixed-leg coupon frequency of the underlying swap (default: semi-annual).
        swap_float_freq : Frequency, optional
            Floating-leg coupon frequency of the underlying swap (default: quarterly).
        swap_day_count : DayCount, optional
            Day-count for the underlying swap fixed leg (default: 30/360).
        day_count : DayCount, optional
            Day-count for accrual fractions and vol interpolation (default: Act/365F).

        Returns
        -------
        CmsOption
            Configured CMS option ready for pricing.

        Raises
        ------
        ValueError
            If the schedule is empty (e.g. ``maturity <= start_date``) or
            any parameter is invalid.

        Examples
        --------
            >>> from datetime import date
            >>> from finstack import Money, Currency
            >>> from finstack.core.dates.schedule import Frequency
            >>> from finstack.valuations.instruments import CmsOption
            >>> option = CmsOption.from_schedule(
            ...     "CMS-CAP-10Y",
            ...     date(2025, 1, 1),
            ...     date(2026, 1, 1),
            ...     Frequency.QUARTERLY,
            ...     10.0,
            ...     0.035,
            ...     "call",
            ...     Money(10_000_000, Currency("USD")),
            ...     "USD-OIS",
            ...     "USD-SOFR-3M",
            ...     "USD-CMS10Y-VOL",
            ... )
        """
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def strike(self) -> float: ...
    @property
    def cms_tenor(self) -> float: ...
    @property
    def option_type(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def fixing_dates(self) -> List[date]: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
