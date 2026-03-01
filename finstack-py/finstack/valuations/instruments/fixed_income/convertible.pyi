"""Convertible bond instrument."""

from __future__ import annotations
import datetime
from typing import List, Tuple
from ....core.money import Money
from ....core.market_data.context import MarketContext
from ...common import InstrumentType
from ...cashflow.builder import FixedCouponSpec, FloatingCouponSpec

class ConversionEvent:
    """Convertible conversion event wrapper."""

    QUALIFIED_IPO: "ConversionEvent"
    CHANGE_OF_CONTROL: "ConversionEvent"
    @classmethod
    def price_trigger(cls, threshold: float, lookback_days: int) -> "ConversionEvent": ...

class ConversionPolicy:
    """Convertible conversion policy wrapper."""
    @classmethod
    def voluntary(cls) -> "ConversionPolicy": ...
    @classmethod
    def mandatory_on(cls, conversion_date: datetime.date) -> "ConversionPolicy": ...
    @classmethod
    def window(cls, start: datetime.date, end: datetime.date) -> "ConversionPolicy": ...
    @classmethod
    def upon_event(cls, event: ConversionEvent) -> "ConversionPolicy": ...

class AntiDilutionPolicy:
    """Anti-dilution policy wrapper."""

    NONE: "AntiDilutionPolicy"
    FULL_RATCHET: "AntiDilutionPolicy"
    WEIGHTED_AVERAGE: "AntiDilutionPolicy"

class DividendAdjustment:
    """Dividend adjustment policy wrapper."""

    NONE: "DividendAdjustment"
    ADJUST_PRICE: "DividendAdjustment"
    ADJUST_RATIO: "DividendAdjustment"

class ConversionSpec:
    """Convertible conversion specification."""
    def __init__(
        self,
        policy: ConversionPolicy,
        *,
        ratio: float | None = None,
        price: float | None = None,
        anti_dilution: AntiDilutionPolicy | None = None,
        dividend_adjustment: DividendAdjustment | None = None,
        dilution_events: list[DilutionEvent] | None = None,
    ) -> None: ...
    @property
    def ratio(self) -> float | None: ...
    @property
    def price(self) -> float | None: ...
    @property
    def policy(self) -> str: ...

class ConvertibleGreeks:
    """Greeks for a convertible bond."""
    @property
    def price(self) -> float: ...
    @property
    def delta(self) -> float: ...
    @property
    def gamma(self) -> float: ...
    @property
    def vega(self) -> float: ...
    @property
    def theta(self) -> float: ...
    @property
    def rho(self) -> float: ...
    def __repr__(self) -> str: ...

class ConvertibleTreeType:
    """Tree type for convertible bond valuation."""
    @classmethod
    def binomial(cls, steps: int = 100) -> "ConvertibleTreeType": ...
    @classmethod
    def trinomial(cls, steps: int = 100) -> "ConvertibleTreeType": ...
    def __repr__(self) -> str: ...

class SoftCallTrigger:
    """Soft-call trigger condition for convertible bonds."""
    def __init__(self, threshold_pct: float, observation_days: int, required_days_above: int) -> None: ...
    @property
    def threshold_pct(self) -> float: ...
    @property
    def observation_days(self) -> int: ...
    @property
    def required_days_above(self) -> int: ...
    def __repr__(self) -> str: ...

class DilutionEvent:
    """Dilution event for anti-dilution adjustment."""
    def __init__(
        self, date: datetime.date, new_issue_price: float, new_shares_issued: float, shares_outstanding_before: float
    ) -> None: ...
    @property
    def date(self) -> datetime.date: ...
    @property
    def new_issue_price(self) -> float: ...
    @property
    def new_shares_issued(self) -> float: ...
    @property
    def shares_outstanding_before(self) -> float: ...
    def __repr__(self) -> str: ...

class ConvertibleBondBuilder:
    """Fluent builder returned by :meth:`ConvertibleBond.builder`."""

    def __init__(self, instrument_id: str) -> None: ...
    def notional(self, notional: Money) -> "ConvertibleBondBuilder": ...
    def issue(self, issue: datetime.date) -> "ConvertibleBondBuilder": ...
    def maturity(self, maturity: datetime.date) -> "ConvertibleBondBuilder": ...
    def discount_curve(self, discount_curve: str) -> "ConvertibleBondBuilder": ...
    def conversion(self, conversion: ConversionSpec) -> "ConvertibleBondBuilder": ...
    def underlying_equity_id(self, underlying_equity_id: str | None = ...) -> "ConvertibleBondBuilder": ...
    def call_schedule(
        self, call_schedule: List[Tuple[datetime.date, float]] | None = ...
    ) -> "ConvertibleBondBuilder": ...
    def put_schedule(
        self, put_schedule: List[Tuple[datetime.date, float]] | None = ...
    ) -> "ConvertibleBondBuilder": ...
    def fixed_coupon(self, fixed_coupon: FixedCouponSpec) -> "ConvertibleBondBuilder": ...
    def floating_coupon(self, floating_coupon: FloatingCouponSpec) -> "ConvertibleBondBuilder": ...
    def soft_call_trigger(self, trigger: SoftCallTrigger) -> "ConvertibleBondBuilder": ...
    def build(self) -> "ConvertibleBond": ...

class ConvertibleBond:
    """Convertible bond with equity conversion option.

    ConvertibleBond represents a bond that can be converted into equity shares
    at the holder's option. Convertibles combine fixed-income characteristics
    with equity optionality, requiring both bond and equity pricing models.

    Convertible bonds are used for financing with equity upside, hedging
    equity exposure, and creating hybrid instruments. They require discount
    curves, equity prices, and volatility surfaces.

    Examples
    --------
    Create a convertible bond:

        >>> from finstack.valuations.instruments import ConvertibleBond, ConversionSpec, ConversionPolicy
        >>> from finstack import Money, Currency
        >>> from datetime import date
        >>> conversion = ConversionSpec(
        ...     ConversionPolicy.voluntary(),  # Holder can convert anytime
        ...     ratio=20.0,  # 20 shares per $1000 bond
        ... )
        >>> convertible = (
        ...     ConvertibleBond
        ...     .builder("CONVERTIBLE-CORP-A")
        ...     .notional(Money(10_000_000, Currency("USD")))
        ...     .issue(date(2024, 1, 1))
        ...     .maturity(date(2029, 1, 1))
        ...     .discount_curve("USD")
        ...     .conversion(conversion)
        ...     .underlying_equity_id("CORP-A")
        ...     .build()
        ... )

    Notes
    -----
    - Convertibles require discount curve, equity price, and volatility surface
    - Conversion can be voluntary, mandatory, or triggered by events
    - Call/put schedules affect optionality and conversion behavior
    - Conversion ratio determines shares per bond
    - Anti-dilution and dividend adjustments protect conversion value

    MarketContext Requirements
    -------------------------
    - Discount curve: ``discount_curve`` (required).
    - Underlying equity spot: ``underlying_equity_id`` (required by equity-linked pricing paths when provided/used).
    - Volatility surface: required by equity-linked pricing paths when used by the selected pricer/model.

    See Also
    --------
    :class:`Bond`: Standard bonds
    :class:`EquityOption`: Equity options
    :class:`PricerRegistry`: Pricing entry point

    Sources
    -------
    - Hull (text): see ``docs/REFERENCES.md#hullOptionsFuturesDerivatives``.
    - Black & Scholes (1973): see ``docs/REFERENCES.md#blackScholes1973``.
    """

    @classmethod
    def builder(cls, instrument_id: str) -> ConvertibleBondBuilder:
        """Start a fluent builder (builder-only API).

        Parameters
        ----------
        instrument_id : str
            Unique identifier for the convertible (e.g., "CONVERTIBLE-CORP-A").
        notional : Money
            Bond principal amount. Currency determines curve currency.
        issue : date
            Bond issue date.
        maturity : date
            Bond maturity date. Must be after issue.
        discount_curve : str
            Discount curve identifier in MarketContext.
        conversion : ConversionSpec
            Conversion specification (policy, ratio, anti-dilution, etc.).
        underlying_equity_id : str, optional
            Underlying equity identifier in MarketContext (default: uses ticker from conversion).
        call_schedule : List[Tuple[date, float]], optional
            Call schedule: list of (date, call_price) tuples. Issuer can call
            the bond at these dates/prices.
        put_schedule : List[Tuple[date, float]], optional
            Put schedule: list of (date, put_price) tuples. Holder can put
            the bond back at these dates/prices.
        fixed_coupon : FixedCouponSpec, optional
            Fixed coupon specification (rate, frequency, day count).
        floating_coupon : FloatingCouponSpec, optional
            Floating coupon specification (forward curve, margin, etc.).
            Either fixed_coupon or floating_coupon must be provided.

        Returns
        -------
        ConvertibleBond
            Configured convertible bond ready for pricing.

        Raises
        ------
        ValueError
            If parameters are invalid (maturity <= issue, no coupon spec, etc.)
            or if required market data is missing.

        """
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    @property
    def conversion_ratio(self) -> float | None: ...
    @property
    def conversion_price(self) -> float | None: ...
    @property
    def conversion_policy(self) -> str: ...
    @property
    def issue(self) -> datetime.date: ...
    @property
    def maturity(self) -> datetime.date: ...
    def greeks(
        self,
        market: MarketContext,
        as_of: datetime.date,
        tree_type: ConvertibleTreeType | None = None,
        bump_size: float | None = None,
    ) -> ConvertibleGreeks:
        """Calculate all Greeks for this convertible bond."""
        ...
    def delta(self, market: MarketContext, as_of: datetime.date) -> float:
        """Calculate delta (equity spot sensitivity)."""
        ...
    def gamma(self, market: MarketContext, as_of: datetime.date) -> float:
        """Calculate gamma (second derivative w.r.t. spot)."""
        ...
    def vega(self, market: MarketContext, as_of: datetime.date) -> float:
        """Calculate vega (volatility sensitivity)."""
        ...
    def rho(self, market: MarketContext, as_of: datetime.date) -> float:
        """Calculate rho (interest rate sensitivity)."""
        ...
    def theta(self, market: MarketContext, as_of: datetime.date) -> float:
        """Calculate theta (time decay per day)."""
        ...
    @property
    def effective_conversion_ratio(self) -> float | None:
        """Effective conversion ratio adjusted for anti-dilution events."""
        ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
