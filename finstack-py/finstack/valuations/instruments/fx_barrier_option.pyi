"""FX barrier option instrument."""

from typing import Optional
from datetime import date
from ...core.money import Money
from ...core.currency import Currency
from ..common import InstrumentType

class FxBarrierOption:
    """FX barrier option with path-dependent payoff.

    FxBarrierOption represents an FX option whose payoff depends on whether
    the FX rate crosses a barrier level. Similar to equity barrier options
    but for currency pairs, requiring both domestic and foreign discount curves.

    FX barrier options are used for cost-effective FX hedging and volatility
    trading. They require discount curves for both currencies, FX spot rates,
    and FX volatility surfaces.

    Examples
    --------
    Create a down-and-out FX call barrier option (requires MarketContext
    with discount curves, FX spot, and FX vol surface):

        from finstack.valuations.instruments import FxBarrierOption
        from finstack import Money, Currency
        from datetime import date
        fx_barrier = FxBarrierOption.builder(
            "FX-BARRIER-EURUSD-DO-CALL",
            strike=1.10,  # EUR/USD strike
            barrier=1.15,  # Barrier level (above strike for down-and-out call)
            option_type="call",
            barrier_type="down_and_out",
            expiry=date(2024, 12, 20),
            notional=Money(1_000_000, Currency("USD")),
            domestic_currency=Currency("USD"),
            foreign_currency=Currency("EUR"),
            discount_curve="USD",
            foreign_discount_curve="EUR",
            fx_spot_id="EURUSD",
            fx_vol_surface="EURUSD-VOL",
            use_gobet_miri=False,
        )

    Notes
    -----
    - FX barrier options require domestic and foreign discount curves
    - Barrier types: "up_and_out", "up_and_in", "down_and_out", "down_and_in"
    - Out options are knocked out if barrier is crossed
    - In options only pay if barrier is crossed
    - FX barrier options account for interest rate differentials

    Conventions
    -----------
    - FX rates (``strike``/``barrier``) are quoted as ``domestic per foreign`` when used with
      ``domestic_currency``/``foreign_currency`` (i.e., quote per base for the currency pair).
    - Volatilities in surfaces are expected as decimals.
    - Required market data is referenced by IDs (``discount_curve``, ``foreign_discount_curve``, ``fx_spot_id``,
      ``fx_vol_surface``) and must be present in ``MarketContext``.

    MarketContext Requirements
    -------------------------
    - Discount curves: ``discount_curve`` and ``foreign_discount_curve`` (required).
    - FX spot: ``fx_spot_id`` (optional; if omitted, spot is sourced from ``FxMatrix``).
    - FX volatility surface: ``fx_vol_surface`` (required).

    See Also
    --------
    :class:`FxOption`: Standard FX options
    :class:`BarrierOption`: Equity barrier options
    :class:`PricerRegistry`: Pricing entry point

    Sources
    -------
    - Garman & Kohlhagen (1983): see ``docs/REFERENCES.md#garmanKohlhagen1983``.
    - Hull (text): see ``docs/REFERENCES.md#hullOptionsFuturesDerivatives``.
    - Gobet (2009): see ``docs/REFERENCES.md#gobet2009BarrierMC``.
    """

    @classmethod
    def builder(
        cls,
        instrument_id: str,
        strike: float,
        barrier: float,
        option_type: str,
        barrier_type: str,
        expiry: date,
        notional: Money,
        domestic_currency: Currency,
        foreign_currency: Currency,
        discount_curve: str,
        foreign_discount_curve: str,
        fx_spot_id: Optional[str],
        fx_vol_surface: str,
        *,
        use_gobet_miri: Optional[bool] = False,
    ) -> "FxBarrierOption":
        """Create an FX barrier option.

        Parameters
        ----------
        instrument_id : str
            Unique identifier for the option.
        strike : float
            Strike exchange rate (quote_currency per base_currency). Must be > 0.
        barrier : float
            Barrier exchange rate. Must be > 0 and typically different from strike.
        option_type : str
            Option type: "call" or "put".
        barrier_type : str
            Barrier type: "up_and_out", "up_and_in", "down_and_out", "down_and_in".
        expiry : date
            Option expiration date.
        notional : Money
            Notional amount in domestic currency.
        domestic_currency : Currency
            Domestic currency (currency for payoff).
        foreign_currency : Currency
            Foreign currency (base currency of FX pair).
        discount_curve : str
            Domestic discount curve identifier in MarketContext.
        foreign_discount_curve : str
            Foreign discount curve identifier in MarketContext.
        fx_spot_id : str, optional
            FX spot rate identifier in MarketContext. If omitted, pricing uses
            ``FxMatrix`` for the ``foreign_currency/domestic_currency`` spot rate.
        fx_vol_surface : str
            FX volatility surface identifier in MarketContext.
        use_gobet_miri : bool, optional
            Use Gobet-Miri approximation for barrier pricing (default: False).

        Returns
        -------
        FxBarrierOption
            Configured FX barrier option ready for pricing.

        Raises
        ------
        ValueError
            If parameters are invalid or if required market data is missing.

        Examples
        --------
            >>> # Note: This example constructs the option but requires MarketContext
            >>> # with discount curves, FX spot, and FX vol surface to actually price.
            >>> # The construction itself may fail validation without proper setup.
            >>> # For a working example, see the class-level docstring above.
        """
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def strike(self) -> Money: ...
    @property
    def barrier(self) -> Money: ...
    @property
    def option_type(self) -> str: ...
    @property
    def barrier_type(self) -> str: ...
    @property
    def expiry(self) -> date: ...
    @property
    def notional(self) -> Money: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
