"""Interest rate option (cap/floor) instrument."""

from typing import Optional
from datetime import date
from ...core.money import Money
from ...core.dates.daycount import DayCount
from ..common import InstrumentType

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
        >>> cap = InterestRateOption.cap(
        ...     "CAP-5Y",
        ...     notional=Money(10_000_000, Currency("USD")),
        ...     strike=0.03,
        ...     start_date=date(2024, 1, 1),
        ...     end_date=date(2029, 1, 1),
        ...     discount_curve="USD-OIS",
        ...     forward_curve="USD-SOFR-3M",
        ...     vol_surface="USD-CAP-VOL",
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
        >>> cap = InterestRateOption.cap(
        ...     "CAP-5Y",
        ...     Money(5_000_000, Currency("USD")),
        ...     0.03,
        ...     date(2024, 1, 1),
        ...     date(2029, 1, 1),
        ...     discount_curve="USD-OIS",
        ...     forward_curve="USD-SOFR-3M",
        ...     vol_surface="USD-CAP-VOL",
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

    See Also
    --------
    :class:`Swaption`: Interest rate swaptions
    :class:`InterestRateSwap`: Underlying floating-rate swaps
    :class:`PricerRegistry`: Pricing entry point
    """

    @classmethod
    def cap(
        cls,
        instrument_id: str,
        notional: Money,
        strike: float,
        start_date: date,
        end_date: date,
        discount_curve: str,
        forward_curve: str,
        vol_surface: str,
        *,
        payments_per_year: int = 4,
        day_count: Optional[DayCount] = None,
    ) -> "InterestRateOption": ...
    """Create a standard interest-rate cap.

    A cap provides protection against rising interest rates. It pays the
    holder when the floating rate exceeds the strike rate. The cap consists
    of multiple caplets, one for each reset period between start_date and
    end_date.

    Parameters
    ----------
    instrument_id : str
        Unique identifier for the cap (e.g., "CAP-5Y-3PCT").
    notional : Money
        Notional principal amount. The currency determines curve currency
        requirements.
    strike : float
        Cap rate (strike) as a decimal (e.g., 0.03 for 3%). The cap pays
        when the floating rate exceeds this level.
    start_date : date
        First reset date of the cap (first caplet start).
    end_date : date
        Last reset date of the cap (last caplet end). Must be after start_date.
    discount_curve : str
        Discount curve identifier in MarketContext for present value calculations.
    forward_curve : str
        Forward curve identifier for projecting floating rates.
    vol_surface : str
        Volatility surface identifier for caplet pricing. The surface must
        cover the cap's reset periods.
    payments_per_year : int, optional
        Number of reset periods per year (default: 4 for quarterly). Common
        values: 2 (semi-annual), 4 (quarterly), 12 (monthly).
    day_count : DayCount, optional
        Day-count convention for reset periods. Defaults to ACT/360 for
        most floating-rate conventions.

    Returns
    -------
    InterestRateOption
        Configured interest rate cap ready for pricing.

    Raises
    ------
    ValueError
        If dates are invalid (end_date <= start_date), if strike is negative,
        if payments_per_year is <= 0, or if notional is invalid.

    Examples
    --------
        >>> from finstack import Money, Currency
        >>> from datetime import date
        >>> 
        >>> cap = InterestRateOption.cap(
        ...     "CAP-5Y",
        ...     Money(10_000_000, Currency("USD")),
        ...     0.03,  # 3% cap
        ...     date(2024, 1, 1),
        ...     date(2029, 1, 1),  # 5-year cap
        ...     discount_curve="USD",
        ...     forward_curve="USD-LIBOR-3M",
        ...     vol_surface="USD-CAP-VOL",
        ...     payments_per_year=4  # Quarterly resets
        ... )
        >>> cap.strike
        0.03
    """

    @classmethod
    def floor(
        cls,
        instrument_id: str,
        notional: Money,
        strike: float,
        start_date: date,
        end_date: date,
        discount_curve: str,
        forward_curve: str,
        vol_surface: str,
        *,
        payments_per_year: int = 4,
        day_count: Optional[DayCount] = None,
    ) -> "InterestRateOption": ...
    """Create a standard interest-rate floor.

    A floor provides protection against falling interest rates. It pays the
    holder when the floating rate falls below the strike rate. The floor
    consists of multiple floorlets, one for each reset period.

    Parameters
    ----------
    instrument_id : str
        Unique identifier for the floor (e.g., "FLOOR-5Y-2PCT").
    notional : Money
        Notional principal amount.
    strike : float
        Floor rate (strike) as a decimal (e.g., 0.02 for 2%). The floor pays
        when the floating rate falls below this level.
    start_date : date
        First reset date of the floor (first floorlet start).
    end_date : date
        Last reset date of the floor (last floorlet end). Must be after start_date.
    discount_curve : str
        Discount curve identifier in MarketContext.
    forward_curve : str
        Forward curve identifier for projecting floating rates.
    vol_surface : str
        Volatility surface identifier for floorlet pricing.
    payments_per_year : int, optional
        Number of reset periods per year (default: 4 for quarterly).
    day_count : DayCount, optional
        Day-count convention for reset periods. Defaults to ACT/360.

    Returns
    -------
    InterestRateOption
        Configured interest rate floor ready for pricing.

    Raises
    ------
    ValueError
        If parameters are invalid.

    Examples
    --------
        >>> floor = InterestRateOption.floor(
        ...     "FLOOR-5Y",
        ...     Money(10_000_000, Currency("USD")),
        ...     0.02,  # 2% floor
        ...     date(2024, 1, 1),
        ...     date(2029, 1, 1),
        ...     discount_curve="USD",
        ...     forward_curve="USD-LIBOR-3M",
        ...     vol_surface="USD-FLOOR-VOL"
        ... )
    """

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
