"""FX digital (binary) option instrument."""

from __future__ import annotations

from datetime import date

from ....core.currency import Currency
from ....core.money import Money
from ...common import InstrumentType

class FxDigitalOption:
    """FX digital (binary) option instrument.

    Pays a fixed cash amount if the option expires in-the-money.

    Two payout types:
    - Cash-or-nothing: pays a fixed amount in the payout currency
    - Asset-or-nothing: pays one unit of foreign currency

    Pricing uses Garman-Kohlhagen adapted formulas:
    - Cash-or-nothing call: PV = e^{-r_d T} * N(d2) * payout_amount
    - Cash-or-nothing put:  PV = e^{-r_d T} * N(-d2) * payout_amount
    - Asset-or-nothing call: PV = S * e^{-r_f T} * N(d1) * notional
    - Asset-or-nothing put:  PV = S * e^{-r_f T} * N(-d1) * notional

    Examples
    --------
    Create a cash-or-nothing digital call:

        >>> from finstack.valuations.instruments import FxDigitalOption
        >>> from finstack import Money, Currency
        >>> from datetime import date
        >>> digital = FxDigitalOption.builder(
        ...     "FXDIG-EURUSD-CALL",
        ...     strike=1.12,
        ...     option_type="call",
        ...     payout_type="cash_or_nothing",
        ...     payout_amount=Money(1_000_000, Currency("USD")),
        ...     expiry=date(2024, 6, 21),
        ...     notional=Money(1_000_000, Currency("EUR")),
        ...     base_currency=Currency("EUR"),
        ...     quote_currency=Currency("USD"),
        ...     domestic_discount_curve="USD-OIS",
        ...     foreign_discount_curve="EUR-OIS",
        ...     vol_surface="EURUSD-VOL",
        ... )

    See Also
    --------
    :class:`FxOption`: Standard FX options
    :class:`FxTouchOption`: Touch/no-touch FX options
    """

    @classmethod
    def builder(
        cls,
        instrument_id: str,
        strike: float,
        option_type: str,
        payout_type: str,
        payout_amount: Money,
        expiry: date,
        notional: Money,
        base_currency: Currency,
        quote_currency: Currency,
        domestic_discount_curve: str,
        foreign_discount_curve: str,
        vol_surface: str,
    ) -> FxDigitalOption:
        """Create an FX digital option.

        Parameters
        ----------
        instrument_id : str
            Unique identifier for the option.
        strike : float
            Strike exchange rate (quote per base). Must be > 0.
        option_type : str
            Option type: "call" or "put".
        payout_type : str
            Payout type: "cash_or_nothing" or "asset_or_nothing".
        payout_amount : Money
            Fixed payout amount.
        expiry : date
            Option expiration date.
        notional : Money
            Notional amount in base currency.
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
        FxDigitalOption
            Configured FX digital option ready for pricing.

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
    def strike(self) -> float:
        """Strike exchange rate (quote per base)."""
        ...
    @property
    def option_type(self) -> str:
        """Option type label ("call" or "put")."""
        ...
    @property
    def payout_type(self) -> str:
        """Payout type label ("cash_or_nothing" or "asset_or_nothing")."""
        ...
    @property
    def payout_amount(self) -> Money:
        """Fixed payout amount."""
        ...
    @property
    def expiry(self) -> date:
        """Expiry date."""
        ...
    @property
    def notional(self) -> Money:
        """Notional amount."""
        ...
    def __repr__(self) -> str: ...
