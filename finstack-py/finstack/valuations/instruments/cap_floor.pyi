"""Interest rate option (cap/floor) instrument (builder-only API)."""

from typing import Optional, Union
from datetime import date
from ...core.currency import Currency
from ...core.money import Money
from ...core.dates.daycount import DayCount
from ..common import InstrumentType

class InterestRateOptionBuilder:
    """Fluent builder returned by :meth:`InterestRateOption.builder`."""

    def __init__(self, instrument_id: str) -> None: ...
    def kind(self, kind: str) -> InterestRateOptionBuilder: ...
    def notional(self, amount: float) -> InterestRateOptionBuilder: ...
    def currency(self, currency: Union[str, Currency]) -> InterestRateOptionBuilder: ...
    def money(self, money: Money) -> InterestRateOptionBuilder: ...
    def strike(self, strike: float) -> InterestRateOptionBuilder: ...
    def start_date(self, start_date: date) -> InterestRateOptionBuilder: ...
    def end_date(self, end_date: date) -> InterestRateOptionBuilder: ...
    def disc_id(self, curve_id: str) -> InterestRateOptionBuilder: ...
    def fwd_id(self, curve_id: str) -> InterestRateOptionBuilder: ...
    def vol_surface(self, vol_surface: str) -> InterestRateOptionBuilder: ...
    def payments_per_year(self, payments_per_year: int) -> InterestRateOptionBuilder: ...
    def day_count(self, day_count: Union[DayCount, str]) -> InterestRateOptionBuilder: ...
    def build(self) -> "InterestRateOption": ...

class InterestRateOption:
    """Interest rate cap and floor instruments for rate protection.

    InterestRateOption represents a cap (protection against rising rates) or
    floor (protection against falling rates) on a floating interest rate.
    Caps/floors are portfolios of caplets/floorlets, each providing protection
    for a single reset period.

    Caps and floors are priced using Black's model, requiring discount curves,
    forward curves, and volatility surfaces. They are commonly used to hedge
    floating-rate exposures or create structured products.

    Examples
    --------
    Create an interest rate cap:

        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.money import Money
        >>> from finstack.valuations.instruments import InterestRateOption
        >>> cap = (
        ...     InterestRateOption
        ...     .builder("CAP-5Y")
        ...     .kind("cap")
        ...     .money(Money(10_000_000, Currency("USD")))
        ...     .strike(0.03)
        ...     .start_date(date(2024, 1, 1))
        ...     .end_date(date(2029, 1, 1))
        ...     .disc_id("USD-OIS")
        ...     .fwd_id("USD-SOFR-3M")
        ...     .vol_surface("USD-CAP-VOL")
        ...     .build()
        ... )

    Price the cap:

        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.market_data.context import MarketContext
        >>> from finstack.core.market_data.surfaces import VolSurface
        >>> from finstack.core.market_data.term_structures import DiscountCurve, ForwardCurve
        >>> from finstack.core.money import Money
        >>> from finstack.valuations.instruments import InterestRateOption
        >>> from finstack.valuations.pricer import create_standard_registry
        >>> cap = (
        ...     InterestRateOption
        ...     .builder("CAP-5Y")
        ...     .kind("cap")
        ...     .money(Money(5_000_000, Currency("USD")))
        ...     .strike(0.03)
        ...     .start_date(date(2024, 1, 1))
        ...     .end_date(date(2029, 1, 1))
        ...     .disc_id("USD-OIS")
        ...     .fwd_id("USD-SOFR-3M")
        ...     .vol_surface("USD-CAP-VOL")
        ...     .build()
        ... )
        >>> ctx = MarketContext()
        >>> ctx.insert_discount(DiscountCurve("USD-OIS", date(2024, 1, 1), [(0.0, 1.0), (5.0, 0.95)]))
        >>> ctx.insert_forward(
        ...     ForwardCurve("USD-SOFR-3M", 0.25, [(0.0, 0.03), (5.0, 0.032)], base_date=date(2024, 1, 1))
        ... )
        >>> expiries = [1.0, 3.0, 5.0]
        >>> strikes = [0.02, 0.03, 0.04]
        >>> grid = [
        ...     [0.22, 0.21, 0.23],
        ...     [0.21, 0.20, 0.22],
        ...     [0.20, 0.19, 0.21],
        ... ]
        >>> ctx.insert_surface(VolSurface("USD-CAP-VOL", expiries, strikes, grid))
        >>> registry = create_standard_registry()
        >>> pv = registry.price(cap, "black76", ctx).value
        >>> pv.currency.code
        'USD'

    Notes
    -----
    - Caps/floors require discount curve, forward curve, and volatility surface
    - Strike is the cap/floor rate (e.g., 3% = 0.03)
    - Payment frequency determines the number of caplets/floorlets
    - Each caplet/floorlet protects one reset period
    - Caps provide protection when rates rise above strike
    - Floors provide protection when rates fall below strike

    Conventions
    -----------
    - Rates are expressed as decimals (e.g., 0.03 for 3%).
    - ``strike`` is a rate level (not bps). Volatilities in surfaces are expected as decimals.
    - Reset/payment schedule is controlled by ``payments_per_year``; if ``day_count`` is omitted, the
      runtime defaults apply (docstring describes the intended convention but the exact default is set in Rust).
    - Required market data is identified by string IDs (``discount_curve``, ``forward_curve``, ``vol_surface``)
      and must be present in ``MarketContext``.

    MarketContext Requirements
    -------------------------
    - Discount curve: ``discount_curve`` (required).
    - Forward curve: ``forward_curve`` (required).
    - Volatility surface: ``vol_surface`` (required).

    See Also
    --------
    :class:`Swaption`: Interest rate swaptions
    :class:`InterestRateSwap`: Underlying floating-rate swaps
    :class:`PricerRegistry`: Pricing entry point

    Sources
    -------
    - Black (1976): see ``docs/REFERENCES.md#black1976``.
    - Hull (text): see ``docs/REFERENCES.md#hullOptionsFuturesDerivatives``.
    """

    @classmethod
    def builder(cls, instrument_id: str) -> InterestRateOptionBuilder:
        """Start a fluent builder (builder-only API)."""
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def strike(self) -> float: ...
    @property
    def start_date(self) -> date: ...
    @property
    def end_date(self) -> date: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def forward_curve(self) -> str: ...
    @property
    def vol_surface(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
