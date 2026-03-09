"""Inflation linked bond instrument."""

from __future__ import annotations
from datetime import date
from ....core.money import Money
from ....core.dates.daycount import DayCount
from ...common import InstrumentType

class InflationLinkedBondBuilder:
    """Fluent builder returned by :meth:`InflationLinkedBond.builder`."""

    def __init__(self, instrument_id: str) -> None: ...
    def notional(self, notional: Money) -> "InflationLinkedBondBuilder": ...
    def real_coupon(self, real_coupon: float) -> "InflationLinkedBondBuilder": ...
    def issue(self, issue: date) -> "InflationLinkedBondBuilder": ...
    def maturity(self, maturity: date) -> "InflationLinkedBondBuilder": ...
    def base_index(self, base_index: float) -> "InflationLinkedBondBuilder": ...
    def discount_curve(self, discount_curve: str) -> "InflationLinkedBondBuilder": ...
    def inflation_curve(self, inflation_curve: str) -> "InflationLinkedBondBuilder": ...
    def indexation(self, indexation: str) -> "InflationLinkedBondBuilder": ...
    def frequency(self, frequency: str) -> "InflationLinkedBondBuilder": ...
    def day_count(self, day_count: DayCount) -> "InflationLinkedBondBuilder": ...
    def deflation_protection(self, deflation_protection: str) -> "InflationLinkedBondBuilder": ...
    def calendar(self, calendar: str | None = ...) -> "InflationLinkedBondBuilder": ...
    def quoted_clean_price(self, price: float | None = ...) -> "InflationLinkedBondBuilder": ...
    def build(self) -> "InflationLinkedBond": ...

class InflationLinkedBond:
    """Inflation-linked bond for real return protection.

    InflationLinkedBond represents a bond whose principal and/or coupon
    payments are adjusted for inflation. The bond pays a real coupon rate
    (adjusted for inflation) and the principal is indexed to an inflation
    measure (e.g., CPI).

    Inflation-linked bonds are used to protect against inflation risk and
    provide real returns. They require both a discount curve (for time value)
    and an inflation curve (for CPI projections).

    Examples
    --------
    Create a TIPS-style inflation-linked bond:

        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.money import Money
        >>> from finstack.valuations.instruments import InflationLinkedBond
        >>> bond = (
        ...     InflationLinkedBond
        ...     .builder("TIPS-2030")
        ...     .notional(Money(1_000_000, Currency("USD")))
        ...     .real_coupon(0.02)
        ...     .issue(date(2024, 1, 1))
        ...     .maturity(date(2030, 1, 1))
        ...     .base_index(300.0)
        ...     .discount_curve("USD-OIS")
        ...     .inflation_curve("US-CPI")
        ...     .indexation("tips")
        ...     .deflation_protection("maturity_only")
        ...     .build()
        ... )

    Price the bond:

        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.market_data.context import MarketContext
        >>> from finstack.core.market_data.term_structures import DiscountCurve, InflationCurve
        >>> from finstack.core.money import Money
        >>> from finstack.valuations.instruments import InflationLinkedBond
        >>> from finstack.valuations.pricer import create_standard_registry
        >>> bond = (
        ...     InflationLinkedBond
        ...     .builder("TIPS-2030")
        ...     .notional(Money(1_000_000, Currency("USD")))
        ...     .real_coupon(0.02)
        ...     .issue(date(2024, 1, 1))
        ...     .maturity(date(2030, 1, 1))
        ...     .base_index(300.0)
        ...     .discount_curve("USD-OIS")
        ...     .inflation_curve("US-CPI")
        ...     .build()
        ... )
        >>> ctx = MarketContext()
        >>> ctx.insert(DiscountCurve("USD-OIS", date(2024, 1, 1), [(0.0, 1.0), (6.0, 0.92)]))
        >>> ctx.insert(InflationCurve("US-CPI", date(2024, 1, 1), 300.0, [(1.0, 304.5), (6.0, 330.0)]))
        >>> registry = create_standard_registry()
        >>> pv = registry.price(bond, "discounting", ctx).value
        >>> pv.currency.code
        'USD'

    Notes
    -----
    - Requires discount curve and inflation curve
    - Real coupon is the coupon rate before inflation adjustment
    - Base index is the CPI level at issue date
    - Indexation style: "tips" (US TIPS), "uk" (UK index-linked gilts)
    - Deflation protection ensures principal doesn't fall below par
    - Coupon payments are adjusted by inflation between payment dates

    MarketContext Requirements
    -------------------------
    - Discount curve: ``discount_curve`` (required).
    - Inflation curve: ``inflation_curve`` (required).

    See Also
    --------
    :class:`Bond`: Standard fixed-rate bonds
    :class:`InflationCurve`: CPI curves for inflation projections
    :class:`PricerRegistry`: Pricing entry point

    Sources
    -------
    - Brigo & Mercurio (2006): see ``docs/REFERENCES.md#brigoMercurio2006``.
    - Hull (text): see ``docs/REFERENCES.md#hullOptionsFuturesDerivatives``.
    """

    @classmethod
    def builder(cls, instrument_id: str) -> InflationLinkedBondBuilder:
        """Start a fluent builder (builder-only API).

        Parameters
        ----------
        instrument_id : str
            Unique identifier for the bond (e.g., "TIPS-2030", "ILG-2035").
        notional : Money
            Principal amount at issue. The currency determines curve currency
            requirements.
        real_coupon : float
            Real coupon rate as a decimal (e.g., 0.02 for 2%). This is the coupon
            rate before inflation adjustment. Actual coupon = real_coupon * inflation_factor.
        issue : date
            Bond issue date (first accrual date).
        maturity : date
            Bond maturity date when principal is repaid. Must be after issue.
        base_index : float
            Base CPI index level at issue date (e.g., 300.0). Used to calculate
            inflation adjustments throughout the bond's life.
        discount_curve : str
            Discount curve identifier in MarketContext for present value calculations.
        inflation_curve : str
            Inflation (CPI) curve identifier in MarketContext for inflation projections.
        indexation : str, optional
            Indexation style: "tips" (US TIPS, default), "uk" (UK index-linked gilts).
            Determines how inflation is applied to principal and coupons.
        frequency : str, optional
            Coupon payment frequency: "semi_annual" (default), "annual", "quarterly".
        day_count : DayCount, optional
            Day-count convention for accrual calculations. Defaults to ACT/365.25
            for TIPS-style bonds.
        deflation_protection : str, optional
            Deflation protection level: "maturity_only" (default, principal protected
            at maturity), "all_payments" (all payments protected), "none" (no protection).
        calendar : str, optional
            Holiday calendar identifier for payment date adjustments (e.g., "USNY").

        Returns
        -------
        InflationLinkedBond
            Configured inflation-linked bond ready for pricing.

        Raises
        ------
        ValueError
            If dates are invalid (maturity <= issue), if real_coupon is negative,
            if base_index <= 0, or if notional is invalid.

        """
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def real_coupon(self) -> float: ...
    @property
    def maturity(self) -> date: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def inflation_curve(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
