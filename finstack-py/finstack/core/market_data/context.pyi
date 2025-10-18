"""Market context for aggregating and managing market data.

Provides a central repository for all market data including curves,
surfaces, FX rates, and other market information.
"""

from typing import Dict, List, Optional, Any, Union
from .term_structures import DiscountCurve, ForwardCurve, HazardCurve, InflationCurve, BaseCorrelationCurve
from .surfaces import VolSurface
from .fx import FxMatrix
from .scalars import MarketScalar, ScalarTimeSeries
from .dividends import DividendSchedule
from ..currency import Currency

class MarketContext:
    """Central repository for market data.
    
    Aggregates all market data including curves, surfaces, FX rates,
    and other market information for use in pricing and risk calculations.
    """
    
    def __init__(self) -> None: ...
    """Create an empty market context."""
    
    def clone(self) -> MarketContext: ...
    """Create a deep copy of this market context.
    
    Returns
    -------
    MarketContext
        Independent copy of the market context.
    """
    
    def insert_discount(self, curve: DiscountCurve) -> None: ...
    """Insert a discount curve.
    
    Parameters
    ----------
    curve : DiscountCurve
        Discount curve to add.
    """
    
    def insert_forward(self, curve: ForwardCurve) -> None: ...
    """Insert a forward curve.
    
    Parameters
    ----------
    curve : ForwardCurve
        Forward curve to add.
    """
    
    def insert_hazard(self, curve: HazardCurve) -> None: ...
    """Insert a hazard curve.
    
    Parameters
    ----------
    curve : HazardCurve
        Hazard curve to add.
    """
    
    def insert_inflation(self, curve: InflationCurve) -> None: ...
    """Insert an inflation curve.
    
    Parameters
    ----------
    curve : InflationCurve
        Inflation curve to add.
    """
    
    def insert_base_correlation(self, curve: BaseCorrelationCurve) -> None: ...
    """Insert a base correlation curve.
    
    Parameters
    ----------
    curve : BaseCorrelationCurve
        Base correlation curve to add.
    """
    
    def insert_fx(self, fx_matrix: FxMatrix) -> None: ...
    """Insert an FX matrix.
    
    Parameters
    ----------
    fx_matrix : FxMatrix
        FX matrix to add.
    """
    
    def insert_surface(self, surface: VolSurface) -> None: ...
    """Insert a volatility surface.
    
    Parameters
    ----------
    surface : VolSurface
        Volatility surface to add.
    """
    
    def insert_price(self, id: str, scalar: MarketScalar) -> None: ...
    """Insert a market price.
    
    Parameters
    ----------
    id : str
        Price identifier.
    scalar : MarketScalar
        Market price scalar.
    """
    
    def insert_series(self, series: ScalarTimeSeries) -> None: ...
    """Insert a time series.
    
    Parameters
    ----------
    series : ScalarTimeSeries
        Time series to add.
    """
    
    def insert_dividends(self, schedule: DividendSchedule) -> None: ...
    """Insert a dividend schedule.
    
    Parameters
    ----------
    schedule : DividendSchedule
        Dividend schedule to add.
    """
    
    def insert_credit_index(self, id: str, data: "CreditIndexData") -> None: ...
    """Insert credit index data.
    
    Parameters
    ----------
    id : str
        Credit index identifier.
    data : CreditIndexData
        Credit index data.
    """
    
    def map_collateral(self, csa_code: str, curve_id: str) -> None: ...
    """Map collateral to a discount curve.
    
    Parameters
    ----------
    csa_code : str
        Collateral Support Annex code.
    curve_id : str
        Discount curve identifier.
    """
    
    def discount(self, id: str) -> DiscountCurve: ...
    """Get a discount curve by ID.
    
    Parameters
    ----------
    id : str
        Curve identifier.
        
    Returns
    -------
    DiscountCurve
        Discount curve.
        
    Raises
    ------
    KeyError
        If curve not found.
    """
    
    def forward(self, id: str) -> ForwardCurve: ...
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
    
    def hazard(self, id: str) -> HazardCurve: ...
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
    
    def inflation(self, id: str) -> InflationCurve: ...
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
    
    def base_correlation(self, id: str) -> BaseCorrelationCurve: ...
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
    
    def surface(self, id: str) -> VolSurface: ...
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
    
    def price(self, id: str) -> MarketScalar: ...
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
    
    def series(self, id: str) -> ScalarTimeSeries: ...
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
    
    def credit_index(self, id: str) -> "CreditIndexData": ...
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
    
    def dividend_schedule(self, id: str) -> Optional[DividendSchedule]: ...
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
    
    def curve_ids(self) -> List[str]: ...
    """Get all curve identifiers.
    
    Returns
    -------
    List[str]
        All curve IDs.
    """
    
    def curve_ids_by_type(self, curve_type: str) -> List[str]: ...
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
    
    def count_by_type(self) -> Dict[str, int]: ...
    """Get count of objects by type.
    
    Returns
    -------
    Dict[str, int]
        Count of objects by type.
    """
    
    def stats(self) -> Dict[str, Any]: ...
    """Get market context statistics.
    
    Returns
    -------
    Dict[str, Any]
        Statistics about the market context.
    """
    
    @property
    def is_empty(self) -> bool: ...
    """Check if the context is empty.
    
    Returns
    -------
    bool
        True if no market data is present.
    """
    
    @property
    def total_objects(self) -> int: ...
    """Get total number of objects.
    
    Returns
    -------
    int
        Total number of market data objects.
    """
    
    @property
    def has_fx(self) -> bool: ...
    """Check if FX data is present.
    
    Returns
    -------
    bool
        True if FX matrix is present.
    """
    
    def __repr__(self) -> str: ...
