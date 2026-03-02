"""Swaption instrument."""

from __future__ import annotations
from datetime import date
from ....core.money import Money
from ....core.dates.schedule import Frequency
from ....core.dates.daycount import DayCount
from ...common import InstrumentType
from ...common.parameters import VolatilityModel, CashSettlementMethod

class SwaptionSettlement:
    """Swaption settlement type."""

    PHYSICAL: SwaptionSettlement
    CASH: SwaptionSettlement
    @classmethod
    def from_name(cls, name: str) -> SwaptionSettlement: ...
    @property
    def name(self) -> str: ...

class SwaptionExercise:
    """Swaption exercise style."""

    EUROPEAN: SwaptionExercise
    BERMUDAN: SwaptionExercise
    AMERICAN: SwaptionExercise
    @classmethod
    def from_name(cls, name: str) -> SwaptionExercise: ...
    @property
    def name(self) -> str: ...

class BermudanType:
    """Bermudan swaption exercise schedule type."""

    CO_TERMINAL: BermudanType
    NON_CO_TERMINAL: BermudanType

    @classmethod
    def from_name(cls, name: str) -> BermudanType: ...
    @property
    def name(self) -> str: ...

class SABRParameters:
    """SABR stochastic-volatility model parameters."""

    def __init__(self, alpha: float, beta: float, nu: float, rho: float) -> None: ...
    @staticmethod
    def with_shift(alpha: float, beta: float, nu: float, rho: float, shift: float) -> SABRParameters: ...
    @staticmethod
    def rates_standard(alpha: float, nu: float, rho: float) -> SABRParameters: ...
    @staticmethod
    def equity_standard(alpha: float, nu: float, rho: float) -> SABRParameters: ...
    @staticmethod
    def normal(alpha: float, nu: float, rho: float) -> SABRParameters: ...
    @staticmethod
    def lognormal(alpha: float, nu: float, rho: float) -> SABRParameters: ...
    @property
    def alpha(self) -> float: ...
    @property
    def beta(self) -> float: ...
    @property
    def nu(self) -> float: ...
    @property
    def rho(self) -> float: ...
    @property
    def shift(self) -> float | None: ...

class BermudanSchedule:
    """Exercise schedule for Bermudan swaptions."""

    def __init__(self, exercise_dates: list[date]) -> None: ...
    @staticmethod
    def co_terminal(first_exercise: date, swap_end: date, fixed_freq: str | Frequency) -> BermudanSchedule: ...
    def with_lockout(self, lockout_end: date) -> BermudanSchedule: ...
    def with_notice_days(self, days: int) -> BermudanSchedule: ...
    @property
    def exercise_dates(self) -> list[date]: ...
    @property
    def effective_dates(self) -> list[date]: ...
    @property
    def num_exercises(self) -> int: ...

class Swaption:
    """Interest rate swaption for pricing options on interest rate swaps.

    Swaption represents an option to enter into an interest rate swap at a
    specified fixed rate (strike) on or before the expiry date. The underlying
    swap starts at swap_start and ends at swap_end.

    Swaptions are commonly priced using lognormal (Black-style) or normal
    (Bachelier) models, requiring discount curves, forward curves, and
    volatility surfaces. They are among the most liquid interest rate options
    and are used for hedging and speculation.

    Examples
    --------
    Create a payer swaption:

        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.money import Money
        >>> from finstack.valuations.instruments import Swaption
        >>> swaption = Swaption.payer(
        ...     "SWAPTION-5Y10Y",
        ...     notional=Money(10_000_000, Currency("USD")),
        ...     strike=0.035,
        ...     expiry=date(2024, 12, 20),
        ...     swap_start=date(2025, 1, 1),
        ...     swap_end=date(2035, 1, 1),
        ...     discount_curve="USD-OIS",
        ...     forward_curve="USD-SOFR-3M",
        ...     vol_surface="USD-SWAPTION-VOL",
        ...     exercise="european",
        ...     settlement="physical",
        ... )

    Price the swaption:

        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.market_data.context import MarketContext
        >>> from finstack.core.market_data.surfaces import VolSurface
        >>> from finstack.core.market_data.term_structures import DiscountCurve, ForwardCurve
        >>> from finstack.core.money import Money
        >>> from finstack.valuations.instruments import Swaption
        >>> from finstack.valuations.pricer import create_standard_registry
        >>> swaption = Swaption.payer(
        ...     "SWAPTION-5Y10Y",
        ...     Money(10_000_000, Currency("USD")),
        ...     0.035,
        ...     date(2025, 1, 1),
        ...     date(2025, 1, 1),
        ...     date(2035, 1, 1),
        ...     discount_curve="USD-OIS",
        ...     forward_curve="USD-SOFR-3M",
        ...     vol_surface="USD-SWAPTION-VOL",
        ...     exercise="european",
        ...     settlement="physical",
        ... )
        >>> ctx = MarketContext()
        >>> ctx.insert_discount(DiscountCurve("USD-OIS", date(2024, 1, 1), [(0.0, 1.0), (10.0, 0.93)]))
        >>> ctx.insert_forward(
        ...     ForwardCurve("USD-SOFR-3M", 0.25, [(0.0, 0.03), (10.0, 0.033)], base_date=date(2024, 1, 1))
        ... )
        >>> expiries = [1.0, 5.0]
        >>> strikes = [0.02, 0.03, 0.04]
        >>> grid = [
        ...     [0.24, 0.25, 0.26],
        ...     [0.22, 0.23, 0.24],
        ... ]
        >>> ctx.insert_surface(VolSurface("USD-SWAPTION-VOL", expiries, strikes, grid))
        >>> registry = create_standard_registry()
        >>> pv = registry.price(swaption, "discounting", ctx, as_of=date(2024, 1, 1)).value
        >>> pv.currency.code
        'USD'

    Notes
    -----
    - Swaptions require discount curve, forward curve, and volatility surface
    - Strike is the fixed rate of the underlying swap
    - Exercise style: "european" (exercise only at expiry) or "bermudan"
    - Settlement: "physical" (enter swap) or "cash" (cash settlement)
    - The underlying swap is a standard fixed-for-floating interest rate swap

    Conventions
    -----------
    - Rates are expressed as decimals (e.g., 0.035 for 3.5%).
    - Exercise is specified by ``exercise`` ("european" or "bermudan"); settlement is specified by
      ``settlement`` ("physical" or "cash").
    - Required market data is identified by string IDs (``discount_curve``, ``forward_curve``, ``vol_surface``)
      and must be present in ``MarketContext``.

    MarketContext Requirements
    -------------------------
    - Discount curve: ``discount_curve`` (required).
    - Forward curve: ``forward_curve`` (required).
    - Volatility surface: ``vol_surface`` (required).

    See Also
    --------
    :class:`InterestRateSwap`: Underlying swap instrument
    :class:`InterestRateOption`: Interest rate caps/floors
    :class:`PricerRegistry`: Pricing entry point

    Sources
    -------
    - Black (1976): see ``docs/REFERENCES.md#black1976``.
    - Bachelier (1900): see ``docs/REFERENCES.md#bachelier1900``.
    - Hull (text): see ``docs/REFERENCES.md#hullOptionsFuturesDerivatives``.
    """

    @classmethod
    def payer(
        cls,
        instrument_id: str,
        notional: Money,
        strike: float,
        expiry: date,
        swap_start: date,
        swap_end: date,
        discount_curve: str,
        forward_curve: str,
        vol_surface: str,
        exercise: str | None = "european",
        settlement: str | None = "physical",
        *,
        fixed_freq: str | Frequency | None = None,
        float_freq: str | Frequency | None = None,
        day_count: str | DayCount | None = None,
        vol_model: str | VolatilityModel | None = None,
        calendar: str | None = None,
        cash_settlement_method: str | CashSettlementMethod | None = None,
        sabr_params: SABRParameters | None = None,
        implied_volatility: float | None = None,
        attributes: dict[str, str] | None = None,
    ) -> "Swaption":
        """Create a payer swaption (option to pay fixed on underlying swap).

        A payer swaption gives the holder the right to enter into a swap where
        they pay the fixed rate (strike) and receive floating. This is equivalent
        to a call option on the fixed rate (or put on the floating rate).

        Parameters
        ----------
        instrument_id : str
            Unique identifier for the swaption (e.g., "SWAPTION-5Y10Y").
        notional : Money
            Notional principal of the underlying swap. Currency determines curve
            currency requirements.
        strike : float
            Fixed rate (strike) of the underlying swap, as a decimal (e.g., 0.035
            for 3.5%). This is the rate the holder would pay if exercised.
        expiry : date
            Swaption expiration date. The option can be exercised on or before
            this date (depending on exercise style).
        swap_start : date
            Start date of the underlying swap if exercised. Typically equals or
            follows expiry (forward-starting swap).
        swap_end : date
            End date of the underlying swap. The swap tenor is swap_end - swap_start.
        discount_curve : str
            Discount curve identifier in MarketContext for present value calculations.
        forward_curve : str
            Forward curve identifier for the floating leg of the underlying swap.
        vol_surface : str
            Volatility surface identifier for swaption pricing. The surface must
            cover the swaption's expiry and the underlying swap's tenor.
        exercise : str, optional
            Exercise style: "european" (exercise only at expiry, default) or
            "bermudan" (exercise on multiple dates). American exercise not typically
            supported for swaptions.
        settlement : str, optional
            Settlement type: "physical" (enter the swap, default) or "cash"
            (cash settlement based on swap value). Physical is standard.
        fixed_freq : str | Frequency, optional
            Fixed leg payment frequency override.
        float_freq : str | Frequency, optional
            Float leg payment frequency override.
        day_count : str | DayCount, optional
            Day count convention override.
        vol_model : str | VolatilityModel, optional
            Volatility model override (e.g., "black76", "normal").
        calendar : str, optional
            Business calendar identifier.
        cash_settlement_method : str | CashSettlementMethod, optional
            Cash settlement method when settlement is "cash".
        sabr_params : SABRParameters, optional
            SABR model parameters for smile-aware pricing.
        implied_volatility : float, optional
            Override implied volatility for pricing.
        attributes : dict[str, str], optional
            User-defined metadata.

        Returns
        -------
        Swaption
            Configured payer swaption ready for pricing.

        Raises
        ------
        ValueError
            If dates are invalid (swap_end <= swap_start, expiry > swap_start),
            if strike is negative, or if notional is invalid.

        Examples
        --------
            >>> from finstack import Money, Currency
            >>> from datetime import date
            >>> swaption = Swaption.payer(
            ...     "SWAPTION-5Y10Y",
            ...     Money(10_000_000, Currency("USD")),
            ...     0.035,  # 3.5% strike
            ...     date(2024, 12, 20),  # Expiry in 1 year
            ...     date(2025, 1, 1),  # Swap starts in 1 year
            ...     date(2035, 1, 1),  # Swap ends in 11 years (10Y swap)
            ...     discount_curve="USD",
            ...     forward_curve="USD-LIBOR-3M",
            ...     vol_surface="USD-SWAPTION-VOL",
            ... )
            >>> swaption.option_type
            'payer'
        """
        ...

    @classmethod
    def receiver(
        cls,
        instrument_id: str,
        notional: Money,
        strike: float,
        expiry: date,
        swap_start: date,
        swap_end: date,
        discount_curve: str,
        forward_curve: str,
        vol_surface: str,
        exercise: str | None = "european",
        settlement: str | None = "physical",
        *,
        fixed_freq: str | Frequency | None = None,
        float_freq: str | Frequency | None = None,
        day_count: str | DayCount | None = None,
        vol_model: str | VolatilityModel | None = None,
        calendar: str | None = None,
        cash_settlement_method: str | CashSettlementMethod | None = None,
        sabr_params: SABRParameters | None = None,
        implied_volatility: float | None = None,
        attributes: dict[str, str] | None = None,
    ) -> "Swaption":
        """Create a receiver swaption (option to receive fixed on underlying swap).

        A receiver swaption gives the holder the right to enter into a swap where
        they receive the fixed rate (strike) and pay floating. This is equivalent
        to a put option on the fixed rate (or call on the floating rate).

        Parameters
        ----------
        instrument_id : str
            Unique identifier for the swaption.
        notional : Money
            Notional principal of the underlying swap.
        strike : float
            Fixed rate (strike) of the underlying swap, as a decimal. This is the
            rate the holder would receive if exercised.
        expiry : date
            Swaption expiration date.
        swap_start : date
            Start date of the underlying swap if exercised.
        swap_end : date
            End date of the underlying swap.
        discount_curve : str
            Discount curve identifier in MarketContext.
        forward_curve : str
            Forward curve identifier for the floating leg.
        vol_surface : str
            Volatility surface identifier for swaption pricing.
        exercise : str, optional
            Exercise style: "european" (default) or "bermudan".
        settlement : str, optional
            Settlement type: "physical" (default) or "cash".
        fixed_freq : str | Frequency, optional
            Fixed leg payment frequency override.
        float_freq : str | Frequency, optional
            Float leg payment frequency override.
        day_count : str | DayCount, optional
            Day count convention override.
        vol_model : str | VolatilityModel, optional
            Volatility model override.
        calendar : str, optional
            Business calendar identifier.
        cash_settlement_method : str | CashSettlementMethod, optional
            Cash settlement method when settlement is "cash".
        sabr_params : SABRParameters, optional
            SABR model parameters for smile-aware pricing.
        implied_volatility : float, optional
            Override implied volatility for pricing.
        attributes : dict[str, str], optional
            User-defined metadata.

        Returns
        -------
        Swaption
            Configured receiver swaption ready for pricing.

        Raises
        ------
        ValueError
            If parameters are invalid.

        Examples
        --------
            >>> swaption = Swaption.receiver(
            ...     "SWAPTION-REC-5Y10Y",
            ...     Money(10_000_000, Currency("USD")),
            ...     0.035,
            ...     date(2024, 12, 20),
            ...     date(2025, 1, 1),
            ...     date(2035, 1, 1),
            ...     discount_curve="USD",
            ...     forward_curve="USD-LIBOR-3M",
            ...     vol_surface="USD-SWAPTION-VOL",
            ... )
        """
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def strike(self) -> float: ...
    @property
    def expiry(self) -> date: ...
    @property
    def swap_start(self) -> date: ...
    @property
    def swap_end(self) -> date: ...
    @property
    def option_type(self) -> str: ...
    @property
    def settlement(self) -> str: ...
    @property
    def exercise(self) -> str: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def forward_curve(self) -> str: ...
    @property
    def vol_surface(self) -> str: ...
    @property
    def fixed_freq(self) -> str: ...
    @property
    def float_freq(self) -> str: ...
    @property
    def day_count(self) -> DayCount: ...
    @property
    def vol_model(self) -> VolatilityModel: ...
    @property
    def calendar(self) -> str | None: ...
    @property
    def cash_settlement_method(self) -> CashSettlementMethod: ...
    @property
    def sabr_params(self) -> SABRParameters | None: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class BermudanSwaption:
    """Bermudan swaption with multiple exercise dates.

    A Bermudan swaption gives the holder the right to exercise on any of
    a set of discrete dates, entering into an underlying interest rate swap.
    """

    @classmethod
    def payer(
        cls,
        instrument_id: str,
        notional: Money,
        strike: float,
        swap_start: date,
        swap_end: date,
        schedule: BermudanSchedule,
        discount_curve: str,
        forward_curve: str,
        vol_surface: str,
        *,
        fixed_freq: str | Frequency | None = None,
        float_freq: str | Frequency | None = None,
        day_count: str | DayCount | None = None,
        settlement: str | None = None,
        bermudan_type: str | BermudanType | None = None,
        calendar: str | None = None,
        implied_volatility: float | None = None,
        attributes: dict[str, str] | None = None,
    ) -> BermudanSwaption: ...
    @classmethod
    def receiver(
        cls,
        instrument_id: str,
        notional: Money,
        strike: float,
        swap_start: date,
        swap_end: date,
        schedule: BermudanSchedule,
        discount_curve: str,
        forward_curve: str,
        vol_surface: str,
        *,
        fixed_freq: str | Frequency | None = None,
        float_freq: str | Frequency | None = None,
        day_count: str | DayCount | None = None,
        settlement: str | None = None,
        bermudan_type: str | BermudanType | None = None,
        calendar: str | None = None,
        implied_volatility: float | None = None,
        attributes: dict[str, str] | None = None,
    ) -> BermudanSwaption: ...
    @property
    def instrument_id(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def strike(self) -> float: ...
    @property
    def swap_start(self) -> date: ...
    @property
    def swap_end(self) -> date: ...
    @property
    def option_type(self) -> str: ...
    @property
    def settlement(self) -> str: ...
    @property
    def bermudan_type(self) -> BermudanType: ...
    @property
    def exercise_dates(self) -> list[date]: ...
    @property
    def first_exercise(self) -> date | None: ...
    @property
    def last_exercise(self) -> date | None: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def forward_curve(self) -> str: ...
    @property
    def vol_surface(self) -> str: ...
    @property
    def fixed_freq(self) -> str: ...
    @property
    def float_freq(self) -> str: ...
    @property
    def day_count(self) -> DayCount: ...
    @property
    def calendar(self) -> str | None: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
