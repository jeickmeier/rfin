"""Term structure bindings for interest rate and credit curves.

Provides discount curves, forward curves, hazard curves, inflation curves,
and base correlation curves for financial modeling.
"""

from typing import List, Tuple, Optional, Dict, Union
from datetime import date
from ..currency import Currency
from ..dates import DayCount
from .interp import InterpStyle, ExtrapolationPolicy

class DiscountCurve:
    """Discount curve wrapper supporting multiple interpolation and extrapolation styles.
    
    Parameters
    ----------
    id : str
        Identifier used to retrieve the curve later.
    base_date : datetime.date
        Anchor date corresponding to t = 0.
    knots : list[tuple[float, float]]
        (time, discount_factor) pairs used to build the curve.
    day_count : DayCount, optional
        Day-count convention for converting dates to year fractions.
    interp : str, optional
        Interpolation style such as "linear" or "monotone_convex".
    extrapolation : str, optional
        Extrapolation policy name (e.g. "flat_zero").
    require_monotonic : bool, default False
        Enforce monotonic discount factors across knots.
        
    Returns
    -------
    DiscountCurve
        Curve object exposing discount factor, zero rate, and forward helpers.
    """
    
    def __init__(
        self,
        id: str,
        base_date: Union[str, date],
        knots: List[Tuple[float, float]],
        day_count: Optional[Union[str, DayCount]] = None,
        interp: Optional[Union[str, InterpStyle]] = None,
        extrapolation: Optional[Union[str, ExtrapolationPolicy]] = None,
        require_monotonic: bool = False,
    ) -> None: ...
    
    @property
    def id(self) -> str: ...
    """Get the curve identifier.
    
    Returns
    -------
    str
        Curve ID.
    """
    
    def base_date(self) -> date: ...
    """Get the base date.
    
    Returns
    -------
    date
        Base date (t=0).
    """
    
    @property
    def day_count(self) -> DayCount: ...
    """Get the day count convention.
    
    Returns
    -------
    DayCount
        Day count convention.
    """
    
    @property
    def points(self) -> List[Tuple[float, float]]: ...
    """Get the knot points.
    
    Returns
    -------
    List[Tuple[float, float]]
        (time, discount_factor) pairs.
    """
    
    def df(self, t: float) -> float: ...
    """Get discount factor at time t.
    
    Parameters
    ----------
    t : float
        Time in years.
        
    Returns
    -------
    float
        Discount factor.
    """
    
    def zero(self, t: float) -> float: ...
    """Get zero rate at time t.
    
    Parameters
    ----------
    t : float
        Time in years.
        
    Returns
    -------
    float
        Zero rate (continuously compounded).
    """
    
    def forward(self, t1: float, t2: float) -> float: ...
    """Get forward rate between times t1 and t2.
    
    Parameters
    ----------
    t1 : float
        Start time in years.
    t2 : float
        End time in years.
        
    Returns
    -------
    float
        Forward rate.
    """
    
    def df_on_date(self, date: Union[str, date]) -> float: ...
    """Get discount factor on a specific date.
    
    Parameters
    ----------
    date : str or date
        Target date.
        
    Returns
    -------
    float
        Discount factor.
    """
    
    def __repr__(self) -> str: ...

class ForwardCurve:
    """Forward curve for interest rate modeling.
    
    Parameters
    ----------
    id : str
        Curve identifier.
    tenor_years : float
        Tenor in years (e.g. 0.25 for 3M).
    knots : list[tuple[float, float]]
        (time, rate) pairs.
    base_date : datetime.date, optional
        Base date for the curve.
    reset_lag : int, optional
        Reset lag in days.
    day_count : DayCount, optional
        Day count convention.
    interp : str, optional
        Interpolation style.
    """
    
    def __init__(
        self,
        id: str,
        tenor_years: float,
        knots: List[Tuple[float, float]],
        base_date: Optional[Union[str, date]] = None,
        reset_lag: Optional[int] = None,
        day_count: Optional[Union[str, DayCount]] = None,
        interp: Optional[Union[str, InterpStyle]] = None,
    ) -> None: ...
    
    @property
    def id(self) -> str: ...
    """Get the curve identifier.
    
    Returns
    -------
    str
        Curve ID.
    """
    
    @property
    def tenor_years(self) -> float: ...
    """Get the tenor in years.
    
    Returns
    -------
    float
        Tenor in years.
    """
    
    def base_date(self) -> date: ...
    """Get the base date.
    
    Returns
    -------
    date
        Base date.
    """
    
    @property
    def day_count(self) -> DayCount: ...
    """Get the day count convention.
    
    Returns
    -------
    DayCount
        Day count convention.
    """
    
    @property
    def reset_lag(self) -> int: ...
    """Get the reset lag in days.
    
    Returns
    -------
    int
        Reset lag in days.
    """
    
    @property
    def points(self) -> List[Tuple[float, float]]: ...
    """Get the knot points.
    
    Returns
    -------
    List[Tuple[float, float]]
        (time, rate) pairs.
    """
    
    def rate(self, t: float) -> float: ...
    """Get forward rate at time t.
    
    Parameters
    ----------
    t : float
        Time in years.
        
    Returns
    -------
    float
        Forward rate.
    """
    
    def __repr__(self) -> str: ...

class HazardCurve:
    """Hazard curve for credit risk modeling.
    
    Parameters
    ----------
    id : str
        Curve identifier.
    base_date : datetime.date
        Base date for the curve.
    knots : list[tuple[float, float]]
        (time, hazard_rate) pairs.
    recovery_rate : float, optional
        Recovery rate (default 0.4).
    day_count : DayCount, optional
        Day count convention.
    issuer : str, optional
        Issuer identifier.
    seniority : str, optional
        Seniority level.
    currency : Currency, optional
        Currency of the curve.
    par_points : list[tuple[float, float]], optional
        Par spread points for calibration.
    """
    
    def __init__(
        self,
        id: str,
        base_date: Union[str, date],
        knots: List[Tuple[float, float]],
        recovery_rate: Optional[float] = None,
        day_count: Optional[Union[str, DayCount]] = None,
        issuer: Optional[str] = None,
        seniority: Optional[str] = None,
        currency: Optional[Currency] = None,
        par_points: Optional[List[Tuple[float, float]]] = None,
    ) -> None: ...
    
    @property
    def id(self) -> str: ...
    """Get the curve identifier.
    
    Returns
    -------
    str
        Curve ID.
    """
    
    def base_date(self) -> date: ...
    """Get the base date.
    
    Returns
    -------
    date
        Base date.
    """
    
    @property
    def recovery_rate(self) -> float: ...
    """Get the recovery rate.
    
    Returns
    -------
    float
        Recovery rate.
    """
    
    @property
    def day_count(self) -> DayCount: ...
    """Get the day count convention.
    
    Returns
    -------
    DayCount
        Day count convention.
    """
    
    @property
    def points(self) -> List[Tuple[float, float]]: ...
    """Get the knot points.
    
    Returns
    -------
    List[Tuple[float, float]]
        (time, hazard_rate) pairs.
    """
    
    @property
    def par_spreads(self) -> List[Tuple[float, float]]: ...
    """Get the par spread points.
    
    Returns
    -------
    List[Tuple[float, float]]
        (time, spread) pairs.
    """
    
    def survival(self, t: float) -> float: ...
    """Get survival probability at time t.
    
    Parameters
    ----------
    t : float
        Time in years.
        
    Returns
    -------
    float
        Survival probability.
    """
    
    def default_prob(self, t1: float, t2: float) -> float: ...
    """Get default probability between times t1 and t2.
    
    Parameters
    ----------
    t1 : float
        Start time in years.
    t2 : float
        End time in years.
        
    Returns
    -------
    float
        Default probability.
    """
    
    def __repr__(self) -> str: ...

class InflationCurve:
    """Inflation curve for inflation-linked instruments.
    
    Parameters
    ----------
    id : str
        Curve identifier.
    base_cpi : float
        Base CPI level.
    knots : list[tuple[float, float]]
        (time, cpi_level) pairs.
    interp : str, optional
        Interpolation style.
    """
    
    def __init__(
        self,
        id: str,
        base_cpi: float,
        knots: List[Tuple[float, float]],
        interp: Optional[Union[str, InterpStyle]] = None,
    ) -> None: ...
    
    @property
    def id(self) -> str: ...
    """Get the curve identifier.
    
    Returns
    -------
    str
        Curve ID.
    """
    
    @property
    def base_cpi(self) -> float: ...
    """Get the base CPI level.
    
    Returns
    -------
    float
        Base CPI level.
    """
    
    @property
    def points(self) -> List[Tuple[float, float]]: ...
    """Get the knot points.
    
    Returns
    -------
    List[Tuple[float, float]]
        (time, cpi_level) pairs.
    """
    
    def cpi(self, t: float) -> float: ...
    """Get CPI level at time t.
    
    Parameters
    ----------
    t : float
        Time in years.
        
    Returns
    -------
    float
        CPI level.
    """
    
    def inflation_rate(self, t1: float, t2: float) -> float: ...
    """Get inflation rate between times t1 and t2.
    
    Parameters
    ----------
    t1 : float
        Start time in years.
    t2 : float
        End time in years.
        
    Returns
    -------
    float
        Inflation rate.
    """
    
    def __repr__(self) -> str: ...

class BaseCorrelationCurve:
    """Base correlation curve for CDO/CDS index modeling.
    
    Parameters
    ----------
    id : str
        Curve identifier.
    points : list[tuple[float, float]]
        (detachment, correlation) pairs.
    """
    
    def __init__(self, id: str, points: List[Tuple[float, float]]) -> None: ...
    
    @property
    def id(self) -> str: ...
    """Get the curve identifier.
    
    Returns
    -------
    str
        Curve ID.
    """
    
    @property
    def points(self) -> List[Tuple[float, float]]: ...
    """Get the knot points.
    
    Returns
    -------
    List[Tuple[float, float]]
        (detachment, correlation) pairs.
    """
    
    def correlation(self, detachment_pct: float) -> float: ...
    """Get correlation at detachment percentage.
    
    Parameters
    ----------
    detachment_pct : float
        Detachment percentage.
        
    Returns
    -------
    float
        Base correlation.
    """
    
    def __repr__(self) -> str: ...

class CreditIndexData:
    """Credit index data for CDS index modeling.
    
    Parameters
    ----------
    num_constituents : int
        Number of constituents in the index.
    recovery_rate : float
        Recovery rate.
    index_curve : HazardCurve
        Index hazard curve.
    base_correlation_curve : BaseCorrelationCurve
        Base correlation curve.
    issuer_curves : dict[str, HazardCurve], optional
        Individual issuer curves.
    """
    
    def __init__(
        self,
        num_constituents: int,
        recovery_rate: float,
        index_curve: HazardCurve,
        base_correlation_curve: BaseCorrelationCurve,
        issuer_curves: Optional[Dict[str, HazardCurve]] = None,
    ) -> None: ...
    
    @property
    def num_constituents(self) -> int: ...
    """Get the number of constituents.
    
    Returns
    -------
    int
        Number of constituents.
    """
    
    @property
    def recovery_rate(self) -> float: ...
    """Get the recovery rate.
    
    Returns
    -------
    float
        Recovery rate.
    """
    
    @property
    def index_curve(self) -> HazardCurve: ...
    """Get the index curve.
    
    Returns
    -------
    HazardCurve
        Index hazard curve.
    """
    
    @property
    def base_correlation_curve(self) -> BaseCorrelationCurve: ...
    """Get the base correlation curve.
    
    Returns
    -------
    BaseCorrelationCurve
        Base correlation curve.
    """
    
    @property
    def has_issuer_curves(self) -> bool: ...
    """Check if issuer curves are present.
    
    Returns
    -------
    bool
        True if issuer curves are available.
    """
    
    def issuer_ids(self) -> List[str]: ...
    """Get issuer identifiers.
    
    Returns
    -------
    List[str]
        List of issuer IDs.
    """
    
    def issuer_curve(self, issuer_id: str) -> Optional[HazardCurve]: ...
    """Get an issuer curve.
    
    Parameters
    ----------
    issuer_id : str
        Issuer identifier.
        
    Returns
    -------
    HazardCurve or None
        Issuer curve if found.
    """
    
    def __repr__(self) -> str: ...
