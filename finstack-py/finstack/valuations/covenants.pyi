"""Covenant forward-projection bindings: spec types, config, and forecast functions.

This module provides covenant specification, forecasting, and breach detection
for financial covenants such as debt-to-EBITDA limits, interest coverage
minimums, and custom threshold tests.

Typical workflow::

    from finstack.valuations.covenants import (
        CovenantType,
        Covenant,
        CovenantSpec,
        CovenantScope,
        CovenantForecastConfig,
        forecast_covenant,
        forecast_breaches,
    )

    ctype = CovenantType.max_debt_to_ebitda(4.0)
    covenant = Covenant(ctype).with_scope(CovenantScope.maintenance())
    spec = CovenantSpec.with_metric(covenant, "debt_to_ebitda")
    forecast = forecast_covenant(spec, model, base_case, periods)
"""

from __future__ import annotations

import datetime
from typing import Any

import polars

from finstack.core.dates.periods import PeriodId
from finstack.statements.types.model import FinancialModelSpec
from finstack.statements.evaluator.evaluator import StatementResult

# ---------------------------------------------------------------------------
# CovenantType
# ---------------------------------------------------------------------------

class CovenantType:
    """Covenant type variants for common financial covenants.

    Factory methods create typed covenant specifications with built-in
    threshold semantics for standard financial ratios.

    Examples
    --------
    >>> from finstack.valuations.covenants import CovenantType
    >>> CovenantType.max_debt_to_ebitda(4.0)
    >>> CovenantType.min_interest_coverage(2.0)
    >>> CovenantType.custom("leverage", "maximum", 5.0)
    """

    @staticmethod
    def max_debt_to_ebitda(threshold: float) -> CovenantType:
        """Maximum debt-to-EBITDA covenant.

        Parameters
        ----------
        threshold : float
            Maximum allowed ratio.

        Returns
        -------
        CovenantType
            Covenant type instance.
        """
        ...

    @staticmethod
    def min_interest_coverage(threshold: float) -> CovenantType:
        """Minimum interest coverage ratio covenant.

        Parameters
        ----------
        threshold : float
            Minimum required ratio.

        Returns
        -------
        CovenantType
            Covenant type instance.
        """
        ...

    @staticmethod
    def min_fixed_charge_coverage(threshold: float) -> CovenantType:
        """Minimum fixed charge coverage ratio covenant.

        Parameters
        ----------
        threshold : float
            Minimum required ratio.

        Returns
        -------
        CovenantType
            Covenant type instance.
        """
        ...

    @staticmethod
    def max_total_leverage(threshold: float) -> CovenantType:
        """Maximum total leverage covenant.

        Parameters
        ----------
        threshold : float
            Maximum allowed leverage ratio.

        Returns
        -------
        CovenantType
            Covenant type instance.
        """
        ...

    @staticmethod
    def max_senior_leverage(threshold: float) -> CovenantType:
        """Maximum senior leverage covenant.

        Parameters
        ----------
        threshold : float
            Maximum allowed senior leverage ratio.

        Returns
        -------
        CovenantType
            Covenant type instance.
        """
        ...

    @staticmethod
    def basket(metric: str, limit: float) -> CovenantType:
        """Basket covenant with a named metric and limit.

        Parameters
        ----------
        metric : str
            Metric name for the basket.
        limit : float
            Maximum allowed value.

        Returns
        -------
        CovenantType
            Covenant type instance.
        """
        ...

    @staticmethod
    def custom(metric: str, comparator: str, threshold: float) -> CovenantType:
        """Custom covenant with user-defined metric and comparator.

        Parameters
        ----------
        metric : str
            Metric name.
        comparator : str
            Comparison operator -- ``"maximum"`` / ``"le"`` / ``"lte"`` / ``"<="``
            for upper bounds, or ``"minimum"`` / ``"ge"`` / ``"gte"`` / ``">="``
            for lower bounds.
        threshold : float
            Threshold value.

        Returns
        -------
        CovenantType
            Covenant type instance.

        Raises
        ------
        ValueError
            If *comparator* is not recognised.
        """
        ...

# ---------------------------------------------------------------------------
# Covenant
# ---------------------------------------------------------------------------

class Covenant:
    """Covenant definition combining a type with optional scope and springing condition.

    Parameters
    ----------
    ctype : CovenantType
        Covenant type with threshold.

    Examples
    --------
    >>> from finstack.valuations.covenants import Covenant, CovenantType, CovenantScope
    >>> cov = Covenant(CovenantType.max_debt_to_ebitda(4.0))
    >>> cov = cov.with_scope(CovenantScope.maintenance())
    """

    def __init__(self, ctype: CovenantType) -> None:
        """Create a new covenant.

        Parameters
        ----------
        ctype : CovenantType
            Covenant type specification.
        """
        ...

    def with_scope(self, scope: CovenantScope) -> Covenant:
        """Attach a scope (maintenance or incurrence) to the covenant.

        Parameters
        ----------
        scope : CovenantScope
            Covenant scope.

        Returns
        -------
        Covenant
            New covenant with the scope set.
        """
        ...

    def with_springing_condition(self, condition: SpringingCondition) -> Covenant:
        """Attach a springing condition that activates the covenant conditionally.

        Parameters
        ----------
        condition : SpringingCondition
            Condition under which the covenant becomes active.

        Returns
        -------
        Covenant
            New covenant with the springing condition set.
        """
        ...

# ---------------------------------------------------------------------------
# CovenantSpec
# ---------------------------------------------------------------------------

class CovenantSpec:
    """Covenant specification binding a covenant to a model metric identifier.

    Examples
    --------
    >>> from finstack.valuations.covenants import CovenantSpec, Covenant, CovenantType
    >>> cov = Covenant(CovenantType.max_debt_to_ebitda(4.0))
    >>> spec = CovenantSpec.with_metric(cov, "debt_to_ebitda")
    """

    @staticmethod
    def with_metric(covenant: Covenant, metric_id: str) -> CovenantSpec:
        """Bind a covenant to a financial model metric identifier.

        Parameters
        ----------
        covenant : Covenant
            Covenant definition.
        metric_id : str
            Metric identifier in the financial model.

        Returns
        -------
        CovenantSpec
            Bound covenant specification.
        """
        ...

# ---------------------------------------------------------------------------
# CovenantScope
# ---------------------------------------------------------------------------

class CovenantScope:
    """Scope of a covenant -- maintenance (ongoing) or incurrence (event-driven).

    Examples
    --------
    >>> from finstack.valuations.covenants import CovenantScope
    >>> CovenantScope.maintenance()
    >>> CovenantScope.incurrence()
    """

    @staticmethod
    def maintenance() -> CovenantScope:
        """Maintenance covenant tested on a periodic schedule.

        Returns
        -------
        CovenantScope
            Maintenance scope.
        """
        ...

    @staticmethod
    def incurrence() -> CovenantScope:
        """Incurrence covenant tested upon specific events.

        Returns
        -------
        CovenantScope
            Incurrence scope.
        """
        ...

    def __repr__(self) -> str: ...

# ---------------------------------------------------------------------------
# SpringingCondition
# ---------------------------------------------------------------------------

class SpringingCondition:
    """Condition that activates a springing covenant.

    A springing covenant only becomes active when a separate metric
    breaches a threshold (e.g., revolver utilisation above 30 %).

    Parameters
    ----------
    metric_id : str
        Metric identifier to monitor.
    comparator : str
        ``"maximum"`` / ``"le"`` / ``"<="`` for upper-bound tests,
        ``"minimum"`` / ``"ge"`` / ``">="`` for lower-bound tests.
    threshold : float
        Threshold that triggers the covenant.

    Examples
    --------
    >>> from finstack.valuations.covenants import SpringingCondition
    >>> cond = SpringingCondition("revolver_utilisation", ">=", 0.30)
    """

    def __init__(self, metric_id: str, comparator: str, threshold: float) -> None:
        """Create a springing condition.

        Parameters
        ----------
        metric_id : str
            Metric identifier.
        comparator : str
            Comparison operator.
        threshold : float
            Activation threshold.

        Raises
        ------
        ValueError
            If *comparator* is not recognised.
        """
        ...

    def __repr__(self) -> str: ...

# ---------------------------------------------------------------------------
# CovenantForecastConfig
# ---------------------------------------------------------------------------

class CovenantForecastConfig:
    """Configuration for covenant forecasting, including optional stochastic simulation.

    Parameters
    ----------
    stochastic : bool, optional
        Enable stochastic simulation (default ``False``).
    num_paths : int, optional
        Number of Monte Carlo paths (default ``0``).
    volatility : float or None, optional
        Volatility for stochastic process.
    seed : int or None, optional
        Random seed for reproducibility.
    antithetic : bool, optional
        Enable antithetic variates (default ``False``).

    Examples
    --------
    >>> from finstack.valuations.covenants import CovenantForecastConfig
    >>> cfg = CovenantForecastConfig(stochastic=True, num_paths=1000, volatility=0.15)
    """

    def __init__(
        self,
        stochastic: bool | None = None,
        num_paths: int | None = None,
        volatility: float | None = None,
        seed: int | None = None,
        antithetic: bool | None = None,
    ) -> None: ...

# ---------------------------------------------------------------------------
# CovenantForecast
# ---------------------------------------------------------------------------

class CovenantForecast:
    """Forecast result for a single covenant across multiple test dates.

    Provides projected values, thresholds, headroom, and breach probabilities
    for each test date.

    Examples
    --------
    >>> forecast = forecast_covenant(spec, model, base_case, periods)
    >>> forecast.covenant_id
    'max_debt_to_ebitda'
    >>> forecast.projected_values
    [3.2, 3.5, 3.8]
    >>> forecast.explain()
    """

    @property
    def covenant_id(self) -> str:
        """Covenant identifier.

        Returns
        -------
        str
            Identifier of the forecasted covenant.
        """
        ...

    @property
    def test_dates(self) -> list[datetime.date]:
        """Dates at which the covenant is tested.

        Returns
        -------
        list[datetime.date]
            Ordered test dates.
        """
        ...

    @property
    def projected_values(self) -> list[float]:
        """Projected metric values at each test date.

        Returns
        -------
        list[float]
            Forecasted values aligned with ``test_dates``.
        """
        ...

    @property
    def thresholds(self) -> list[float]:
        """Covenant thresholds at each test date.

        Returns
        -------
        list[float]
            Threshold values aligned with ``test_dates``.
        """
        ...

    @property
    def headroom(self) -> list[float]:
        """Headroom (distance to threshold) at each test date.

        Returns
        -------
        list[float]
            Headroom values aligned with ``test_dates``.
        """
        ...

    @property
    def breach_probability(self) -> list[float]:
        """Probability of breach at each test date (from stochastic simulation).

        Returns
        -------
        list[float]
            Breach probabilities aligned with ``test_dates`` (0.0 -- 1.0).
        """
        ...

    @property
    def first_breach_date(self) -> datetime.date | None:
        """First date where a breach is projected, or ``None``.

        Returns
        -------
        datetime.date or None
            Earliest projected breach date.
        """
        ...

    @property
    def min_headroom_date(self) -> datetime.date:
        """Date with the minimum headroom.

        Returns
        -------
        datetime.date
            Date where headroom is tightest.
        """
        ...

    @property
    def min_headroom_value(self) -> float:
        """Minimum headroom value across all test dates.

        Returns
        -------
        float
            Smallest headroom value.
        """
        ...

    def explain(self) -> str:
        """Human-readable multi-period explanation of the forecast.

        Returns
        -------
        str
            Narrative description of the covenant forecast.
        """
        ...

    def to_polars(self) -> polars.DataFrame:
        """Export the forecast to a Polars DataFrame.

        Returns
        -------
        polars.DataFrame
            DataFrame with columns ``test_date``, ``projected_value``,
            ``threshold``, ``headroom``, ``breach_prob``.

        Raises
        ------
        RuntimeError
            If DataFrame construction fails.
        """
        ...

# ---------------------------------------------------------------------------
# FutureBreach
# ---------------------------------------------------------------------------

class FutureBreach:
    """A single projected future covenant breach.

    Attributes are available as read-only properties.

    Examples
    --------
    >>> breaches = forecast_breaches(specs, model, base_case)
    >>> for b in breaches:
    ...     print(b.covenant_id, b.breach_date, b.headroom)
    """

    @property
    def covenant_id(self) -> str:
        """Covenant identifier.

        Returns
        -------
        str
            Which covenant is breached.
        """
        ...

    @property
    def breach_date(self) -> datetime.date:
        """Date of the projected breach.

        Returns
        -------
        datetime.date
            When the breach is expected.
        """
        ...

    @property
    def projected_value(self) -> float:
        """Projected metric value at the breach date.

        Returns
        -------
        float
            Metric value at breach.
        """
        ...

    @property
    def threshold(self) -> float:
        """Covenant threshold at the breach date.

        Returns
        -------
        float
            Threshold value.
        """
        ...

    @property
    def headroom(self) -> float:
        """Headroom at the breach date (negative indicates breach).

        Returns
        -------
        float
            Distance to threshold.
        """
        ...

    @property
    def breach_probability(self) -> float:
        """Probability of breach (from stochastic simulation).

        Returns
        -------
        float
            Breach probability (0.0 -- 1.0).
        """
        ...

    def __repr__(self) -> str: ...

__all__ = [
    "CovenantType",
    "Covenant",
    "CovenantSpec",
    "CovenantScope",
    "SpringingCondition",
    "CovenantForecastConfig",
    "CovenantForecast",
    "FutureBreach",
]
