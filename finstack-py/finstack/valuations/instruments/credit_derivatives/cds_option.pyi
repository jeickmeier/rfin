"""CDS option instrument."""

from __future__ import annotations
from datetime import date
from ....core.money import Money
from ...common import InstrumentType
from ...common.parameters import OptionType, ExerciseStyle, SettlementType
from ....core.market_data.context import MarketContext

class CDSOptionBuilder:
    """Fluent builder returned by :meth:`CDSOption.builder`."""

    def __init__(self, instrument_id: str) -> None: ...
    def notional(self, notional: Money) -> "CDSOptionBuilder": ...
    def money(self, money: Money) -> "CDSOptionBuilder": ...
    def strike(self, strike: float) -> "CDSOptionBuilder": ...
    def strike_spread_bp(self, strike_spread_bp: float) -> "CDSOptionBuilder": ...
    def expiry(self, expiry: date) -> "CDSOptionBuilder": ...
    def cds_maturity(self, cds_maturity: date) -> "CDSOptionBuilder": ...
    def discount_curve(self, discount_curve: str) -> "CDSOptionBuilder": ...
    def credit_curve(self, credit_curve: str) -> "CDSOptionBuilder": ...
    def vol_surface(self, vol_surface: str) -> "CDSOptionBuilder": ...
    def option_type(self, option_type: str | None) -> "CDSOptionBuilder": ...
    def recovery_rate(self, recovery_rate: float) -> "CDSOptionBuilder": ...
    def underlying_is_index(self, underlying_is_index: bool) -> "CDSOptionBuilder": ...
    def index_factor(self, index_factor: float | None = ...) -> "CDSOptionBuilder": ...
    def forward_adjust(self, forward_adjust: float) -> "CDSOptionBuilder": ...
    def forward_adjust_bp(self, forward_adjust_bp: float) -> "CDSOptionBuilder": ...
    def build(self) -> "CDSOption": ...

class CDSOption:
    """Option on CDS spread for credit volatility exposure.

    CDSOption represents an option to enter into a CDS at a specified spread
    (strike) on or before expiry.

    Examples
    --------
        >>> from finstack.valuations.instruments import CDSOption
        >>> from finstack import Money, Currency
        >>> from datetime import date
        >>> cds_option = (
        ...     CDSOption
        ...     .builder("CDS-OPT-CORP-A")
        ...     .money(Money(10_000_000, Currency("USD")))
        ...     .strike(0.015)
        ...     .expiry(date(2024, 12, 20))
        ...     .cds_maturity(date(2029, 1, 1))
        ...     .discount_curve("USD")
        ...     .credit_curve("CORP-A-HAZARD")
        ...     .vol_surface("CDS-VOL")
        ...     .option_type("call")
        ...     .build()
        ... )

    Sources
    -------
    - O'Kane (2008): see ``docs/REFERENCES.md#okane2008``.
    - ISDA (2006) Definitions: see ``docs/REFERENCES.md#isda2006Definitions``.
    """

    @classmethod
    def builder(cls, instrument_id: str) -> CDSOptionBuilder:
        """Start a fluent builder (builder-only API).

        Parameters
        ----------
        instrument_id : str
            Unique identifier for the option.

        Returns
        -------
        CDSOptionBuilder
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
    def cds_maturity(self) -> date: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def credit_curve(self) -> str: ...
    @property
    def option_type(self) -> OptionType: ...
    @property
    def exercise_style(self) -> ExerciseStyle: ...
    @property
    def settlement(self) -> SettlementType: ...
    @property
    def recovery_rate(self) -> float: ...
    @property
    def vol_surface(self) -> str: ...
    @property
    def underlying_is_index(self) -> bool: ...
    @property
    def index_factor(self) -> float | None: ...
    @property
    def forward_spread_adjust(self) -> float: ...
    @property
    def day_count(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def delta(self, market: MarketContext, as_of: date) -> float:
        """Calculate delta (spread sensitivity)."""
        ...

    def gamma(self, market: MarketContext, as_of: date) -> float:
        """Calculate gamma (second-order spread sensitivity)."""
        ...

    def vega(self, market: MarketContext, as_of: date) -> float:
        """Calculate vega (volatility sensitivity)."""
        ...

    def theta(self, market: MarketContext, as_of: date) -> float:
        """Calculate theta (time decay per day)."""
        ...

    def implied_vol(
        self,
        market: MarketContext,
        as_of: date,
        target_price: float,
        initial_guess: float | None = None,
    ) -> float:
        """Calculate implied volatility from a target price.

        Parameters
        ----------
        market : MarketContext
            Market context with discount, hazard, and vol surfaces.
        as_of : date
            Valuation date.
        target_price : float
            Observed market price to match.
        initial_guess : float, optional
            Starting volatility for the solver.

        Returns
        -------
        float
            Implied lognormal volatility in decimal form.
        """
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
