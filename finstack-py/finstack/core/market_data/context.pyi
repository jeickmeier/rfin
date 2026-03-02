"""Market context for aggregating and managing market data.

Provides a central repository for all market data including curves,
surfaces, FX rates, and other market information.
"""

from __future__ import annotations
from typing import Dict, List, Any
from .term_structures import (
    DiscountCurve,
    ForwardCurve,
    HazardCurve,
    InflationCurve,
    BaseCorrelationCurve,
    CreditIndexData,
)
from .surfaces import VolSurface
from .fx import FxMatrix
from .scalars import MarketScalar, ScalarTimeSeries
from .dividends import DividendSchedule
from ..currency import Currency

class MarketContext:
    """Central repository for all market data used in pricing and risk calculations.

    MarketContext is the primary container for market data in finstack. It
    aggregates discount curves, forward curves, volatility surfaces, FX rates,
    scalar prices, and other market information needed for instrument valuation.
    All valuation functions require a MarketContext to resolve market data
    dependencies by identifier.

    MarketContext instances are mutable and can be populated incrementally using
    the various ``insert_*`` methods. Once populated, curves and surfaces can
    be retrieved by their identifiers. The context can be cloned to create
    independent copies for scenario analysis.

    Parameters
    ----------
    None
        Construct via ``MarketContext()`` to create an empty context.

    Returns
    -------
    MarketContext
        Empty market context ready for population with market data.

    Examples
    --------
        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.market_data.context import MarketContext
        >>> from finstack.core.market_data.term_structures import DiscountCurve
        >>> from finstack.core.market_data.fx import FxMatrix
        >>> ctx = MarketContext()
        >>> ctx.insert_discount(DiscountCurve("USD", date(2024, 1, 1), [(0.0, 1.0), (1.0, 0.99)]))
        >>> fx = FxMatrix()
        >>> fx.set_quote(Currency("EUR"), Currency("USD"), 1.10)
        >>> ctx.insert_fx(fx)
        >>> sorted(ctx.curve_ids())
        ['USD']

    Notes
    -----
    - MarketContext is mutable - use :meth:`clone` to create independent copies
    - Insertion methods replace existing entries with the same identifier
    - Retrieval methods raise ``ValueError`` if the requested item is not found
    - FX matrix is optional but required for multi-currency valuations
    - Use :meth:`apply_bumps` to create scenario variants with shifted market data
    - Context statistics are available via :meth:`stats` for debugging

    See Also
    --------
    :class:`DiscountCurve`: Discount curve construction
    :class:`FxMatrix`: FX rate management
    :class:`MarketBump`: Scenario bump specifications
    """

    def __init__(self) -> None: ...
    def clone(self) -> MarketContext:
        """Create a deep copy of this market context.

        Returns
        -------
        MarketContext
            Independent copy of the market context.
        """
        ...

    def insert_discount(self, curve: DiscountCurve) -> None:
        """Insert a discount curve.

        Parameters
        ----------
        curve : DiscountCurve
            Discount curve to add.
        """
        ...

    def insert_forward(self, curve: ForwardCurve) -> None:
        """Insert a forward curve.

        Parameters
        ----------
        curve : ForwardCurve
            Forward curve to add.
        """
        ...

    def insert_hazard(self, curve: HazardCurve) -> None:
        """Insert a hazard curve.

        Parameters
        ----------
        curve : HazardCurve
            Hazard curve to add.
        """
        ...

    def insert_inflation(self, curve: InflationCurve) -> None:
        """Insert an inflation curve.

        Parameters
        ----------
        curve : InflationCurve
            Inflation curve to add.
        """
        ...

    def insert_base_correlation(self, curve: BaseCorrelationCurve) -> None:
        """Insert a base correlation curve.

        Parameters
        ----------
        curve : BaseCorrelationCurve
            Base correlation curve to add.
        """
        ...

    def insert_fx(self, fx_matrix: FxMatrix) -> None:
        """Insert an FX matrix.

        Parameters
        ----------
        fx_matrix : FxMatrix
            FX matrix to add.
        """
        ...

    def insert_surface(self, surface: VolSurface) -> None:
        """Insert a volatility surface.

        Parameters
        ----------
        surface : VolSurface
            Volatility surface to add.
        """
        ...

    def insert_price(self, id: str, scalar: MarketScalar) -> None:
        """Insert a market price.

        Parameters
        ----------
        id : str
            Price identifier.
        scalar : MarketScalar
            Market price scalar.
        """
        ...

    def insert_series(self, series: ScalarTimeSeries) -> None:
        """Insert a time series.

        Parameters
        ----------
        series : ScalarTimeSeries
            Time series to add.
        """
        ...

    def insert_dividends(self, schedule: DividendSchedule) -> None:
        """Insert a dividend schedule.

        Parameters
        ----------
        schedule : DividendSchedule
            Dividend schedule to add.
        """
        ...

    def insert_credit_index(self, id: str, data: "CreditIndexData") -> None:
        """Insert credit index data.

        Parameters
        ----------
        id : str
            Credit index identifier.
        data : CreditIndexData
            Credit index data.
        """
        ...

    def map_collateral(self, csa_code: str, curve_id: str) -> None:
        """Map collateral to a discount curve.

        Parameters
        ----------
        csa_code : str
            Collateral Support Annex code.
        curve_id : str
            Discount curve identifier.
        """
        ...

    def discount(self, id: str) -> DiscountCurve:
        """Retrieve a discount curve by identifier.

        Parameters
        ----------
        id : str
            Discount curve identifier (e.g., "USD", "EUR-LIBOR-3M").

        Returns
        -------
        DiscountCurve
            Discount curve with the specified identifier.

        Raises
        ------
        ValueError
            If no discount curve with the given identifier exists in the context.
            Use :meth:`curve_ids_by_type` to list available curves.

        Examples
        --------
            >>> ctx = MarketContext()
            >>> ctx.insert_discount(my_curve)
            >>> curve = ctx.discount("USD")
            >>> df = curve.discount_factor(0.5)  # 6-month discount factor
        """
        ...

    def forward(self, id: str) -> ForwardCurve:
        """Get a forward curve by ID.

        Parameters
        ----------
        id : str
            Curve identifier.

        Returns
        -------
        ForwardCurve
            Forward curve.

        Raises
        ------
        KeyError
            If curve not found.
        """
        ...

    def hazard(self, id: str) -> HazardCurve:
        """Get a hazard curve by ID.

        Parameters
        ----------
        id : str
            Curve identifier.

        Returns
        -------
        HazardCurve
            Hazard curve.

        Raises
        ------
        KeyError
            If curve not found.
        """
        ...

    def inflation(self, id: str) -> InflationCurve:
        """Get an inflation curve by ID.

        Parameters
        ----------
        id : str
            Curve identifier.

        Returns
        -------
        InflationCurve
            Inflation curve.

        Raises
        ------
        KeyError
            If curve not found.
        """
        ...

    def base_correlation(self, id: str) -> BaseCorrelationCurve:
        """Get a base correlation curve by ID.

        Parameters
        ----------
        id : str
            Curve identifier.

        Returns
        -------
        BaseCorrelationCurve
            Base correlation curve.

        Raises
        ------
        KeyError
            If curve not found.
        """
        ...

    def surface(self, id: str) -> VolSurface:
        """Get a volatility surface by ID.

        Parameters
        ----------
        id : str
            Surface identifier.

        Returns
        -------
        VolSurface
            Volatility surface.

        Raises
        ------
        KeyError
            If surface not found.
        """
        ...

    def price(self, id: str) -> MarketScalar:
        """Get a market price by ID.

        Parameters
        ----------
        id : str
            Price identifier.

        Returns
        -------
        MarketScalar
            Market price.

        Raises
        ------
        KeyError
            If price not found.
        """
        ...

    def series(self, id: str) -> ScalarTimeSeries:
        """Get a time series by ID.

        Parameters
        ----------
        id : str
            Series identifier.

        Returns
        -------
        ScalarTimeSeries
            Time series.

        Raises
        ------
        KeyError
            If series not found.
        """
        ...

    def credit_index(self, id: str) -> "CreditIndexData":
        """Get credit index data by ID.

        Parameters
        ----------
        id : str
            Credit index identifier.

        Returns
        -------
        CreditIndexData
            Credit index data.

        Raises
        ------
        KeyError
            If credit index not found.
        """
        ...

    def dividend_schedule(self, id: str) -> DividendSchedule | None:
        """Get a dividend schedule by ID.

        Parameters
        ----------
        id : str
            Schedule identifier.

        Returns
        -------
        DividendSchedule or None
            Dividend schedule if found.
        """
        ...

    def curve_ids(self) -> List[str]:
        """Get all curve identifiers.

        Returns
        -------
        List[str]
            All curve IDs.
        """
        ...

    def curve_ids_by_type(self, curve_type: str) -> List[str]:
        """Get curve IDs by type.

        Parameters
        ----------
        curve_type : str
            Curve type ("discount", "forward", "hazard", "inflation").

        Returns
        -------
        List[str]
            Curve IDs of the specified type.
        """
        ...

    def count_by_type(self) -> Dict[str, int]:
        """Get count of objects by type.

        Returns
        -------
        Dict[str, int]
            Count of objects by type.
        """
        ...

    def stats(self) -> Dict[str, Any]:
        """Get market context statistics.

        Returns
        -------
        Dict[str, Any]
            Statistics about the market context.
        """
        ...

    def is_empty(self) -> bool:
        """Check if the context is empty.

        Returns
        -------
        bool
            True if no market data is present.
        """
        ...

    def total_objects(self) -> int:
        """Get total number of objects.

        Returns
        -------
        int
            Total number of market data objects.
        """
        ...

    def has_fx(self) -> bool:
        """Check if FX data is present.

        Returns
        -------
        bool
            True if FX matrix is present.
        """
        ...

    def __copy__(self) -> MarketContext:
        """Support copy.copy(ctx)."""
        ...

    def __deepcopy__(self, memo: Any) -> MarketContext:
        """Support copy.deepcopy(ctx)."""
        ...

    def __contains__(self, id: str) -> bool:
        """Check if a curve or surface with the given ID exists.

        Parameters
        ----------
        id : str
            Identifier to look up.

        Returns
        -------
        bool
            True if found.
        """
        ...

    def __repr__(self) -> str: ...
