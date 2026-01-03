"""Total return swap instruments."""

from typing import Optional
from datetime import date
from ...core.money import Money
from ...core.currency import Currency
from ...core.dates.daycount import DayCount
from ..common import InstrumentType
from ..cashflow.builder import ScheduleParams

class TrsSide:
    """Total return swap side wrapper."""

    RECEIVE_TOTAL_RETURN: "TrsSide"
    PAY_TOTAL_RETURN: "TrsSide"

class TrsFinancingLegSpec:
    """Financing leg specification wrapper."""
    @classmethod
    def new(
        cls,
        discount_curve: str,
        forward_curve: str,
        day_count: DayCount,
        *,
        spread_bp: Optional[float] = 0.0,
    ) -> "TrsFinancingLegSpec": ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def forward_curve(self) -> str: ...
    @property
    def spread_bp(self) -> float: ...
    @property
    def day_count(self) -> str: ...

class TrsScheduleSpec:
    """TRS schedule specification wrapper."""
    @classmethod
    def new(
        cls,
        start: date,
        end: date,
        schedule_params: ScheduleParams,
    ) -> "TrsScheduleSpec": ...
    @property
    def start(self) -> date: ...
    @property
    def end(self) -> date: ...

class EquityUnderlying:
    """Equity underlying parameters wrapper."""
    @classmethod
    def new(
        cls,
        ticker: str,
        spot_id: str,
        currency: Currency,
        *,
        div_yield_id: Optional[str] = None,
        contract_size: Optional[float] = None,
    ) -> "EquityUnderlying": ...
    @property
    def ticker(self) -> str: ...
    @property
    def spot_id(self) -> str: ...
    @property
    def currency(self) -> Currency: ...

class IndexUnderlying:
    """Fixed-income index underlying parameters wrapper."""
    @classmethod
    def new(
        cls,
        index_id: str,
        base_currency: Currency,
        *,
        yield_id: Optional[str] = None,
        duration_id: Optional[str] = None,
        convexity_id: Optional[str] = None,
        contract_size: Optional[float] = None,
    ) -> "IndexUnderlying": ...
    @property
    def index_id(self) -> str: ...
    @property
    def base_currency(self) -> Currency: ...

class EquityTotalReturnSwap:
    """Equity total return swap for synthetic equity exposure.

    EquityTotalReturnSwap (TRS) is a derivative where one party pays the total
    return (price appreciation + dividends) of an equity and receives a
    financing rate, while the other party does the opposite.

    Equity TRS allows investors to gain synthetic exposure to equities without
    owning the underlying, or to hedge equity positions. The total return
    includes both capital gains/losses and dividend payments.

    Examples
    --------
    Create an equity TRS:

        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.dates.daycount import DayCount
        >>> from finstack.core.money import Money
        >>> from finstack.valuations.cashflow.builder import ScheduleParams
        >>> from finstack.valuations.instruments import (
        ...     EquityTotalReturnSwap,
        ...     EquityUnderlying,
        ...     TrsFinancingLegSpec,
        ...     TrsScheduleSpec,
        ...     TrsSide,
        ... )
        >>> underlying = EquityUnderlying.new(
        ...     ticker="SPX", spot_id="SPX", currency=Currency("USD"), div_yield_id=None, contract_size=None
        ... )
        >>> financing = TrsFinancingLegSpec.new(
        ...     discount_curve="USD",
        ...     forward_curve="USD-LIBOR-3M",
        ...     day_count=DayCount.ACT_360,
        ...     spread_bp=25.0,  # 25bp spread
        ... )
        >>> schedule = TrsScheduleSpec.new(
        ...     start=date(2024, 1, 1),
        ...     end=date(2025, 1, 1),
        ...     schedule_params=ScheduleParams.quarterly_act360(),
        ... )
        >>> trs = EquityTotalReturnSwap.create(
        ...     "TRS-SPX",
        ...     notional=Money(10_000_000, Currency("USD")),
        ...     underlying=underlying,
        ...     financing=financing,
        ...     schedule=schedule,
        ...     side=TrsSide.RECEIVE_TOTAL_RETURN,
        ...     initial_level=4_000.0,
        ... )

    Notes
    -----
    - TRS requires underlying equity spot price and dividend yield
    - Financing leg uses discount and forward curves
    - Total return leg pays equity performance (price + dividends)
    - Financing leg pays/receives floating rate + spread
    - Settlement occurs on schedule dates (typically quarterly)

    MarketContext Requirements
    -------------------------
    - Underlying spot: ``underlying.spot_id`` (required).
    - Dividend yield: ``underlying.div_yield_id`` (optional; used when provided).
    - Discount curve: ``financing.discount_curve`` (required).
    - Forward curve: ``financing.forward_curve`` (required).

    See Also
    --------
    :class:`FiIndexTotalReturnSwap`: Fixed-income index TRS
    :class:`EquityOption`: Equity options
    :class:`PricerRegistry`: Pricing entry point

    Sources
    -------
    - ISDA (2006) Definitions: see ``docs/REFERENCES.md#isda2006Definitions``.
    - Hull (text): see ``docs/REFERENCES.md#hullOptionsFuturesDerivatives``.
    """

    @classmethod
    def create(
        cls,
        instrument_id: str,
        notional: Money,
        underlying: EquityUnderlying,
        financing: TrsFinancingLegSpec,
        schedule: TrsScheduleSpec,
        side: TrsSide,
        *,
        initial_level: Optional[float] = None,
    ) -> "EquityTotalReturnSwap":
        """Create an equity total return swap.

        Parameters
        ----------
        instrument_id : str
            Unique identifier for the TRS (e.g., "TRS-SPX", "TRS-AAPL").
        notional : Money
            Notional principal amount. The currency should match the underlying
            equity currency.
        underlying : EquityUnderlying
            Equity underlying specification (ticker, spot_id, currency, etc.).
        financing : TrsFinancingLegSpec
            Financing leg specification (discount curve, forward curve, spread).
        schedule : TrsScheduleSpec
            TRS schedule specification (start date, end date, payment frequency).
        side : TrsSide
            TRS side: RECEIVE_TOTAL_RETURN (receive equity return, pay financing)
            or PAY_TOTAL_RETURN (pay equity return, receive financing).
        initial_level : float, optional
            Initial equity level for calculating returns. If None, uses spot
            price at start date.

        Returns
        -------
        EquityTotalReturnSwap
            Configured equity TRS ready for pricing.

        Raises
        ------
        ValueError
            If parameters are invalid or if required market data is missing.

        Examples
        --------
            >>> trs = EquityTotalReturnSwap.create(
            ...     "TRS-SPX",
            ...     Money(10_000_000, Currency("USD")),
            ...     underlying,
            ...     financing,
            ...     schedule,
            ...     TrsSide.RECEIVE_TOTAL_RETURN,
            ... )
        """
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    @property
    def side(self) -> str: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class FiIndexTotalReturnSwap:
    """Fixed-income index total return swap for synthetic bond index exposure.

    FiIndexTotalReturnSwap (TRS) is a derivative where one party pays the total
    return (price appreciation + interest) of a fixed-income index and receives
    a financing rate, while the other party does the opposite.

    Fixed-income TRS allows investors to gain synthetic exposure to bond indices
    without owning the underlying bonds, or to hedge bond positions. The total
    return includes both price changes and interest payments.

    Examples
    --------
    Create a fixed-income index TRS:

        >>> from finstack.valuations.instruments import (
        ...     FiIndexTotalReturnSwap,
        ...     IndexUnderlying,
        ...     TrsFinancingLegSpec,
        ...     TrsScheduleSpec,
        ...     TrsSide,
        ... )
        >>> from finstack import Money, Currency
        >>> underlying = IndexUnderlying.new(
        ...     index_id="AGG",  # Aggregate bond index
        ...     base_currency=Currency("USD"),
        ...     yield_id="AGG-YIELD",
        ...     duration_id="AGG-DURATION",
        ...     convexity_id="AGG-CONVEXITY",
        ...     contract_size=None,
        ... )
        >>> from finstack.core.dates.daycount import DayCount
        >>> from finstack.valuations.cashflow.builder import ScheduleParams
        >>> financing = TrsFinancingLegSpec.new(
        ...     discount_curve="USD",
        ...     forward_curve="USD-LIBOR-3M",
        ...     day_count=DayCount.ACT_360,
        ...     spread_bp=25.0,
        ... )
        >>> schedule = TrsScheduleSpec.new(
        ...     start=date(2024, 1, 1),
        ...     end=date(2025, 1, 1),
        ...     schedule_params=ScheduleParams.quarterly_act360(),
        ... )
        >>> trs = FiIndexTotalReturnSwap.create(
        ...     "TRS-AGG",
        ...     Money(10_000_000, Currency("USD")),
        ...     underlying,
        ...     financing,
        ...     schedule,
        ...     TrsSide.RECEIVE_TOTAL_RETURN,
        ...     initial_level=100.0,  # Initial index level
        ... )

    Notes
    -----
    - TRS requires underlying index level and yield/duration metrics
    - Financing leg uses discount and forward curves
    - Total return leg pays index performance (price + interest)
    - Financing leg pays/receives floating rate + spread
    - Settlement occurs on schedule dates

    MarketContext Requirements
    -------------------------
    - Underlying index/metrics: referenced by IDs in ``underlying`` (e.g., ``yield_id``, ``duration_id``, ``convexity_id``)
      when provided.
    - Discount curve: ``financing.discount_curve`` (required).
    - Forward curve: ``financing.forward_curve`` (required).

    See Also
    --------
    :class:`EquityTotalReturnSwap`: Equity TRS
    :class:`Bond`: Individual bonds
    :class:`PricerRegistry`: Pricing entry point

    Sources
    -------
    - ISDA (2006) Definitions: see ``docs/REFERENCES.md#isda2006Definitions``.
    - Hull (text): see ``docs/REFERENCES.md#hullOptionsFuturesDerivatives``.
    """

    @classmethod
    def create(
        cls,
        instrument_id: str,
        notional: Money,
        underlying: IndexUnderlying,
        financing: TrsFinancingLegSpec,
        schedule: TrsScheduleSpec,
        side: TrsSide,
        *,
        initial_level: Optional[float] = None,
    ) -> "FiIndexTotalReturnSwap":
        """Create a fixed income index total return swap.

        Parameters
        ----------
        instrument_id : str
            Unique identifier for the TRS (e.g., "TRS-AGG", "TRS-HY").
        notional : Money
            Notional principal amount. Currency should match index currency.
        underlying : IndexUnderlying
            Fixed-income index underlying specification (index_id, currency, etc.).
        financing : TrsFinancingLegSpec
            Financing leg specification (discount curve, forward curve, spread).
        schedule : TrsScheduleSpec
            TRS schedule specification (start date, end date, payment frequency).
        side : TrsSide
            TRS side: RECEIVE_TOTAL_RETURN or PAY_TOTAL_RETURN.
        initial_level : float, optional
            Initial index level for calculating returns. If None, uses index
            level at start date.

        Returns
        -------
        FiIndexTotalReturnSwap
            Configured fixed-income index TRS ready for pricing.

        Raises
        ------
        ValueError
            If parameters are invalid or if required market data is missing.

        Examples
        --------
            >>> trs = FiIndexTotalReturnSwap.create(
            ...     "TRS-AGG",
            ...     Money(10_000_000, Currency("USD")),
            ...     underlying,
            ...     financing,
            ...     schedule,
            ...     TrsSide.RECEIVE_TOTAL_RETURN,
            ...     initial_level=100.0,  # Initial index level
            ... )
        """
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    @property
    def side(self) -> str: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
