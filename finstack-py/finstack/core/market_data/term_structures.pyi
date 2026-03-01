"""Term structure bindings for interest rate and credit curves.

Provides discount curves, forward curves, hazard curves, inflation curves,
and base correlation curves for financial modeling.
"""

from __future__ import annotations
from typing import List, Tuple, Dict
from datetime import date
from ..currency import Currency
from ..money import Money
from ..dates.daycount import DayCount
from ..math.interp import InterpStyle, ExtrapolationPolicy

class DiscountCurve:
    """Discount curve for present value calculations and interest rate modeling.

    DiscountCurve represents a term structure of discount factors used to
    calculate present values of future cashflows. It supports multiple
    interpolation methods (linear, monotone convex, etc.) and extrapolation
    policies for handling dates beyond the curve's knot points.

    Discount curves are the foundation of most financial calculations,
    including bond pricing, swap valuation, and risk metrics. They can be
    constructed from market rates (via :meth:`from_rates`) or directly from
    discount factors.

    Parameters
    ----------
    id : str
        Unique identifier for the curve (e.g., "USD", "EUR-LIBOR-3M").
        Used to retrieve the curve from a :class:`MarketContext`.
    base_date : datetime.date
        Anchor date corresponding to t=0. All time calculations are
        relative to this date.
    knots : list[tuple[float, float]]
        List of (time_years, discount_factor) pairs defining the curve.
        Times are in years from base_date. Discount factors must be in (0, 1]
        and typically decrease with time.
    day_count : DayCount or str, optional
        Day-count convention for converting dates to year fractions.
        Defaults to ACT/365.25 if not specified.
    interp : InterpStyle or str, optional
        Interpolation method between knots. Defaults to ``"log_linear"`` which
        guarantees positive forward rates (the QuantLib standard). Other options:
        ``"linear"``, ``"monotone_convex"``, ``"cubic_hermite"``.
    extrapolation : ExtrapolationPolicy or str, optional
        How to handle dates beyond the curve's range. Defaults to ``"flat_forward"``
        for smooth instantaneous forwards beyond the last knot.
        Other option: ``"flat_zero"``.
    require_monotonic : bool, default True
        If True, enforce that discount factors are monotonically decreasing
        (i.e., longer times have smaller discount factors). Set False only when
        you intentionally need to allow non-monotonic discount factors.

    Returns
    -------
    DiscountCurve
        Immutable discount curve ready for queries and use in MarketContext.

    Raises
    ------
    ValueError
        If knots are empty, if discount factors are invalid (<= 0 or > 1),
        if times are not in ascending order, or if require_monotonic=True
        and knots are not monotonic.

    Examples
    --------
        >>> from datetime import date
        >>> from finstack.core.market_data.term_structures import DiscountCurve
        >>> curve = DiscountCurve("USD", date(2024, 1, 1), [(0.0, 1.0), (1.0, 0.99), (2.0, 0.98)])
        >>> print(round(curve.df(1.0), 6), f"{curve.zero(1.0):.4f}")
        (0.99, '0.0101')

    Notes
    -----
    - Discount curves are immutable once constructed
    - Time is always in years from base_date
    - Discount factors must be positive and typically <= 1.0
    - Use :meth:`from_rates` for convenience when starting from market rates
    - Log-linear interpolation (flat forward) is the default and prevents negative implied forwards
    - Extrapolation policy determines behavior beyond the curve's range

    See Also
    --------
    :meth:`from_rates`: Factory method to build from zero rates
    :class:`MarketContext`: Container for discount curves
    :class:`ForwardCurve`: Forward rate curves
    """

    def __init__(
        self,
        id: str,
        base_date: str | date,
        knots: List[Tuple[float, float]],
        day_count: str | DayCount | None = None,
        interp: str | InterpStyle | None = None,
        extrapolation: str | ExtrapolationPolicy | None = None,
        require_monotonic: bool = True,
    ) -> None: ...
    @property
    def id(self) -> str:
        """Get the curve identifier.

        Returns
        -------
        str
            Curve ID.
        """
        ...

    @property
    def base_date(self) -> date:
        """Get the base date.

        Returns
        -------
        date
            Base date (t=0).
        """
        ...

    @property
    def day_count(self) -> DayCount:
        """Get the day count convention.

        Returns
        -------
        DayCount
            Day count convention.
        """
        ...

    @property
    def points(self) -> List[Tuple[float, float]]:
        """Get the knot points.

        Returns
        -------
        List[Tuple[float, float]]
            (time, discount_factor) pairs.
        """
        ...

    def df(self, t: float) -> float:
        """Get the discount factor at a given time.

        The discount factor represents the present value of $1 received at time t.
        It is interpolated from the curve's knot points using the configured
        interpolation method.

        Parameters
        ----------
        t : float
            Time in years from the base_date. Must be >= 0.

        Returns
        -------
        float
            Discount factor in the range (0, 1]. For t=0, returns 1.0.
            For times beyond the curve's range, extrapolation policy applies.

        Raises
        ------
        ValueError
            If t < 0 or if extrapolation fails.
        """
        ...

    def zero(self, t: float) -> float:
        """Get the continuously compounded zero rate at a given time.

        The zero rate is the interest rate for a zero-coupon bond maturing at
        time t. It is derived from the discount factor: r(t) = -ln(DF(t)) / t.

        Parameters
        ----------
        t : float
            Time in years from the base_date. Must be > 0.

        Returns
        -------
        float
            Continuously compounded zero rate (as a decimal, e.g., 0.05 for 5%).

        Raises
        ------
        ValueError
            If t <= 0 or if the discount factor is invalid.
        """
        ...

    def forward(self, t1: float, t2: float) -> float:
        """Get the continuously compounded forward rate between two times.

        The forward rate is the interest rate agreed today for borrowing/lending
        from time t1 to t2. It is derived from discount factors:
        F(t1, t2) = (ln(DF(t1)) - ln(DF(t2))) / (t2 - t1).

        Parameters
        ----------
        t1 : float
            Start time in years from base_date. Must be >= 0.
        t2 : float
            End time in years from base_date. Must be > t1.

        Returns
        -------
        float
            Continuously compounded forward rate (as a decimal, e.g., 0.025 for 2.5%).

        Raises
        ------
        ValueError
            If t1 < 0, t2 <= t1, or if discount factors are invalid.
        """
        ...

    def df_on_date(self, date: str | date) -> float:
        """Get discount factor on a specific date.

        Parameters
        ----------
        date : str or date
            Target date.

        Returns
        -------
        float
            Discount factor.
        """
        ...

    def npv(
        self,
        cash_flows: List[Tuple[date, Money]],
        day_count: str | DayCount | None = None,
    ) -> Money:
        """Calculate the Net Present Value of a series of cashflows.

        Discounts all cashflows to the curve's base_date using the discount
        curve. All cashflows must have the same currency. The result is in
        that currency.

        Parameters
        ----------
        cash_flows : list[tuple[date, Money]]
            List of (date, Money) pairs representing future cashflows.
            Dates can be before or after base_date (negative/positive times).
            All Money amounts must have the same currency.
        day_count : DayCount or str, optional
            Day-count convention for converting dates to year fractions.
            Defaults to the curve's day_count if not specified.

        Returns
        -------
        Money
            Net present value in the currency of the cashflows. Sum of all
            discounted cashflows.

        Raises
        ------
        ValueError
            If cash_flows is empty, if cashflows have different currencies,
            or if discounting fails.
        """
        ...

    def __repr__(self) -> str: ...

class ForwardCurve:
    """Forward rate curve for floating-rate instrument modeling.

    ForwardCurve represents a term structure of forward rates for a specific
    tenor (e.g., 3-month LIBOR, 6-month SOFR). It is used to project future
    floating rate resets for swaps, floating-rate bonds, and other instruments
    with periodic rate resets.

    Unlike DiscountCurve which stores discount factors, ForwardCurve stores
    forward rates directly. The curve defines the forward rate for each reset
    period, allowing instruments to project their floating cashflows.

    Parameters
    ----------
    id : str
        Unique identifier for the curve (e.g., "USD-LIBOR-3M", "EUR-EURIBOR-6M").
        Used to retrieve the curve from a :class:`MarketContext`.
    tenor_years : float
        Tenor of the forward rate in years (e.g., 0.25 for 3-month, 0.5 for
        6-month, 1.0 for 1-year). This defines the reset frequency.
    knots : list[tuple[float, float]]
        List of (time_years, forward_rate) pairs defining the curve.
        Times are in years from base_date. Rates are typically in decimal form
        (e.g., 0.05 for 5%).
    base_date : datetime.date, optional
        Anchor date corresponding to t=0. If None, defaults to today's date.
    reset_lag : int, optional
        Number of days between the reset date and the payment date (settlement
        lag). Defaults to 0 if not specified. Common values: 0 (same day),
        2 (T+2 settlement).
    day_count : DayCount or str, optional
        Day-count convention for converting dates to year fractions.
        Defaults to ACT/365.25 if not specified.
    interp : InterpStyle or str, optional
        Interpolation method between knots. Options: "linear", "monotone_convex",
        "log_linear", "cubic_spline". Defaults to "linear".

    Returns
    -------
    ForwardCurve
        Immutable forward curve ready for queries and use in MarketContext.

    Raises
    ------
    ValueError
        If knots are empty, if times are not in ascending order, if tenor_years
        is <= 0, or if rates are invalid.

    Examples
    --------
        >>> from datetime import date
        >>> from finstack.core.market_data.term_structures import ForwardCurve
        >>> curve = ForwardCurve("USD-LIBOR-3M", 0.25, [(0.0, 0.015), (0.5, 0.017)], base_date=date(2024, 1, 1))
        >>> print(f"{curve.rate(0.5):.4f}")
        0.0170

    Notes
    -----
    - Forward curves are immutable once constructed
    - The tenor defines the reset frequency (not the curve's time grid)
    - Reset lag affects the timing of cashflow projections
    - Forward rates are typically simple rates (not continuously compounded)
    - Use with :class:`DiscountCurve` for complete swap/floating bond pricing

    See Also
    --------
    :class:`DiscountCurve`: Discount curves for present value calculations
    :class:`MarketContext`: Container for forward curves
    """

    def __init__(
        self,
        id: str,
        tenor_years: float,
        knots: List[Tuple[float, float]],
        base_date: str | date | None = None,
        reset_lag: int | None = None,
        day_count: str | DayCount | None = None,
        interp: str | InterpStyle | None = None,
    ) -> None: ...
    @property
    def id(self) -> str:
        """Get the curve identifier.

        Returns
        -------
        str
            Curve ID.
        """
        ...

    @property
    def tenor_years(self) -> float:
        """Get the tenor in years.

        Returns
        -------
        float
            Tenor in years.
        """
        ...

    @property
    def base_date(self) -> date:
        """Get the base date.

        Returns
        -------
        date
            Base date.
        """
        ...

    @property
    def day_count(self) -> DayCount:
        """Get the day count convention.

        Returns
        -------
        DayCount
            Day count convention.
        """
        ...

    @property
    def reset_lag(self) -> int:
        """Get the reset lag in days.

        Returns
        -------
        int
            Reset lag in days.
        """
        ...

    @property
    def points(self) -> List[Tuple[float, float]]:
        """Get the knot points.

        Returns
        -------
        List[Tuple[float, float]]
            (time, rate) pairs.
        """
        ...

    def rate(self, t: float) -> float:
        """Get the forward rate at a given time.

        Returns the forward rate for the curve's tenor starting at time t.
        The rate is interpolated from the curve's knot points using the configured
        interpolation method.

        Parameters
        ----------
        t : float
            Time in years from the base_date when the forward period starts.
            Must be >= 0.

        Returns
        -------
        float
            Forward rate as a decimal (e.g., 0.05 for 5%). The rate applies to
            the curve's tenor period starting at time t.

        Raises
        ------
        ValueError
            If t < 0 or if extrapolation fails.
        """
        ...

    def df(self, t: float) -> float:
        """Implied projection discount factor from 0 to ``t`` (years).

        Notes
        -----
        This is a *projection DF* implied by chaining the forward curve's simple rates;
        it is not a PV discount curve.
        """
        ...

    def df_on_date(self, date: str | date) -> float:
        """Implied projection discount factor on a calendar date using the curve's day-count."""
        ...

    def __repr__(self) -> str: ...

class HazardCurve:
    """Hazard rate curve for credit risk and default probability modeling.

    HazardCurve represents a term structure of hazard rates (default intensities)
    used to model credit risk. It is essential for pricing credit default swaps
    (CDS), calculating default probabilities, and valuing credit-sensitive
    instruments.

    The hazard rate at time t represents the instantaneous probability of default
    per unit time, conditional on survival until time t. The curve can be
    calibrated from market CDS spreads or constructed from model parameters.

    Parameters
    ----------
    id : str
        Unique identifier for the curve (e.g., "CORP-ISSUER-A", "CDS-INDEX").
        Used to retrieve the curve from a :class:`MarketContext`.
    base_date : datetime.date
        Anchor date corresponding to t=0. All time calculations are relative
        to this date.
    knots : list[tuple[float, float]]
        List of (time_years, hazard_rate) pairs defining the curve.
        Times are in years from base_date. Hazard rates are typically in
        decimal form (e.g., 0.02 for 2% annual hazard rate).
    recovery_rate : float, optional
        Recovery rate assumed in case of default, as a decimal (e.g., 0.40
        for 40% recovery). Defaults to 0.40 (40%) if not specified. Used
        for CDS pricing and default probability calculations.
    day_count : DayCount or str, optional
        Day-count convention for converting dates to year fractions.
        Defaults to ACT/365.25 if not specified.
    issuer : str, optional
        Issuer identifier or name associated with this curve. Useful for
        tracking and reporting.
    seniority : str, optional
        Seniority level of the credit (e.g., "senior", "subordinated").
        Affects recovery rate assumptions in some models.
    currency : Currency, optional
        Currency associated with the credit curve. Used for multi-currency
        credit portfolios.
    par_points : list[tuple[float, float]], optional
        Par CDS spread points used for calibration, as (time, spread_bp) pairs.
        Used to validate or calibrate the hazard curve against market data.

    Returns
    -------
    HazardCurve
        Immutable hazard curve ready for queries and use in MarketContext.

    Raises
    ------
    ValueError
        If knots are empty, if times are not in ascending order, if recovery_rate
        is not in [0, 1], or if hazard rates are invalid (negative).

    Examples
    --------
        >>> from datetime import date
        >>> from finstack.core.market_data.term_structures import HazardCurve
        >>> curve = HazardCurve("CORP-ISSUER-A", date(2024, 1, 1), [(0.5, 0.01), (1.0, 0.015)], recovery_rate=0.4)
        >>> print(f"{curve.survival(1.0):.4f}")
        0.9851

    Notes
    -----
    - Hazard curves are immutable once constructed
    - Hazard rates represent instantaneous default probabilities
    - Recovery rate is a key parameter for CDS pricing (affects protection leg)
    - Survival probabilities are derived from hazard rates via integration
    - Use with :class:`DiscountCurve` for complete CDS valuation
    - Seniority and issuer metadata are for tracking, not calculation

    See Also
    --------
    :class:`DiscountCurve`: Discount curves for CDS pricing
    :class:`MarketContext`: Container for hazard curves
    """

    def __init__(
        self,
        id: str,
        base_date: str | date,
        knots: List[Tuple[float, float]],
        recovery_rate: float | None = None,
        day_count: str | DayCount | None = None,
        issuer: str | None = None,
        seniority: str | None = None,
        currency: Currency | None = None,
        par_points: List[Tuple[float, float]] | None = None,
    ) -> None: ...
    @property
    def id(self) -> str:
        """Get the curve identifier.

        Returns
        -------
        str
            Curve ID.
        """
        ...

    @property
    def base_date(self) -> date:
        """Get the base date.

        Returns
        -------
        date
            Base date.
        """
        ...

    @property
    def recovery_rate(self) -> float:
        """Get the recovery rate.

        Returns
        -------
        float
            Recovery rate.
        """
        ...

    @property
    def day_count(self) -> DayCount:
        """Get the day count convention.

        Returns
        -------
        DayCount
            Day count convention.
        """
        ...

    @property
    def points(self) -> List[Tuple[float, float]]:
        """Get the knot points.

        Returns
        -------
        List[Tuple[float, float]]
            (time, hazard_rate) pairs.
        """
        ...

    @property
    def par_spreads(self) -> List[Tuple[float, float]]:
        """Get the par spread points.

        Returns
        -------
        List[Tuple[float, float]]
            (time, spread) pairs.
        """
        ...

    def survival(self, t: float) -> float:
        """Get the survival probability (probability of no default) at time t.

        The survival probability is the probability that the credit entity has
        not defaulted by time t. It is derived from the hazard rate curve via
        integration: S(t) = exp(-∫₀ᵗ h(s) ds).

        Parameters
        ----------
        t : float
            Time in years from the base_date. Must be >= 0.

        Returns
        -------
        float
            Survival probability in the range [0, 1]. For t=0, returns 1.0.
            Decreases with time as default risk accumulates.

        Raises
        ------
        ValueError
            If t < 0 or if the hazard rate integration fails.

        Examples
        --------
        """
        ...

    def default_prob(self, t1: float, t2: float) -> float:
        """Get the probability of default occurring between times t1 and t2.

        Calculates the probability that default occurs in the interval [t1, t2],
        conditional on survival until t1. This is used for CDS pricing to
        determine protection leg cashflows.

        The formula is: P(default in [t1, t2]) = S(t1) - S(t2), where S(t)
        is the survival probability.

        Parameters
        ----------
        t1 : float
            Start time in years from base_date. Must be >= 0.
        t2 : float
            End time in years from base_date. Must be > t1.

        Returns
        -------
        float
            Default probability in the range [0, 1]. Represents the probability
            of default occurring between t1 and t2.

        Raises
        ------
        ValueError
            If t1 < 0, t2 <= t1, or if survival probability calculation fails.

        Examples
        --------
        """
        ...

    def __repr__(self) -> str: ...

class InflationCurve:
    """Consumer Price Index (CPI) curve for inflation-linked instrument pricing.

    InflationCurve represents a term structure of CPI levels used to model
    inflation for inflation-linked bonds, inflation swaps, and other
    instruments with inflation-indexed cashflows. The curve defines expected
    CPI levels at future dates, allowing calculation of inflation rates and
    real returns.

    Parameters
    ----------
    id : str
        Unique identifier for the curve (e.g., "US-CPI", "EU-HICP").
        Used to retrieve the curve from a :class:`MarketContext`.
    base_cpi : float
        Base CPI level at t=0 (the reference CPI level). This is typically
        the most recent published CPI value (e.g., 300.0 for a CPI index
        with base year 1982-84=100).
    knots : list[tuple[float, float]]
        List of (time_years, cpi_level) pairs defining the curve.
        Times are in years from the base date. CPI levels are absolute
        index values (not ratios).
    interp : InterpStyle or str, optional
        Interpolation method between knots. Options: "linear", "monotone_convex",
        "log_linear", "cubic_spline". Defaults to "linear". Log-linear
        interpolation is often preferred for CPI curves.

    Returns
    -------
    InflationCurve
        Immutable inflation curve ready for queries and use in MarketContext.

    Raises
    ------
    ValueError
        If knots are empty, if times are not in ascending order, if base_cpi
        is <= 0, or if CPI levels are invalid (negative or decreasing).

    Examples
    --------
        >>> from finstack.core.market_data.term_structures import InflationCurve
        >>> curve = InflationCurve("US-CPI", 300.0, [(1.0, 304.5), (2.0, 309.0)])
        >>> print((curve.cpi(1.0), f"{curve.inflation_rate(1.0, 2.0):.4f}"))
        (304.5, '0.0148')

    Notes
    -----
    - Inflation curves are immutable once constructed
    - CPI levels are absolute index values (not ratios to base)
    - Base CPI represents the reference level at t=0
    - Inflation rates are calculated as: (CPI(t2)/CPI(t1))^(1/(t2-t1)) - 1
    - Use log-linear interpolation for smoother inflation rate curves
    - CPI curves are typically calibrated from inflation swap quotes or
      inflation-linked bond prices

    See Also
    --------
    :class:`DiscountCurve`: Discount curves for inflation-linked bond pricing
    :class:`MarketContext`: Container for inflation curves
    """

    def __init__(
        self,
        id: str,
        base_cpi: float,
        knots: List[Tuple[float, float]],
        interp: str | InterpStyle | None = None,
    ) -> None: ...
    @property
    def id(self) -> str:
        """Get the curve identifier.

        Returns
        -------
        str
            Curve ID.
        """
        ...

    @property
    def base_cpi(self) -> float:
        """Get the base CPI level.

        Returns
        -------
        float
            Base CPI level.
        """
        ...

    @property
    def points(self) -> List[Tuple[float, float]]:
        """Get the knot points.

        Returns
        -------
        List[Tuple[float, float]]
            (time, cpi_level) pairs.
        """
        ...

    def cpi(self, t: float) -> float:
        """Get the CPI level at a given time.

        Returns the interpolated CPI index value at time t. The value is
        interpolated from the curve's knot points using the configured
        interpolation method.

        Parameters
        ----------
        t : float
            Time in years from the base date. Must be >= 0.

        Returns
        -------
        float
            CPI index level (absolute value, not a ratio). For t=0, returns
            base_cpi. For times beyond the curve's range, extrapolation applies.

        Raises
        ------
        ValueError
            If t < 0 or if extrapolation fails.
        """
        ...

    def inflation_rate(self, t1: float, t2: float) -> float:
        """Get the annualized inflation rate between two times.

        Calculates the continuously compounded annualized inflation rate from
        time t1 to t2 based on the CPI levels at those times. The formula is:
        rate = (ln(CPI(t2)) - ln(CPI(t1))) / (t2 - t1).

        Parameters
        ----------
        t1 : float
            Start time in years from base date. Must be >= 0.
        t2 : float
            End time in years from base date. Must be > t1.

        Returns
        -------
        float
            Annualized inflation rate as a decimal (e.g., 0.02 for 2% inflation).
            The rate is continuously compounded.

        Raises
        ------
        ValueError
            If t1 < 0, t2 <= t1, or if CPI levels are invalid.
        """
        ...

    def __repr__(self) -> str: ...

class BaseCorrelationCurve:
    """Base correlation curve for CDO/CDS index modeling.

    Parameters
    ----------
    id : str
        Curve identifier.
    points : list[tuple[float, float]]
        (detachment, correlation) pairs.
    """

    def __init__(self, id: str, points: List[Tuple[float, float]]) -> None: ...
    @property
    def id(self) -> str:
        """Get the curve identifier.

        Returns
        -------
        str
            Curve ID.
        """
        ...

    @property
    def points(self) -> List[Tuple[float, float]]:
        """Get the knot points.

        Returns
        -------
        List[Tuple[float, float]]
            (detachment, correlation) pairs.
        """
        ...

    def correlation(self, detachment_pct: float) -> float:
        """Get correlation at detachment percentage.

        Parameters
        ----------
        detachment_pct : float
            Detachment percentage.

        Returns
        -------
        float
            Base correlation.
        """
        ...

    def __repr__(self) -> str: ...

class CreditIndexData:
    """Credit index data for CDS index modeling.

    Parameters
    ----------
    num_constituents : int
        Number of constituents in the index.
    recovery_rate : float
        Recovery rate.
    index_curve : HazardCurve
        Index hazard curve.
    base_correlation_curve : BaseCorrelationCurve
        Base correlation curve.
    issuer_curves : dict[str, HazardCurve], optional
        Individual issuer curves.
    """

    def __init__(
        self,
        num_constituents: int,
        recovery_rate: float,
        index_curve: HazardCurve,
        base_correlation_curve: BaseCorrelationCurve,
        issuer_curves: Dict[str, HazardCurve] | None = None,
    ) -> None: ...
    @property
    def num_constituents(self) -> int:
        """Get the number of constituents.

        Returns
        -------
        int
            Number of constituents.
        """
        ...

    @property
    def recovery_rate(self) -> float:
        """Get the recovery rate.

        Returns
        -------
        float
            Recovery rate.
        """
        ...

    @property
    def index_curve(self) -> HazardCurve:
        """Get the index curve.

        Returns
        -------
        HazardCurve
            Index hazard curve.
        """
        ...

    @property
    def base_correlation_curve(self) -> BaseCorrelationCurve:
        """Get the base correlation curve.

        Returns
        -------
        BaseCorrelationCurve
            Base correlation curve.
        """
        ...

    @property
    def has_issuer_curves(self) -> bool:
        """Check if issuer curves are present.

        Returns
        -------
        bool
            True if issuer curves are available.
        """
        ...

    def issuer_ids(self) -> List[str]:
        """Get issuer identifiers.

        Returns
        -------
        List[str]
            List of issuer IDs.
        """
        ...

    def issuer_curve(self, issuer_id: str) -> HazardCurve | None:
        """Get an issuer curve.

        Parameters
        ----------
        issuer_id : str
            Issuer identifier.

        Returns
        -------
        HazardCurve or None
            Issuer curve if found.
        """
        ...

    def __repr__(self) -> str: ...
