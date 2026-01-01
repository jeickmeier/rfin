"""Fluent API builder for scenario specifications.

This module provides a builder pattern for constructing ScenarioSpec objects
with a fluent, chainable API. This is more ergonomic than manually constructing
OperationSpec objects.

Examples:
--------
>>> from finstack.scenarios import ScenarioBuilder
>>> scenario = (
...     ScenarioBuilder("stress_test")
...     .name("Q1 2024 Stress Test")
...     .description("Rate shock + equity drawdown")
...     .shift_curve("USD.OIS", 50)  # +50bp
...     .shift_equities(-10)  # -10%
...     .roll_forward("1m")
...     .build()
... )
"""

from finstack import Currency  # type: ignore
from finstack.scenarios import (  # type: ignore
    CurveKind,
    OperationSpec,
    ScenarioSpec,
    VolSurfaceKind,
)


class ScenarioBuilder:
    """Fluent builder for ScenarioSpec.

    This class provides a chainable API for constructing scenarios without
    manually creating OperationSpec objects.

    Parameters
    ----------
    scenario_id : str
        Unique scenario identifier.

    Examples:
    --------
    >>> builder = ScenarioBuilder("stress_test")
    >>> scenario = builder.name("Q1 Stress").shift_curve("USD.OIS", 50).shift_equities(-10).build()
    """

    def __init__(self, scenario_id: str) -> None:
        """Initialize a scenario builder.

        Parameters
        ----------
        scenario_id : str
            Unique identifier for the scenario.
        """
        self._id = scenario_id
        self._name: str | None = None
        self._description: str | None = None
        self._operations: list[OperationSpec] = []
        self._priority: int = 0

    def name(self, name: str) -> "ScenarioBuilder":
        """Set scenario name.

        Parameters
        ----------
        name : str
            Display name.

        Returns:
        -------
        ScenarioBuilder
            Self for chaining.
        """
        self._name = name
        return self

    def description(self, description: str) -> "ScenarioBuilder":
        """Set scenario description.

        Parameters
        ----------
        description : str
            Description text.

        Returns:
        -------
        ScenarioBuilder
            Self for chaining.
        """
        self._description = description
        return self

    def priority(self, priority: int) -> "ScenarioBuilder":
        """Set scenario priority for composition.

        Lower values have higher priority (execute first).

        Parameters
        ----------
        priority : int
            Priority value.

        Returns:
        -------
        ScenarioBuilder
            Self for chaining.
        """
        self._priority = priority
        return self

    # --- Curve Operations ---

    def shift_curve(
        self,
        curve_id: str,
        bp: float,
        curve_kind: CurveKind | None = None,
    ) -> "ScenarioBuilder":
        """Add parallel curve shift.

        Parameters
        ----------
        curve_id : str
            Curve identifier.
        bp : float
            Basis points to shift (positive = increase rates).
        curve_kind : CurveKind, optional
            Type of curve (default: Discount).

        Returns:
        -------
        ScenarioBuilder
            Self for chaining.

        Examples:
        --------
        >>> builder.shift_curve("USD.OIS", 50)  # +50bp discount curve
        >>> builder.shift_curve("USD.SOFR", -25, CurveKind.Forward)  # -25bp forward
        """
        kind = curve_kind or CurveKind.Discount
        op = OperationSpec.curve_parallel_bp(kind, curve_id, bp)
        self._operations.append(op)
        return self

    def shift_discount_curve(self, curve_id: str, bp: float) -> "ScenarioBuilder":
        """Add discount curve parallel shift.

        Parameters
        ----------
        curve_id : str
            Curve identifier.
        bp : float
            Basis points to shift.

        Returns:
        -------
        ScenarioBuilder
            Self for chaining.
        """
        return self.shift_curve(curve_id, bp, CurveKind.Discount)

    def shift_forward_curve(self, curve_id: str, bp: float) -> "ScenarioBuilder":
        """Add forward curve parallel shift.

        Parameters
        ----------
        curve_id : str
            Curve identifier.
        bp : float
            Basis points to shift.

        Returns:
        -------
        ScenarioBuilder
            Self for chaining.
        """
        return self.shift_curve(curve_id, bp, CurveKind.Forward)

    def shift_hazard_curve(self, curve_id: str, bp: float) -> "ScenarioBuilder":
        """Add hazard (credit) curve parallel shift.

        Parameters
        ----------
        curve_id : str
            Curve identifier.
        bp : float
            Basis points to shift.

        Returns:
        -------
        ScenarioBuilder
            Self for chaining.
        """
        return self.shift_curve(curve_id, bp, CurveKind.Hazard)

    def shift_inflation_curve(self, curve_id: str, bp: float) -> "ScenarioBuilder":
        """Add inflation curve parallel shift.

        Parameters
        ----------
        curve_id : str
            Curve identifier.
        bp : float
            Basis points to shift.

        Returns:
        -------
        ScenarioBuilder
            Self for chaining.
        """
        return self.shift_curve(curve_id, bp, CurveKind.Inflation)

    # --- Equity Operations ---

    def shift_equities(self, pct: float, ids: list[str] | None = None) -> "ScenarioBuilder":
        """Add equity price percent shock.

        Parameters
        ----------
        pct : float
            Percentage change (positive = increase prices).
        ids : list[str], optional
            Specific equity IDs to shock. If None, shocks all equities.

        Returns:
        -------
        ScenarioBuilder
            Self for chaining.

        Examples:
        --------
        >>> builder.shift_equities(-10)  # -10% all equities
        >>> builder.shift_equities(5, ["SPY", "QQQ"])  # +5% specific equities
        """
        op = OperationSpec.equity_price_pct(ids or [], pct)
        self._operations.append(op)
        return self

    # --- FX Operations ---

    def shift_fx(self, base: str, quote: str, pct: float) -> "ScenarioBuilder":
        """Add FX rate percent shock.

        Parameters
        ----------
        base : str
            Base currency code (e.g., "USD").
        quote : str
            Quote currency code (e.g., "EUR").
        pct : float
            Percentage change (positive = base strengthens).

        Returns:
        -------
        ScenarioBuilder
            Self for chaining.

        Examples:
        --------
        >>> builder.shift_fx("USD", "EUR", 5)  # USD strengthens 5% vs EUR
        """
        base_ccy = Currency.from_code(base)
        quote_ccy = Currency.from_code(quote)
        op = OperationSpec.market_fx_pct(base_ccy, quote_ccy, pct)
        self._operations.append(op)
        return self

    # --- Volatility Operations ---

    def shift_vol_surface(
        self,
        surface_id: str,
        pct: float,
        surface_kind: VolSurfaceKind | None = None,
    ) -> "ScenarioBuilder":
        """Add volatility surface parallel shift.

        Parameters
        ----------
        surface_id : str
            Surface identifier.
        pct : float
            Percentage change in volatility.
        surface_kind : VolSurfaceKind, optional
            Type of surface (default: Equity).

        Returns:
        -------
        ScenarioBuilder
            Self for chaining.

        Examples:
        --------
        >>> builder.shift_vol_surface("SPX_VOL", 10)  # +10% equity vol
        """
        kind = surface_kind or VolSurfaceKind.Equity
        op = OperationSpec.vol_surface_parallel_pct(kind, surface_id, pct)
        self._operations.append(op)
        return self

    # --- Time Operations ---

    def roll_forward(self, period: str) -> "ScenarioBuilder":
        """Add time roll-forward operation.

        Parameters
        ----------
        period : str
            Period to roll forward (e.g., "1d", "1w", "1m", "3m", "1y").

        Returns:
        -------
        ScenarioBuilder
            Self for chaining.

        Examples:
        --------
        >>> builder.roll_forward("1m")  # Roll forward 1 month
        >>> builder.roll_forward("3m")  # Roll forward 3 months
        """
        op = OperationSpec.time_roll_forward(period)
        self._operations.append(op)
        return self

    # --- Statement Operations ---

    def adjust_forecast(
        self,
        node_id: str,
        pct: float,
        period_id: str | None = None,
    ) -> "ScenarioBuilder":
        """Add statement forecast percent adjustment.

        Parameters
        ----------
        node_id : str
            Statement node identifier.
        pct : float
            Percentage change.
        period_id : str, optional
            Specific period to adjust. If None, adjusts all periods.

        Returns:
        -------
        ScenarioBuilder
            Self for chaining.

        Examples:
        --------
        >>> builder.adjust_forecast("revenue", 10)  # +10% revenue all periods
        """
        op = OperationSpec.stmt_forecast_percent(node_id, period_id, pct)
        self._operations.append(op)
        return self

    def set_forecast(
        self,
        node_id: str,
        value: float,
        period_id: str | None = None,
    ) -> "ScenarioBuilder":
        """Add statement forecast assignment.

        Parameters
        ----------
        node_id : str
            Statement node identifier.
        value : float
            New forecast value.
        period_id : str, optional
            Specific period to set. If None, sets all periods.

        Returns:
        -------
        ScenarioBuilder
            Self for chaining.

        Examples:
        --------
        >>> builder.set_forecast("revenue", 1000000)  # Set revenue to 1M
        """
        op = OperationSpec.stmt_forecast_assign(node_id, period_id, value)
        self._operations.append(op)
        return self

    # --- Build ---

    def build(self) -> ScenarioSpec:
        """Build the ScenarioSpec.

        Returns:
        -------
        ScenarioSpec
            Constructed scenario specification.

        Examples:
        --------
        >>> scenario = builder.build()
        """
        return ScenarioSpec(
            self._id,
            self._operations,
            name=self._name,
            description=self._description,
            priority=self._priority,
        )


def scenario(scenario_id: str) -> ScenarioBuilder:
    """Create a new ScenarioBuilder.

    This is a convenience function for creating builders.

    Parameters
    ----------
    scenario_id : str
        Scenario identifier.

    Returns:
    -------
    ScenarioBuilder
        New builder instance.

    Examples:
    --------
    >>> from finstack.scenarios import scenario
    >>> spec = scenario("stress_test").shift_curve("USD.OIS", 50).build()
    """
    return ScenarioBuilder(scenario_id)
