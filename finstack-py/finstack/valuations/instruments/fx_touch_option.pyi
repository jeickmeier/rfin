"""FX touch option (American binary option) instrument."""

from __future__ import annotations

from datetime import date

from ...core.currency import Currency
from ...core.money import Money
from ..common import InstrumentType


class FxTouchOption:
    """FX touch option (American binary option) instrument.

    Touch options pay a fixed amount if the spot rate touches a barrier
    level at any time before expiry:
    - One-touch: pays if barrier is touched
    - No-touch: pays if barrier is NOT touched

    Pricing uses closed-form formulas for continuous monitoring
    (Rubinstein & Reiner 1991).

    Examples
    --------
    Create a down-and-in one-touch option:

        >>> from finstack.valuations.instruments import FxTouchOption
        >>> from finstack import Money, Currency
        >>> from datetime import date
        >>> touch = FxTouchOption.builder(
        ...     "FXTOUCH-EURUSD-OT",
        ...     barrier_level=1.05,
        ...     touch_type="one_touch",
        ...     barrier_direction="down",
        ...     payout_amount=Money(1_000_000, Currency("USD")),
        ...     payout_timing="at_expiry",
        ...     expiry=date(2024, 6, 21),
        ...     base_currency=Currency("EUR"),
        ...     quote_currency=Currency("USD"),
        ...     domestic_discount_curve="USD-OIS",
        ...     foreign_discount_curve="EUR-OIS",
        ...     vol_surface="EURUSD-VOL",
        ... )

    See Also
    --------
    :class:`FxDigitalOption`: Digital/binary FX options
    :class:`FxBarrierOption`: FX barrier options
    """

    @classmethod
    def builder(
        cls,
        instrument_id: str,
        barrier_level: float,
        touch_type: str,
        barrier_direction: str,
        payout_amount: Money,
        payout_timing: str,
        expiry: date,
        base_currency: Currency,
        quote_currency: Currency,
        domestic_discount_curve: str,
        foreign_discount_curve: str,
        vol_surface: str,
    ) -> FxTouchOption:
        """Create an FX touch option.

        Parameters
        ----------
        instrument_id : str
            Unique identifier for the option.
        barrier_level : float
            Barrier exchange rate level. Must be > 0.
        touch_type : str
            Touch type: "one_touch" or "no_touch".
        barrier_direction : str
            Barrier direction: "up" or "down".
        payout_amount : Money
            Fixed payout amount.
        payout_timing : str
            Payout timing: "at_hit" or "at_expiry".
        expiry : date
            Option expiration date.
        base_currency : Currency
            Base (foreign) currency.
        quote_currency : Currency
            Quote (domestic) currency.
        domestic_discount_curve : str
            Domestic discount curve identifier in MarketContext.
        foreign_discount_curve : str
            Foreign discount curve identifier in MarketContext.
        vol_surface : str
            FX volatility surface identifier in MarketContext.

        Returns
        -------
        FxTouchOption
            Configured FX touch option ready for pricing.

        Raises
        ------
        ValueError
            If parameters are invalid.
        """
        ...

    @property
    def instrument_id(self) -> str:
        """Instrument identifier."""
        ...
    @property
    def instrument_type(self) -> InstrumentType:
        """Instrument type."""
        ...
    @property
    def base_currency(self) -> Currency:
        """Base currency (foreign currency)."""
        ...
    @property
    def quote_currency(self) -> Currency:
        """Quote currency (domestic currency)."""
        ...
    @property
    def barrier_level(self) -> float:
        """Barrier level (exchange rate)."""
        ...
    @property
    def touch_type(self) -> str:
        """Touch type label ("one_touch" or "no_touch")."""
        ...
    @property
    def barrier_direction(self) -> str:
        """Barrier direction label ("up" or "down")."""
        ...
    @property
    def payout_amount(self) -> Money:
        """Fixed payout amount."""
        ...
    @property
    def payout_timing(self) -> str:
        """Payout timing label ("at_hit" or "at_expiry")."""
        ...
    @property
    def expiry(self) -> date:
        """Expiry date."""
        ...
    def __repr__(self) -> str: ...
