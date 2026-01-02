"""Convertible bond instrument."""

from typing import Optional, List, Tuple, Union
from datetime import date
from ...core.money import Money
from ..common import InstrumentType
from ..cashflow.builder import FixedCouponSpec, FloatingCouponSpec

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
    def mandatory_on(cls, conversion_date: date) -> "ConversionPolicy": ...
    @classmethod
    def window(cls, start: date, end: date) -> "ConversionPolicy": ...
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
    @classmethod
    def create(
        cls,
        policy: ConversionPolicy,
        *,
        ratio: Optional[float] = None,
        price: Optional[float] = None,
        anti_dilution: Optional[AntiDilutionPolicy] = None,
        dividend_adjustment: Optional[DividendAdjustment] = None,
    ) -> "ConversionSpec": ...
    @property
    def ratio(self) -> Optional[float]: ...
    @property
    def price(self) -> Optional[float]: ...
    @property
    def policy(self) -> str: ...

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
        >>> conversion = ConversionSpec.create(
        ...     ConversionPolicy.voluntary(),  # Holder can convert anytime (positional)
        ...     ratio=20.0,  # 20 shares per $1000 bond
        ... )
        >>> convertible = ConvertibleBond.create(
        ...     "CONVERTIBLE-CORP-A",
        ...     Money(10_000_000, Currency("USD")),  # notional (positional)
        ...     date(2024, 1, 1),  # issue (positional)
        ...     date(2029, 1, 1),  # maturity (positional)
        ...     "USD",  # discount_curve (positional)
        ...     conversion,  # conversion (positional)
        ...     underlying_equity_id="CORP-A",
        ... )

    Notes
    -----
    - Convertibles require discount curve, equity price, and volatility surface
    - Conversion can be voluntary, mandatory, or triggered by events
    - Call/put schedules affect optionality and conversion behavior
    - Conversion ratio determines shares per bond
    - Anti-dilution and dividend adjustments protect conversion value

    See Also
    --------
    :class:`Bond`: Standard bonds
    :class:`EquityOption`: Equity options
    :class:`PricerRegistry`: Pricing entry point
    """

    @classmethod
    def create(
        cls,
        instrument_id: str,
        notional: Money,
        issue: date,
        maturity: date,
        discount_curve: str,
        conversion: ConversionSpec,
        *,
        underlying_equity_id: Optional[str] = None,
        call_schedule: Optional[List[Tuple[date, float]]] = None,
        put_schedule: Optional[List[Tuple[date, float]]] = None,
        fixed_coupon: Optional[FixedCouponSpec] = None,
        floating_coupon: Optional[FloatingCouponSpec] = None,
    ) -> "ConvertibleBond":
        """Create a convertible bond.

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

        Examples
        --------
            >>> convertible = ConvertibleBond.create(
            ...     "CONVERTIBLE-CORP-A",
            ...     Money(10_000_000, Currency("USD")),
            ...     date(2024, 1, 1),
            ...     date(2029, 1, 1),
            ...     discount_curve="USD",
            ...     conversion=conversion,
            ... )
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
    def conversion_ratio(self) -> Optional[float]: ...
    @property
    def conversion_price(self) -> Optional[float]: ...
    @property
    def conversion_policy(self) -> str: ...
    @property
    def issue(self) -> date: ...
    @property
    def maturity(self) -> date: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
