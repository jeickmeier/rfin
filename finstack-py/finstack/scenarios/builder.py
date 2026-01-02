"""Fluent builder for scenario specifications.

This module provides a convenient builder pattern for constructing
ScenarioSpec objects with method chaining.

Examples:
--------
    >>> from finstack.scenarios.builder import ScenarioBuilder, scenario
    >>> scenario_spec = (
    ...     ScenarioBuilder("stress_test")
    ...     .name("Q1 2024 Stress Test")
    ...     .description("Rate shock + equity drawdown")
    ...     .shift_curve("USD.OIS", 50)
    ...     .shift_equities(-10)
    ...     .roll_forward("1m")
    ...     .build()
    ... )
"""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    # Use public import paths for best IDE support.
    from finstack import Currency
    from finstack.scenarios import CurveKind, OperationSpec, ScenarioSpec, VolSurfaceKind
else:
    # Runtime: these imports are expected to succeed when the package is installed.
    from finstack import Currency
    from finstack.scenarios import CurveKind, OperationSpec, ScenarioSpec, VolSurfaceKind


class ScenarioBuilder:
    """Fluent builder for scenario specifications.

    Provides a chainable API for constructing ScenarioSpec objects
    with various market shocks and operations.

    Parameters
    ----------
    scenario_id : str
        Unique identifier for the scenario.

    Examples:
    --------
        >>> builder = ScenarioBuilder("stress_test")
        >>> scenario = builder.name("Q1 Stress").shift_curve("USD.OIS", 50).shift_equities(-10).build()
    """

    def __init__(self, scenario_id: str) -> None:
        """Create a new scenario builder.

        Parameters
        ----------
        scenario_id : str
            Unique identifier for the scenario.
        """
        self._id = scenario_id
        self._name: str | None = None
        self._description: str | None = None
        self._priority: int = 0
        self._operations: list[OperationSpec] = []

    def name(self, name: str) -> ScenarioBuilder:
        """Set the scenario name.

        Parameters
        ----------
        name : str
            Human-readable name.

        Returns:
        -------
        ScenarioBuilder
            Self for method chaining.
        """
        self._name = name
        return self

    def description(self, description: str) -> ScenarioBuilder:
        """Set the scenario description.

        Parameters
        ----------
        description : str
            Detailed description.

        Returns:
        -------
        ScenarioBuilder
            Self for method chaining.
        """
        self._description = description
        return self

    def priority(self, priority: int) -> ScenarioBuilder:
        """Set the scenario priority.

        Parameters
        ----------
        priority : int
            Execution priority (lower = higher priority).

        Returns:
        -------
        ScenarioBuilder
            Self for method chaining.
        """
        self._priority = priority
        return self

    # Curve operations
    def shift_curve(self, curve_id: str, bp_shift: float, kind: CurveKind | None = None) -> ScenarioBuilder:
        """Add a parallel curve shift.

        Parameters
        ----------
        curve_id : str
            Curve identifier (e.g., "USD.OIS").
        bp_shift : float
            Shift in basis points.
        kind : CurveKind, optional
            Curve type (defaults to Discount).

        Returns:
        -------
        ScenarioBuilder
            Self for method chaining.
        """
        curve_kind = kind if kind is not None else CurveKind.Discount
        self._operations.append(OperationSpec.curve_parallel_bp(curve_kind, curve_id, bp_shift))
        return self

    def shift_discount_curve(self, curve_id: str, bp_shift: float) -> ScenarioBuilder:
        """Add a discount curve parallel shift.

        Parameters
        ----------
        curve_id : str
            Curve identifier.
        bp_shift : float
            Shift in basis points.

        Returns:
        -------
        ScenarioBuilder
            Self for method chaining.
        """
        return self.shift_curve(curve_id, bp_shift, CurveKind.Discount)

    def shift_forward_curve(self, curve_id: str, bp_shift: float) -> ScenarioBuilder:
        """Add a forward curve parallel shift.

        Parameters
        ----------
        curve_id : str
            Curve identifier.
        bp_shift : float
            Shift in basis points.

        Returns:
        -------
        ScenarioBuilder
            Self for method chaining.
        """
        return self.shift_curve(curve_id, bp_shift, CurveKind.Forecast)

    def shift_hazard_curve(self, curve_id: str, bp_shift: float) -> ScenarioBuilder:
        """Add a hazard/credit curve parallel shift.

        Parameters
        ----------
        curve_id : str
            Curve identifier.
        bp_shift : float
            Shift in basis points.

        Returns:
        -------
        ScenarioBuilder
            Self for method chaining.
        """
        return self.shift_curve(curve_id, bp_shift, CurveKind.ParCDS)

    def shift_inflation_curve(self, curve_id: str, bp_shift: float) -> ScenarioBuilder:
        """Add an inflation curve parallel shift.

        Parameters
        ----------
        curve_id : str
            Curve identifier.
        bp_shift : float
            Shift in basis points.

        Returns:
        -------
        ScenarioBuilder
            Self for method chaining.
        """
        return self.shift_curve(curve_id, bp_shift, CurveKind.Inflation)

    # Equity operations
    def shift_equities(self, pct_shift: float, symbols: list[str] | None = None) -> ScenarioBuilder:
        """Add an equity price shift.

        Parameters
        ----------
        pct_shift : float
            Shift as percentage (e.g., -10 for -10%).
        symbols : list of str, optional
            Specific symbols to shift. If None, shifts all equities (use ["*"]).

        Returns:
        -------
        ScenarioBuilder
            Self for method chaining.
        """
        ids = symbols if symbols else ["*"]  # "*" means all equities
        self._operations.append(OperationSpec.equity_price_pct(ids, pct_shift))
        return self

    # FX operations
    def shift_fx(self, base_ccy: str, quote_ccy: str, pct_shift: float) -> ScenarioBuilder:
        """Add an FX rate shift.

        Parameters
        ----------
        base_ccy : str
            Base currency.
        quote_ccy : str
            Quote currency.
        pct_shift : float
            Shift as percentage.

        Returns:
        -------
        ScenarioBuilder
            Self for method chaining.
        """
        self._operations.append(OperationSpec.market_fx_pct(Currency(base_ccy), Currency(quote_ccy), pct_shift))
        return self

    # Volatility operations
    def shift_vol_surface(
        self,
        surface_id: str,
        pct_shift: float,
        kind: VolSurfaceKind | None = None,
    ) -> ScenarioBuilder:
        """Add a volatility surface shift.

        Parameters
        ----------
        surface_id : str
            Surface identifier.
        pct_shift : float
            Shift as percentage.
        kind : VolSurfaceKind, optional
            Surface type (defaults to Equity).

        Returns:
        -------
        ScenarioBuilder
            Self for method chaining.
        """
        surface_kind = kind if kind is not None else VolSurfaceKind.Equity
        self._operations.append(OperationSpec.vol_surface_parallel_pct(surface_kind, surface_id, pct_shift))
        return self

    # Time operations
    def roll_forward(self, tenor: str) -> ScenarioBuilder:
        """Add a time roll-forward operation.

        Parameters
        ----------
        tenor : str
            Tenor string (e.g., "1d", "1w", "1m", "3m", "1y").

        Returns:
        -------
        ScenarioBuilder
            Self for method chaining.
        """
        self._operations.append(OperationSpec.time_roll_forward(tenor, True, None))
        return self

    # Statement operations
    def adjust_forecast(self, metric: str, pct_adjust: float, period: str | None = None) -> ScenarioBuilder:
        """Add a statement forecast adjustment.

        Parameters
        ----------
        metric : str
            Metric name to adjust.
        pct_adjust : float
            Adjustment as percentage.
        period : str, optional
            Specific period (e.g., "2024Q1"). If None, adjusts all periods.

        Returns:
        -------
        ScenarioBuilder
            Self for method chaining.
        """
        _ = period
        # Period-level targeting not currently supported in the bindings; apply globally
        self._operations.append(OperationSpec.stmt_forecast_percent(metric, pct_adjust))
        return self

    def set_forecast(self, metric: str, value: float, period: str | None = None) -> ScenarioBuilder:
        """Set a statement forecast value.

        Parameters
        ----------
        metric : str
            Metric name to set.
        value : float
            New value.
        period : str, optional
            Specific period. If None, sets all periods.

        Returns:
        -------
        ScenarioBuilder
            Self for method chaining.
        """
        _ = period
        # Period-level targeting not currently supported in the bindings; apply globally
        self._operations.append(OperationSpec.stmt_forecast_assign(metric, value))
        return self

    def build(self) -> ScenarioSpec:
        """Build the scenario specification.

        Returns:
        -------
        ScenarioSpec
            The completed scenario specification.
        """
        return ScenarioSpec(
            self._id,
            self._operations,
            name=self._name,
            description=self._description,
            priority=self._priority,
        )


def scenario(scenario_id: str) -> ScenarioBuilder:
    """Create a new scenario builder.

    Convenience function that creates a ScenarioBuilder instance.

    Parameters
    ----------
    scenario_id : str
        Unique identifier for the scenario.

    Returns:
    -------
    ScenarioBuilder
        A new builder instance.

    Examples:
    --------
        >>> spec = scenario("stress").shift_curve("USD.OIS", 50).build()
    """
    return ScenarioBuilder(scenario_id)


__all__ = ["ScenarioBuilder", "scenario"]
